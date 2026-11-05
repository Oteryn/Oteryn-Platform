//! G-8 V2 statistics: private intervals and public UTC world kill totals.
//! No interval or derived owner duration is written into public daily statistics.
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

mod http;
pub(crate) mod storage;
mod wire;
pub use http::{configured_router, router};
pub use storage::{ingest, purge};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    CharacterOnlineSessions,
    WorldDailyKills,
}
impl Family {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CharacterOnlineSessions => "character_online_sessions",
            Self::WorldDailyKills => "world_daily_kills",
        }
    }
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "character_online_sessions" => Ok(Self::CharacterOnlineSessions),
            "world_daily_kills" => Ok(Self::WorldDailyKills),
            _ => Err(Error::Invalid),
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub enabled: bool,
    pub intervals_enabled: bool,
    pub world_totals_enabled: bool,
    pub owner_rankings_enabled: bool,
    pub source_authority: String,
}
impl Config {
    pub(crate) fn permits(&self, family: Family) -> bool {
        self.enabled
            && wire::authority(&self.source_authority)
            && match family {
                Family::CharacterOnlineSessions => self.intervals_enabled,
                Family::WorldDailyKills => self.world_totals_enabled,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unauthorized,
    Conflict,
    Unavailable,
}
impl Error {
    pub const fn status(self) -> StatusCode {
        match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receipt {
    Accepted,
    Superseded,
}
impl Receipt {
    pub const fn body(self) -> &'static str {
        match self {
            Self::Accepted => "{\"contract_version\":2,\"result\":\"accepted\"}",
            Self::Superseded => "{\"contract_version\":2,\"result\":\"superseded\"}",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Session {
    world_id: String,
    started_at: String,
    ended_at: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Day {
    day_end: String,
    monsters_killed: String,
    bosses_killed: String,
    complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntervalWire {
    contract_version: u8,
    operation: String,
    source_authority: String,
    character_id: String,
    statistics_id: String,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    sessions: Vec<Session>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldWire {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WatermarkWire {
    contract_version: u8,
    operation: String,
    source_authority: String,
    family: String,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}
#[derive(Debug)]
pub(crate) struct Publication {
    family: Family,
    subject: String,
    authority: String,
    epoch: String,
    revision: String,
    observed: i64,
    content: serde_json::Value,
    expires: i64,
    watermark: Option<i64>,
    interval: Option<IntervalWire>,
}
/// Serving requires a qualified retained-set rebuild, independently of heartbeat delivery.
/// The integration must validate generation coverage and the external public feed fence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub epoch: String,
    pub start: i64,
    pub through: i64,
}
/// A current trusted adapter supplies coverage on every read; a watermark alone
/// cannot establish baseline coverage after projection storage loss.
pub trait CoveragePolicy: Send + Sync {
    fn coverage(&self, family: Family) -> Result<Coverage, Error>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldTotal {
    pub monsters_killed: u64,
    pub bosses_killed: u64,
    pub complete: bool,
}
/// Anonymous public aggregate only; intervals are never consulted by this function.
pub async fn world_total(
    db: &PgPool,
    config: &Config,
    world: uuid::Uuid,
    day_start: i64,
    now: i64,
    coverage: &impl CoveragePolicy,
) -> Result<WorldTotal, Error> {
    if !config.permits(Family::WorldDailyKills) {
        return Err(Error::Unavailable);
    }
    wire::id(&world.to_string())?;
    if day_start <= 0 || day_start % 86400 != 0 {
        return Err(Error::Invalid);
    }
    let mut tx = db.begin().await?;
    let cut = storage::fresh(&mut tx, config, Family::WorldDailyKills, now, coverage).await?;
    if day_start < cut.start || day_start > cut.through {
        return Err(Error::Unavailable);
    }
    let payload: Option<serde_json::Value> = sqlx::query_scalar("SELECT payload FROM stats_v2_subjects WHERE source_authority=$1 AND family='world_daily_kills' AND subject=$2 AND epoch=$3::numeric AND expires_at>$4 AND NOT erased AND observed_at<=$5")
        .bind(&config.source_authority).bind(format!("{world}/{day_start}")).bind(&cut.epoch).bind(now).bind(cut.through).fetch_optional(&mut *tx).await?.flatten();
    let payload = payload.ok_or(Error::Unavailable)?;
    let day: Day = serde_json::from_value(
        payload
            .get("days")
            .and_then(|v| v.get(0))
            .cloned()
            .ok_or(Error::Unavailable)?,
    )
    .map_err(|_| Error::Unavailable)?;
    let result = WorldTotal {
        monsters_killed: wire::decimal(&day.monsters_killed, true)?,
        bosses_killed: wire::decimal(&day.bosses_killed, true)?,
        complete: day.complete,
    };
    tx.commit().await?;
    Ok(result)
}
pub(crate) const INTERVAL_RETENTION: i64 = 30 * 86400;
pub(crate) const WORLD_RETENTION: i64 = 400 * 86400;

/// Retention continues after presentation/ingestion are disabled.
#[derive(Debug)]
pub struct Retention {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for Retention {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            purge(&self.pool, time::OffsetDateTime::now_utc().unix_timestamp())
                .await
                .map(|_| ())
                .map_err(|_| "statistics retention failed".to_owned())
        })
    }
}
