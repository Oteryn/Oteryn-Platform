//! Shared guards and ordered JSON codecs for the private contract surface.
//!
//! Mount each handler with [`guard::protect`]. The trusted TLS adapter must supply
//! [`guard::VerifiedClient`]; HTTP headers never establish certificate provenance.
//! Domain handlers retain responsibility for semantic validation, source authority,
//! scope, replay protection and rate limits. No routes are enabled by this crate.

pub mod canonical;
pub mod fixtures;
pub mod guard;

pub mod bazaar;
