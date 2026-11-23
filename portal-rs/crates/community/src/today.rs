//! G-12 public daily projection. Rendering reads only local, fenced evidence.
//! Activation is restricted to testing/preproduction. No prediction outcome exists.
mod http;
mod operator;
mod presentation;
mod schema;
mod selection;
mod store;
mod view;

pub use http::{configured_mount, configured_watermark_router, mount, router};
use std::collections::BTreeSet;
pub use store::Retention;
use uuid::Uuid;
pub use view::{Boost, DailyView, Raid, read};

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub ingest_enabled: bool,
    pub public_enabled: bool,
    pub source_authority: String,
    pub worlds: BTreeSet<Uuid>,
}
impl Config {
    fn valid(&self) -> bool {
        schema::authority(&self.source_authority)
            && !self.worlds.is_empty()
            && self.worlds.len() <= 256
            && self.worlds.iter().all(|id| id.get_version_num() == 7)
    }
    pub fn configured() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let environment = oteryn_core::restricted_environment(&get);
        let worlds = get("PORTAL_WORLD_DAILY_WORLDS")
            .and_then(|s| {
                let values = s
                    .split(',')
                    .map(schema::world_id)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                let set: BTreeSet<_> = values.iter().copied().collect();
                (set.len() == values.len()).then_some(set)
            })
            .unwrap_or_default();
        Self {
            ingest_enabled: environment
                && get("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Some("true")
                && get("PORTAL_WORLD_DAILY_INGEST_ENABLED").as_deref() == Some("true"),
            public_enabled: environment
                && get("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Some("true")
                && get("PORTAL_WORLD_DAILY_PUBLIC_ENABLED").as_deref() == Some("true"),
            source_authority: get("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default(),
            worlds,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadInput,
    Unavailable,
    Unauthorized,
    Conflict,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
