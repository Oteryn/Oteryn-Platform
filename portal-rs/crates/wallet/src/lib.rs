//! Transactional ledger for one coin class in integer smallest units.
//!
//! Identity and staff authorization are supplied by fail-closed adapters. HTTP
//! authentication and viewer authorization belong to the calling crate.
mod debt;
mod domain;
mod ledger;
pub use debt::outstanding_debt;
pub use domain::*;
pub use ledger::Ledger;
pub mod wire;
