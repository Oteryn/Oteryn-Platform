//! Character auctions. Game evidence is supplied by the T58 transaction-local seam.
pub mod http;
mod model;
mod service;
pub use model::*;
pub use service::*;

#[cfg(test)]
mod tests;
