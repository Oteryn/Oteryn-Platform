//! Item advertisements and auctions. All effects use one caller-owned SQL transaction.
//! G-7 revision 2 is proposed: local outbox work is retained but never dispatched.
use std::future::Future;

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{Accounts, Currency, Error, Form, Kind, Listing, Reservations, State};

mod service;
#[cfg(test)]
mod tests;
pub mod web;
pub use service::Items;

/// The rollout flag controls local item operations only. No command dispatcher exists here.
#[derive(Debug, Clone, Copy, Default)]
pub struct ItemFlags {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Fixed,
    Offers,
    Buy,
    Auction,
}
impl Mode {
    fn form(self) -> Form {
        match self {
            Self::Auction => Form::Auction,
            Self::Buy => Form::WantToBuy,
            _ => Form::FixedPrice,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateItem {
    pub request_id: Uuid,
    pub mode: Mode,
    pub definition_key: String,
    pub definition_revision: String,
    pub world_id: Uuid,
    pub seller_character_id: Option<Uuid>,
    pub item_instance_id: Option<Uuid>,
    pub quantity: u32,
    /// Per-unit amount. Absence is allowed only for negotiated advertisements.
    pub unit_price: Option<i64>,
    pub buy_now_unit_price: Option<i64>,
    pub auction_days: Option<u8>,
    pub message: String,
}
impl CreateItem {
    pub fn validate(&self) -> Result<(), Error> {
        if !id(self.request_id)
            || !id(self.world_id)
            || self.quantity == 0
            || !key(&self.definition_key)
            || !key(&self.definition_revision)
            || self.message.chars().count() > 280
            || self.message.chars().any(char::is_control)
            || self.unit_price.is_some_and(|p| p <= 0)
            || self.buy_now_unit_price.is_some_and(|p| p <= 0)
        {
            return Err(Error::InvalidInput);
        }
        match self.mode {
            Mode::Auction
                if !matches!(self.auction_days, Some(1..=14)) || self.unit_price.is_none() =>
            {
                return Err(Error::InvalidInput);
            }
            Mode::Fixed if self.unit_price.is_none() => return Err(Error::InvalidInput),
            _ => {}
        }
        if (self.mode != Mode::Auction
            && (self.auction_days.is_some() || self.buy_now_unit_price.is_some()))
            || self
                .buy_now_unit_price
                .is_some_and(|p| Some(p) < self.unit_price)
            || (self.mode != Mode::Buy
                && (!self.seller_character_id.is_some_and(id)
                    || !self.item_instance_id.is_some_and(id)))
            || (self.mode == Mode::Buy
                && (self.seller_character_id.is_some() || self.item_instance_id.is_some()))
        {
            return Err(Error::InvalidInput);
        }
        self.total(self.unit_price.unwrap_or(1))?;
        if let Some(p) = self.buy_now_unit_price {
            self.total(p)?;
        }
        Ok(())
    }
    fn total(&self, price: i64) -> Result<i64, Error> {
        price
            .checked_mul(i64::from(self.quantity))
            .filter(|v| *v > 0)
            .ok_or(Error::Overflow)
    }
}
fn id(value: Uuid) -> bool {
    value.get_version_num() == 7
}
fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
}

/// An approved public projection, never raw Game facts or private proxy maxima.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ItemView {
    pub id: Uuid,
    pub name: String,
    pub definition_key: String,
    pub quantity: i64,
    pub unit_price: Option<i64>,
    pub total_price: i64,
    pub buy_now_unit_price: Option<i64>,
    pub attributes: sqlx::types::Json<Vec<(String, String)>>,
    pub message: String,
    pub ends_at: OffsetDateTime,
    pub form: Form,
    pub accept_offers: bool,
}

/// T58's transaction-local seam. Implementations must lock and validate current policy,
/// watermark (30s/1s), owner/privacy floors, epoch and recovery generation. Unknown denies.
/// Raw receipt decoding belongs to T58; this does not invent a Bazaar receipt schema.
pub trait ItemAuthority: Send + Sync {
    /// Default-deny terminal cancellation/expiry without authoritative return.
    fn cancellation_ready(
        &self,
        _db: &mut PgConnection,
        _listing: &Listing,
    ) -> impl Future<Output = Result<bool, Error>> + Send {
        async { Ok(false) }
    }

    fn preflight(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        input: &CreateItem,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
    /// Returns only an approved immutable public projection after matching escrow receipt,
    /// definition/quantity/attributes and current owner/world. Buy notices need policy only.
    fn publication(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> impl Future<Output = Result<Option<Publication>, Error>> + Send;
    /// Locks current authority fences and confirms publication is still valid.
    fn current(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
    /// A buy-order seller response must prove its own exact deposited asset.
    fn offer_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        offer: Uuid,
        seller: Uuid,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}
#[derive(Debug, Clone)]
pub struct Publication {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub escrow_id: Option<Uuid>,
    pub escrow_receipt_digest: Option<String>,
    pub projection_epoch: String,
    pub recovery_generation: String,
}
impl Publication {
    fn validate(&self, buy: bool) -> Result<(), Error> {
        let decimal = |s: &str| s.parse::<u64>().is_ok_and(|n| n.to_string() == s);
        if self.name.is_empty()
            || self.name.len() > 96
            || self.name.chars().any(char::is_control)
            || !decimal(&self.projection_epoch)
            || !decimal(&self.recovery_generation)
            || self.attributes.len() > 6
            || self.attributes.windows(2).any(|p| p[0].0 >= p[1].0)
            || self.attributes.iter().any(|(k, v)| {
                let max = match k.as_str() {
                    "tier" | "upgrade_level" => 255,
                    "charges" => u32::MAX as u64,
                    "attack" | "defense" | "armor" => 65535,
                    _ => return true,
                };
                !v.parse::<u64>()
                    .is_ok_and(|n| n <= max && n.to_string() == *v)
            })
            || (!buy
                && (!self.escrow_id.is_some_and(id)
                    || !self.escrow_receipt_digest.as_ref().is_some_and(|s| {
                        s.len() == 64
                            && s.bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })))
            || (buy
                && (self.escrow_id.is_some()
                    || self.escrow_receipt_digest.is_some()
                    || !self.attributes.is_empty()))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}

/// No inferred capabilities: the composing application cannot enable mutations until
/// the missing integration is replaced with a qualified T58 implementation.
#[derive(Debug, Clone, Copy)]
pub struct Unavailable;
impl ItemAuthority for Unavailable {
    async fn preflight(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: &CreateItem,
    ) -> Result<bool, Error> {
        Ok(false)
    }
    async fn publication(
        &self,
        _: &mut PgConnection,
        _: &Listing,
    ) -> Result<Option<Publication>, Error> {
        Ok(None)
    }
    async fn current(&self, _: &mut PgConnection, _: &Listing) -> Result<bool, Error> {
        Ok(false)
    }
    async fn offer_ready(
        &self,
        _: &mut PgConnection,
        _: &Listing,
        _: Uuid,
        _: Uuid,
    ) -> Result<bool, Error> {
        Ok(false)
    }
}
