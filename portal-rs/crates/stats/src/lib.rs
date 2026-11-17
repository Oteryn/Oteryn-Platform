//! Durable public-projection statistics. Missing game metrics remain unavailable.
#[cfg(test)]
mod begin_guard_tests;
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

pub mod privacy;
