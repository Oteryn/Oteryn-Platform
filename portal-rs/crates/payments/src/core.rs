use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("payments are disabled")]
    Disabled,
    #[error("account is not eligible")]
    AccountDenied,
    #[error("invalid payment input")]
    InvalidInput,
    #[error("invalid webhook signature")]
    BadSignature,
    #[error("illegal payment transition")]
    IllegalTransition,
    #[error("payment not found")]
    NotFound,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

/// Explicit opt-in for all payment mutations.
#[derive(Debug, Clone, Copy, Default)]
pub struct PaymentFlags {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Restricted,
    Unknown,
    Ambiguous,
}

/// Implementations must resolve current authoritative state; failures must never grant access.
#[allow(async_fn_in_trait)]
pub trait AccountReader: Send + Sync {
    async fn state(&self, account_id: Uuid) -> Result<AccountState, Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Created,
    Pending,
    Succeeded,
    Failed,
    Refunded,
}

impl IntentState {
    pub fn transition(self, next: Self) -> Result<Self, Error> {
        match (self, next) {
            (Self::Created, Self::Pending | Self::Succeeded | Self::Failed)
            | (Self::Pending, Self::Succeeded | Self::Failed)
            | (Self::Succeeded, Self::Refunded) => Ok(next),
            _ => Err(Error::IllegalTransition),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Refunded => "refunded",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "created" => Ok(Self::Created),
            "pending" => Ok(Self::Pending),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "refunded" => Ok(Self::Refunded),
            _ => Err(Error::IllegalTransition),
        }
    }
}

/// Integer minor units, never floating point. No exchange rate is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Amount {
    pub minor_units: i64,
    pub currency: String,
}

impl Amount {
    pub fn validate(&self) -> Result<(), Error> {
        if self.minor_units <= 0
            || self.currency.len() != 3
            || !self.currency.bytes().all(|c| c.is_ascii_uppercase())
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub id: Uuid,
    pub account_id: Uuid,
    pub amount: Amount,
    pub state: IntentState,
    pub provider_reference: String,
}

/// Stable contract for the wallet handler. `amount` is the purchased minor-unit quantity.
/// The handler must deduplicate by intent_id and apply the configured conversion policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopUpJob {
    pub intent_id: Uuid,
    pub account_id: Uuid,
    pub amount: i64,
    pub currency: String,
}

pub(crate) fn identifier(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
