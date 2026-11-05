use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_TIME: u64 = 253_402_300_799_999;
pub const HISTORY_MS: u64 = 90 * 24 * 60 * 60 * 1000;
pub const STALE_MS: u64 = 24 * 60 * 60 * 1000;

/// Empty-body status semantics from G-11 §7. Never include rejected private input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    BadInput,
    Denied,
    Conflict,
    TooLarge,
    Unavailable,
}
impl Failure {
    pub fn status(self) -> u16 {
        match self {
            Self::BadInput => 400,
            Self::Denied => 401,
            Self::Conflict => 409,
            Self::TooLarge => 413,
            Self::Unavailable => 503,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    DeathRecap,
    Pvp,
    Skull,
}
impl Family {
    pub fn key(self) -> &'static str {
        match self {
            Self::DeathRecap => "death_recap",
            Self::Pvp => "pvp",
            Self::Skull => "skull",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Loss {
    pub loss_index: u8,
    pub item_key: String,
    pub quantity: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Death {
    pub death_occurrence_id: String,
    pub level_before: u32,
    pub level_after: u32,
    pub experience_lost: String,
    pub blessings_before: Vec<String>,
    pub blessings_after: Vec<String>,
    pub lost_items: Vec<Loss>,
    pub recap_state: String,
    pub window_ms: u32,
    pub actions: Vec<serde_json::Value>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Kill,
    Assist,
    Victim,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Justification {
    Justified,
    Unjustified,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pvp {
    pub death_occurrence_id: String,
    pub role: Role,
    pub counterpart_character_id: Option<String>,
    pub justification: Justification,
    pub points_milli: Option<u16>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkullColor {
    None,
    Red,
    Black,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Skull {
    pub skull: SkullColor,
    pub skull_until_unix_ms: Option<String>,
    pub kill_block_until_unix_ms: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Payload {
    Death(Death),
    Pvp(Pvp),
    Skull(Skull),
}

pub fn decimal(value: &str, zero: bool, max: u64) -> Result<u64, Failure> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.starts_with('0') && value != "0")
    {
        return Err(Failure::BadInput);
    }
    let parsed: u64 = value.parse().map_err(|_| Failure::BadInput)?;
    if parsed > max || (!zero && parsed == 0) {
        return Err(Failure::BadInput);
    }
    Ok(parsed)
}
pub fn id(value: &str) -> Result<Uuid, Failure> {
    let parsed = Uuid::parse_str(value).map_err(|_| Failure::BadInput)?;
    if parsed.get_version_num() != 7
        || parsed.get_variant() != uuid::Variant::RFC4122
        || parsed.to_string() != value
    {
        return Err(Failure::BadInput);
    }
    Ok(parsed)
}
fn key(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn blessings(values: &[String]) -> bool {
    values.len() <= 32
        && values.iter().all(|v| key(v, 128))
        && values.windows(2).all(|p| p[0] < p[1])
}
impl Payload {
    pub fn validate(&self, family: Family, entity: &str) -> Result<(), Failure> {
        let valid = match (self, family) {
            (Self::Death(d), Family::DeathRecap) => {
                if d.lost_items.len() > 256
                    || d.blessings_before.len() > 32
                    || d.blessings_after.len() > 32
                {
                    return Err(Failure::TooLarge);
                }
                id(&d.death_occurrence_id)?;
                decimal(&d.experience_lost, true, i64::MAX as u64)?;
                d.death_occurrence_id == entity
                    && d.level_after >= 1
                    && d.level_after <= d.level_before
                    && blessings(&d.blessings_before)
                    && blessings(&d.blessings_after)
                    && d.blessings_after
                        .iter()
                        .all(|b| d.blessings_before.contains(b))
                    && d.lost_items.len() <= 256
                    && d.lost_items
                        .iter()
                        .all(|i| i.quantity > 0 && key(&i.item_key, 512))
                    && d.lost_items
                        .windows(2)
                        .all(|p| p[0].loss_index < p[1].loss_index)
                    && d.recap_state == "UNAVAILABLE"
                    && d.window_ms == 0
                    && d.actions.is_empty()
            }
            (Self::Pvp(p), Family::Pvp) => {
                id(&p.death_occurrence_id)?;
                if let Some(other) = &p.counterpart_character_id {
                    id(other)?;
                }
                p.death_occurrence_id == entity
                    && match p.justification {
                        Justification::Unknown => p.points_milli.is_none(),
                        Justification::Justified => p.points_milli == Some(0),
                        Justification::Unjustified => p.points_milli.is_some(),
                    }
            }
            (Self::Skull(s), Family::Skull) => {
                for time in [&s.skull_until_unix_ms, &s.kill_block_until_unix_ms]
                    .into_iter()
                    .flatten()
                {
                    decimal(time, true, MAX_TIME)?;
                }
                entity == "skull"
                    && (s.skull == SkullColor::None) == s.skull_until_unix_ms.is_none()
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(Failure::BadInput)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordKind {
    Upsert,
    Remove,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub character_id: String,
    pub world_id: String,
    pub projection_revision: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: Family,
    pub source_revision: String,
    pub record_id: String,
    pub entity_key: String,
    pub character_revision: String,
    pub occurred_at_unix_ms: String,
    pub record_kind: RecordKind,
    pub payload: Option<Payload>,
}
impl Record {
    pub fn validate(&self) -> Result<(), Failure> {
        if self.contract_version != 1
            || self.operation != "PublishGameJournalProgressV1"
            || !authority(&self.source_authority)
            || !key(&self.entity_key, 512)
        {
            return Err(Failure::BadInput);
        }
        for value in [
            &self.account_id,
            &self.character_id,
            &self.world_id,
            &self.record_id,
        ] {
            id(value)?;
        }
        for value in [
            &self.projection_revision,
            &self.projection_epoch,
            &self.recovery_generation,
            &self.source_revision,
            &self.character_revision,
        ] {
            decimal(value, false, u64::MAX)?;
        }
        decimal(&self.occurred_at_unix_ms, true, MAX_TIME)?;
        match (&self.record_kind, &self.payload) {
            (RecordKind::Upsert, Some(p)) => p.validate(self.family, &self.entity_key),
            (RecordKind::Remove, None) => {
                if self.family == Family::Skull {
                    if self.entity_key != "skull" {
                        return Err(Failure::BadInput);
                    }
                } else {
                    id(&self.entity_key)?;
                }
                Ok(())
            }
            _ => Err(Failure::BadInput),
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, Failure> {
        let record: Self = canonical(bytes, 262144)?;
        record.validate()?;
        Ok(record)
    }
    pub fn stream(&self) -> Stream {
        Stream {
            source_authority: self.source_authority.clone(),
            account_id: self.account_id.clone(),
            projection_epoch: self.projection_epoch.clone(),
            recovery_generation: self.recovery_generation.clone(),
            family: self.family,
        }
    }
}
pub(crate) fn authority(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(crate) fn canonical<T: serde::de::DeserializeOwned + Serialize>(
    bytes: &[u8],
    cap: usize,
) -> Result<T, Failure> {
    if bytes.len() > cap {
        return Err(Failure::TooLarge);
    }
    let value: T = serde_json::from_slice(bytes).map_err(|_| Failure::BadInput)?;
    if bytes.contains(&b'\\') || serde_json::to_vec(&value).map_err(|_| Failure::BadInput)? != bytes
    {
        return Err(Failure::BadInput);
    }
    Ok(value)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub source_authority: String,
    pub account_id: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: Family,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: Family,
    pub complete_through: String,
    pub observed_at_unix_ms: String,
    pub baseline_complete: bool,
}
impl Watermark {
    pub fn parse(bytes: &[u8]) -> Result<Self, Failure> {
        let mark: Self = canonical(bytes, 1024)?;
        id(&mark.account_id)?;
        decimal(&mark.projection_epoch, false, u64::MAX)?;
        decimal(&mark.recovery_generation, false, u64::MAX)?;
        decimal(&mark.complete_through, true, u64::MAX)?;
        decimal(&mark.observed_at_unix_ms, true, MAX_TIME)?;
        if mark.contract_version != 1
            || mark.operation != "PublishGameJournalProgressWatermarkV1"
            || !authority(&mark.source_authority)
        {
            return Err(Failure::BadInput);
        }
        Ok(mark)
    }
    pub fn stream(&self) -> Stream {
        Stream {
            source_authority: self.source_authority.clone(),
            account_id: self.account_id.clone(),
            projection_epoch: self.projection_epoch.clone(),
            recovery_generation: self.recovery_generation.clone(),
            family: self.family,
        }
    }
}
