//! Transactional ledger for one coin class in integer smallest units.
//!
//! Identity and staff authorization are supplied by fail-closed adapters. HTTP
//! authentication and viewer authorization belong to the calling crate.
mod debt;
mod domain;
mod eligibility;
pub use eligibility::{safe_return_in, value_eligible_in};
mod ledger;
pub use debt::{credit_in, outstanding_debt, unlocked_in};
pub use domain::*;
pub use ledger::Ledger;
pub mod wire;
