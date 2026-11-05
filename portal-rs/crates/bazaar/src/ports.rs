use std::future::Future;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Currency, Error, Listing, Offer, Settlement};

/// Runtime flags must be explicitly enabled by the composing application.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    /// Gold requires a transactional escrow adapter; the core performs no game I/O.
    pub gold_enabled: bool,
}

/// Missing, stale, conflicting or unknown account information must return `None` or an error.
#[derive(Debug, Clone, Copy, Default)]
pub struct AccountState {
    pub active: bool,
    pub email_verified: bool,
    pub mfa: bool,
    pub sudo: bool,
    /// Includes the 72-hour email-change restriction and any other bazaar restriction.
    pub bazaar_blocked: bool,
}

impl AccountState {
    pub fn allows_trade(self) -> bool {
        self.active && self.email_verified && !self.bazaar_blocked
    }

    pub fn allows_listing(self) -> bool {
        self.allows_trade() && self.mfa && self.sudo
    }
}

pub trait Accounts: Send + Sync {
    /// Read and, where necessary, lock current account state in this transaction.
    fn state(
        &self,
        db: &mut PgConnection,
        account: Uuid,
    ) -> impl Future<Output = Result<Option<AccountState>, Error>> + Send;
}

#[derive(Debug, Clone, Copy)]
pub struct Reservation {
    /// Stable reservation identity: listing for an auction, offer for an advertisement.
    pub id: Uuid,
    pub account: Uuid,
    pub currency: Currency,
    pub amount: i64,
}

/// Production adapters MUST make all effects through `db`, without committing it or doing I/O.
/// Lock wallet rows in deterministic order; forbid negative balances and non-transferable coins.
/// Reserve sets the absolute hold, including when the current winner raises their maximum.
/// Every method must support replay. A SQL error must propagate, rolling back the whole action.
pub trait Reservations: Send + Sync {
    /// Lock all affected account rows in the supplied sorted order before any hold changes.
    fn lock_accounts(
        &self,
        db: &mut PgConnection,
        accounts: &[Uuid],
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn reserve(
        &self,
        db: &mut PgConnection,
        hold: Reservation,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn release(
        &self,
        db: &mut PgConnection,
        id: Uuid,
        account: Uuid,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Consume only the clearing price, release excess and put seller net into pending proceeds.
    /// The commission and seller credit belong to the same ledger transaction.
    fn settle(
        &self,
        db: &mut PgConnection,
        settlement: &Settlement,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Make pending seller proceeds available only after verified delivery.
    fn complete(
        &self,
        db: &mut PgConnection,
        settlement: &Settlement,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Transaction-local evidence supplied by the future game integration. Unknown evidence denies.
/// This interface reads durable verified evidence; it must never issue a game command here.
pub trait Evidence: Send + Sync {
    /// Verify the selected seller's deposited item, including a want-to-buy response.
    fn offer_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
        offer: &Offer,
    ) -> impl Future<Output = Result<bool, Error>> + Send;

    fn escrow_ready(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> impl Future<Output = Result<bool, Error>> + Send;

    fn delivered(
        &self,
        db: &mut PgConnection,
        settlement: &Settlement,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}
