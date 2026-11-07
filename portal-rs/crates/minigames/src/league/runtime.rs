//! Concrete purpose-key authority and durable, replay-safe Game push-feed worker.
use super::{
    admission::{self, Meta, Watermark},
    config::{self, Config},
    *,
};
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub(super) fn now() -> Result<u64> {
    u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp()).map_err(|_| Error::Unavailable)
}
#[derive(Debug, Clone)]
pub struct GameAuthority {
    pub(super) config: Option<Arc<Config>>,
}
impl GameAuthority {
    pub fn configured() -> Self {
        Self {
            config: Config::from_env().ok().map(Arc::new),
        }
    }
    pub fn new(config: Config) -> Self {
        Self {
            config: Some(Arc::new(config)),
        }
    }
    async fn checked_fence(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        clock: u64,
        serving: bool,
    ) -> Result<Fence> {
        let c = self.config.as_ref().ok_or(Error::Unavailable)?;
        let recovery = c.recovery()?;
        let database_now: i64 =
            sqlx::query_scalar("SELECT extract(epoch FROM clock_timestamp())::bigint")
                .fetch_one(&mut **tx)
                .await?;
        schema::ensure(database_now > 0 && (database_now as u64).abs_diff(clock) <= 1)?;
        let r=sqlx::query("SELECT epoch::text,recovery_generation::text,ready,conflicted,event_complete,season_complete FROM league_feed_state WHERE authority=$1 FOR SHARE")
            .bind(&c.authority).fetch_optional(&mut **tx).await?.ok_or(Error::Unavailable)?;
        if r.try_get::<bool, _>("conflicted")?
            || r.try_get::<String, _>("epoch")? != recovery.projection_epoch
            || r.try_get::<String, _>("recovery_generation")? != recovery.recovery_generation
        {
            return Err(Error::Unavailable);
        }
        if serving && !r.try_get::<bool, _>("ready")? {
            return Err(Error::Unavailable);
        }
        let complete = r
            .try_get::<i64, _>("event_complete")?
            .min(r.try_get::<i64, _>("season_complete")?);
        let f = Fence {
            epoch: schema::number(&recovery.projection_epoch, false)?,
            recovery_generation: schema::number(&recovery.recovery_generation, false)?,
            complete_through: u64::try_from(complete).map_err(|_| Error::Unavailable)?,
        };
        if serving && !f.valid(clock) {
            return Err(Error::Unavailable);
        }
        Ok(f)
    }
}
impl Authority for GameAuthority {
    async fn verify_served(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        s: &schema::Snapshot,
        wire: &[u8],
    ) -> Result<()> {
        let c = self.config.as_ref().ok_or(Error::Unavailable)?;
        let row=sqlx::query("SELECT i.meta,i.signature,i.body,a.signature AS audit_signature,a.payload AS audit_payload FROM league_audit a JOIN league_feed_inbox i ON i.request_id=a.envelope_id WHERE a.authority=$1 AND a.family=$2 AND a.world_id=$3 AND a.subject_id=$4 AND a.epoch=$5::numeric AND a.revision=$6::numeric")
            .bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).bind(s.epoch()).bind(s.revision()).fetch_optional(&mut **tx).await?.ok_or(Error::Unavailable)?;
        let meta: Vec<u8> = row.try_get("meta")?;
        let signature: Vec<u8> = row.try_get("signature")?;
        let body: Vec<u8> = row.try_get("body")?;
        schema::ensure(body == wire)?;
        let m = admission::authenticate(
            c,
            if matches!(s, schema::Snapshot::Event(_)) {
                admission::EVENT
            } else {
                admission::SEASON
            },
            &meta,
            &body,
            &signature,
        )?;
        let proof = admission::Decision::decode(&m, s)?;
        schema::ensure(
            signature == row.try_get::<Vec<u8>, _>("audit_signature")?
                && proof.delivery(&body, &signature, s)?.payload
                    == row.try_get::<Vec<u8>, _>("audit_payload")?,
        )?;
        self.verify(tx, &proof.delivery(&body, &signature, s)?, s)
            .await
    }

    fn reconciled_recovery(&self) -> bool {
        true
    }
    async fn fence(&self, tx: &mut Transaction<'_, Postgres>, clock: u64) -> Result<Fence> {
        self.checked_fence(tx, clock, true).await
    }
    async fn admission_fence(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        clock: u64,
    ) -> Result<Fence> {
        self.checked_fence(tx, clock, false).await
    }
    async fn verify(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        d: &Delivery<'_>,
        s: &schema::Snapshot,
    ) -> Result<()> {
        let c = self.config.as_ref().ok_or(Error::Unavailable)?;
        registered_world(tx, s.world()).await?;
        let row=sqlx::query("SELECT path,body,meta,signature,peer_certificate_sha256 FROM league_feed_inbox WHERE request_id=$1 AND state <> 'refused'")
            .bind(d.envelope_id).fetch_optional(&mut **tx).await?.ok_or(Error::Unavailable)?;
        let path: String = row.try_get("path")?;
        let body: Vec<u8> = row.try_get("body")?;
        let meta: Vec<u8> = row.try_get("meta")?;
        let signature: Vec<u8> = row.try_get("signature")?;
        schema::ensure(
            row.try_get::<String, _>("peer_certificate_sha256")? == c.client_spki_sha256
                && body == d.wire
                && signature == d.signature,
        )?;
        let m = admission::authenticate(c, &path, &meta, &body, &signature)?;
        let proof = admission::Decision::decode(&m, s)?;
        let expected = proof.delivery(&body, &signature, s)?;
        schema::ensure(
            expected.envelope_id == d.envelope_id
                && expected.transaction_id == d.transaction_id
                && expected.ordinal == d.ordinal
                && expected.count == d.count
                && expected.payload == d.payload
                && expected.event_type == d.event_type
                && expected.occurred_at_unix_ms == d.occurred_at_unix_ms
                && expected.recovery_generation == d.recovery_generation
                && expected.finalized_at == d.finalized_at,
        )?;
        let inconsistent:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_receipts WHERE transaction_id=$1 AND event_count<>$2)")
            .bind(d.transaction_id).bind(d.count as i32).fetch_one(&mut **tx).await?;
        schema::ensure(!inconsistent)?;
        Ok(())
    }
}
async fn question(pool: &PgPool, s: &schema::Snapshot, c: &Config, m: &Meta) -> Result<()> {
    let schema::Snapshot::Event(event) = s else {
        return Ok(());
    };
    let Some(record) = event.event.first() else {
        return Ok(());
    };
    let mut identity = record.clone();
    identity.state.clear();
    identity.outcome.clear();
    identity.decided_at.clear();
    identity.correction_of.clear();
    identity.reason_code.clear();
    identity.evidence_sha256.clear();
    let digest = schema::hash(&serde_json::to_vec(&identity).map_err(|_| Error::Invalid)?);
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
        .execute(&mut *tx)
        .await?;
    context(&mut tx, c, m).await?;
    registered_world(&mut tx, s.world()).await?;
    let prior:Option<String>=sqlx::query_scalar("SELECT question_hash FROM league_event_questions WHERE authority=$1 AND world_id=$2 AND subject_id=$3 FOR UPDATE").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).fetch_optional(&mut *tx).await?;
    if prior.is_some_and(|h| h != digest) {
        return Err(Error::Conflict);
    }
    if nonterminal(s) {
        let terminal: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_subjects WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4)").bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).fetch_one(&mut *tx).await?;
        let phase: Option<String> = sqlx::query_scalar("SELECT last_nonterminal FROM league_event_questions WHERE authority=$1 AND world_id=$2 AND subject_id=$3").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).fetch_optional(&mut *tx).await?;
        if terminal || (phase.as_deref() == Some("LOCKED") && record.state == "SCHEDULED") {
            return Err(Error::Conflict);
        }
    }
    let prior:Option<Uuid>=sqlx::query_scalar("SELECT subject_id FROM league_occurrences WHERE authority=$1 AND world_id=$2 AND occurrence_id=$3").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(&record.occurrence_id)?).fetch_optional(&mut *tx).await?;
    if prior.is_some_and(|id| id.to_string() != s.subject()) {
        return Err(Error::Conflict);
    }
    sqlx::query("INSERT INTO league_event_questions(authority,world_id,subject_id,question_hash) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).bind(digest).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO league_occurrences(authority,world_id,occurrence_id,subject_id) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(&record.occurrence_id)?).bind(schema::id(s.subject())?).execute(&mut *tx).await?;
    if nonterminal(s) {
        sqlx::query("UPDATE league_event_questions SET last_nonterminal=$4 WHERE authority=$1 AND world_id=$2 AND subject_id=$3").bind(s.authority()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).bind(&record.state).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
fn nonterminal(s: &schema::Snapshot) -> bool {
    matches!(s,schema::Snapshot::Event(e) if e.event.first().is_some_and(|e|matches!(e.state.as_str(),"SCHEDULED"|"LOCKED")))
}
async fn context(tx: &mut Transaction<'_, Postgres>, c: &Config, m: &Meta) -> Result<()> {
    let current = c.recovery()?;
    if current.projection_epoch != m.projection_epoch
        || current.recovery_generation != m.recovery_generation
    {
        return Err(Error::Unavailable);
    }
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_feed_state WHERE authority=$1 AND epoch=$2::numeric AND recovery_generation=$3::numeric AND NOT conflicted)").bind(&c.authority).bind(&m.projection_epoch).bind(&m.recovery_generation).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Unavailable);
    }
    Ok(())
}
async fn registered_world(tx: &mut Transaction<'_, Postgres>, world: &str) -> Result<()> {
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM world_status WHERE id=$1 FOR KEY SHARE")
            .bind(schema::id(world)?)
            .fetch_optional(&mut **tx)
            .await?;
    if found.is_none() {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(super) async fn initialize(pool: &PgPool, c: &Config) -> Result<()> {
    let f = c.recovery()?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
        .execute(&mut *tx)
        .await?;
    let old:Option<(String,String)>=sqlx::query_as("SELECT epoch::text,recovery_generation::text FROM league_feed_state WHERE authority=$1 FOR UPDATE").bind(&c.authority).fetch_optional(&mut *tx).await?;
    if let Some((e, g)) = &old
        && (schema::number(e, false)? > schema::number(&f.projection_epoch, false)?
            || schema::number(g, false)? > schema::number(&f.recovery_generation, false)?)
    {
        return Err(Error::Conflict);
    }
    if old.as_ref() != Some(&(f.projection_epoch.clone(), f.recovery_generation.clone())) {
        sqlx::query("INSERT INTO league_feed_state(authority,epoch,recovery_generation) VALUES($1,$2::numeric,$3::numeric) ON CONFLICT(authority) DO UPDATE SET epoch=EXCLUDED.epoch,recovery_generation=EXCLUDED.recovery_generation,ready=false,event_complete=0,season_complete=0,manifest=NULL,manifest_meta=NULL,manifest_signature=NULL")
            .bind(&c.authority).bind(&f.projection_epoch).bind(&f.recovery_generation).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
/// Enqueues immutable authenticated bytes; 200 is returned only AFTER the worker
/// commits the projection. Initial enqueue returns retryable 503, never false ack.
pub(super) async fn receive(
    pool: &PgPool,
    c: &Config,
    path: &str,
    body: &[u8],
    meta: &[u8],
    signature: &[u8],
) -> Result<bool> {
    let m = admission::authenticate(c, path, meta, body, signature)?;
    initialize(pool, c).await?;
    let fingerprint = schema::hash(&[path.as_bytes(), meta, body].concat());
    let id = schema::id(&m.request_id)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
        .execute(&mut *tx)
        .await?;
    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT fingerprint,state FROM league_feed_requests WHERE request_id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((hash, state)) = existing {
        if hash != fingerprint {
            sqlx::query(
                "UPDATE league_feed_state SET conflicted=true,ready=false WHERE authority=$1",
            )
            .bind(&c.authority)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Err(Error::Conflict);
        }
        return match state.as_str() {
            "done" => Ok(true),
            "refused" => Err(Error::Conflict),
            "gone" => Err(Error::Unavailable),
            _ => Ok(false),
        };
    }
    if let Err(error) = validate(c, path, body, &m, now()?) {
        sqlx::query("UPDATE league_feed_state SET conflicted=true,ready=false WHERE authority=$1")
            .bind(&c.authority)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(error);
    }
    let expires = if matches!(path, admission::EVENT | admission::SEASON) {
        let snapshot = schema::Snapshot::decode(body, path == admission::SEASON, now()?)?;
        i64::try_from(snapshot.observed()?.saturating_add(90 * 86400))
            .map_err(|_| Error::Invalid)?
    } else {
        now()? as i64 + 90 * 86400
    };
    if expires <= now()? as i64 {
        return Err(Error::Unavailable);
    }
    sqlx::query(
        "INSERT INTO league_feed_requests(request_id,fingerprint,state) VALUES($1,$2,'pending')",
    )
    .bind(id)
    .bind(&fingerprint)
    .execute(&mut *tx)
    .await?;
    if path == admission::EVENT || path == admission::SEASON {
        sqlx::query("UPDATE league_feed_state SET ready=false WHERE authority=$1")
            .bind(&c.authority)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO league_feed_inbox(request_id,path,body,meta,signature,fingerprint,peer_certificate_sha256,received_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(id).bind(path).bind(body).bind(meta).bind(signature).bind(fingerprint).bind(&c.client_spki_sha256).bind(now()? as i64).bind(expires).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(false)
}
fn validate(_config: &Config, path: &str, body: &[u8], m: &Meta, clock: u64) -> Result<()> {
    if matches!(path, admission::EVENT | admission::SEASON) {
        let s = schema::Snapshot::decode(body, path == admission::SEASON, clock)?;
        schema::ensure(s.authority() == m.source_authority && s.epoch() == m.projection_epoch)?;
    } else if path == admission::WATERMARK {
        let w: Watermark = admission::json(body)?;
        let complete = schema::number(&w.complete_through, false)?;
        let observed = schema::number(&w.observed_at, false)?;
        schema::ensure(
            w.contract_version == 1
                && w.operation == "PublishLeagueWatermarkV1"
                && w.source_authority == m.source_authority
                && w.projection_epoch == m.projection_epoch
                && matches!(w.family.as_str(), "world_event_outcome" | "guild_season")
                && complete <= observed
                && observed <= clock.saturating_add(1),
        )?;
    } else {
        return Err(Error::Invalid);
    }
    Ok(())
}
#[derive(Debug, Clone)]
pub struct FeedJob {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for FeedJob {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async {
            self.run()
                .await
                .map_err(|_| "league ingestion refused".to_owned())
        })
    }
}
impl FeedJob {
    pub async fn run(&self) -> Result<()> {
        if !config::enabled() {
            return Ok(());
        }
        self.run_config(Config::from_env()?).await
    }
    pub(super) async fn run_config(&self, c: Config) -> Result<()> {
        initialize(&self.pool, &c).await?;
        if !config::owner_flags(true).ready() {
            return Err(Error::Unavailable);
        }
        // Close-on-drop makes cancellation release the session leadership lock.
        let mut leader = self.pool.acquire().await?;
        leader.close_on_drop();
        let owns: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(94000004)")
            .fetch_one(&mut *leader)
            .await?;
        if !owns {
            return Ok(());
        }
        let authority = GameAuthority::new(c.clone());
        let league = League::new(self.pool.clone(), config::owner_flags(true), authority);
        for _ in 0..c.batch_limit()? {
            let row=sqlx::query("SELECT request_id,path,body,meta,signature FROM league_feed_inbox WHERE state='pending' ORDER BY sequence LIMIT 1").fetch_optional(&self.pool).await?;
            let Some(row) = row else {
                break;
            };
            let id: Uuid = row.try_get("request_id")?;
            let path: String = row.try_get("path")?;
            let body: Vec<u8> = row.try_get("body")?;
            let meta: Vec<u8> = row.try_get("meta")?;
            let signature: Vec<u8> = row.try_get("signature")?;
            let result = self
                .apply(&c, &league, &path, &body, &meta, &signature)
                .await;
            match result {
                Ok(()) | Err(Error::Superseded) => {
                    sqlx::query("WITH updated AS (UPDATE league_feed_inbox SET state='done' WHERE request_id=$1 RETURNING request_id) UPDATE league_feed_requests SET state='done' WHERE request_id IN (SELECT request_id FROM updated)")
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                }
                Err(Error::Unavailable) | Err(Error::Database(_)) => break,
                Err(_) => {
                    let mut tx = self.pool.begin().await?;
                    sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("WITH updated AS (UPDATE league_feed_inbox SET state='refused' WHERE request_id=$1 RETURNING request_id) UPDATE league_feed_requests SET state='refused' WHERE request_id IN (SELECT request_id FROM updated)")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("UPDATE league_feed_state SET conflicted=true,ready=false WHERE authority=$1").bind(&c.authority).execute(&mut *tx).await?;
                    tx.commit().await?;
                    break;
                }
            }
        }
        self.install(&c).await?;
        Ok(())
    }
    async fn apply(
        &self,
        c: &Config,
        league: &League<GameAuthority>,
        path: &str,
        body: &[u8],
        meta: &[u8],
        signature: &[u8],
    ) -> Result<()> {
        let m = admission::authenticate(c, path, meta, body, signature)?;
        validate(c, path, body, &m, now()?)?;
        if path == admission::EVENT || path == admission::SEASON {
            let s = schema::Snapshot::decode(body, path == admission::SEASON, now()?)?;
            let withdrawn = s.record()?.is_empty();
            let previous: Option<(String, String, String)> = sqlx::query_as("SELECT epoch::text,revision::text,record_hash FROM league_feed_withdrawals WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4")
                .bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).fetch_optional(&self.pool).await?;
            if let Some((epoch, revision, hash)) = previous {
                let old_epoch = schema::number(&epoch, false)?;
                let next_epoch = schema::number(s.epoch(), false)?;
                let old_revision = schema::number(&revision, false)?;
                let next_revision = schema::number(s.revision(), false)?;
                if next_epoch < old_epoch
                    || (next_epoch == old_epoch
                        && (next_revision < old_revision
                            || next_revision > old_revision.saturating_add(1)
                            || (next_revision == old_revision
                                && hash != schema::hash(&s.record()?))))
                {
                    return Err(Error::Conflict);
                }
            }
            question(&self.pool, &s, c, &m).await?;
            if !withdrawn && !nonterminal(&s) {
                let proof = admission::Decision::decode(&m, &s)?;
                league
                    .ingest(proof.delivery(body, signature, &s)?, now()?)
                    .await?;
            }
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
                .execute(&mut *tx)
                .await?;
            context(&mut tx, c, &m).await?;
            if withdrawn {
                registered_world(&mut tx, s.world()).await?;
                let old:Option<String>=sqlx::query_scalar("SELECT revision::text FROM league_subjects WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4 FOR UPDATE").bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).fetch_optional(&mut *tx).await?;
                if old.as_ref().is_some_and(|r| {
                    schema::number(r, false)
                        .is_ok_and(|r| r >= schema::number(s.revision(), false).unwrap_or(0))
                }) {
                    return Err(Error::Conflict);
                }
                sqlx::query("UPDATE league_subjects SET revision=$5::numeric WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4")
                    .bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).bind(s.revision()).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO league_feed_withdrawals(authority,family,world_id,subject_id,epoch,revision,withdrawn,record_hash) VALUES($1,$2,$3,$4,$5::numeric,$6::numeric,$7,$8) ON CONFLICT(authority,family,world_id,subject_id) DO UPDATE SET epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,withdrawn=EXCLUDED.withdrawn,record_hash=EXCLUDED.record_hash")
                .bind(s.authority()).bind(s.family()).bind(schema::id(s.world())?).bind(schema::id(s.subject())?).bind(s.epoch()).bind(s.revision()).bind(withdrawn).bind(schema::hash(&s.record()?)).execute(&mut *tx).await?;
            tx.commit().await?;
        } else if path == admission::WATERMARK {
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
                .execute(&mut *tx)
                .await?;
            context(&mut tx, c, &m).await?;
            let w: Watermark = admission::json(body)?;
            let previous: i64 = sqlx::query_scalar(if w.family == "world_event_outcome" {
                "SELECT event_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE"
            } else {
                "SELECT season_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE"
            })
            .bind(&c.authority)
            .fetch_one(&mut *tx)
            .await?;
            if schema::number(&w.complete_through, false)? < previous as u64 {
                return Err(Error::Conflict);
            }
            let sql = if w.family == "world_event_outcome" {
                "UPDATE league_feed_state SET event_complete=GREATEST(event_complete,$2) WHERE authority=$1"
            } else {
                "UPDATE league_feed_state SET season_complete=GREATEST(season_complete,$2) WHERE authority=$1"
            };
            sqlx::query(sql)
                .bind(&c.authority)
                .bind(schema::number(&w.complete_through, false)? as i64)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        Ok(())
    }
    async fn install(&self, c: &Config) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
            .execute(&mut *tx)
            .await?;
        let r = sqlx::query("SELECT conflicted,event_complete,season_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE").bind(&c.authority).fetch_one(&mut *tx).await?;
        if r.try_get::<bool, _>("conflicted")? {
            return Err(Error::Unavailable);
        }
        // The producer ACKs every retained snapshot before advancing watermarks.
        // Old subjects absent from the current epoch keep the entire feed pending.
        let unknown: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_subjects s WHERE s.authority=$1 AND (s.finalized_at IS NULL OR s.finalized_at > floor(extract(epoch FROM clock_timestamp()))::bigint-7776000) AND NOT EXISTS(SELECT 1 FROM league_feed_withdrawals w JOIN league_feed_state f ON f.authority=w.authority WHERE w.authority=s.authority AND w.family=s.family AND w.world_id=s.world_id AND w.subject_id=s.subject_id AND w.epoch=f.epoch))").bind(&c.authority).fetch_one(&mut *tx).await?;
        let complete = r
            .try_get::<i64, _>("event_complete")?
            .min(r.try_get::<i64, _>("season_complete")?);
        let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_feed_inbox WHERE state='pending' AND path IN ('/internal/v1/game-public/world-event-outcome','/internal/v1/game-public/guild-season'))").fetch_one(&mut *tx).await?;
        let old_subject: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM league_feed_withdrawals w JOIN league_feed_state f ON f.authority=w.authority WHERE w.authority=$1 AND w.epoch<>f.epoch AND NOT EXISTS(SELECT 1 FROM league_subjects s WHERE s.authority=w.authority AND s.family=w.family AND s.world_id=w.world_id AND s.subject_id=w.subject_id AND s.finalized_at <= floor(extract(epoch FROM clock_timestamp()))::bigint-7776000))").bind(&c.authority).fetch_one(&mut *tx).await?;
        let ready = !pending
            && !unknown
            && !old_subject
            && complete > 0
            && now()?.saturating_sub(complete as u64).saturating_add(1) <= 30;
        sqlx::query("UPDATE league_feed_state SET ready=$2 WHERE authority=$1")
            .bind(&c.authority)
            .bind(ready)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Privacy cleanup is independent of source availability, quarantine and feature
/// activation. It never creates source facts or clears minimal replay controls.
#[derive(Debug, Clone)]
pub struct Retention {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async {
            self.run()
                .await
                .map_err(|_| "league retention failed".into())
        })
    }
}
impl Retention {
    pub async fn run(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
            .execute(&mut *tx)
            .await?;
        let clock: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        schema::ensure(clock > 0)?;
        sqlx::query("DELETE FROM league_audit WHERE occurred_at_unix_ms/1000 <= $1")
            .bind(clock.saturating_sub(90 * 86400))
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE league_subjects SET wire=''::bytea WHERE finalized_at <= $1 AND octet_length(wire)>0").bind(clock.saturating_sub(90*86400)).execute(&mut *tx).await?;
        sqlx::query("WITH deleted AS (DELETE FROM league_feed_inbox WHERE expires_at <= $1 RETURNING request_id) UPDATE league_feed_requests SET state=CASE WHEN state='pending' THEN 'gone' ELSE state END WHERE request_id IN (SELECT request_id FROM deleted)").bind(clock).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
