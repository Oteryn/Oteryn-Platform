use sqlx::PgConnection;
use std::future::Future;
use time::OffsetDateTime;
use uuid::Uuid;

/// Runtime money flag. All APIs refuse while disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct WalletFlags {
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Inactive,
    Unknown,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPermission {
    Allowed,
    Blocked,
    Unknown,
}
/// Read authoritative state in the wallet transaction. Lock identity state when
/// needed to serialize security changes. Missing/ambiguous/failed reads are Unknown.
pub trait AccountStateReader: Send + Sync {
    fn state(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> impl Future<Output = AccountState> + Send;
}
/// Check both parties for identity restrictions, including D14 cooldowns.
pub trait TransferBlockCheck: Send + Sync {
    fn permission(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> impl Future<Output = TransferPermission> + Send;
}
/// Verify actor authorization and persist auditing in this transaction. External
/// effects require an outbox. False denies; replay does not repeat the audit.
pub trait AuditHook: Send + Sync {
    fn adjustment(
        &self,
        connection: &mut PgConnection,
        event: &AdjustmentAudit,
    ) -> impl Future<Output = bool> + Send;
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Balance {
    /// There is one coin class: every credit adds to and every spend, reserve
    /// and transfer takes from this amount.
    pub available: i64,
    pub reserved: i64,
}
impl Balance {
    pub(crate) fn apply(self, available: i64, reserved: i64) -> Result<Self, Error> {
        let available = self
            .available
            .checked_add(available)
            .ok_or(Refusal::Overflow)?;
        let reserved = self
            .reserved
            .checked_add(reserved)
            .ok_or(Refusal::Overflow)?;
        if available < 0 || reserved < 0 {
            return Err(Refusal::InsufficientFunds.into());
        }
        available.checked_add(reserved).ok_or(Refusal::Overflow)?;
        Ok(Self {
            available,
            reserved,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("wallet feature is disabled")]
    Disabled,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("invalid identifier")]
    InvalidIdentifier,
    #[error("sender and recipient must differ")]
    SameAccount,
    #[error("account state does not permit wallet access")]
    AccountDenied,
    #[error("account does not exist")]
    UnknownAccount,
    #[error("transfer is blocked")]
    TransferBlocked,
    #[error("transfer permission is unknown")]
    TransferStateUnknown,
    #[error("insufficient available balance")]
    InsufficientFunds,
    #[error("balance exceeds bigint range")]
    Overflow,
    #[error("idempotency key was used with a different payload")]
    IdempotencyConflict,
    #[error("adjustment requires a reason of 1 to 1000 characters")]
    InvalidReason,
    #[error("adjustment audit or actor authorization was denied")]
    AuditDenied,
    #[error("reservation is missing or belongs to another account")]
    ReservationMissing,
    #[error("reservation has already been settled")]
    ReservationSettled,
    #[error("history limit must be between 1 and 100")]
    InvalidPage,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Refused(#[from] Refusal),
    #[error("wallet storage operation failed")]
    Storage(#[from] sqlx::Error),
}
/// Retain a UUIDv7 operation key for each intended mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub operation_id: Uuid,
    pub replayed: bool,
    pub kind: String,
    pub account_id: Uuid,
    pub counterparty_account_id: Option<Uuid>,
    pub amount: i64,
    pub available_after: i64,
    pub reserved_after: i64,
    pub occurred_at: OffsetDateTime,
}
#[derive(Debug, Clone)]
pub struct AdjustmentAudit {
    pub operation_id: Uuid,
    pub account_id: Uuid,
    pub actor_id: Uuid,
    pub amount: i64,
    pub reason: String,
    pub before: Balance,
    pub after: Balance,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub counterparty_account_id: Option<Uuid>,
    pub id: Uuid,
    pub operation_id: Uuid,
    pub kind: String,
    pub available_delta: i64,
    pub reserved_delta: i64,
    pub created_at: OffsetDateTime,
}
/// Descending keyset pagination; the last entry ID is the next cursor.
#[derive(Debug, Clone, Copy)]
pub struct HistoryPage {
    pub before: Option<Uuid>,
    pub limit: u16,
}
impl Default for HistoryPage {
    fn default() -> Self {
        Self {
            before: None,
            limit: 25,
        }
    }
}
/// Account creation and row locking use this order for all multi-account effects.
pub fn lock_order(first: Uuid, second: Uuid) -> [Uuid; 2] {
    if first < second {
        [first, second]
    } else {
        [second, first]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balance_checks_negative_and_overflow() -> Result<(), Error> {
        let balance = Balance::default().apply(10, 0)?;
        assert_eq!(
            balance,
            Balance {
                available: 10,
                reserved: 0
            }
        );
        assert!(matches!(
            balance.apply(-11, 0),
            Err(Error::Refused(Refusal::InsufficientFunds))
        ));
        assert!(matches!(
            balance.apply(i64::MAX, 0),
            Err(Error::Refused(Refusal::Overflow))
        ));
        assert!(matches!(
            Balance::default().apply(i64::MAX, 1),
            Err(Error::Refused(Refusal::Overflow))
        ));
        Ok(())
    }
    #[test]
    fn opposite_transfers_have_identical_lock_order() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        assert_eq!(lock_order(a, b), [a, b]);
        assert_eq!(lock_order(a, b), lock_order(b, a));
        assert!(!WalletFlags::default().enabled);
    }
}

/// Classification for the existing caller-owned adjustment API. Opt-in only.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentKind {
    #[default]
    Operator,
    StorePurchase,
    TopUp,
}
impl AdjustmentKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Operator => "adjustment",
            Self::StorePurchase => "store_purchase",
            Self::TopUp => "top_up",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct NumericHistoryPage {
    pub before: u64,
    pub limit: u8,
}
#[derive(Debug, Clone)]
pub struct NumericHistoryEntry {
    pub sequence: i64,
    pub operation_id: Uuid,
    pub kind: String,
    pub available_delta: i64,
    pub reserved_delta: i64,
    pub created_at: OffsetDateTime,
}

/// Audited source of a granted credit to the one coin balance.
#[derive(Debug, Clone, Copy)]
pub enum GrantKind {
    Reward,
    StaffGrant,
}
impl GrantKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Reward => "reward",
            Self::StaffGrant => "staff_grant",
        }
    }
}
