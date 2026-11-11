//! Premium time core. Paid issuance is intentionally unavailable in version 1;
//! purchase settlement belongs to the store purchase task.
use uuid::Uuid;

#[path = "premium_http.rs"]
pub mod http;
pub mod repository;
#[path = "premium_wire.rs"]
pub mod wire;

pub const MAX_REVISION: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Invalid,
    Denied,
    UnknownAccount,
    Unavailable,
    Conflict,
    NotActive,
    Horizon,
}
impl From<Error> for oteryn_db::retry::TransactionError<Error> {
    fn from(error: Error) -> Self {
        Self::Application(error)
    }
}
impl From<oteryn_db::retry::TransactionError<Error>> for Error {
    fn from(error: oteryn_db::retry::TransactionError<Error>) -> Self {
        error.into_inner()
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "premium operation refused: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

/// Unknown and ambiguous identities must never issue authority or mutate time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Known,
    Unknown,
    Ambiguous,
}
/// Implementations read account authority through the same transaction. A future
/// identity adapter must lock its account row here to fence deletion/status races.
pub trait Accounts: Send + Sync + std::fmt::Debug {
    fn state<'a>(
        &'a self,
        account: Uuid,
        connection: &'a mut sqlx::PgConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AccountState, Error>> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entitlement {
    pub id: Uuid,
    pub revoked: bool,
    pub revision: i64,
    pub from: i64,
    pub until: i64,
}
impl Entitlement {
    pub fn validate(&self) -> Result<(), Error> {
        if self.id.get_version_num() != 7
            || self.id.get_variant() != uuid::Variant::RFC4122
            || !(1..=MAX_REVISION).contains(&self.revision)
            || self.from >= self.until
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub fn classify(&self, now: i64) -> &'static str {
        if self.revoked {
            "REVOKED"
        } else if now >= self.until {
            "EXPIRED"
        } else if now < self.from {
            "NOT_YET_EFFECTIVE"
        } else {
            "ACTIVE"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Grant(u16),
    Revoke,
}
/// The caller supplies verified session authority; UI/RBAC adapters own obtaining
/// these facts. No permission other than the exact premium permission is accepted.
#[derive(Debug, Clone)]
pub struct Operator {
    pub actor: Uuid,
    pub confirmed_mfa: bool,
    pub permission: String,
}
#[derive(Debug, Clone)]
pub struct Mutation {
    pub account: Uuid,
    pub request: Uuid,
    pub change: Change,
    pub reason: String,
}

pub fn transition(
    existing: Option<&Entitlement>,
    change: Change,
    now: i64,
) -> Result<Entitlement, Error> {
    if let Some(value) = existing {
        value.validate()?;
    }
    let revision = existing.map_or(1, |e| e.revision + 1);
    if revision > MAX_REVISION {
        return Err(Error::Unavailable);
    }
    match change {
        Change::Grant(days) => {
            if !(1..=366).contains(&days) {
                return Err(Error::Invalid);
            }
            let active = existing.filter(|e| e.classify(now) == "ACTIVE");
            let from = active.map_or(now, |e| e.from);
            let until = active
                .map_or(now, |e| e.until)
                .checked_add(i64::from(days) * 86400)
                .ok_or(Error::Unavailable)?;
            if until > now.checked_add(3660 * 86400).ok_or(Error::Unavailable)? {
                return Err(Error::Horizon);
            }
            Ok(Entitlement {
                id: existing.map_or_else(Uuid::now_v7, |e| e.id),
                revoked: false,
                revision,
                from,
                until,
            })
        }
        Change::Revoke => {
            let current = existing
                .filter(|e| e.classify(now) == "ACTIVE")
                .ok_or(Error::NotActive)?;
            Ok(Entitlement {
                id: current.id,
                revoked: true,
                revision,
                from: current.from,
                until: current.until.min(now).max(current.from + 1),
            })
        }
    }
}

#[cfg(test)]
#[path = "premium_tests.rs"]
mod tests;
