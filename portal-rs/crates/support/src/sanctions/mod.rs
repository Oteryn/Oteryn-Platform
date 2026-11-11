//! Platform-owned moderation decisions (plan D9). No game commands are emitted.
//!
//! Staff HTTP belongs to T51 and player history/appeals to T34b. Callers pass only
//! trusted authenticated staff context, never actor/MFA values from request bodies.
//! All times at this boundary are Unix milliseconds, stored as UTC timestamptz.
mod access;
mod store;

pub use access::{AdmissionAccounts, GuardedAdmission, require_game_access, ticket_transaction};
pub use store::Service;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct Features {
    pub sanctions: bool,
    pub game_access: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Warning,
    Restriction,
    Suspension,
}
impl Category {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "warning" => Ok(Self::Warning),
            "restriction" => Ok(Self::Restriction),
            "suspension" => Ok(Self::Suspension),
            _ => Err(Error::InvalidInput),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Restriction => "restriction",
            Self::Suspension => "suspension",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub account: Uuid,
    pub category: Category,
    pub public_reason: String,
    pub moderator_notes: Option<String>,
    pub effective_at_ms: i64,
    pub expires_at_ms: Option<i64>,
}
impl Decision {
    fn validate(&self) -> Result<(), Error> {
        valid_id(self.account)?;
        if self.public_reason.trim().is_empty()
            || self.public_reason.chars().count() > 4000
            || self
                .moderator_notes
                .as_ref()
                .is_some_and(|s| s.chars().count() > 8000)
            || self.effective_at_ms <= 0
            || self.effective_at_ms > 253_402_300_799_000
            || self
                .expires_at_ms
                .is_some_and(|end| end <= self.effective_at_ms || end > 253_402_300_799_000)
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub id: Uuid,
    pub version: i64,
    pub pending: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("feature unavailable")]
    Disabled,
    #[error("sanctions dependency unavailable")]
    Unavailable,
    #[error("access denied")]
    Denied,
    #[error("invalid sanction input")]
    InvalidInput,
    #[error("sanction not found")]
    NotFound,
    #[error("sanction changed; reload before continuing")]
    Conflict,
    #[error("sanction revision is stale")]
    StaleRevision,
    #[error("sanction revision has different intent")]
    RevisionConflict,
    #[error("sanction revision exhausted")]
    RevisionExhausted,
    #[error("sanctions storage unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("sanctions authorization unavailable")]
    Authorization(#[from] oteryn_admin::rbac::Error),
}
fn valid_id(id: Uuid) -> Result<(), Error> {
    if id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122 {
        Ok(())
    } else {
        Err(Error::InvalidInput)
    }
}

#[cfg(test)]
mod tests;

pub use access::admission_router;

/// Largest ADM safe integer. Independent of session generation and lock_version.
pub const MAX_SANCTION_REVISION: i64 = 9_007_199_254_740_991;

/// Trusted, authorized decision input for the future ADM dispatcher. Times are UTC
/// Unix milliseconds. The store supplies decision time from the database clock.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SanctionAction {
    Ban {
        expires_at_ms: Option<i64>,
    },
    Unban,
    Namelock {
        expires_at_ms: Option<i64>,
    },
    LiftNamelock,
    /// Player-visible; delivery belongs to the dispatcher.
    Warning,
    /// Staff-only; never affects admission or effective restrictions.
    Notation,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SanctionIntent {
    pub account: Uuid,
    pub sanction_id: Uuid,
    pub character: Option<Uuid>,
    pub action: SanctionAction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SanctionReceipt {
    pub sanction_id: Uuid,
    pub revision: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSanction {
    pub sanction_id: Uuid,
    pub revision: i64,
    pub expires_at_ms: Option<i64>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveState {
    pub revision: i64,
    pub bans: Vec<EffectiveSanction>,
    pub namelocks: Vec<EffectiveSanction>,
}

pub mod export;
