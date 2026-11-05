use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const GOLD_MAX: u64 = 999_999_999_999;
pub const RETENTION: u64 = 30 * 86400;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid record")]
    Invalid,
    #[error("record conflicts with durable evidence")]
    Conflict,
    #[error("access denied")]
    Denied,
    #[error("projection unavailable")]
    Unavailable,
    #[error("storage unavailable")]
    Storage(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn counter(value: &str, zero: bool) -> Result<u64> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.starts_with('0') && value != "0")
        || (!zero && value == "0")
    {
        return Err(Error::Invalid);
    }
    value.parse().map_err(|_| Error::Invalid)
}
pub fn gold(value: &str, positive: bool) -> Result<u64> {
    let n = counter(value, !positive)?;
    if n > GOLD_MAX {
        return Err(Error::Invalid);
    }
    Ok(n)
}
pub fn id(value: &str) -> Result<()> {
    let id = Uuid::parse_str(value).map_err(|_| Error::Invalid)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != value
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub fn key(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
    {
        return Err(Error::Invalid);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct Cut {
    pub character_revision: String,
    pub from_projection_revision: String,
    pub to_projection_revision: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
}

/// Field declaration order is the canonical wire order in the proposed G-10.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub world_id: String,
    pub character_id: Option<String>,
    pub character_revision: Option<String>,
    pub from_projection_revision: Option<String>,
    pub to_projection_revision: Option<String>,
    pub projection_epoch: Option<String>,
    pub family: String,
    pub recovery_generation: String,
    pub source_revision: String,
    pub record_id: String,
    pub occurred_at: String,
    pub coverage_start_at: String,
    pub kind: String,
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Begin(Begin),
    Page(Page),
    End(End),
    Bank(Bank),
    Market(Market),
    Trade(Trade),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Begin {
    pub snapshot_id: String,
    pub character_revision: String,
    pub page_count: u32,
    pub item_count: u32,
    pub depots: Vec<u16>,
    pub houses: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub snapshot_id: String,
    pub page_index: u32,
    pub items: Vec<Item>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct End {
    pub snapshot_id: String,
    pub page_count: u32,
    pub item_count: u32,
    pub pages_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Root {
    Equipment,
    Depot,
    Inbox,
    House,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub item_instance_id: String,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub quantity: u32,
    pub root: Root,
    pub town_id: Option<u16>,
    pub depot_box: Option<u8>,
    pub house_id: Option<String>,
    pub spatial_position_hex: Option<String>,
    pub equipment_slot: Option<u8>,
    pub parent_item_instance_id: Option<String>,
    pub placement_ordinal: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bank {
    pub operation_occurrence_id: String,
    pub event_id: String,
    pub entry_id: String,
    pub previous_entry_id: Option<String>,
    pub kind: String,
    pub direction: String,
    pub amount: String,
    pub balance_before: String,
    pub balance_after: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Market {
    pub event_id: String,
    pub offer_id: String,
    pub side: String,
    pub state: String,
    pub end_state: Option<String>,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub amount: u32,
    pub remaining: u32,
    pub piece_price: String,
    pub fee_gold: String,
    pub held_credit_gold: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub item_instance_id: String,
    pub definition_family: String,
    pub definition_production_key: String,
    pub definition_revision_ref: String,
    pub quantity: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trade {
    pub occurrence_id: String,
    pub transaction_id: String,
    pub event_id: String,
    pub given: Vec<Line>,
    pub received: Vec<Line>,
}

impl Record {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 65536 || bytes.contains(&b'\\') {
            return Err(Error::Invalid);
        }
        let record: Self = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        if serde_json::to_vec(&record).map_err(|_| Error::Invalid)? != bytes {
            return Err(Error::Invalid);
        }
        record.validate()?;
        Ok(record)
    }
    pub fn cut(&self) -> Option<Cut> {
        Some(Cut {
            character_revision: self.character_revision.clone()?,
            from_projection_revision: self.from_projection_revision.clone()?,
            to_projection_revision: self.to_projection_revision.clone()?,
            projection_epoch: self.projection_epoch.clone()?,
            recovery_generation: self.recovery_generation.clone(),
        })
    }
    pub fn stream(&self) -> Result<String> {
        serde_json::to_string(&(
            &self.source_authority,
            &self.account_id,
            &self.world_id,
            &self.character_id,
            self.cut(),
            &self.family,
            &self.recovery_generation,
        ))
        .map_err(|_| Error::Invalid)
    }
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != 1 || self.operation != "PublishGameJournalRecordV1" {
            return Err(Error::Invalid);
        }
        key(&self.source_authority, 128)?;
        for v in [&self.account_id, &self.world_id, &self.record_id] {
            id(v)?;
        }
        for v in [
            &self.source_revision,
            &self.recovery_generation,
            &self.occurred_at,
            &self.coverage_start_at,
        ] {
            counter(v, false)?;
        }
        if counter(&self.coverage_start_at, false)? > counter(&self.occurred_at, false)? {
            return Err(Error::Invalid);
        }
        if self.family == "BANK" {
            if self.character_id.is_some()
                || self.character_revision.is_some()
                || self.from_projection_revision.is_some()
                || self.to_projection_revision.is_some()
                || self.projection_epoch.is_some()
            {
                return Err(Error::Invalid);
            }
        } else {
            id(self.character_id.as_deref().ok_or(Error::Invalid)?)?;
            let c = self.cut().ok_or(Error::Invalid)?;
            for v in [
                &c.character_revision,
                &c.from_projection_revision,
                &c.to_projection_revision,
                &c.projection_epoch,
            ] {
                counter(v, false)?;
            }
        }
        let (family, kind) = match &self.payload {
            Payload::Begin(b) => {
                id(&b.snapshot_id)?;
                if self.character_revision.as_ref() != Some(&b.character_revision)
                    || b.page_count > 16384
                    || b.item_count > 524288
                    || b.item_count < b.page_count
                    || b.item_count > 32 * b.page_count
                    || ((b.item_count == 0) != (b.page_count == 0))
                    || b.depots.len() > 256
                    || b.houses.len() > 256
                    || b.depots.windows(2).any(|w| w[0] >= w[1])
                    || b.houses.windows(2).any(|w| w[0] >= w[1])
                    || !b.houses.is_empty()
                {
                    return Err(Error::Invalid);
                }
                // House mapping and eligibility are unverified in revision 2. Deny.
                ("ITEMS", "ITEMS_BEGIN")
            }
            Payload::Page(p) => {
                id(&p.snapshot_id)?;
                if p.page_index >= 16384
                    || p.items.is_empty()
                    || p.items.len() > 32
                    || p.items
                        .windows(2)
                        .any(|w| w[0].item_instance_id >= w[1].item_instance_id)
                {
                    return Err(Error::Invalid);
                }
                for i in &p.items {
                    i.validate()?;
                }
                ("ITEMS", "ITEMS_PAGE")
            }
            Payload::End(e) => {
                id(&e.snapshot_id)?;
                if e.page_count > 16384
                    || e.item_count > 524288
                    || e.pages_sha256.len() != 64
                    || !e
                        .pages_sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(Error::Invalid);
                }
                ("ITEMS", "ITEMS_END")
            }
            Payload::Bank(b) => {
                b.validate()?;
                ("BANK", "BANK_ENTRY")
            }
            Payload::Market(m) => {
                m.validate()?;
                ("MARKET", "MARKET_EVENT")
            }
            Payload::Trade(t) => {
                t.validate()?;
                ("PLAYER_TRADE", "PLAYER_TRADE_EVENT")
            }
        };
        if self.family != family
            || self.kind != kind
            || (family == "ITEMS" && self.coverage_start_at != self.occurred_at)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
impl Item {
    pub fn validate(&self) -> Result<()> {
        id(&self.item_instance_id)?;
        definition(
            &self.definition_family,
            &self.definition_production_key,
            &self.definition_revision_ref,
        )?;
        if self.quantity == 0 {
            return Err(Error::Invalid);
        }
        counter(&self.placement_ordinal, false)?;
        if let Some(p) = &self.parent_item_instance_id {
            id(p)?;
        }
        if self.house_id.is_some()
            || self.spatial_position_hex.is_some()
            || self.root == Root::House
        {
            return Err(Error::Invalid);
        }
        match self.root {
            Root::Depot
                if self.town_id.is_some()
                    && self.depot_box.is_some_and(|b| (1..=17).contains(&b))
                    && self.equipment_slot.is_none() => {}
            Root::Equipment
                if self.town_id.is_none()
                    && self.depot_box.is_none()
                    && self.equipment_slot.is_some_and(|s| s > 0) => {}
            Root::Inbox
                if self.town_id.is_none()
                    && self.depot_box.is_none()
                    && self.equipment_slot.is_none() => {}
            _ => return Err(Error::Invalid),
        }
        Ok(())
    }
    fn coordinates(&self) -> String {
        format!(
            "{:?}/{:?}/{:?}/{:?}/{:?}/{:?}",
            self.root,
            self.town_id,
            self.depot_box,
            self.house_id,
            self.spatial_position_hex,
            self.equipment_slot
        )
    }
}
fn definition(f: &str, k: &str, r: &str) -> Result<()> {
    key(f, 128)?;
    key(k, 512)?;
    key(r, 512)
}
impl Bank {
    fn validate(&self) -> Result<()> {
        for v in [
            &self.operation_occurrence_id,
            &self.event_id,
            &self.entry_id,
        ] {
            id(v)?;
        }
        if let Some(v) = &self.previous_entry_id {
            id(v)?;
            if v == &self.entry_id {
                return Err(Error::Invalid);
            }
        }
        let a = gold(&self.amount, true)?;
        let before = gold(&self.balance_before, false)?;
        let after = gold(&self.balance_after, false)?;
        match (self.kind.as_str(), self.direction.as_str()) {
            ("DEPOSIT", "CREDIT") if a <= 20_000_000 => (),
            ("WITHDRAW", "DEBIT") if a <= 1_009_999 => (),
            ("TRANSFER", "CREDIT" | "DEBIT") => (),
            _ => return Err(Error::Invalid),
        }
        let result = if self.direction == "CREDIT" {
            before.checked_add(a)
        } else {
            before.checked_sub(a)
        };
        if result != Some(after) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
impl Market {
    fn validate(&self) -> Result<()> {
        id(&self.event_id)?;
        id(&self.offer_id)?;
        definition(
            &self.definition_family,
            &self.definition_production_key,
            &self.definition_revision_ref,
        )?;
        if !matches!(self.side.as_str(), "BUY" | "SELL")
            || self.amount == 0
            || self.amount > 64000
            || self.remaining > self.amount
        {
            return Err(Error::Invalid);
        }
        if u64::from(self.amount) * gold(&self.piece_price, true)? > GOLD_MAX
            || gold(&self.fee_gold, false)? > 2_500_000
        {
            return Err(Error::Invalid);
        }
        let held = gold(&self.held_credit_gold, false)?;
        match self.state.as_str() {
            "OPEN" | "FILLED" | "CANCELLED" | "EXPIRED"
                if self.end_state.is_none() && held == 0 => {}
            "CREDIT_HELD"
                if self
                    .end_state
                    .as_deref()
                    .is_some_and(|s| matches!(s, "FILLED" | "CANCELLED" | "EXPIRED")) => {}
            _ => return Err(Error::Invalid),
        }
        if (self.state == "FILLED" || self.end_state.as_deref() == Some("FILLED"))
            && self.remaining != 0
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
impl Trade {
    fn validate(&self) -> Result<()> {
        for v in [&self.occurrence_id, &self.transaction_id, &self.event_id] {
            id(v)?;
        }
        if self.given.len() != 1
            || self.received.len() != 1
            || self.given[0].item_instance_id == self.received[0].item_instance_id
        {
            return Err(Error::Invalid);
        }
        for l in self.given.iter().chain(&self.received) {
            id(&l.item_instance_id)?;
            definition(
                &l.definition_family,
                &l.definition_production_key,
                &l.definition_revision_ref,
            )?;
            if l.quantity == 0 || l.quantity > 100 {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
}

/// Validate the entire flattened custody graph before any snapshot becomes visible.
pub fn graph(begin: &Begin, items: &[Item]) -> Result<()> {
    if items.len() != begin.item_count as usize {
        return Err(Error::Conflict);
    }
    let by_id: HashMap<_, _> = items
        .iter()
        .map(|i| (i.item_instance_id.as_str(), i))
        .collect();
    if by_id.len() != items.len()
        || items
            .windows(2)
            .any(|w| w[0].item_instance_id >= w[1].item_instance_id)
    {
        return Err(Error::Invalid);
    }
    let mut positions = HashSet::new();
    for i in items {
        i.validate()?;
        if i.town_id.is_some_and(|t| !begin.depots.contains(&t)) {
            return Err(Error::Invalid);
        }
        if !positions.insert((
            i.coordinates(),
            &i.parent_item_instance_id,
            &i.placement_ordinal,
        )) {
            return Err(Error::Invalid);
        }
        let mut current = i;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(&current.item_instance_id) || visited.len() > 32 {
                return Err(Error::Invalid);
            }
            let Some(p) = &current.parent_item_instance_id else {
                break;
            };
            current = by_id.get(p.as_str()).copied().ok_or(Error::Invalid)?;
            if current.coordinates() != i.coordinates() {
                return Err(Error::Invalid);
            }
        }
    }
    Ok(())
}
