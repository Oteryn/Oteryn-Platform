//! G-7 revision 2 preparation boundary. Stored intents are not executable authority.
//!
//! The proposed CHAR_CMD result-data representation and capability advertisement
//! are unallocated. No staged command is listable, no receipt is synthesized and
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
    #[error("bazaar command backlog full")]
    Backlog,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod game_tests;

mod game_projection;
pub use game_projection::IngestResult;
