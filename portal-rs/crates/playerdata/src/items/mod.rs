//! Private, read-only logout custody and economic projections (G-10 revision 2).
//!
//! T72's journal and ownership implementation and T74's pricing implementation
//! are not yet present. `Evidence` is their narrow transactional integration seam.
//! No configured adapter fabricates an ownership baseline or accepts a house key.
pub mod ingest;
pub mod model;
pub mod store;
#[cfg(test)]
mod tests;
pub mod web;

pub use model::{Error, Record};
pub use store::{Evidence, Flags, Projection};
pub use web::configured_router;
