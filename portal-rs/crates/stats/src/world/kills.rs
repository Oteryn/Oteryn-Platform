//! V2 world/day replacement snapshots. Transport authentication belongs to the
//! purpose-bound internal listener; this repository API never establishes trust.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const RETENTION: u64 = 400 * 86400;
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub public_projections_enabled: bool,
    pub ingest_enabled: bool,
    /// Reserved independently of ingestion. Q3 prevents presentation even if set.
    pub presentation_enabled: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    BadInput,
    Unauthorized,
    Conflict,
    Unavailable,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Receipt {
    Accepted,
    Superseded,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    contract_version: u8,
    operation: String,
    source_authority: String,
    world_id: String,
    day_start: String,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    days: Vec<Day>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Day {
    day_end: String,
    monsters_killed: String,
    bosses_killed: String,
    complete: bool,
}
fn number(value: &str, zero: bool) -> Result<u64, Error> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.starts_with('0') && value != "0")
    {
        return Err(Error::BadInput);
    }
    let parsed = value.parse::<u64>().map_err(|_| Error::BadInput)?;
    if !zero && parsed == 0 {
        return Err(Error::BadInput);
    }
    Ok(parsed)
}
fn authority(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
struct Parsed {
    snapshot: Snapshot,
    world: Uuid,
    start: i64,
    epoch: u64,
    revision: u64,
    observed: i64,
    digest: Vec<u8>,
    expired: bool,
}
fn parse(bytes: &[u8], source: &str, now: u64) -> Result<Parsed, Error> {
    if bytes.is_empty() || bytes.len() > 2048 || !authority(source) {
        return Err(Error::BadInput);
    }
    let s: Snapshot = serde_json::from_slice(bytes).map_err(|_| Error::BadInput)?;
    // Exact serialization rejects duplicate keys, whitespace, escapes, reordered
    // members and noncanonical numbers in addition to the typed shape checks.
    if serde_json::to_vec(&s).map_err(|_| Error::BadInput)? != bytes
        || s.contract_version != 2
        || s.operation != "PublishWorldDailyKillsV2"
        || s.days.len() > 1
    {
        return Err(Error::BadInput);
    }
    if s.source_authority != source {
        return Err(Error::Unauthorized);
    }
    let world = Uuid::parse_str(&s.world_id).map_err(|_| Error::BadInput)?;
    if world.get_version_num() != 7
        || world.get_variant() != uuid::Variant::RFC4122
        || world.to_string() != s.world_id
    {
        return Err(Error::BadInput);
    }
    let start = number(&s.day_start, false)?;
    let end = start.checked_add(86400).ok_or(Error::BadInput)?;
    let expires = end.checked_add(RETENTION).ok_or(Error::BadInput)?;
    let observed = number(&s.source_observed_at, false)?;
    if start % 86400 != 0 || start > observed || observed > now.saturating_add(1) {
        return Err(Error::BadInput);
    }
    for day in &s.days {
        if number(&day.day_end, false)? != end || (day.complete && end > observed) {
            return Err(Error::BadInput);
        }
        number(&day.monsters_killed, true)?;
        number(&day.bosses_killed, true)?;
    }
    let epoch = number(&s.projection_epoch, false)?;
    let revision = number(&s.projection_revision, false)?;
    // Value maps use sorted object keys; only subject and complete body are hashed.
    let content = serde_json::json!({"world_id":s.world_id,"day_start":s.day_start,"days":s.days});
    let digest =
        Sha256::digest(serde_json::to_vec(&content).map_err(|_| Error::BadInput)?).to_vec();
    Ok(Parsed {
        world,
        start: i64::try_from(start).map_err(|_| Error::BadInput)?,
        epoch,
        revision,
        observed: i64::try_from(observed).map_err(|_| Error::BadInput)?,
        digest,
        expired: now >= expires,
        snapshot: s,
    })
}
/// Call only after the internal transport verified the public-projection peer,
/// purpose, authority binding and rate limit. `source` is trusted configuration,
/// never a request header. The shared feed must already establish its epoch.
/// No public HTTP route is installed by this module.
pub async fn ingest(
    db: &PgPool,
    config: Config,
    source: &str,
    bytes: &[u8],
    now: u64,
) -> Result<Receipt, Error> {
    if !config.public_projections_enabled || !config.ingest_enabled {
        return Err(Error::Disabled);
    }
    let p = parse(bytes, source, now)?;
    let mut tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    // Serialize with the existing shared epoch writer and all family writes.
    let floor: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| Error::Unavailable)?;
    let floor = floor
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(Error::Unavailable)?;
    if p.epoch < floor {
        return Ok(Receipt::Superseded);
    }
    if p.epoch != floor {
        return Err(Error::Unavailable);
    }
    let old = sqlx::query("SELECT epoch::text,revision::text,content_digest,retired FROM stats_world_kills_v2 WHERE source_authority=$1 AND world_id=$2 AND day_start=$3 FOR UPDATE")
        .bind(source).bind(p.world).bind(p.start).fetch_optional(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    let mut retired = p.expired;
    if let Some(row) = old {
        let previous_epoch: String = row.try_get("epoch").map_err(|_| Error::Unavailable)?;
        let previous_revision: String = row.try_get("revision").map_err(|_| Error::Unavailable)?;
        let previous = (
            number(&previous_epoch, false).map_err(|_| Error::Unavailable)?,
            number(&previous_revision, false).map_err(|_| Error::Unavailable)?,
        );
        if (p.epoch, p.revision) < previous {
            return Ok(Receipt::Superseded);
        }
        if (p.epoch, p.revision) == previous {
            let digest: Vec<u8> = row
                .try_get("content_digest")
                .map_err(|_| Error::Unavailable)?;
            return if digest == p.digest {
                Ok(Receipt::Accepted)
            } else {
                Err(Error::Conflict)
            };
        }
        retired |= row
            .try_get::<bool, _>("retired")
            .map_err(|_| Error::Unavailable)?;
    }
    let payload = if retired || p.snapshot.days.is_empty() {
        None
    } else {
        Some(serde_json::to_value(&p.snapshot.days).map_err(|_| Error::BadInput)?)
    };
    sqlx::query("INSERT INTO stats_world_kills_v2 (source_authority,world_id,day_start,epoch,revision,content_digest,source_observed_at,payload,retired) VALUES ($1,$2,$3,$4::numeric,$5::numeric,$6,$7,$8,$9) ON CONFLICT (source_authority,world_id,day_start) DO UPDATE SET epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,content_digest=EXCLUDED.content_digest,source_observed_at=EXCLUDED.source_observed_at,payload=EXCLUDED.payload,retired=EXCLUDED.retired")
        .bind(source).bind(p.world).bind(p.start).bind(p.epoch.to_string()).bind(p.revision.to_string()).bind(p.digest).bind(p.observed).bind(payload).bind(retired)
        .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(Receipt::Accepted)
}
/// Run at least daily. Payload erasure preserves subject ordering/digest fences.
pub async fn purge(db: &PgPool, now: i64) -> Result<u64, Error> {
    if now < 0 {
        return Err(Error::BadInput);
    }
    sqlx::query("UPDATE stats_world_kills_v2 SET payload=NULL,retired=true WHERE NOT retired AND day_start::numeric + 86400 + 34560000 <= $1::numeric")
        .bind(now.to_string()).execute(db).await.map(|r| r.rows_affected()).map_err(|_| Error::Unavailable)
}
/// Q3 is unresolved: no flag value can publish the unqualified counters.
pub fn presentation(_config: Config) -> Result<(), Error> {
    Err(Error::Unavailable)
}

/// Retention continues when publication and presentation are disabled.
#[derive(Debug)]
pub struct Retention(pub PgPool);
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            purge(&self.0, time::OffsetDateTime::now_utc().unix_timestamp())
                .await
                .map(|_| ())
                .map_err(|_| "world statistics retention unavailable".to_owned())
        })
    }
}
