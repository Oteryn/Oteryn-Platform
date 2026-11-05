//! G-12 public daily projection. Rendering reads only local, fenced evidence.
//! Activation is restricted to testing/preproduction. No prediction outcome exists.
mod http;
mod schema;
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
        let environment = matches!(
            std::env::var("APP_ENV").as_deref(),
            Ok("testing" | "preproduction")
        );
        let worlds = std::env::var("PORTAL_WORLD_DAILY_WORLDS")
            .ok()
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
                && std::env::var("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Ok("true")
                && std::env::var("PORTAL_WORLD_DAILY_INGEST_ENABLED").as_deref() == Ok("true"),
            public_enabled: environment
                && std::env::var("PORTAL_WORLD_DAILY_PUBLIC_ENABLED").as_deref() == Ok("true"),
            source_authority: std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY")
                .unwrap_or_default(),
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
