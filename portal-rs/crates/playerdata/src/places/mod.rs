//! T81 fixed daily cohorts. No raw G-9 wire is accepted here.
//!
//! T73 must supply a transaction-bound `JournalSource` before activation. Unknown
//! completion, catalogue, pricing, stream or ownership evidence denies contribution.
//! Public sharing and game commands are deliberately outside this interface.
mod http;
mod store;
#[cfg(test)]
mod tests;
pub use http::{configured_router, router};
pub use store::{Cohort, Error, Flags, JournalSource, PlacesStore, Session, UnresolvedSource};
mod retention;
pub use retention::PlacesRetention;
