use crate::{Currency, Error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterFlags {
    pub enabled: bool,
    pub gold: bool,
    /// Requires accepted capability evidence and shipped consumers, never just an env switch.
    pub game_commands: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListingRequest {
    pub character_id: String,
    pub currency: String,
    pub starting_bid: String,
    pub duration_days: u8,
    pub request_id: String,
    pub publish_snapshot: bool,
}
impl ListingRequest {
    pub fn validate(&self) -> Result<(Uuid, Uuid, Currency, i64), Error> {
        let character = id(&self.character_id)?;
        let request = id(&self.request_id)?;
        let currency = match self.currency.as_str() {
            "coins" => Currency::Coins,
            "gold" => Currency::Gold,
            _ => return Err(Error::InvalidInput),
        };
        let amount = amount(&self.starting_bid, currency)?;
        if !(1..=14).contains(&self.duration_days) || !self.publish_snapshot {
            return Err(Error::InvalidInput);
        }
        Ok((character, request, currency, amount))
    }
}
pub fn id(value: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(value).map_err(|_| Error::InvalidInput)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != value
    {
        return Err(Error::InvalidInput);
    }
    Ok(id)
}
pub fn amount(value: &str, currency: Currency) -> Result<i64, Error> {
    if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidInput);
    }
    let n = value.parse::<i64>().map_err(|_| Error::InvalidInput)?;
    if n <= 0 || (currency == Currency::Gold && n > 999_999_999_999) {
        return Err(Error::InvalidInput);
    }
    Ok(n)
}

/// Closed, separately approved public projection. Never deserialize the raw signed envelope here.
/// T58 verifies signature, trust, lock/receipt bindings, restore cut, privacy and current eligibility.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaleSnapshot {
    pub name: String,
    pub world: String,
    pub vocation: u8,
    pub level: u64,
    pub captured_at: u64,
    pub snapshot_hash: String,
    pub health: u64,
    pub health_max: u64,
    pub mana_max: u64,
    pub created_at: Option<u64>,
    pub blessings: Vec<String>,
    pub world_transfer_eligible: Option<bool>,
    pub mana: u64,
    pub capacity_centi_oz: u64,
    pub speed: u64,
    pub experience: u64,
    pub skills: Vec<Skill>,
    pub items: Option<Vec<SaleItem>>,
    pub store_grants: Option<Vec<String>>,
    pub outfits: Option<Vec<Progress>>,
    pub mounts: Option<Vec<Progress>>,
    pub achievements: Option<Vec<Progress>>,
    pub quests: Option<Vec<Progress>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub key: String,
    pub level: u64,
    pub progress_bps: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub key: String,
    pub value: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaleItem {
    /// Definition key, never an instance ID or private attribute blob.
    pub key: String,
    pub quantity: u32,
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:/-".contains(&b))
        && s.as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}
impl SaleSnapshot {
    pub fn validate(&self) -> Result<(), Error> {
        if self.health > self.health_max
            || self.mana > self.mana_max
            || self.blessings.len() > 32
            || self.blessings.iter().any(|s| !key(s))
            || self.blessings.windows(2).any(|v| v[0] >= v[1])
            || i64::try_from(self.captured_at)
                .ok()
                .and_then(|v| time::OffsetDateTime::from_unix_timestamp(v).ok())
                .is_none()
            || self.created_at.is_some_and(|v| {
                v == 0
                    || i64::try_from(v)
                        .ok()
                        .and_then(|n| time::OffsetDateTime::from_unix_timestamp(n).ok())
                        .is_none()
            })
        {
            return Err(Error::Denied);
        }
        let name = &self.name;
        if !(2..=29).contains(&name.len())
            || name.trim() != name
            || name.contains("  ")
            || !name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
            || !key(&self.world)
            || !(1..=6).contains(&self.vocation)
            || self.level == 0
            || self.captured_at == 0
            || self.snapshot_hash.len() != 64
            || !self
                .snapshot_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.skills.len() > 16
        {
            return Err(Error::Denied);
        }
        let mut seen = std::collections::BTreeSet::new();
        for s in &self.skills {
            if !matches!(
                s.key.as_str(),
                "fist" | "club" | "sword" | "axe" | "distance" | "shielding" | "fishing" | "magic"
            ) || s.progress_bps > 10_000
                || !seen.insert(&s.key)
            {
                return Err(Error::Denied);
            }
        }
        for values in [
            &self.outfits,
            &self.mounts,
            &self.achievements,
            &self.quests,
        ]
        .into_iter()
        .flatten()
        {
            let mut seen = std::collections::BTreeSet::new();
            if values.len() > 512 || values.iter().any(|v| !key(&v.key) || !seen.insert(&v.key)) {
                return Err(Error::Denied);
            }
        }
        if let Some(v) = &self.store_grants {
            let mut seen = std::collections::BTreeSet::new();
            if v.len() > 128 || v.iter().any(|s| !key(s) || !seen.insert(s)) {
                return Err(Error::Denied);
            }
        }
        if let Some(v) = &self.items
            && (v.len() > 512 || v.iter().any(|s| !key(&s.key) || s.quantity == 0))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}

/// Explicit public DTO: no owner, bidder, lock, receipt, signature or private maximum.
#[derive(Clone, Debug)]
pub struct CharacterCard {
    pub id: Uuid,
    pub snapshot: SaleSnapshot,
    pub price: i64,
    pub currency: Currency,
    pub ends_at: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct CharacterChoice {
    pub id: Uuid,
    pub name: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct OwnRequest {
    pub id: Uuid,
    pub state: String,
}
