//! T94: non-authoritative, display-only consumer. No stakes, rewards or predictions.
mod admission;
pub mod config;
pub mod runtime;
pub mod schema;
mod store;
#[cfg(test)]
mod tests;
pub mod transport;
pub mod web;

use sqlx::{Postgres, Transaction};
use std::future::Future;
pub use store::League;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("league authority unavailable")]
    Unavailable,
    #[error("invalid league evidence")]
    Invalid,
    #[error("league publication superseded by the recovery fence")]
    Superseded,
    #[error("league decision conflict")]
    Conflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub ingest: bool,
    pub public: bool,
    pub accepted_sources: bool,
    pub accepted_season_rules: bool,
    pub accepted_final_freeze: bool,
    pub accepted_pki: bool,
}
impl Flags {
    fn ready(self) -> bool {
        self.enabled
            && self.accepted_sources
            && self.accepted_season_rules
            && self.accepted_final_freeze
            && self.accepted_pki
    }
}
/// The registered payload definitions, copied field-for-field from the pinned IDL.
#[derive(Clone, PartialEq, prost::Message)]
pub struct WorldEventOutcomeDecidedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub record: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub record_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub source_evidence_sha256: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub recovery_generation: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub subject_event_id: Vec<u8>,
}
#[derive(Clone, PartialEq, prost::Message)]
pub struct GuildSeasonStandingsDecidedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub record_sha256: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub recovery_generation: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub season_id: Vec<u8>,
}
/// Trusted adapter input, never an HTTP/browser form. Signature grammar/key custody
/// remain adapter-owned; the candidate Game contract specifies no signed HTTP wire.
#[derive(Debug)]
pub struct Delivery<'a> {
    pub envelope_id: Uuid,
    pub transaction_id: Uuid,
    pub ordinal: u32,
    pub count: u32,
    pub event_type: u32,
    /// Adapter-authenticated observation timestamp; anchors local audit expiry.
    pub occurred_at_unix_ms: i64,
    pub payload: &'a [u8],
    pub wire: &'a [u8],
    pub signature: &'a [u8],
    pub recovery_generation: u64,
    /// Adapter-authenticated first final-freeze time, never the retry receipt time.
    pub finalized_at: Option<u64>,
}
#[derive(Debug, Clone, Copy)]
pub struct Fence {
    pub epoch: u64,
    pub recovery_generation: u64,
    pub complete_through: u64,
}
impl Fence {
    fn valid(self, now: u64) -> bool {
        self.epoch > 0
            && self.recovery_generation > 0
            && self.complete_through > 0
            && self.complete_through <= now.saturating_add(1)
            && now.saturating_sub(self.complete_through).saturating_add(1) <= 30
    }
}
/// Implementations lock current nonrollback recovery/epoch authority and verify
/// the exact admitted snapshot, dedicated peer, correction history and watermark.
/// Native ANL envelopes remain Game-owned and are not part of the HTTP wire.
/// Internal Delivery payloads are local adapter records, never native provenance.
pub trait Authority: Send + Sync {
    fn verify_served(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        _snapshot: &schema::Snapshot,
        _wire: &[u8],
    ) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
    fn reconciled_recovery(&self) -> bool {
        false
    }
    fn fence(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        now: u64,
    ) -> impl Future<Output = Result<Fence>> + Send;
    fn admission_fence(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        now: u64,
    ) -> impl Future<Output = Result<Fence>> + Send {
        self.fence(tx, now)
    }
    fn verify(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        delivery: &Delivery<'_>,
        snapshot: &schema::Snapshot,
    ) -> impl Future<Output = Result<()>> + Send;
}
#[derive(Debug)]
pub struct Unavailable;
impl Authority for Unavailable {
    async fn fence(&self, _: &mut Transaction<'_, Postgres>, _: u64) -> Result<Fence> {
        Err(Error::Unavailable)
    }
    async fn verify(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: &Delivery<'_>,
        _: &schema::Snapshot,
    ) -> Result<()> {
        Err(Error::Unavailable)
    }
}
/// Local adapter utility; this does not define a new Game signature wire format.
pub fn verify_ed25519(key: &[u8; 32], exact_bytes: &[u8], signature: &[u8]) -> Result<()> {
    let key = ed25519_dalek::VerifyingKey::from_bytes(key).map_err(|_| Error::Invalid)?;
    let signature = ed25519_dalek::Signature::from_slice(signature).map_err(|_| Error::Invalid)?;
    key.verify_strict(exact_bytes, &signature)
        .map_err(|_| Error::Invalid)
}
