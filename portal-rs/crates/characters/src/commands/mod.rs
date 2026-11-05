//! Durable, default-off character intents. Game pulls intents and owns execution.
//!
//! Callers must authorize the user or Bazaar saga and supply its immutable decision
//! ID. Account state is rechecked through `AccountStateReader`; projections never
//! establish ownership. Mount `router` only on the private verified TLS listener.
mod http;
mod storage;
mod wire;
pub use http::router;
use sqlx::PgPool;
use std::{future::Future, sync::Arc};
use uuid::Uuid;

pub const GROUP: &str = "OTERYN_GAME_CHARACTER_COMMANDS";
pub const ISSUER: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
pub const CREATE: &str = "CreateCharacter";
pub const TRANSFER: &str = "TransferCharacterOwnership";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Eligible,
    Denied,
    Unknown,
    Ambiguous,
}
/// Implement against current account authority. Unavailable or uncertain state
/// must return Unknown/Ambiguous, never Eligible.
pub trait AccountStateReader: Send + Sync {
    fn state(&self, account: Uuid) -> impl Future<Output = AccountState> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPurpose {
    ListingEscrow,
    Settlement,
    EscrowReturn,
}
impl TransferPurpose {
    fn wire(self) -> &'static str {
        match self {
            Self::ListingEscrow => "BAZAAR_LISTING_ESCROW",
            Self::Settlement => "BAZAAR_SETTLEMENT",
            Self::EscrowReturn => "BAZAAR_ESCROW_RETURN",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Create {
        account_id: Uuid,
        world_id: Uuid,
        name: String,
    },
    Transfer {
        character_id: Uuid,
        from_account_id: Uuid,
        to_account_id: Uuid,
        purpose: TransferPurpose,
    },
}
impl Intent {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Create { .. } => CREATE,
            Self::Transfer { .. } => TRANSFER,
        }
    }
    fn accounts(&self) -> (Uuid, Option<Uuid>) {
        match self {
            Self::Create { account_id, .. } => (*account_id, None),
            Self::Transfer {
                from_account_id,
                to_account_id,
                ..
            } => (*from_account_id, Some(*to_account_id)),
        }
    }
    fn validate(&self) -> Result<(), Error> {
        let (account, target) = self.accounts();
        wire::uuid(&account.to_string())?;
        if let Some(target) = target {
            wire::uuid(&target.to_string())?;
            if target == account {
                return Err(Error::BadInput);
            }
        }
        match self {
            Self::Create { world_id, name, .. } => {
                wire::uuid(&world_id.to_string())?;
                wire::name(name)?;
            }
            Self::Transfer { character_id, .. } => {
                wire::uuid(&character_id.to_string())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub create_enabled: bool,
    pub transfer_enabled: bool,
    pub source_authority: String,
    pub ttl_seconds: u16,
    pub max_pending: u16,
    /// Shared across the dedicated identity group; zero fails closed.
    pub requests_per_minute: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            create_enabled: false,
            transfer_enabled: false,
            source_authority: String::new(),
            ttl_seconds: 300,
            max_pending: 40,
            requests_per_minute: 120,
        }
    }
}
impl Settings {
    fn available(&self) -> bool {
        self.enabled
            && wire::authority(&self.source_authority).is_ok()
            && (1..=300).contains(&self.ttl_seconds)
            && (1..=40).contains(&self.max_pending)
            && self.requests_per_minute > 0
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid character command input")]
    BadInput,
    #[error("character commands unavailable")]
    Unavailable,
    #[error("account is not eligible")]
    Denied,
    #[error("character command conflict")]
    Conflict,
    #[error("character command not found")]
    NotFound,
    #[error("character command source not authorized")]
    Unauthorized,
    #[error("character command backlog full")]
    BacklogFull,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandStatus {
    pub operation_id: Uuid,
    pub command: String,
    pub intent_json: String,
    pub receipt_json: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Commands {
    pool: PgPool,
    settings: Arc<Settings>,
}
impl Commands {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self {
            pool,
            settings: Arc::new(settings),
        }
    }
    /// One key per authorized submission/saga step. An exact retry returns the
    /// original operation even after expiry or receipt; changed intent conflicts.
    pub async fn enqueue(
        &self,
        accounts: &impl AccountStateReader,
        decision_id: Uuid,
        key: &str,
        intent: Intent,
    ) -> Result<CommandStatus, Error> {
        if !self.settings.available()
            || !(match intent {
                Intent::Create { .. } => self.settings.create_enabled,
                Intent::Transfer { .. } => self.settings.transfer_enabled,
            })
        {
            return Err(Error::Unavailable);
        }
        wire::uuid(&decision_id.to_string())?;
        if key.is_empty() || key.len() > 191 || !key.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
            return Err(Error::BadInput);
        }
        intent.validate()?;
        let (account, target) = intent.accounts();
        if accounts.state(account).await != AccountState::Eligible {
            return Err(Error::Denied);
        }
        if let Some(target) = target
            && accounts.state(target).await != AccountState::Eligible
        {
            return Err(Error::Denied);
        }
        self.issue(decision_id, key, intent).await
    }
    /// Internal saga API; callers must apply object authorization before showing
    /// any returned private intent or receipt to a player.
    pub async fn status(&self, operation_id: Uuid) -> Result<CommandStatus, Error> {
        wire::uuid(&operation_id.to_string())?;
        self.load(operation_id).await
    }
}
#[cfg(test)]
mod tests;

pub mod services;
