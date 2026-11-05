//! G-11 revision 2 death/PvP projection. No gameplay commands or action capture.
//!
//! T72 integration seam: an Authority must lock and revalidate the current Identity,
//! ownership publication, transfer visibility floor and recovery fences in the supplied
//! transaction on every call. The adapter also verifies pinned catalog mappings.
//! Until that adapter and the proposed gate are accepted, use DenyAll and default flags.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod schema;
mod storage;
mod view;
pub use schema::*;
pub use storage::*;
pub use view::*;

mod http;
#[cfg(test)]
mod tests;
pub use http::*;
