//! T72 production composition of the private game journal ingest (G-10 items,
//! G-11 progress, both ACCEPTED revision 3) for the internal mTLS listener.
//!
//! | routes | flag | identity | source |
//! |---|---|---|---|
//! | `journal/{records,cursor}` (G-10) | `PORTAL_PLAYERDATA_G10_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN` | `PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY` |
//! | `progress/{record,watermark}` (G-11) | `PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN` | `PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY` |
//!
//! Both also need the family in `PORTAL_PLAYERDATA_JOURNAL_FAMILIES` (comma
//! separated, default empty). Every flag defaults to false; the caller adds its
//! environment and identity checks. With both ingest flags off the adapters are
//! the fail-closed defaults of [`crate::store`]. With an ingest flag on, the
//! PostgreSQL admission fence ([`crate::authority::DbAuthority`]) and the
//! database clock ([`crate::authority::DbClock`]) admit records, and an enabled
//! G-11 ingest promotes `game_login` records into the login projection.
use crate::{
    authority::{DbAuthority, DbClock, JournalLoginAuthority, LoginPromoter},
    logins::{self, Logins},
    store::{
        Authority, Consumer, Flags, Store, UnavailableAuthority,
        http::{self, Clock, HttpState, Sources},
    },
};
use axum::Router;
use oteryn_internal_api::guard::IdentityMap;
use sqlx::PgPool;
use std::{collections::BTreeSet, sync::Arc};

pub use http::{ITEMS_GROUP, PROGRESS_GROUP};
/// DNS SAN of the G-10 producer; bound to [`ITEMS_GROUP`] only.
pub const ITEMS_IDENTITY: &str = "PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN";
/// DNS SAN of the G-11 producer; bound to [`PROGRESS_GROUP`] only.
pub const PROGRESS_IDENTITY: &str = "PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN";
pub const ITEMS_ENABLED: &str = "PORTAL_PLAYERDATA_G10_INGEST_ENABLED";
pub const PROGRESS_ENABLED: &str = "PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED";
pub const ITEMS_SOURCE: &str = "PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY";
pub const PROGRESS_SOURCE: &str = "PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY";
pub const FAMILIES: &str = "PORTAL_PLAYERDATA_JOURNAL_FAMILIES";

/// Ingest settings; reads, reminders and exports stay off on this listener.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub items_ingest: bool,
    pub progress_ingest: bool,
    pub sources: Sources,
    pub families: BTreeSet<String>,
}
impl Settings {
    /// Missing configuration equals disabled.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let on = |key: &str| get(key).as_deref() == Some("true");
        Self {
            items_ingest: on(ITEMS_ENABLED),
            progress_ingest: on(PROGRESS_ENABLED),
            sources: Sources {
                items: get(ITEMS_SOURCE).unwrap_or_default(),
                progress: get(PROGRESS_SOURCE).unwrap_or_default(),
            },
            families: get(FAMILIES)
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|family| !family.is_empty())
                .map(str::to_owned)
                .collect(),
        }
    }
    pub(crate) fn flags(&self) -> Flags {
        Flags {
            items_ingest: self.items_ingest,
            progress_ingest: self.progress_ingest,
            families: self.families.clone(),
            ..Flags::default()
        }
    }
}

/// The four ingest routes over the given adapters. `identities` is the shared
/// map of every internal purpose, so a reused identity fails closed.
pub fn router(
    pool: PgPool,
    authority: Arc<dyn Authority>,
    clock: Arc<dyn Clock>,
    settings: &Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    router_with(pool, authority, clock, Vec::new(), settings, identities)
}

/// [`router`] with read-model consumers assembled in the ingest transaction.
pub fn router_with(
    pool: PgPool,
    authority: Arc<dyn Authority>,
    clock: Arc<dyn Clock>,
    consumers: Vec<Arc<dyn Consumer>>,
    settings: &Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    let store = consumers.into_iter().fold(
        Store::new(pool, authority, settings.flags()),
        Store::with_consumer,
    );
    http::router(
        HttpState::new(store, clock).with_sources(settings.sources.clone()),
        identities,
    )
}

/// The login read model fed by an enabled G-11 ingest.
pub fn login_promoter(pool: PgPool, settings: &Settings) -> LoginPromoter {
    LoginPromoter {
        logins: Logins {
            pool,
            flags: logins::Flags {
                progress_ingest_enabled: settings.progress_ingest,
                ..logins::Flags::default()
            },
            authority: Arc::new(JournalLoginAuthority {
                source: settings.sources.progress.clone(),
            }),
        },
    }
}

/// Production composition (see the module comment).
pub fn configured_router(
    pool: PgPool,
    settings: &Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    if !settings.items_ingest && !settings.progress_ingest {
        return router(
            pool,
            Arc::new(UnavailableAuthority),
            Arc::new(http::NoClock),
            settings,
            identities,
        );
    }
    let mut consumers: Vec<Arc<dyn Consumer>> = Vec::new();
    if settings.progress_ingest {
        consumers.push(Arc::new(login_promoter(pool.clone(), settings)));
    }
    router_with(
        pool.clone(),
        Arc::new(DbAuthority),
        Arc::new(DbClock::new(pool)),
        consumers,
        settings,
        identities,
    )
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
