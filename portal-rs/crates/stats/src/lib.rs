//! Durable public-projection statistics. Missing game metrics remain unavailable.
pub mod rankings;
pub mod store;
#[cfg(test)]
mod tests;
pub mod watch;
pub mod worker;
pub mod world;

pub mod character;
pub mod guilds;
pub mod online;

/// Shared portal navigation registration.
pub mod navigation;
