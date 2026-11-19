//! Transactional ledger for one coin class in integer smallest units.
//!
//! Identity and staff authorization are supplied by fail-closed adapters. HTTP
//! authentication and viewer authorization belong to the calling crate.
mod debt;
mod domain;
mod eligibility;
pub use eligibility::{safe_return_in, value_eligible_in};
mod ledger;
pub use debt::{check_payer_in, credit_in, outstanding_debt, unlocked_in};
pub use domain::*;
pub use ledger::{Ledger, read_snapshot_in};
pub mod wire;

pub use ledger::{incoming_capacity_in, incoming_capacity_with_topup_in, reserve_incoming_in};

#[cfg(test)]
mod payer_guard_tests;
#[cfg(test)]
mod property_tests;
