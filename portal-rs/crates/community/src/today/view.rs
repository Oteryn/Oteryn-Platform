use super::{
    Config, Error,
    schema::{self, Daily},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Boost {
    pub creature: String,
    pub boss: String,
    pub from: i64,
    pub until: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Raid {
    pub name: String,
    pub started_at: i64,
    #[serde(serialize_with = "slot")]
    pub scheduled_at: Option<i64>,
    pub state: String,
}
fn slot<S: serde::Serializer>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(seconds) => serializer.serialize_i64(*seconds),
        None => serializer.serialize_str("unavailable"),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct DailyView {
    #[serde(skip)]
    complete_through: i64,
    #[serde(skip)]
    watermark_observed: i64,
    #[serde(skip)]
    observed: i64,
    pub world: String,
    pub current: Boost,
    pub history_complete_from: i64,
    pub history: Vec<Boost>,
    pub raids: Vec<Raid>,
}
impl DailyView {
    pub(super) fn eligible(&self, now: i64) -> bool {
        now.saturating_sub(self.complete_through).saturating_add(1) <= 30
            && self.complete_through <= now.saturating_add(1)
            && self.watermark_observed <= now.saturating_add(1)
            && self.observed <= now.saturating_add(1)
            && now.saturating_sub(1) >= self.current.from
            && now.saturating_add(1) < self.current.until
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Content {
    world_id: String,
    daily: Vec<Daily>,
}

/// One repeatable-read transaction fences the snapshot, watermark and current
/// catalogue visibility/names. No network provider participates in rendering.
async fn read_inner(
    db: &PgPool,
    config: &Config,
    world: Uuid,
    lang: &str,
    now: i64,
    serving_clock: impl Fn() -> i64,
) -> Result<DailyView, Error> {
    if !matches!(lang, "pl" | "en") {
        return Err(Error::BadInput);
    }
    if !config.public_enabled || !config.valid() || !config.worlds.contains(&world) || now < 1 {
        return Err(Error::Unavailable);
    }
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query("SELECT s.epoch::text AS epoch,s.revision::text AS revision,s.content,s.observed,w.complete_through,w.observed AS watermark_observed,r.name FROM today_snapshots s JOIN today_watermarks w ON w.authority=s.authority AND w.epoch=s.epoch JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch JOIN world_status r ON r.id=s.world_id WHERE s.authority=$1 AND s.world_id=$2 AND NOT s.conflicted AND s.content IS NOT NULL AND s.received_at >= now()-interval '30 days'")
        .bind(&config.source_authority).bind(world).fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
    let complete: i64 = row.try_get("complete_through")?;
    let observed: i64 = row.try_get("observed")?;
    let watermark_observed: i64 = row.try_get("watermark_observed")?;
    if now.saturating_sub(complete).saturating_add(1) > 30
        || complete > watermark_observed
        || watermark_observed > now.saturating_add(1)
        || observed > complete
        || observed > now.saturating_add(1)
    {
        return Err(Error::Unavailable);
    }
    // A family watermark covers every configured World. Missing baseline denies
    // the generation, rather than publishing whichever World arrived first.
    let worlds: Vec<Uuid> = config.worlds.iter().copied().collect();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM today_snapshots s JOIN world_status r ON r.id=s.world_id WHERE s.authority=$1 AND s.world_id=ANY($2) AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND s.observed <= $3 AND NOT s.conflicted AND s.content IS NOT NULL AND s.received_at >= now()-interval '30 days'")
        .bind(&config.source_authority).bind(&worlds).bind(complete).fetch_one(&mut *tx).await?;
    if count != worlds.len() as i64 {
        return Err(Error::Unavailable);
    }
    let content: String = row.try_get("content")?;
    let content: Content = serde_json::from_str(&content).map_err(|_| Error::Unavailable)?;
    if content.world_id != world.to_string() || content.daily.len() != 1 {
        return Err(Error::Unavailable);
    }
    let d = &content.daily[0];
    d.validate(observed).map_err(|_| Error::Unavailable)?;
    let (from, until) = (
        schema::seconds(&d.effective_from)?,
        schema::seconds(&d.effective_until)?,
    );
    // One second of clock uncertainty must fit entirely inside the half-open interval.
    if now - 1 < from || now.saturating_add(1) >= until {
        return Err(Error::Unavailable);
    }
    let mut creature_keys = vec![d.creature_key.as_str(), d.boss_key.as_str()];
    for h in &d.history {
        creature_keys.extend([h.creature_key.as_str(), h.boss_key.as_str()]);
    }
    let raid_keys: Vec<&str> = d.raids.iter().map(|r| r.raid_key.as_str()).collect();
    let catalogue_version: String =
        sqlx::query_scalar("SELECT version_id::text FROM world_active_version")
            .fetch_one(&mut *tx)
            .await?;
    let rows: Vec<(String,String,String)> = sqlx::query_as("SELECT e.kind,e.key,CASE WHEN $3='pl' THEN n.pl ELSE n.en END FROM world_entities e JOIN world_active_version a ON a.version_id=e.version_id JOIN world_versions r ON r.id=e.version_id AND r.status='ready' JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed JOIN world_admin_names n ON n.kind=e.kind AND n.key=e.key WHERE e.publishable AND ((e.kind='Creature' AND e.key=ANY($1)) OR (e.kind='Raid' AND e.key=ANY($2)))")
        .bind(&creature_keys).bind(&raid_keys).bind(lang).fetch_all(&mut *tx).await?;
    let names: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(kind, key, name)| ((kind, key), name))
        .collect();
    let current = boost(&names, &d.creature_key, &d.boss_key, from, until)?;
    let mut history = Vec::new();
    for h in &d.history {
        history.push(boost(
            &names,
            &h.creature_key,
            &h.boss_key,
            schema::seconds(&h.effective_from)?,
            schema::seconds(&h.effective_until)?,
        )?);
    }
    let mut raids = Vec::new();
    for r in &d.raids {
        raids.push(Raid {
            name: name(&names, "Raid", &r.raid_key)?,
            started_at: schema::seconds(&r.started_at)?,
            scheduled_at: (r.scheduled_at != "unavailable")
                .then(|| schema::seconds(&r.scheduled_at))
                .transpose()?,
            state: r.state.clone(),
        });
    }
    let result = DailyView {
        complete_through: complete,
        watermark_observed,
        observed,
        world: row.try_get("name")?,
        current,
        history_complete_from: schema::seconds(&d.history_complete_from)?,
        history,
        raids,
    };
    tx.commit().await?;
    // Leave the catalogue snapshot before validating the exact generation. Every
    // publisher takes the exclusive epoch fence, including withdrawals/conflicts.
    let mut validation = db.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *validation)
        .await?;
    sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE")
        .execute(&mut *validation)
        .await?;
    sqlx::query("SELECT 1 FROM today_snapshots WHERE authority=$1 AND world_id=ANY($2) FOR SHARE")
        .bind(&config.source_authority)
        .bind(&worlds)
        .execute(&mut *validation)
        .await?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM today_snapshots s JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=s.epoch JOIN today_watermarks w ON w.authority=s.authority AND w.epoch=s.epoch WHERE s.authority=$1 AND s.world_id=$2 AND s.epoch=$3::numeric AND s.revision=$4::numeric AND s.content=$5 AND NOT s.conflicted AND s.content IS NOT NULL AND w.complete_through >= $6 AND NOT EXISTS(SELECT 1 FROM unnest($7::uuid[]) configured(world) WHERE NOT EXISTS(SELECT 1 FROM today_snapshots b WHERE b.authority=s.authority AND b.world_id=configured.world AND b.epoch=s.epoch AND NOT b.conflicted AND b.content IS NOT NULL AND b.observed <= w.complete_through AND b.received_at >= now()-interval '30 days')))")
        .bind(&config.source_authority).bind(world).bind(row.try_get::<String,_>("epoch")?).bind(row.try_get::<String,_>("revision")?).bind(row.try_get::<String,_>("content")?).bind(complete).bind(&worlds).fetch_one(&mut *validation).await?;
    if !valid {
        return Err(Error::Unavailable);
    }
    let current_rows: Vec<(String,String,String)> = sqlx::query_as("SELECT e.kind,e.key,CASE WHEN $3='pl' THEN n.pl ELSE n.en END FROM world_entities e JOIN world_active_version a ON a.version_id=e.version_id JOIN world_versions r ON r.id=e.version_id AND r.status='ready' JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed JOIN world_admin_names n ON n.kind=e.kind AND n.key=e.key WHERE e.publishable AND ((e.kind='Creature' AND e.key=ANY($1)) OR (e.kind='Raid' AND e.key=ANY($2))) FOR SHARE OF e,a,r,v,n")
        .bind(&creature_keys).bind(&raid_keys).bind(lang).fetch_all(&mut *validation).await?;
    let current_names: BTreeMap<_, _> = current_rows
        .into_iter()
        .map(|(kind, key, name)| ((kind, key), name))
        .collect();
    let current_version: String =
        sqlx::query_scalar("SELECT version_id::text FROM world_active_version FOR SHARE")
            .fetch_one(&mut *validation)
            .await?;
    if current_names != names || current_version != catalogue_version {
        return Err(Error::Unavailable);
    }
    validation.commit().await?;
    if !result.eligible(serving_clock()) {
        return Err(Error::Unavailable);
    }
    Ok(result)
}
fn boost(
    names: &BTreeMap<(String, String), String>,
    creature: &str,
    boss: &str,
    from: i64,
    until: i64,
) -> Result<Boost, Error> {
    Ok(Boost {
        creature: name(names, "Creature", creature)?,
        boss: name(names, "Creature", boss)?,
        from,
        until,
    })
}
fn name(
    names: &BTreeMap<(String, String), String>,
    kind: &str,
    key: &str,
) -> Result<String, Error> {
    let name = names
        .get(&(kind.to_owned(), key.to_owned()))
        .ok_or(Error::Unavailable)?;
    if name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
        return Err(Error::Unavailable);
    }
    Ok(name.clone())
}

/// Read at an explicit, fixed clock cut for deterministic contract evaluation.
/// HTTP serving uses `read_current`, which resamples wall time after database waits.
pub async fn read(
    db: &PgPool,
    config: &Config,
    world: Uuid,
    lang: &str,
    now: i64,
) -> Result<DailyView, Error> {
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        read_inner(db, config, world, lang, now, || now),
    )
    .await
    .map_err(|_| Error::Unavailable)?
}

/// Production reads sample the wall clock again after the last database await.
pub(super) async fn read_current(
    db: &PgPool,
    config: &Config,
    world: Uuid,
    lang: &str,
) -> Result<DailyView, Error> {
    let clock = || time::OffsetDateTime::now_utc().unix_timestamp();
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        read_inner(db, config, world, lang, clock(), clock),
    )
    .await
    .map_err(|_| Error::Unavailable)?
}
