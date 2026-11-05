//! Transactional ledger for one transferable coin in integer smallest units.
//!
//! Identity and staff authorization are supplied by fail-closed adapters. HTTP
//! authentication and viewer authorization belong to the calling crate.
mod domain;
mod ledger;
pub use domain::*;
pub use ledger::Ledger;
