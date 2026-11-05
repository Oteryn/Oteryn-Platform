//! Private, non-authoritative account character projection. Reads require a live
//! watermark and explicit current account permission; this never proves ownership.
mod http;
mod store;
mod wire;

pub use http::{Settings, router};
pub use store::{
    AccountState, AccountStateLookup, AccountView, DenyAccounts, Projection, ReadState,
};
pub use wire::{
    AccountId, Availability, CharacterId, CharacterSummary, Snapshot, Watermark, WorldId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadInput,
    Unauthorized,
    Conflict,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "account character projection refused: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Accepted,
    Superseded,
}

#[cfg(test)]
mod tests;
