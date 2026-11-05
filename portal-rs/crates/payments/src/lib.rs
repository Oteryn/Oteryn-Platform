//! Payment provider trait, test provider and idempotent webhooks (design T31). Depends on wallet, never the reverse.

mod core;
mod provider;
mod storage;
pub use core::*;
pub use provider::*;
pub use storage::*;
pub mod account;

/// Shared portal navigation registration.
pub mod navigation;
