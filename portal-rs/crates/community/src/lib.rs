//! Public game projections: rankings, profiles, guilds, houses, deaths, online and worlds (design T24, T25).

pub mod home;
pub mod ingest;

pub mod online;

pub mod rankings;
pub mod worlds;

mod world_directories;
pub use world_directories::directory_status;

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

// Only the disposable source may use public readers that lack a live Game producer.
fn sample_public_feed(get: impl Fn(&str) -> Option<String>) -> bool {
    get("OTERYN_ENV").as_deref() == Some("testing")
        && get("PORTAL_STAGING_SAMPLE_DATA").as_deref() == Some("true")
        && get("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").as_deref() == Some("staging-sample")
}
