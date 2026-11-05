//! Season passes use verified journal/portal facts and cosmetic entitlements only.
//! Definitions are immutable; browser requests cannot submit progress or prices.
mod repository;
pub mod web;
pub use repository::Passes;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub paid: bool,
    pub rewards: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("season pass is disabled")]
    Disabled,
    #[error("invalid season pass input")]
    Invalid,
    #[error("season pass access denied")]
    Denied,
    #[error("season pass operation conflicts")]
    Conflict,
    #[error("season pass dependency is unavailable")]
    Unavailable,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Journal,
    Portal,
}
impl Source {
    pub fn key(self) -> &'static str {
        match self {
            Self::Journal => "journal",
            Self::Portal => "portal",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: Uuid,
    pub source: Source,
    pub metric: String,
    pub target: i64,
    pub xp: i64,
}
#[derive(Debug, Clone)]
pub struct Reward {
    pub id: Uuid,
    pub paid: bool,
    pub xp: i64,
    pub grant_key: String,
    pub grant_revision: String,
}
#[derive(Debug, Clone)]
pub struct Season {
    pub id: Uuid,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub price: i64,
    pub challenges: Vec<Challenge>,
    pub rewards: Vec<Reward>,
}
impl Season {
    pub fn validate(&self) -> Result<()> {
        let days = (self.ends_at - self.starts_at).whole_seconds();
        if !valid_id(self.id)
            || !(42 * 86400..=56 * 86400).contains(&days)
            || self.price <= 0
            || self.challenges.is_empty()
            || self.challenges.len() > 200
            || self.rewards.is_empty()
            || self.rewards.len() > 200
        {
            return Err(Error::Invalid);
        }
        let mut ids = std::collections::HashSet::new();
        let mut total = 0_i64;
        for c in &self.challenges {
            if !valid_id(c.id) || !ids.insert(c.id) || !key(&c.metric) || c.target <= 0 || c.xp <= 0
            {
                return Err(Error::Invalid);
            }
            total = total.checked_add(c.xp).ok_or(Error::Invalid)?;
        }
        for r in &self.rewards {
            if !valid_id(r.id)
                || !ids.insert(r.id)
                || r.xp <= 0
                || r.xp > total
                || !key(&r.grant_key)
                || !wire_revision(&r.grant_revision)
            {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && (s.as_bytes()[0].is_ascii_lowercase() || s.as_bytes()[0].is_ascii_digit())
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._:-".contains(&c))
}
fn wire_revision(s: &str) -> bool {
    s.parse::<u64>()
        .is_ok_and(|value| value > 0 && value.to_string() == s)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: Uuid,
    pub account: Uuid,
    pub season: Uuid,
    pub challenge: Uuid,
    pub source: Source,
    pub metric: String,
    pub amount: i64,
    pub occurred_at: OffsetDateTime,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub xp: i64,
    pub paid: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claim {
    pub operation: Uuid,
    pub delivery: Uuid,
}

/// Trusted adapters must validate current account eligibility and hold the
/// ownership/erasure/restore fence until the caller transaction ends. Unknown
/// authority denies. Progress adapters verify immutable source bytes, custody,
/// completion, catalog revision and event identity, never browser assertions.
pub trait Authority: Send + Sync {
    fn account(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> impl Future<Output = Result<()>> + Send;
    fn publish(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
    ) -> impl Future<Output = Result<()>> + Send;
    fn event(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &Event,
    ) -> impl Future<Output = Result<()>> + Send;
}
/// The store adapter must atomically issue a nontransferable, cosmetic-only
/// entitlement and its DELIVER outbox through the store wire. The operation is
/// immutable/idempotent and its ID is not a Game delivery receipt. No local item
/// writes or direct Game writes are permitted. Until the store exposes this
/// issuance API and cosmetic authority read, the production adapter denies.
pub trait Rewards: Send + Sync {
    fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        operation: Uuid,
        reward: &Reward,
    ) -> impl Future<Output = Result<Uuid>> + Send;
}
pub trait Payment: Send + Sync {
    fn debit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        operation: Uuid,
        amount: i64,
    ) -> impl Future<Output = Result<()>> + Send;
}
#[derive(Debug)]
pub struct Wallet<A, B, H>(oteryn_wallet::Ledger<A, B, H>);
impl<
    A: oteryn_wallet::AccountStateReader,
    B: oteryn_wallet::TransferBlockCheck,
    H: oteryn_wallet::AuditHook,
> Wallet<A, B, H>
{
    pub fn new(ledger: oteryn_wallet::Ledger<A, B, H>) -> Self {
        Self(ledger.with_adjustment_kind(oteryn_wallet::AdjustmentKind::StorePurchase))
    }
}
impl<
    A: oteryn_wallet::AccountStateReader,
    B: oteryn_wallet::TransferBlockCheck,
    H: oteryn_wallet::AuditHook,
> Payment for Wallet<A, B, H>
{
    async fn debit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        operation: Uuid,
        amount: i64,
    ) -> Result<()> {
        if amount <= 0 {
            return Err(Error::Invalid);
        }
        self.0
            .adjust_in(
                tx,
                operation,
                account,
                -amount,
                account,
                &format!("store-purchase:{operation}"),
            )
            .await
            .map_err(|_| Error::Denied)?;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Unavailable;
impl Authority for Unavailable {
    async fn account(&self, _: &mut Transaction<'_, Postgres>, _: Uuid) -> Result<()> {
        Err(Error::Unavailable)
    }
    async fn publish(&self, _: &mut Transaction<'_, Postgres>, _: Uuid) -> Result<()> {
        Err(Error::Unavailable)
    }
    async fn event(&self, _: &mut Transaction<'_, Postgres>, _: &Event) -> Result<()> {
        Err(Error::Unavailable)
    }
}
impl Rewards for Unavailable {
    async fn enqueue(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: Uuid,
        _: Uuid,
        _: &Reward,
    ) -> Result<Uuid> {
        Err(Error::Unavailable)
    }
}
impl Payment for Unavailable {
    async fn debit(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: Uuid,
        _: Uuid,
        _: i64,
    ) -> Result<()> {
        Err(Error::Unavailable)
    }
}
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct ChallengeView {
    pub definition: Challenge,
    pub amount: i64,
}
#[derive(Debug, Clone)]
pub struct RewardView {
    pub operation: Uuid,
    pub definition: Reward,
    pub claimed: bool,
    pub eligible: bool,
}
#[derive(Debug, Clone)]
pub struct Overview {
    pub can_buy: bool,
    pub season: Uuid,
    pub ends_at: OffsetDateTime,
    pub price: i64,
    pub progress: Progress,
    pub challenges: Vec<ChallengeView>,
    pub rewards: Vec<RewardView>,
}
