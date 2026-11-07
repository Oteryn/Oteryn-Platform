//! Accepted revision-2 wire. This module never accepts a resync/control envelope.
use oteryn_internal_api::canonical::{self, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    CreatureKillsDaily,
    BossKillsDaily,
    PlayerDeathsDaily,
    WorldActivityHourly,
    CharactersCreatedDaily,
    LevelDistributionDaily,
}
impl Family {
    pub fn interval(self) -> u64 {
        if self == Self::WorldActivityHourly {
            3600
        } else {
            86400
        }
    }
    pub fn retention(self) -> u64 {
        if self == Self::WorldActivityHourly {
            30 * 86400
        } else {
            400 * 86400
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub key: String,
    pub state: String,
    pub values: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: Family,
    pub world_id: String,
    pub bucket_start: String,
    pub bucket_end: String,
    pub analytics_epoch: String,
    pub snapshot_revision: String,
    pub source_observed_at: String,
    pub privacy_policy_revision: String,
    pub page_index: String,
    pub page_count: String,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: Family,
    pub world_id: String,
    pub bucket_start: String,
    pub bucket_end: String,
    pub analytics_epoch: String,
    pub snapshot_revision: String,
    pub source_observed_at: String,
    pub privacy_policy_revision: String,
    pub state: String,
    pub row_count: String,
    pub page_digests: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: Family,
    pub world_id: String,
    pub analytics_epoch: String,
    pub privacy_policy_revision: String,
    pub complete_through: String,
    pub observed_at: String,
}
#[derive(Debug, Clone)]
pub enum Message {
    Page(Page),
    Commit(Commit),
    Watermark(Watermark),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid admin analytics wire")]
pub struct Invalid;
pub fn uint(s: &str) -> Result<u64, Invalid> {
    if s.is_empty()
        || s.len() > 20
        || (s.len() > 1 && s.starts_with('0'))
        || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Invalid);
    }
    s.parse().map_err(|_| Invalid)
}
fn positive(s: &str) -> Result<u64, Invalid> {
    let n = uint(s)?;
    if n == 0 { Err(Invalid) } else { Ok(n) }
}
fn token(s: &str, max: usize, extra: &[u8]) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
}
fn common(
    version: u8,
    authority: &str,
    world: &str,
    epoch: &str,
    privacy: &str,
) -> Result<(), Invalid> {
    let id = Uuid::parse_str(world).map_err(|_| Invalid)?;
    if version != 1
        || !token(authority, 128, b"._:/-")
        || id.is_nil()
        || id.get_version_num() != 7
        || id.to_string() != world
        || !token(privacy, 64, b"._-")
    {
        return Err(Invalid);
    }
    positive(epoch)?;
    Ok(())
}
fn bucket(
    family: Family,
    start: &str,
    end: &str,
    revision: &str,
    observed: &str,
    now: u64,
) -> Result<(), Invalid> {
    let start = positive(start)?;
    let end = positive(end)?;
    let observed = positive(observed)?;
    positive(revision)?;
    if start % family.interval() != 0
        || start.checked_add(family.interval()) != Some(end)
        || end.checked_add(86400).ok_or(Invalid)? > observed
        || observed > now.checked_add(2).ok_or(Invalid)?
    {
        return Err(Invalid);
    }
    end.checked_add(family.retention()).ok_or(Invalid)?;
    Ok(())
}
const LEVELS: [&str; 9] = [
    "1000_plus",
    "100_199",
    "1_7",
    "200_499",
    "20_49",
    "500_999",
    "50_99",
    "8_19",
    "unknown",
];
pub fn rows(family: Family, rows: &[Row], complete: bool) -> Result<(), Invalid> {
    let mut previous: Option<&str> = None;
    for row in rows {
        if previous.is_some_and(|p| p >= row.key.as_str()) {
            return Err(Invalid);
        }
        previous = Some(&row.key);
        let key_ok = match family {
            Family::WorldActivityHourly | Family::CharactersCreatedDaily => row.key == "world",
            Family::LevelDistributionDaily => LEVELS.contains(&row.key.as_str()),
            _ => {
                token(&row.key, 512, b"_./:-")
                    && row
                        .key
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
            }
        };
        if !key_ok {
            return Err(Invalid);
        }
        match row.state.as_str() {
            "suppressed" | "incomplete" if row.values.is_empty() => (),
            "visible" => {
                let expected = if family == Family::WorldActivityHourly {
                    4
                } else {
                    1
                };
                if row.values.len() != expected {
                    return Err(Invalid);
                }
                let values: Vec<u64> = row
                    .values
                    .iter()
                    .map(|v| uint(v))
                    .collect::<Result<_, _>>()?;
                if expected == 4 {
                    let milli = values[1].checked_mul(1000).ok_or(Invalid)? / 3600;
                    let max = values[2].checked_mul(1000).ok_or(Invalid)?;
                    if values[3] != milli || milli > max {
                        return Err(Invalid);
                    }
                }
            }
            _ => return Err(Invalid),
        }
    }
    if complete {
        match family {
            Family::WorldActivityHourly | Family::CharactersCreatedDaily if rows.len() != 1 => {
                return Err(Invalid);
            }
            Family::LevelDistributionDaily if rows.len() != 9 => return Err(Invalid),
            _ => (),
        }
    }
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned + Serialize>(
    raw: &[u8],
    max: usize,
) -> Result<T, Invalid> {
    let _: Value = canonical::decode(raw, max, 6).map_err(|_| Invalid)?;
    let result: T = serde_json::from_slice(raw).map_err(|_| Invalid)?;
    if serde_json::to_vec(&result).map_err(|_| Invalid)? != raw || raw.contains(&b'\\') {
        return Err(Invalid);
    }
    Ok(result)
}
pub fn parse(operation: &str, raw: &[u8], now: u64) -> Result<Message, Invalid> {
    match operation {
        "page" => {
            let p: Page = decode(raw, 131072)?;
            common(
                p.contract_version,
                &p.source_authority,
                &p.world_id,
                &p.analytics_epoch,
                &p.privacy_policy_revision,
            )?;
            bucket(
                p.family,
                &p.bucket_start,
                &p.bucket_end,
                &p.snapshot_revision,
                &p.source_observed_at,
                now,
            )?;
            let count = uint(&p.page_count)?;
            let index = uint(&p.page_index)?;
            if p.operation != "PublishAdminAnalyticsPageV1"
                || !(1..=64).contains(&count)
                || index >= count
                || p.rows.len() > 128
                || (p.rows.is_empty() && count != 1)
            {
                return Err(Invalid);
            }
            rows(p.family, &p.rows, false)?;
            Ok(Message::Page(p))
        }
        "commit" => {
            let c: Commit = decode(raw, 8192)?;
            common(
                c.contract_version,
                &c.source_authority,
                &c.world_id,
                &c.analytics_epoch,
                &c.privacy_policy_revision,
            )?;
            bucket(
                c.family,
                &c.bucket_start,
                &c.bucket_end,
                &c.snapshot_revision,
                &c.source_observed_at,
                now,
            )?;
            let count = uint(&c.row_count)?;
            if c.operation != "CommitAdminAnalyticsSnapshotV1"
                || count > 8192
                || c.page_digests.len() > 64
                || c.page_digests.iter().any(|d| {
                    d.len() != 64
                        || !d
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
            {
                return Err(Invalid);
            }
            if !matches!(c.state.as_str(), "replace" | "erase")
                || (c.state == "erase" && (count != 0 || !c.page_digests.is_empty()))
                || (c.state == "replace" && c.page_digests.is_empty())
            {
                return Err(Invalid);
            }
            let unique: std::collections::HashSet<_> = c.page_digests.iter().collect();
            if unique.len() != c.page_digests.len() {
                return Err(Invalid);
            }
            Ok(Message::Commit(c))
        }
        "watermark" => {
            let w: Watermark = decode(raw, 1024)?;
            common(
                w.contract_version,
                &w.source_authority,
                &w.world_id,
                &w.analytics_epoch,
                &w.privacy_policy_revision,
            )?;
            let cut = positive(&w.complete_through)?;
            let observed = positive(&w.observed_at)?;
            if w.operation != "PublishAdminAnalyticsWatermarkV1"
                || cut > observed
                || observed > now.checked_add(2).ok_or(Invalid)?
                || now.saturating_sub(observed) > 120
            {
                return Err(Invalid);
            }
            Ok(Message::Watermark(w))
        }
        _ => Err(Invalid),
    }
}
