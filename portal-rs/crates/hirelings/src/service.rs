use oteryn_db::outbox::{PgPool, Uuid};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn refused() -> Box<dyn std::error::Error + Send + Sync> {
    "hireling operation refused or invalid".into()
}
/// Both switches default off. No worker for this command kind is registered.
#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub enabled: bool,
    pub command_outbox: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Profession {
    Miner,
    Herbalist,
    Fisher,
    Merchant,
}
/// Supplied by a reviewed catalogue; never inferred from existing house services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub item: String,
    pub minimum: u32,
    pub maximum: u32,
    pub chance_ppm: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub key: String,
    pub profession: Profession,
    pub hours: u8,
    pub weekly_limit: i32,
    pub rewards: Vec<Reward>,
}
impl Destination {
    pub fn validate(&self) -> Result<()> {
        if self.key.is_empty()
            || self.key.len() > 100
            || !(1..=12).contains(&self.hours)
            || self.weekly_limit <= 0
            || self.rewards.is_empty()
            || self.rewards.len() > 100
            || self.rewards.iter().any(|r| {
                r.item.is_empty()
                    || r.item.len() > 100
                    || r.minimum > r.maximum
                    || r.maximum == 0
                    || !(1..=1_000_000).contains(&r.chance_ppm)
            })
        {
            return Err(refused());
        }
        Ok(())
    }
}
/// Adapter must verify active account, current house ownership and committed store
/// entitlement, including the profession. Unknown or stale evidence returns false.
pub trait Eligibility: std::fmt::Debug + Send + Sync {
    fn active_account(&self, _account: Uuid) -> bool {
        false
    }
    fn allowed(
        &self,
        account: Uuid,
        house: Uuid,
        entitlement: Uuid,
        profession: Profession,
    ) -> bool;
}
#[derive(Debug)]
pub struct DenyEligibility;
impl Eligibility for DenyEligibility {
    fn allowed(&self, _: Uuid, _: Uuid, _: Uuid, _: Profession) -> bool {
        false
    }
}
#[derive(Debug)]
pub struct Hirelings<E> {
    pub pool: PgPool,
    pub flags: Flags,
    /// Reviewed account-wide cap; zero denies expedition creation.
    pub account_weekly_limit: i32,
    pub eligibility: E,
}
#[derive(Debug, sqlx::FromRow)]
pub struct Hireling {
    pub id: Uuid,
    pub house_id: Uuid,
    pub profession: String,
    pub level: i32,
}
#[derive(Debug, sqlx::FromRow)]
pub struct Expedition {
    pub id: Uuid,
    pub hireling_id: Uuid,
    pub destination: String,
    pub state: String,
    pub reward_table: String,
}
impl<E: Eligibility> Hirelings<E> {
    /// Purchase is blocked until the Store grant mapping is accepted and implemented.
    pub async fn purchase(
        &self,
        _account: Uuid,
        _house: Uuid,
        _entitlement: Uuid,
        _profession: Profession,
    ) -> std::result::Result<Uuid, crate::commands::Reject> {
        Err(crate::commands::Reject::PurchaseBlocked)
    }
    pub async fn state(&self, account: Uuid) -> Result<Vec<Hireling>> {
        if !self.flags.enabled || !self.eligibility.active_account(account) || account.is_nil() {
            return Err(refused());
        }
        Ok(sqlx::query_as(
            "SELECT id,house_id,profession,level FROM hirelings WHERE account_id=$1 ORDER BY slot",
        )
        .bind(account)
        .fetch_all(&self.pool)
        .await?)
    }
    pub async fn history(&self, account: Uuid) -> Result<Vec<Expedition>> {
        if !self.flags.enabled || !self.eligibility.active_account(account) || account.is_nil() {
            return Err(refused());
        }
        Ok(sqlx::query_as("SELECT id,hireling_id,destination,state,reward_table::text AS reward_table FROM hireling_expeditions WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100").bind(account).fetch_all(&self.pool).await?)
    }
    /// The legacy local model cannot authorize a contract command; use `Commands`.
    pub async fn start(
        &self,
        _account: Uuid,
        _hireling: Uuid,
        _request: Uuid,
        _destination: &Destination,
    ) -> Result<Uuid> {
        Err(crate::commands::Reject::LegacyBoundaryUnavailable.into())
    }
}

/// The future private G-14 adapter must authenticate the producer and validate the
/// complete receipt before returning true. No network endpoint accepts this type.
pub trait ReceiptVerifier: std::fmt::Debug + Send + Sync {
    fn returned(&self, account: Uuid, expedition: Uuid, receipt: Uuid) -> bool;
}
#[derive(Debug)]
pub struct DenyReceipts;
impl ReceiptVerifier for DenyReceipts {
    fn returned(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        false
    }
}
impl<E: Eligibility> Hirelings<E> {
    /// Legacy return evidence is insufficient for the accepted command boundary.
    pub async fn record_return(
        &self,
        _account: Uuid,
        _expedition: Uuid,
        _receipt: Uuid,
        _verifier: &impl ReceiptVerifier,
        _notifications: &oteryn_notify::Notifications,
    ) -> Result<()> {
        Err(crate::commands::Reject::LegacyBoundaryUnavailable.into())
    }
    /// Collection requires the typed boundary and authoritative expedition evidence.
    pub async fn claim(&self, _account: Uuid, _expedition: Uuid) -> Result<Uuid> {
        Err(crate::commands::Reject::LegacyBoundaryUnavailable.into())
    }
}
