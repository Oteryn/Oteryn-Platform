//! G-7 preparation and PLAT3 candidate consumer. Stored intents are not executable authority.
//!
//! The candidate v2 receipt consumer is separately gated by AuthorityConfig.
//! The original preparation settings cannot activate it; no receipt is synthesized and
//! no timeout releases custody or money. Existing Character transfer bytes remain
//! owned by the Character command API; a digest here cannot authorize a transfer.
mod game_snapshot;
mod game_store;
mod game_wire;
pub use game_snapshot::{SnapshotBinding, verify_snapshot};
pub use game_store::{Outbox, StagedCommand};
pub use game_wire::{Command, PreparedIntent, gold_payout};

/// Local preparation is separate from game dispatch. Both default off.
#[derive(Debug, Clone, Copy, Default)]
pub struct Settings {
    pub preparation_enabled: bool,
    pub game_enabled: bool,
}
impl Settings {
    /// No accepted capability advertisement or result-data wire exists in rev 2.
    /// Setting a flag alone cannot promote a proposal to executable authority.
    pub fn dispatch_available(self) -> bool {
        false
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bazaar game integration unavailable")]
    Unavailable,
    #[error("invalid bazaar contract input")]
    Invalid,
    #[error("bazaar authorization denied")]
    Denied,
    #[error("immutable bazaar binding conflict")]
    Conflict,
    #[error("bazaar operation not found")]
    NotFound,
    #[error("bazaar command backlog full")]
    Backlog,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod game_tests;

mod game_projection;
pub use game_projection::IngestResult;

mod authority_wire;
pub use authority_wire::Receipt;
mod authority;
pub use authority::{
    Authority, AuthorityConfig, BazaarAuthorityDecisionV1, DecisionIdentity,
    PINNED_CONTRACT_SHA256, ReceiptResult, RecoveryFence,
};
mod coin_ports;
pub use coin_ports::{CoinReservations, PlatformAccounts};
mod authority_evidence;
mod authority_exchange;

#[cfg(test)]
mod authority_tests;
pub mod event_wire;

pub mod authority_http;
mod authority_items;
pub mod runtime;
