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

/// Native coin purchases (sandbox) per OTERYN_COIN_PURCHASE_V1.
pub mod coin;
pub mod coin_http;
pub mod coin_store;
