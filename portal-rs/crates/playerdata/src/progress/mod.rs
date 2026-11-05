//! Private progress projections. Transport and ownership authority belong to T72.
//! This module never mounts an unguarded ingest route or consults public statistics
//! as evidence of private ownership.
mod payload;
mod store;
pub use payload::*;
use serde::{Deserialize, Serialize};
pub use store::*;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Disabled,
    Denied,
    BadInput,
    Conflict,
    Unavailable,
    Database(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
impl Error {
    pub fn status(&self) -> u16 {
        match self {
            Self::Disabled | Self::Unavailable | Self::Database(_) => 503,
            Self::Denied => 403,
            Self::BadInput => 400,
            Self::Conflict => 409,
        }
    }
}
/// Separate flags; enabling ingestion never enables private reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub ingest: bool,
    pub read: bool,
}
/// Trusted, freshly revalidated T72 seam. Implementations must bind the dedicated
/// peer purpose, source, partition, accepted catalog, ownership transfer cut and
/// restore fences. Unknown, stale, erased or transitioning state returns false.
/// Browser input must never implement or construct this authority.
pub trait Authority {
    fn permits(&self, subject: Subject<'_>) -> bool;
    fn source(&self) -> &str;
    fn catalog_accepts(&self, family: &str, payload: &Payload) -> bool;
}
/// Scope and ownership evidence required for a single decision.
#[derive(Debug, Clone, Copy)]
pub struct Subject<'a> {
    pub account: Uuid,
    pub character: Option<Uuid>,
    pub epoch: &'a str,
    pub recovery: &'a str,
    pub projection: Option<&'a str>,
    pub character_revision: Option<&'a str>,
    pub world: Option<Uuid>,
}
/// Supplied exclusively by authenticated session middleware; staff have no bypass.
#[derive(Debug, Clone, Copy)]
pub enum Actor {
    Owner(Uuid),
    Staff,
    Anonymous,
}
impl Actor {
    fn check(self, account: Uuid) -> Result<()> {
        if matches!(self, Self::Owner(id) if id == account) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
pub(crate) fn decimal(v: &str, positive: bool) -> Option<u64> {
    let n = v.parse::<u64>().ok()?;
    (n.to_string() == v && (!positive || n > 0)).then_some(n)
}
pub(crate) fn timestamp(v: &str) -> bool {
    decimal(v, false).is_some_and(|n| n <= 253402300799999)
}
pub(crate) fn id(v: &str) -> bool {
    Uuid::parse_str(v).is_ok_and(|u| {
        u.get_version_num() == 7 && u.get_variant() == uuid::Variant::RFC4122 && u.to_string() == v
    })
}
pub(crate) fn key(v: &str) -> bool {
    (1..=512).contains(&v.len())
        && v.as_bytes()[0].is_ascii_alphanumeric()
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(crate) fn revision(v: &str) -> bool {
    v.len() <= 128 && key(&v.replace('+', "-"))
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: Uuid,
    pub character_id: Option<Uuid>,
    pub world_id: Option<Uuid>,
    pub projection_revision: Option<String>,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: String,
    pub source_revision: String,
    pub record_id: Uuid,
    pub entity_key: String,
    pub character_revision: Option<String>,
    pub occurred_at_unix_ms: String,
    pub record_kind: String,
    pub payload: Option<Box<serde_json::value::RawValue>>,
}
impl Record {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 262144 {
            return Err(Error::BadInput);
        }
        let r: Self = serde_json::from_slice(bytes).map_err(|_| Error::BadInput)?;
        if serde_json::to_vec(&r).map_err(|_| Error::BadInput)? != bytes {
            return Err(Error::BadInput);
        }
        r.validate()?;
        Ok(r)
    }
    fn validate(&self) -> Result<Option<Payload>> {
        let scoped = self.character_id.is_some();
        let ids = [
            Some(self.account_id),
            self.character_id,
            self.world_id,
            Some(self.record_id),
        ];
        if self.contract_version != 1
            || self.operation != "PublishGameJournalProgressV1"
            || !ids.into_iter().flatten().all(|u| id(&u.to_string()))
            || !key(&self.entity_key)
            || self.source_authority.is_empty()
            || self.source_authority.len() > 128
            || !self
                .source_authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
            || ![
                &self.projection_epoch,
                &self.recovery_generation,
                &self.source_revision,
            ]
            .iter()
            .all(|v| decimal(v, true).is_some())
            || !timestamp(&self.occurred_at_unix_ms)
            || self.world_id.is_some() != scoped
            || self.projection_revision.is_some() != scoped
            || self.character_revision.is_some() != scoped
            || ![&self.projection_revision, &self.character_revision]
                .into_iter()
                .flatten()
                .all(|v| decimal(v, true).is_some())
            || !matches!(
                self.family.as_str(),
                "bestiary" | "boss_cooldown" | "quest" | "achievement" | "skill_history"
            )
            || (!scoped && self.family != "achievement")
        {
            return Err(Error::BadInput);
        }
        match (self.record_kind.as_str(), &self.payload) {
            ("REMOVE", None) => {
                if (self.family == "skill_history" && !id(&self.entity_key))
                    || (self.family == "achievement"
                        && (!self.entity_key.starts_with("oteryn:achievement/")
                            || self.entity_key.len() > 160))
                {
                    return Err(Error::BadInput);
                }
                Ok(None)
            }
            ("UPSERT", Some(raw)) => {
                let p = Payload::decode(
                    &self.family,
                    serde_json::from_str(raw.get()).map_err(|_| Error::BadInput)?,
                    !scoped,
                )?;
                if p.entity() != self.entity_key
                    || serde_json::to_string(&p).map_err(|_| Error::BadInput)? != raw.get()
                {
                    return Err(Error::BadInput);
                }
                Ok(Some(p))
            }
            _ => Err(Error::BadInput),
        }
    }
    fn authorize(&self, a: &impl Authority) -> Result<()> {
        if a.source() != self.source_authority
            || !a.permits(Subject {
                account: self.account_id,
                character: self.character_id,
                epoch: &self.projection_epoch,
                recovery: &self.recovery_generation,
                projection: self.projection_revision.as_deref(),
                character_revision: self.character_revision.as_deref(),
                world: self.world_id,
            })
        {
            return Err(Error::Denied);
        }
        if self
            .validate()?
            .as_ref()
            .is_some_and(|p| !a.catalog_accepts(&self.family, p))
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
}
/// Forecast input must be private, owner-authorized observations from a single
/// ownership cut and unit. Never mix skill counters with percentages or levels.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub at_ms: u64,
    pub value: u64,
}
/// Conservative linear estimate; absent, stale, falling or flat evidence gives no date.
pub fn forecast(samples: &[Sample], target: u64, now_ms: u64) -> Result<Option<u64>> {
    if target == 0
        || samples
            .iter()
            .any(|s| s.at_ms > now_ms || s.at_ms > 253402300799999 || s.value > i64::MAX as u64)
        || samples.windows(2).any(|w| w[0].at_ms >= w[1].at_ms)
    {
        return Err(Error::BadInput);
    }
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return Ok(None);
    };
    if now_ms.saturating_sub(last.at_ms) > 86400000
        || samples.len() < 2
        || samples.windows(2).any(|w| w[0].value > w[1].value)
    {
        return Ok(None);
    }
    if last.value >= target {
        return Ok(Some(now_ms));
    }
    let gain = last.value - first.value;
    if gain == 0 {
        return Ok(None);
    }
    let duration = u128::from(target - last.value) * u128::from(last.at_ms - first.at_ms);
    let wait = duration.div_ceil(u128::from(gain));
    Ok(u64::try_from(wait)
        .ok()
        .and_then(|w| now_ms.checked_add(w))
        .filter(|v| *v <= 253402300799999))
}
mod presentation;
#[cfg(test)]
mod tests;
pub use presentation::*;
/// Private pace provider for T65/T72 adapters. Observations must use the goal's
/// metric, character and ownership cut, exclude public/stale projections, and
/// contain no samples from before transfer. Unknown evidence returns Unavailable.
pub trait PrivatePace {
    fn samples(&self, goal: &Goal) -> impl std::future::Future<Output = Result<Vec<Sample>>>;
}
