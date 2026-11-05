use super::Error;
use oteryn_internal_api::canonical;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub world_id: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub source_observed_at: String,
    pub daily: Vec<Daily>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Daily {
    pub reset_epoch: String,
    pub effective_from: String,
    pub effective_until: String,
    pub creature_key: String,
    pub boss_key: String,
    pub history_complete_from: String,
    pub history: Vec<History>,
    pub raids: Vec<Raid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    pub reset_epoch: String,
    pub effective_from: String,
    pub effective_until: String,
    pub creature_key: String,
    pub boss_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raid {
    pub raid_key: String,
    pub scheduled_at: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: String,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}
#[derive(Debug)]
pub(super) enum Publication {
    Snapshot(Snapshot),
    Watermark(Watermark),
}
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(super) fn world_id(s: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(s).map_err(|_| Error::BadInput)?;
    if id.get_version_num() != 7 || id.to_string() != s || !b"89ab".contains(&s.as_bytes()[19]) {
        return Err(Error::BadInput);
    }
    Ok(id)
}
pub(super) fn decimal(s: &str, zero: bool) -> Result<u64> {
    if s == "0" && zero {
        return Ok(0);
    }
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::BadInput);
    }
    s.parse().map_err(|_| Error::BadInput)
}
pub(super) fn seconds(s: &str) -> Result<i64> {
    let n = decimal(s, true)?;
    if n > 9_223_372_036_854_775 {
        return Err(Error::BadInput);
    }
    Ok(n as i64)
}
fn key(s: &str, max: usize) -> Result<()> {
    if s.is_empty()
        || s.len() > max
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":._/-".contains(&b))
    {
        return Err(Error::BadInput);
    }
    Ok(())
}
fn interval(start: &str, end: &str) -> Result<(i64, i64)> {
    let (start, end) = (seconds(start)?, seconds(end)?);
    if start >= end {
        return Err(Error::BadInput);
    }
    Ok((start, end))
}
impl Snapshot {
    pub fn validate(&self, now: i64) -> Result<()> {
        if self.contract_version != 2
            || self.operation != "PublishWorldDailyV2"
            || !authority(&self.source_authority)
            || self.daily.len() > 1
        {
            return Err(Error::BadInput);
        }
        world_id(&self.world_id)?;
        decimal(&self.projection_epoch, false)?;
        decimal(&self.projection_revision, false)?;
        let observed = seconds(&self.source_observed_at)?;
        if observed > now.saturating_add(1) {
            return Err(Error::BadInput);
        }
        for d in &self.daily {
            d.validate(observed)?;
        }
        Ok(())
    }
    pub fn content(&self) -> Result<String> {
        // Struct field order is the contract order; provenance is deliberately excluded.
        #[derive(Serialize)]
        struct Content<'a> {
            world_id: &'a str,
            daily: &'a [Daily],
        }
        serde_json::to_string(&Content {
            world_id: &self.world_id,
            daily: &self.daily,
        })
        .map_err(|_| Error::BadInput)
    }
}
impl Daily {
    pub fn validate(&self, observed: i64) -> Result<()> {
        let epoch = decimal(&self.reset_epoch, true)?;
        let (start, end) = interval(&self.effective_from, &self.effective_until)?;
        key(&self.creature_key, 255)?;
        key(&self.boss_key, 255)?;
        let floor = seconds(&self.history_complete_from)?;
        if start > observed
            || floor > start
            || floor < start.saturating_sub(2_592_000).max(0)
            || self.history.len() > 32
            || self.raids.len() > 256
        {
            return Err(Error::BadInput);
        }
        let (mut previous_epoch, mut previous_start) = (epoch, start);
        for h in &self.history {
            let epoch = decimal(&h.reset_epoch, true)?;
            let (from, until) = interval(&h.effective_from, &h.effective_until)?;
            key(&h.creature_key, 255)?;
            key(&h.boss_key, 255)?;
            if epoch >= previous_epoch || from < floor || until > previous_start {
                return Err(Error::BadInput);
            }
            (previous_epoch, previous_start) = (epoch, from);
        }
        let mut previous = None;
        for r in &self.raids {
            key(&r.raid_key, 128)?;
            let scheduled = seconds(&r.scheduled_at)?;
            if scheduled < start
                || scheduled >= end
                || scheduled > observed
                || !matches!(r.state.as_str(), "running" | "done" | "cancelled")
                || previous.is_some_and(|p| p >= (scheduled, r.raid_key.as_str()))
            {
                return Err(Error::BadInput);
            }
            previous = Some((scheduled, r.raid_key.as_str()));
        }
        Ok(())
    }
}
impl Watermark {
    fn validate(&self, now: i64) -> Result<()> {
        if self.contract_version != 2
            || self.operation != "PublishPublicProjectionWatermarkV2"
            || self.family != "world_daily"
            || !authority(&self.source_authority)
            || seconds(&self.complete_through)? > seconds(&self.observed_at)?
            || seconds(&self.observed_at)? > now.saturating_add(1)
        {
            return Err(Error::BadInput);
        }
        decimal(&self.projection_epoch, false)?;
        Ok(())
    }
}
pub(super) fn decode(bytes: &[u8], watermark: bool, now: i64) -> Result<Publication> {
    let max = if watermark { 512 } else { 131_072 };
    let value = canonical::decode(bytes, max, 6).map_err(|_| Error::BadInput)?;
    // Typed re-encoding checks exact member order and unescaped canonical JSON.
    // The ordered decoder additionally rejects duplicate keys before serde conversion.
    let value: serde_json::Value = serde_json::to_value(value).map_err(|_| Error::BadInput)?;
    if watermark {
        let w: Watermark = serde_json::from_value(value).map_err(|_| Error::BadInput)?;
        if serde_json::to_vec(&w).map_err(|_| Error::BadInput)? != bytes {
            return Err(Error::BadInput);
        }
        w.validate(now)?;
        Ok(Publication::Watermark(w))
    } else {
        let s: Snapshot = serde_json::from_value(value).map_err(|_| Error::BadInput)?;
        if serde_json::to_vec(&s).map_err(|_| Error::BadInput)? != bytes {
            return Err(Error::BadInput);
        }
        s.validate(now)?;
        Ok(Publication::Snapshot(s))
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
