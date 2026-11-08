//! Public game projections: rankings, profiles, guilds, houses, deaths, online and worlds (design T24, T25).

pub mod home;
pub mod ingest;

pub mod online;

pub mod rankings;
pub mod worlds;

pub mod deaths;
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
