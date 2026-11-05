//! Durable admin-control intents (ADM-01). No transport or game executor lives here.
//!
//! Callers must supply authenticated staff/agent identities and verified signed
//! envelopes. This store does not grant RBAC permissions or verify signatures.
//! Canonicalisation, signing, operation schemas and HTTP belong to later packets.
pub mod canonical;
pub mod catalog;
#[path = "http.rs"]
pub mod control_http;
pub mod controls;
pub mod signer;
pub mod store;

pub use store::*;

#[cfg(test)]
mod tests;
