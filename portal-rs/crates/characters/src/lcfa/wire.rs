use super::Error;
use oteryn_internal_api::canonical::{self, Value};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
use uuid::Uuid;

pub const SNAPSHOT_MAX_BYTES: usize = 16384;
pub const WATERMARK_MAX_BYTES: usize = 512;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub(super) Uuid);
        impl $name {
            pub fn parse(raw: &str) -> Result<Self, Error> {
                let uuid = Uuid::parse_str(raw).map_err(|_| Error::BadInput)?;
                if uuid.get_version_num() != 7
                    || uuid.get_variant() != uuid::Variant::RFC4122
                    || uuid.to_string() != raw
                {
                    return Err(Error::BadInput);
                }
                Ok(Self(uuid))
            }
            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
identifier!(AccountId);
identifier!(CharacterId);
identifier!(WorldId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Unavailable,
}
impl Availability {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Available => "AVAILABLE",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
    pub(super) fn parse(raw: &str) -> Result<Self, Error> {
        match raw {
            "AVAILABLE" => Ok(Self::Available),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            _ => Err(Error::BadInput),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    pub availability: Availability,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub(super) authority: String,
    pub(super) account: AccountId,
    pub(super) epoch: u64,
    pub(super) revision: u64,
    pub(super) observed: i64,
    pub(super) characters: Vec<CharacterSummary>,
    pub(super) digest: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct Watermark {
    pub(super) authority: String,
    pub(super) epoch: u64,
    pub(super) complete: i64,
    pub(super) observed: i64,
}

fn decode(raw: &[u8], limit: usize) -> Result<Value, Error> {
    let value = canonical::decode(raw, limit, 4).map_err(|_| Error::BadInput)?;
    if canonical::encode(&value).map_err(|_| Error::BadInput)? != raw {
        return Err(Error::BadInput);
    }
    Ok(value)
}
fn keys(value: &Value, expected: &[&str]) -> Result<(), Error> {
    match value {
        Value::Object(members)
            if members.len() == expected.len()
                && members
                    .iter()
                    .all(|(key, _)| expected.contains(&key.as_str())) =>
        {
            Ok(())
        }
        _ => Err(Error::BadInput),
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Error> {
    match value.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::BadInput),
    }
}
fn decimal(value: &Value, key: &str) -> Result<u64, Error> {
    let raw = text(value, key)?;
    if raw.is_empty()
        || (raw.starts_with('0') && raw.len() > 1)
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::BadInput);
    }
    raw.parse().map_err(|_| Error::BadInput)
}
fn uint64(value: &Value, key: &str) -> Result<u64, Error> {
    let number = decimal(value, key)?;
    if number == 0 {
        return Err(Error::BadInput);
    }
    Ok(number)
}
fn time(value: &Value, key: &str) -> Result<i64, Error> {
    i64::try_from(decimal(value, key)?).map_err(|_| Error::BadInput)
}
pub(super) fn authority_valid(raw: &str) -> bool {
    (1..=128).contains(&raw.len())
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(super) fn name_valid(raw: &str) -> bool {
    (1..=64).contains(&raw.len())
        && raw != "."
        && raw != ".."
        && !raw.contains(['"', '\\', '/'])
        && !raw.chars().any(|c| {
            matches!(
                c.general_category(),
                GeneralCategory::Control | GeneralCategory::Format
            )
        })
        && raw.nfc().eq(raw.chars())
}
fn envelope(value: &Value, operation: &str) -> Result<String, Error> {
    if !matches!(value.get("contract_version"), Some(Value::Unsigned(1)))
        || text(value, "operation")? != operation
    {
        return Err(Error::BadInput);
    }
    let authority = text(value, "source_authority")?;
    if !authority_valid(authority) {
        return Err(Error::BadInput);
    }
    Ok(authority.to_owned())
}
impl Snapshot {
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        let value = decode(raw, SNAPSHOT_MAX_BYTES)?;
        keys(
            &value,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "account_id",
                "projection_epoch",
                "projection_revision",
                "source_observed_at",
                "characters",
            ],
        )?;
        let authority = envelope(&value, "PublishAccountCharactersV1")?;
        let Some(Value::Array(entries)) = value.get("characters") else {
            return Err(Error::BadInput);
        };
        if entries.len() > 64 {
            return Err(Error::BadInput);
        }
        let mut characters = Vec::with_capacity(entries.len());
        let mut canonical_entries = Vec::with_capacity(entries.len());
        let mut previous = None;
        for entry in entries {
            keys(entry, &["character_id", "world_id", "name", "availability"])?;
            let character_id = CharacterId::parse(text(entry, "character_id")?)?;
            if previous.is_some_and(|id| id >= character_id) {
                return Err(Error::BadInput);
            }
            previous = Some(character_id);
            let world_id = WorldId::parse(text(entry, "world_id")?)?;
            let name = text(entry, "name")?;
            if !name_valid(name) {
                return Err(Error::BadInput);
            }
            let availability = Availability::parse(text(entry, "availability")?)?;
            canonical_entries.push(Value::Object(vec![
                (
                    "character_id".into(),
                    Value::String(character_id.to_string()),
                ),
                ("world_id".into(), Value::String(world_id.to_string())),
                ("name".into(), Value::String(name.into())),
                (
                    "availability".into(),
                    Value::String(availability.as_str().into()),
                ),
            ]));
            characters.push(CharacterSummary {
                character_id,
                world_id,
                name: name.into(),
                availability,
            });
        }
        let content =
            canonical::encode(&Value::Array(canonical_entries)).map_err(|_| Error::BadInput)?;
        Ok(Self {
            authority,
            account: AccountId::parse(text(&value, "account_id")?)?,
            epoch: uint64(&value, "projection_epoch")?,
            revision: uint64(&value, "projection_revision")?,
            observed: time(&value, "source_observed_at")?,
            characters,
            digest: Sha256::digest(content).to_vec(),
        })
    }
}
impl Watermark {
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        let value = decode(raw, WATERMARK_MAX_BYTES)?;
        keys(
            &value,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "projection_epoch",
                "complete_through",
                "observed_at",
            ],
        )?;
        let watermark = Self {
            authority: envelope(&value, "PublishProjectionWatermarkV1")?,
            epoch: uint64(&value, "projection_epoch")?,
            complete: time(&value, "complete_through")?,
            observed: time(&value, "observed_at")?,
        };
        if watermark.complete > watermark.observed {
            return Err(Error::BadInput);
        }
        Ok(watermark)
    }
}
