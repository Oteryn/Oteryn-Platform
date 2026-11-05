//! Default-off restore reconciliation. Mount `router` instead of the bare command
//! router: it composes the public command API with the restore serialization lock.
//! Use a pool with at least two connections. No account or Game outcome is inferred.
mod http;
mod storage;
mod wire;
pub use http::router;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub const GROUP: &str = "OTERYN_GAME_RESTORE_NOTICE";
pub const PATH: &str = "/internal/v1/game-auth/game-restore/notice";

#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub source_authority: String,
    pub requests_per_minute: u32,
    /// Set only after validating the command exchange environment and identities.
    /// No issuance flag is changed by enabling the exchange during recovery.
    pub exchange_available: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            source_authority: String::new(),
            requests_per_minute: 60,
            exchange_available: false,
        }
    }
}
impl Settings {
    fn available(&self) -> bool {
        self.enabled
            && wire::authority(&self.source_authority)
            && (1..=600).contains(&self.requests_per_minute)
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid restore notice")]
    BadInput,
    #[error("restore source is not authorized")]
    Unauthorized,
    #[error("restore conflict requires reconciliation")]
    Conflict,
    #[error("restore reconciliation unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Debug, Clone)]
pub struct Restore {
    pool: PgPool,
    settings: Arc<Settings>,
}
impl Restore {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self {
            pool,
            settings: Arc::new(settings),
        }
    }
    /// Accept a trusted notice. Network callers must use the dedicated guarded router.
    pub async fn notice(&self, raw: &[u8]) -> Result<String, Error> {
        if !self.settings.available() {
            return Err(Error::Unavailable);
        }
        let notice = wire::decode(raw)?;
        use subtle::ConstantTimeEq;
        if !bool::from(
            notice
                .source
                .as_bytes()
                .ct_eq(self.settings.source_authority.as_bytes()),
        ) {
            return Err(Error::Unauthorized);
        }
        self.record(notice, raw).await
    }
    /// Staff service API: unresolved alarms only. This is deliberately not an HTTP surface.
    pub async fn queue(&self) -> Result<Vec<QueueItem>, Error> {
        storage::queue(&self.pool).await
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: i64,
    pub event_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub reason: String,
}
#[cfg(test)]
mod tests;
