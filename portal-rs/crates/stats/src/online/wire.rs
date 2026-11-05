use super::*;
use oteryn_internal_api::canonical;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(crate) fn decimal(s: &str, zero: bool) -> Result<u64, Error> {
    let n = s.parse::<u64>().map_err(|_| Error::Invalid)?;
    if n.to_string() != s || (!zero && n == 0) {
        return Err(Error::Invalid);
    }
    Ok(n)
}
pub(crate) fn id(s: &str) -> Result<(), Error> {
    let id = Uuid::parse_str(s).map_err(|_| Error::Invalid)?;
    if id.to_string() != s
        || id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn timestamp(s: &str, zero: bool) -> Result<i64, Error> {
    i64::try_from(decimal(s, zero)?).map_err(|_| Error::Invalid)
}
fn decode<T: DeserializeOwned + Serialize>(raw: &[u8], bound: usize) -> Result<T, Error> {
    canonical::decode(raw, bound, 6).map_err(|_| Error::Invalid)?;
    let value: T = serde_json::from_slice(raw).map_err(|_| Error::Invalid)?;
    // Exact member order, no escapes, whitespace, duplicates, coercion or extra fields.
    if raw.contains(&b'\\') || serde_json::to_vec(&value).map_err(|_| Error::Invalid)? != raw {
        return Err(Error::Invalid);
    }
    Ok(value)
}
fn envelope(
    version: u8,
    operation: &str,
    expected: &str,
    source: &str,
    epoch: &str,
    observed: i64,
    now: i64,
) -> Result<(), Error> {
    if version != 2
        || operation != expected
        || !authority(source)
        || observed > now.saturating_add(1)
        || now <= 0
    {
        return Err(Error::Invalid);
    }
    decimal(epoch, false)?;
    Ok(())
}
pub(super) fn publication(
    family: Option<Family>,
    raw: &[u8],
    now: i64,
) -> Result<Publication, Error> {
    match family {
        Some(Family::CharacterOnlineSessions) => {
            let w: IntervalWire = decode(raw, 2048)?;
            let observed = timestamp(&w.source_observed_at, false)?;
            envelope(
                w.contract_version,
                &w.operation,
                "PublishCharacterOnlineSessionV2",
                &w.source_authority,
                &w.projection_epoch,
                observed,
                now,
            )?;
            decimal(&w.projection_revision, false)?;
            id(&w.character_id)?;
            id(&w.statistics_id)?;
            if w.sessions.len() > 1 {
                return Err(Error::Invalid);
            }
            let mut expires = observed
                .checked_add(INTERVAL_RETENTION)
                .ok_or(Error::Invalid)?;
            if let Some(s) = w.sessions.first() {
                id(&s.world_id)?;
                let start = timestamp(&s.started_at, false)?;
                if start > observed || s.ended_at.len() > 1 {
                    return Err(Error::Invalid);
                }
                let end = s
                    .ended_at
                    .first()
                    .map(|s| timestamp(s, false))
                    .transpose()?;
                if end.is_some_and(|end| end < start || end > observed) {
                    return Err(Error::Invalid);
                }
                expires = end
                    .unwrap_or(start)
                    .checked_add(INTERVAL_RETENTION)
                    .ok_or(Error::Invalid)?;
            }
            let content = json!({"character_id":w.character_id,"statistics_id":w.statistics_id,"sessions":w.sessions});
            Ok(Publication {
                family: Family::CharacterOnlineSessions,
                subject: format!("{}/{}", w.character_id, w.statistics_id),
                authority: w.source_authority.clone(),
                epoch: w.projection_epoch.clone(),
                revision: w.projection_revision.clone(),
                observed,
                content,
                expires,
                watermark: None,
                interval: Some(w),
            })
        }
        Some(Family::WorldDailyKills) => {
            let w: WorldWire = decode(raw, 2048)?;
            let observed = timestamp(&w.source_observed_at, false)?;
            envelope(
                w.contract_version,
                &w.operation,
                "PublishWorldDailyKillsV2",
                &w.source_authority,
                &w.projection_epoch,
                observed,
                now,
            )?;
            decimal(&w.projection_revision, false)?;
            id(&w.world_id)?;
            let start = timestamp(&w.day_start, false)?;
            let end = start.checked_add(86400).ok_or(Error::Invalid)?;
            if start % 86400 != 0 || start > observed || w.days.len() > 1 {
                return Err(Error::Invalid);
            }
            if let Some(day) = w.days.first() {
                if timestamp(&day.day_end, false)? != end || (day.complete && end > observed) {
                    return Err(Error::Invalid);
                }
                decimal(&day.monsters_killed, true)?;
                decimal(&day.bosses_killed, true)?;
            }
            let expires = end.checked_add(WORLD_RETENTION).ok_or(Error::Invalid)?;
            let content = json!({"world_id":w.world_id,"day_start":w.day_start,"days":w.days});
            Ok(Publication {
                family: Family::WorldDailyKills,
                subject: format!("{}/{}", w.world_id, w.day_start),
                authority: w.source_authority,
                epoch: w.projection_epoch,
                revision: w.projection_revision,
                observed,
                content,
                expires,
                watermark: None,
                interval: None,
            })
        }
        None => {
            let w: WatermarkWire = decode(raw, 512)?;
            let observed = timestamp(&w.observed_at, true)?;
            envelope(
                w.contract_version,
                &w.operation,
                "PublishPublicProjectionWatermarkV2",
                &w.source_authority,
                &w.projection_epoch,
                observed,
                now,
            )?;
            let family = Family::parse(&w.family)?;
            let complete = timestamp(&w.complete_through, true)?;
            if complete > observed {
                return Err(Error::Invalid);
            }
            Ok(Publication {
                family,
                subject: String::new(),
                authority: w.source_authority,
                epoch: w.projection_epoch,
                revision: "1".into(),
                observed,
                content: Value::Null,
                expires: now,
                watermark: Some(complete),
                interval: None,
            })
        }
    }
}
