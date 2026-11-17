//! Public game projections: rankings, profiles, guilds, houses, deaths, online and worlds (design T24, T25).

pub mod home;
pub mod ingest;

pub mod online;

pub mod rankings;
pub mod worlds;

pub mod deaths;
pub mod families;
pub mod guilds;
pub mod houses;
pub mod party;
pub mod profiles;
pub mod referrals;
pub mod today;
pub mod v2_watermark;

/// Shared portal navigation registration.
pub mod navigation;

pub mod privacy;

#[cfg(test)]
mod audit06_test_db;

#[cfg(test)]
mod begin_guard_tests;

#[cfg(test)]
mod restricted_environment_tests;
