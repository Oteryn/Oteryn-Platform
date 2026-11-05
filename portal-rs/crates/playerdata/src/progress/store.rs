use super::*;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct ProgressStore {
    pool: PgPool,
    flags: Flags,
    families: Vec<String>,
}
impl ProgressStore {
    pub fn new(pool: PgPool, flags: Flags, enabled_families: Vec<String>) -> Self {
        Self {
            pool,
            flags,
            families: enabled_families,
        }
    }
    fn enabled(&self, family: &str, ingest: bool) -> Result<()> {
        if (if ingest {
            self.flags.ingest
        } else {
            self.flags.read
        }) && self.families.iter().any(|f| f == family)
        {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    /// Called only behind the T72 authenticated transport guard. Generation changes
    /// require explicit rebootstrap; this module never silently lowers a fence.
    pub async fn ingest(&self, bytes: &[u8], authority: &impl Authority) -> Result<Vec<u8>> {
        if !self.flags.ingest {
            return Err(Error::Disabled);
        }
        let r = Record::decode(bytes)?;
        self.enabled(&r.family, true)?;
        r.authorize(authority)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO pd_progress_streams (account_id,family,authority,epoch,recovery) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind(r.account_id).bind(&r.family).bind(&r.source_authority).bind(&r.projection_epoch).bind(&r.recovery_generation).execute(&mut *tx).await?;
        let stream = sqlx::query("SELECT authority,epoch,recovery,complete_through::text AS cut FROM pd_progress_streams WHERE account_id=$1 AND family=$2 FOR UPDATE")
            .bind(r.account_id).bind(&r.family).fetch_one(&mut *tx).await?;
        if stream.try_get::<String, _>("authority")? != r.source_authority
            || stream.try_get::<String, _>("epoch")? != r.projection_epoch
            || stream.try_get::<String, _>("recovery")? != r.recovery_generation
        {
            return Err(Error::Conflict);
        }
        let previous: Option<Vec<u8>> = sqlx::query_scalar("SELECT canonical FROM pd_progress_records WHERE record_id=$1 OR (account_id=$2 AND family=$3 AND source_revision=$4::text::numeric)")
            .bind(r.record_id).bind(r.account_id).bind(&r.family).bind(&r.source_revision).fetch_optional(&mut *tx).await?;
        if let Some(old) = previous {
            if old != bytes {
                return Err(Error::Conflict);
            }
        } else {
            let cut =
                decimal(&stream.try_get::<String, _>("cut")?, false).ok_or(Error::Unavailable)?;
            if decimal(&r.source_revision, true).ok_or(Error::BadInput)? <= cut {
                return Err(Error::Conflict);
            }
            sqlx::query("INSERT INTO pd_progress_records (account_id,family,source_revision,record_id,character_id,entity_key,canonical,occurred_at) VALUES ($1,$2,$3::text::numeric,$4,$5,$6,$7,$8)")
                .bind(r.account_id).bind(&r.family).bind(&r.source_revision).bind(r.record_id).bind(r.character_id).bind(&r.entity_key).bind(bytes).bind(r.occurred_at_unix_ms.parse::<i64>().map_err(|_| Error::BadInput)?).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(format!("{{\"contract_version\":1,\"result\":\"accepted\",\"record_id\":\"{}\",\"accepted_source_revision\":\"{}\"}}",r.record_id,r.source_revision).into_bytes())
    }
    /// Gaps never advance freshness. A zero watermark is an explicit empty baseline.
    pub async fn watermark(
        &self,
        bytes: &[u8],
        authority: &impl Authority,
        now_ms: u64,
    ) -> Result<Vec<u8>> {
        if !self.flags.ingest {
            return Err(Error::Disabled);
        }
        let w: Watermark = serde_json::from_slice(bytes).map_err(|_| Error::BadInput)?;
        if bytes.len() > 1024
            || serde_json::to_vec(&w).map_err(|_| Error::BadInput)? != bytes
            || w.contract_version != 1
            || w.operation != "PublishGameJournalProgressWatermarkV1"
            || !id(&w.account_id.to_string())
            || decimal(&w.projection_epoch, true).is_none()
            || decimal(&w.recovery_generation, true).is_none()
            || !timestamp(&w.observed_at_unix_ms)
        {
            return Err(Error::BadInput);
        }
        self.enabled(&w.family, true)?;
        let through = decimal(&w.complete_through, false).ok_or(Error::BadInput)?;
        let observed = decimal(&w.observed_at_unix_ms, false).ok_or(Error::BadInput)?;
        if observed > now_ms
            || now_ms - observed > 10000
            || authority.source() != w.source_authority
            || !authority.permits(Subject {
                account: w.account_id,
                character: None,
                epoch: &w.projection_epoch,
                recovery: &w.recovery_generation,
                projection: None,
                character_revision: None,
                world: None,
            })
        {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO pd_progress_streams (account_id,family,authority,epoch,recovery) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind(w.account_id).bind(&w.family).bind(&w.source_authority).bind(&w.projection_epoch).bind(&w.recovery_generation).execute(&mut *tx).await?;
        let row = sqlx::query("SELECT authority,epoch,recovery,complete_through::text AS cut,observed_at FROM pd_progress_streams WHERE account_id=$1 AND family=$2 FOR UPDATE").bind(w.account_id).bind(&w.family).fetch_one(&mut *tx).await?;
        if row.try_get::<String, _>("authority")? != w.source_authority
            || row.try_get::<String, _>("epoch")? != w.projection_epoch
            || row.try_get::<String, _>("recovery")? != w.recovery_generation
            || decimal(&row.try_get::<String, _>("cut")?, false).is_none_or(|v| through < v)
            || observed < row.try_get::<i64, _>("observed_at")? as u64
        {
            return Err(Error::Conflict);
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_progress_records WHERE account_id=$1 AND family=$2 AND source_revision <= $3::text::numeric")
            .bind(w.account_id).bind(&w.family).bind(&w.complete_through).fetch_one(&mut *tx).await?;
        if count as u64 != through {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE pd_progress_streams SET complete_through=$3::text::numeric,baseline_complete=$4,observed_at=$5 WHERE account_id=$1 AND family=$2")
            .bind(w.account_id).bind(&w.family).bind(&w.complete_through).bind(w.baseline_complete).bind(observed as i64).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(format!(
            "{{\"contract_version\":1,\"result\":\"accepted\",\"complete_through\":\"{}\"}}",
            w.complete_through
        )
        .into_bytes())
    }
    /// Empty means proven empty, never missing evidence. A caller must distinguish
    /// Unavailable from success. All rows are reauthorized against current ownership.
    pub async fn read(
        &self,
        actor: Actor,
        scope: &ReadScope<'_>,
        authority: &impl Authority,
        now_ms: u64,
    ) -> Result<Vec<Payload>> {
        self.enabled(scope.family, false)?;
        actor.check(scope.account)?;
        let stream = sqlx::query("SELECT epoch,recovery,authority,baseline_complete,observed_at FROM pd_progress_streams WHERE account_id=$1 AND family=$2")
            .bind(scope.account).bind(scope.family).fetch_optional(&self.pool).await?.ok_or(Error::Unavailable)?;
        let observed = stream.try_get::<i64, _>("observed_at")?;
        if !stream.try_get::<bool, _>("baseline_complete")?
            || observed < 0
            || observed as u64 > now_ms
            || now_ms - observed as u64 > 86400000
            || stream.try_get::<String, _>("authority")? != authority.source()
            || !authority.permits(Subject {
                account: scope.account,
                character: scope.character,
                epoch: &stream.try_get::<String, _>("epoch")?,
                recovery: &stream.try_get::<String, _>("recovery")?,
                projection: None,
                character_revision: None,
                world: None,
            })
        {
            return Err(Error::Unavailable);
        }
        let rows: Vec<Vec<u8>> = sqlx::query_scalar("SELECT canonical FROM (SELECT DISTINCT ON (r.entity_key) r.entity_key,r.canonical FROM pd_progress_records r JOIN pd_progress_streams s USING (account_id,family) WHERE r.account_id=$1 AND r.family=$2 AND r.character_id IS NOT DISTINCT FROM $3 AND r.source_revision <= s.complete_through ORDER BY r.entity_key,r.source_revision DESC) p ORDER BY entity_key")
            .bind(scope.account).bind(scope.family).bind(scope.character).fetch_all(&self.pool).await?;
        let mut result = Vec::new();
        for bytes in rows {
            let r = Record::decode(&bytes)?;
            r.authorize(authority)?;
            if scope.family == "skill_history"
                && decimal(&r.occurred_at_unix_ms, false)
                    .is_none_or(|v| v > now_ms || now_ms - v > 90 * 86400000)
            {
                continue;
            }
            if let Some(p) = r.validate()? {
                result.push(p);
            }
        }
        Ok(result)
    }
    /// Session/CSRF validation belongs to the future browser adapter.
    pub async fn save_goal(
        &self,
        actor: Actor,
        goal: &Goal,
        authority: &impl Authority,
    ) -> Result<Uuid> {
        if !self.flags.read {
            return Err(Error::Disabled);
        }
        actor.check(goal.account)?;
        goal.validate()?;
        if !authority.permits(Subject {
            account: goal.account,
            character: Some(goal.character),
            epoch: &goal.epoch,
            recovery: &goal.recovery,
            projection: Some(&goal.projection),
            character_revision: Some(&goal.character_revision),
            world: None,
        }) {
            return Err(Error::Denied);
        }
        let goal_id = Uuid::now_v7();
        sqlx::query("INSERT INTO pd_progress_goals (id,account_id,character_id,ownership_cut,kind,entity_key,target) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(goal_id).bind(goal.account).bind(goal.character).bind(goal.cut()).bind(&goal.kind).bind(&goal.entity_key).bind(goal.target as i64).execute(&self.pool).await?;
        Ok(goal_id)
    }
    pub async fn goals(
        &self,
        actor: Actor,
        scope: &Goal,
        authority: &impl Authority,
    ) -> Result<Vec<(Uuid, String, String, i64)>> {
        if !self.flags.read {
            return Err(Error::Disabled);
        }
        actor.check(scope.account)?;
        scope.validate()?;
        if !authority.permits(Subject {
            account: scope.account,
            character: Some(scope.character),
            epoch: &scope.epoch,
            recovery: &scope.recovery,
            projection: Some(&scope.projection),
            character_revision: Some(&scope.character_revision),
            world: None,
        }) {
            return Err(Error::Denied);
        }
        Ok(sqlx::query_as("SELECT id,kind,entity_key,target FROM pd_progress_goals WHERE account_id=$1 AND character_id=$2 AND ownership_cut=$3 ORDER BY created_at,id")
            .bind(scope.account).bind(scope.character).bind(scope.cut()).fetch_all(&self.pool).await?)
    }
}
#[derive(Debug)]
pub struct ReadScope<'a> {
    pub account: Uuid,
    pub character: Option<Uuid>,
    pub family: &'a str,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: Uuid,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: String,
    pub complete_through: String,
    pub observed_at_unix_ms: String,
    pub baseline_complete: bool,
}
#[derive(Debug)]
pub struct Goal {
    pub account: Uuid,
    pub character: Uuid,
    pub epoch: String,
    pub recovery: String,
    pub projection: String,
    pub character_revision: String,
    pub kind: String,
    pub entity_key: String,
    pub target: u64,
}
impl Goal {
    fn cut(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.epoch, self.recovery, self.projection, self.character_revision
        )
    }
    fn validate(&self) -> Result<()> {
        if !id(&self.account.to_string())
            || !id(&self.character.to_string())
            || ![
                &self.epoch,
                &self.recovery,
                &self.projection,
                &self.character_revision,
            ]
            .iter()
            .all(|v| decimal(v, true).is_some())
            || self.target == 0
            || self.target > i64::MAX as u64
            || !match self.kind.as_str() {
                "level" => self.entity_key == "level" && self.target <= u32::MAX as u64,
                "skill" => skill(&self.entity_key) && self.target <= 1000,
                "bestiary" => key(&self.entity_key) && self.target <= u32::MAX as u64,
                _ => false,
            }
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }
}
impl ProgressStore {
    /// Forecast only a goal actually stored for the current owner and receipt cut.
    /// The pace adapter is deliberately separate from public statistics visibility.
    pub async fn forecast_goal(
        &self,
        actor: Actor,
        goal: &Goal,
        authority: &impl Authority,
        pace: &impl PrivatePace,
        now_ms: u64,
    ) -> Result<Option<u64>> {
        let saved = self.goals(actor, goal, authority).await?;
        if !saved.iter().any(|(_, kind, key, target)| {
            kind == &goal.kind && key == &goal.entity_key && *target as u64 == goal.target
        }) {
            return Err(Error::Denied);
        }
        let samples = pace.samples(goal).await?;
        forecast(&samples, goal.target, now_ms)
    }
}
