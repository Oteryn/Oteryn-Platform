//! Staff-managed discovery for public editorial documents.
mod public;
mod staff;

pub use public::{public_router, shell};
pub(crate) use staff::configured_router;

#[cfg(test)]
mod tests;
