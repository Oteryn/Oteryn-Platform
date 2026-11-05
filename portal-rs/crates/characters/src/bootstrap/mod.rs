//! Private v3 reconciliation reads. This interface never issues or changes an intent.
mod http;
mod wire;

use crate::lcfa::{AccountId, AccountState, AccountStateLookup};
pub use http::router;
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub const PATH: &str = "/internal/v1/game-auth/character-bootstrap-intents/read";
pub const GROUP: &str = "character-bootstrap-intents";

/// Default-off feature switch. Mount only on the verified private TLS listener.
#[derive(Debug, Clone, Copy, Default)]
pub struct Settings {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid bootstrap request")]
    BadInput,
    #[error("bootstrap intent not found")]
    NotFound,
    #[error("bootstrap intent unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Clone)]
pub struct Reader {
    pool: PgPool,
    settings: Settings,
    accounts: Arc<dyn AccountStateLookup>,
}
impl Reader {
    pub fn new(pool: PgPool, settings: Settings, accounts: Arc<dyn AccountStateLookup>) -> Self {
        Self {
            pool,
            settings,
            accounts,
        }
    }

    /// Account authority is shared with LCFA; a projection cannot grant permission.
    pub async fn current(&self, operation: &str, now: i64) -> Result<Vec<u8>, Error> {
        if !self.settings.enabled || now < 0 {
            return Err(Error::Unavailable);
        }
        wire::uuid(operation).map_err(|_| Error::BadInput)?;
        // One statement observes both the immutable row and its authority consistently.
        let row = sqlx::query("SELECT i.*, a.last_source_revision, a.last_issuer_decision_id FROM character_bootstrap_intents i LEFT JOIN character_bootstrap_intent_authority a ON a.id = 1 WHERE i.operation_id = $1")
            .bind(operation).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let raw: String = row.try_get("intent_json")?;
        let payload = wire::stored(&row, raw.as_bytes())?;
        let revision = wire::decimal(wire::string(&payload, "source_revision")?, 1)?;
        let high = wire::decimal(&row.try_get::<String, _>("last_source_revision")?, 0)?;
        if high < revision
            || (high == revision
                && row.try_get::<String, _>("last_issuer_decision_id")?
                    != wire::string(&payload, "issuer_decision_id")?)
        {
            return Err(Error::Unavailable);
        }
        let issued = wire::decimal(wire::string(&payload, "issued_at_source")?, 0)?;
        let expires = wire::decimal(wire::string(&payload, "expires_at_source")?, 0)?;
        if issued > now {
            return Err(Error::Unavailable);
        }
        if expires <= now {
            return Err(Error::NotFound);
        }
        let account = AccountId::parse(wire::string(&payload, "account_id")?)
            .map_err(|_| Error::Unavailable)?;
        if self.accounts.state(account).await != AccountState::Active {
            return Err(Error::Unavailable);
        }
        Ok(raw.into_bytes())
    }
}

#[cfg(test)]
mod tests;
