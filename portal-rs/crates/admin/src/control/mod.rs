//! Durable admin-control intents (ADM-01). No transport or game executor lives here.
//!
//! Callers must supply authenticated staff/agent identities and verified signed
//! envelopes. This store does not grant RBAC permissions or verify signatures.
//! Canonicalisation, signing, operation schemas and HTTP belong to later packets.
pub mod store;

pub use store::*;

#[cfg(test)]
mod tests;

pub mod canonical;
pub mod signer;
