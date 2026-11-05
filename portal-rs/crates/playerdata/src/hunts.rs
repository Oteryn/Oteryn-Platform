//! Owner-private hunt analysis. Only T72's verified projection may supply facts;
//! this module deliberately does not decode or acknowledge the proposed wire.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[path = "hunts_store.rs"]
pub mod store;
#[path = "hunts_web.rs"]
pub mod web;

/// No game feature is enabled by constructing default configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub read: bool,
    pub party: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("feature unavailable")]
    Unavailable,
    #[error("session not found")]
    NotFound,
    #[error("invalid hunt evidence")]
    Invalid,
    #[error("storage unavailable")]
    Storage,
}

/// A current request-scoped decision from T72, never a browser-supplied claim.
/// Unknown privacy, transfer, restore, retention or stream state produces no permit.
#[derive(Debug, Clone)]
pub struct Permit {
    account: Uuid,
    generation: Uuid,
    character_revision: u64,
    now: i64,
}
impl Permit {
    /// Integration seam for T72. Call only after authoritative privacy/ownership
    /// and stream reconciliation; public profiles cannot authorize this decision.
    pub fn from_verified_journal(
        account: Uuid,
        generation: Uuid,
        character_revision: u64,
        now: i64,
    ) -> Result<Self, Error> {
        if !v7(account) || !v7(generation) || character_revision == 0 || now <= 0 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            account,
            generation,
            character_revision,
            now,
        })
    }
}

fn v7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn key(value: &str) -> bool {
    let Some((family, name)) = value.split_once(':') else {
        return false;
    };
    value.len() <= 512
        && !family.is_empty()
        && !name.is_empty()
        && family.as_bytes()[0].is_ascii_alphanumeric()
        && family
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b))
}

/// Normalized, complete session projection, not an ingest envelope. Counters are
/// signed to match PostgreSQL bigint and reject negative measurements explicitly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub session_id: Uuid,
    pub character_id: Uuid,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_seconds: i64,
    pub experience_gained: i64,
    pub experience_lost: i64,
    pub places: BTreeMap<String, i64>,
    pub loot: BTreeMap<String, i64>,
    pub supplies: BTreeMap<String, i64>,
    pub damage_dealt: Vec<Damage>,
    pub damage_taken: Vec<Damage>,
    pub healing_done: i64,
    pub healing_received: i64,
    pub monsters_killed: BTreeMap<String, i64>,
    pub party: Option<Party>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Damage {
    pub damage_type_key: String,
    pub source_kind: Source,
    pub source_key: Option<String>,
    pub amount: i64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Source {
    #[serde(rename = "SELF")]
    SelfActor,
    Monster,
    Player,
    Environment,
    Unknown,
}

/// Anonymous slots only. This is an advisory ledger, never a payment command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Party {
    pub party_id: Uuid,
    pub subject_slot: u8,
    pub members: BTreeMap<u8, BTreeMap<String, i64>>,
}

fn rows(rows: &BTreeMap<String, i64>, max: usize, zero: bool) -> bool {
    rows.len() <= max && rows.iter().all(|(k, n)| key(k) && *n >= i64::from(!zero))
}
fn damage(rows: &[Damage]) -> bool {
    let mut seen = BTreeSet::new();
    rows.len() <= 8192
        && rows.iter().all(|d| {
            key(&d.damage_type_key)
                && d.amount > 0
                && match (&d.source_kind, &d.source_key) {
                    (Source::Monster, Some(k)) => key(k),
                    (Source::Monster, None) | (_, Some(_)) => false,
                    (_, None) => true,
                }
                && seen.insert((&d.damage_type_key, d.source_kind, &d.source_key))
        })
}
impl Session {
    pub fn validate(&self) -> Result<(), Error> {
        let sum = self
            .places
            .values()
            .try_fold(0_i64, |a, b| a.checked_add(*b));
        if !v7(self.session_id)
            || !v7(self.character_id)
            || self.started_at <= 0
            || self.ended_at > 253402300799
            || self.ended_at < self.started_at
            || self.duration_seconds < 0
            || self.duration_seconds > self.ended_at - self.started_at
            || self.experience_gained < 0
            || self.experience_lost < 0
            || self.healing_done < 0
            || self.healing_received < 0
            || !rows(&self.places, 256, true)
            || sum != Some(self.duration_seconds)
            || !rows(&self.loot, 4096, false)
            || !rows(&self.supplies, 4096, false)
            || !rows(&self.monsters_killed, 4096, false)
            || !damage(&self.damage_dealt)
            || !damage(&self.damage_taken)
        {
            return Err(Error::Invalid);
        }
        if let Some(party) = &self.party
            && (!v7(party.party_id)
                || party.members.is_empty()
                || party.members.len() > 64
                || party.members.keys().any(|slot| *slot > 63)
                || party.members.get(&party.subject_slot) != Some(&self.loot)
                || party.members.values().any(|r| !rows(r, 4096, false)))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    /// Floor of the exact integer rate. Unknown duration never fabricates zero.
    pub fn experience_per_hour(&self) -> Option<i128> {
        rate(i128::from(self.experience_gained), self.duration_seconds)
    }
}
fn rate(value: i128, seconds: i64) -> Option<i128> {
    (seconds > 0).then(|| value * 3600 / i128::from(seconds))
}

/// T74 supplies per-item values in the smallest gold unit with reproducible
/// provenance. A missing price makes the entire monetary result unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub unit_gold: i64,
    pub provenance: String,
}
pub trait Pricing: Send + Sync {
    fn quote(&self, item: &str, character: Uuid) -> Option<Quote>;
}
#[derive(Debug)]
pub struct Unpriced;
impl Pricing for Unpriced {
    fn quote(&self, _: &str, _: Uuid) -> Option<Quote> {
        None
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valuation {
    pub loot: i128,
    pub supplies: i128,
    pub balance: i128,
    pub profit_per_hour: Option<i128>,
    pub provenance: BTreeSet<String>,
}
fn value(
    rows: &BTreeMap<String, i64>,
    character: Uuid,
    prices: &dyn Pricing,
    provenance: &mut BTreeSet<String>,
) -> Option<i128> {
    rows.iter().try_fold(0_i128, |total, (item, quantity)| {
        let quote = prices.quote(item, character)?;
        if !(0..=999_999_999_999).contains(&quote.unit_gold) || quote.provenance.is_empty() {
            return None;
        }
        provenance.insert(quote.provenance);
        total.checked_add(i128::from(quote.unit_gold) * i128::from(*quantity))
    })
}
pub fn valuation(session: &Session, prices: &dyn Pricing) -> Option<Valuation> {
    session.validate().ok()?;
    let mut provenance = BTreeSet::new();
    let loot = value(&session.loot, session.character_id, prices, &mut provenance)?;
    let supplies = value(
        &session.supplies,
        session.character_id,
        prices,
        &mut provenance,
    )?;
    let balance = loot - supplies;
    Some(Valuation {
        loot,
        supplies,
        balance,
        profit_per_hour: rate(balance, session.duration_seconds),
        provenance,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub from_slot: u8,
    pub to_slot: u8,
    pub gold: i128,
}
/// Equal split with deterministic remainder assignment to ascending slots.
/// Party supplies are absent in G-9: this splits gross loot only, never net profit.
pub fn split_loot(
    session: &Session,
    flags: Flags,
    prices: &dyn Pricing,
) -> Result<Vec<Transfer>, Error> {
    if !flags.read || !flags.party {
        return Err(Error::Unavailable);
    }
    session.validate()?;
    let party = session.party.as_ref().ok_or(Error::Invalid)?;
    let mut balances = Vec::new();
    let mut total = 0_i128;
    for (slot, items) in &party.members {
        let gold = value(items, session.character_id, prices, &mut BTreeSet::new())
            .ok_or(Error::Unavailable)?;
        total = total.checked_add(gold).ok_or(Error::Invalid)?;
        balances.push((*slot, gold));
    }
    let count = balances.len() as i128;
    let equal = total / count;
    let remainder = total % count;
    let mut debtors = Vec::new();
    let mut creditors = Vec::new();
    for (index, (slot, held)) in balances.into_iter().enumerate() {
        let target = equal + i128::from((index as i128) < remainder);
        if held > target {
            debtors.push((slot, held - target));
        }
        if held < target {
            creditors.push((slot, target - held));
        }
    }
    let mut transfers = Vec::new();
    let (mut debtor, mut creditor) = (0, 0);
    while debtor < debtors.len() && creditor < creditors.len() {
        let amount = debtors[debtor].1.min(creditors[creditor].1);
        transfers.push(Transfer {
            from_slot: debtors[debtor].0,
            to_slot: creditors[creditor].0,
            gold: amount,
        });
        debtors[debtor].1 -= amount;
        creditors[creditor].1 -= amount;
        if debtors[debtor].1 == 0 {
            debtor += 1;
        }
        if creditors[creditor].1 == 0 {
            creditor += 1;
        }
    }
    Ok(transfers)
}

/// Strictly bounded list filters; timestamps are UTC Unix seconds, end exclusive.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    #[serde(default, deserialize_with = "empty_option")]
    pub character: Option<Uuid>,
    #[serde(default, deserialize_with = "empty_option")]
    pub place: Option<String>,
    #[serde(default, deserialize_with = "empty_option")]
    pub from: Option<i64>,
    #[serde(default, deserialize_with = "empty_option")]
    pub until: Option<i64>,
    #[serde(default, deserialize_with = "empty_option")]
    pub party: Option<bool>,
    #[serde(default, deserialize_with = "empty_option")]
    pub offset: Option<i64>,
}
fn empty_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = String::deserialize(deserializer)?;
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

impl Filter {
    pub fn validate(&self) -> Result<(), Error> {
        if self.character.is_some_and(|id| !v7(id))
            || self.place.as_ref().is_some_and(|p| !key(p))
            || self.from.is_some_and(|n| n < 1)
            || self.until.is_some_and(|n| !(1..=253402300799).contains(&n))
            || matches!((self.from, self.until), (Some(a), Some(b)) if a >= b)
            || self.offset.is_some_and(|n| !(0..=10000).contains(&n))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub experience_per_hour: Vec<Option<i128>>,
    pub valuations: Vec<Option<Valuation>>,
}
pub fn compare(sessions: &[Session], prices: &dyn Pricing) -> Result<Comparison, Error> {
    if !(2..=8).contains(&sessions.len()) {
        return Err(Error::Invalid);
    }
    let mut seen = BTreeSet::new();
    for session in sessions {
        session.validate()?;
        if !seen.insert(session.session_id) {
            return Err(Error::Invalid);
        }
    }
    Ok(Comparison {
        experience_per_hour: sessions.iter().map(Session::experience_per_hour).collect(),
        valuations: sessions.iter().map(|s| valuation(s, prices)).collect(),
    })
}

#[cfg(test)]
#[path = "hunts_tests.rs"]
mod tests;
