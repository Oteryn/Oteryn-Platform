//! Production composition of the game-facing internal routes that the module
//! crates already implement, for the internal mTLS listener (plan §5.3.1, §6).
//!
//! Each surface keeps its own default-off flag and its own client identity:
//!
//! | routes | flag | identity |
//! |---|---|---|
//! | `character-commands/{pending,read,receipt}` (v1 and v2) | `PORTAL_CHARACTER_COMMANDS_ENABLED` (v1), `PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED` (v2) | `PORTAL_CHARACTER_COMMAND_IDENTITIES` |
//! | `game-restore/notice` | `PORTAL_GAME_RESTORE_NOTICE_ENABLED` | `PORTAL_GAME_RESTORE_NOTICE_IDENTITY` |
//! | v2 `character-services/{state,watermark}` | `PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED` | `PORTAL_CHARACTER_SERVICES_IDENTITY` |
//! | `character-bootstrap-intents/read` | `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED` | `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY` |
//! | `premium-snapshots/read` | `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED` | `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY` |
//! | `loyalty-snapshots/read` | `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED` | `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY` |
//! | `store/operations/{pending,read,receipt,reconcile}` | `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED` | `PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY` |
//! | v2 `game-public/{character-online-session,world-daily-kills}` and their watermarks | `PORTAL_PUBLIC_PROJECTIONS_ENABLED` with the G-8 family flags | `PORTAL_PUBLIC_PROJECTIONS_SAN` |
//! | `playerdata/journal/{records,cursor}` (G-10) | `PORTAL_PLAYERDATA_G10_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN` |
//! | `playerdata/progress/{record,watermark}` (G-11) | `PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN` |
//!
//! Character commands and both character-service exchanges also require
//! `OTERYN_ENV` `testing` or `preproduction`, as the enablement runbook states.
//! So do both journal ingest purposes (T72, G-11 §9 rollout), which also need
//! their source authority and family list (`oteryn_playerdata::journal`).
//! `PORTAL_CHARACTER_COMMAND_IDENTITIES` is a JSON object keyed by one DNS SAN
//! per Character Authority host, each value `[]`; those identities are bound to
//! the command routes only, so every other route refuses them.
//!
//! Every internal identity variable (`*_IDENTITY`, `*_IDENTITIES` object keys,
//! `*_SAN`) takes part in one overlap check; a reused or malformed identity
//! keeps every route here unavailable.
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post_service,
};
use oteryn_characters::{bootstrap, commands, overview, restore, services};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{Binding, ConfigError, Identity, IdentityMap},
};
use oteryn_playerdata::journal;
use oteryn_store::{delivery, loyalty, premium, purchase};
use sqlx::PgPool;
use std::{collections::BTreeMap, sync::Arc};
use tower::ServiceExt;

pub const CHARACTER_COMMAND_IDENTITIES: &str = "PORTAL_CHARACTER_COMMAND_IDENTITIES";
const RESTORE_IDENTITY: &str = "PORTAL_GAME_RESTORE_NOTICE_IDENTITY";
const SERVICES_IDENTITY: &str = "PORTAL_CHARACTER_SERVICES_IDENTITY";
const BOOTSTRAP_IDENTITY: &str = "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY";
const PREMIUM_IDENTITY: &str = "PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY";
const LOYALTY_IDENTITY: &str = "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY";
const STORE_IDENTITY: &str = "PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY";
const PUBLIC_IDENTITY: &str = "PORTAL_PUBLIC_PROJECTIONS_SAN";
/// The stats ingest group, as `oteryn_stats::online::configured_router` binds it.
const PUBLIC_GROUP: &str = "game-public";

const ONLINE_SESSION_PATH: &str = "/internal/v2/game-public/character-online-session";
const WORLD_DAILY_KILLS_PATH: &str = "/internal/v2/game-public/world-daily-kills";
/// Same bound as the community watermark dispatcher; each family guard enforces its own.
const WATERMARK_BUFFER: usize = 4096;

type Values = BTreeMap<String, String>;

/// Merge every route below into the internal-listener router.
pub async fn mount(app: Router, pool: PgPool) -> Router {
    let values: Values = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let routes = Routes::configured(pool, &values).await;
    if !routes.identities_valid {
        tracing::error!(
            "internal identity configuration is invalid or reuses an identity; game routes unavailable"
        );
    }
    routes.mount(app)
}

/// Built routers; `stats` also answers its own families on the shared v2 watermark path.
pub struct Routes {
    routes: Router,
    stats: Router,
    identities_valid: bool,
}

impl Routes {
    pub async fn configured(pool: PgPool, values: &Values) -> Self {
        let get = |key: &str| values.get(key).cloned();
        let on = |key: &str| values.get(key).map(String::as_str) == Some("true");
        let qualified = matches!(
            values.get("OTERYN_ENV").map(String::as_str),
            Some("testing" | "preproduction")
        );
        let bindings = bindings(values);
        let valid = bindings.is_ok();
        let bindings = bindings.unwrap_or_default();
        // Validated in `bindings`; the empty map refuses every peer.
        let identities = Arc::new(IdentityMap::new(bindings.clone()).unwrap_or_default());

        let authority = get("PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY").unwrap_or_default();
        let command_settings = commands::Settings {
            enabled: valid && qualified && on("PORTAL_CHARACTER_COMMANDS_ENABLED"),
            source_authority: authority.clone(),
            ..commands::Settings::default()
        };
        let seconds = |key: &str| get(key).and_then(|value| value.parse::<u16>().ok());
        let service_settings = services::Settings {
            exchange_enabled: valid
                && qualified
                && on("PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED"),
            command_authority: authority,
            projection_enabled: valid
                && qualified
                && on("PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED"),
            projection_authority: get("PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY")
                .unwrap_or_default(),
            clock_uncertainty_seconds: seconds(
                "PORTAL_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS",
            ),
            freshness_seconds: seconds("PORTAL_CHARACTER_SERVICES_FRESHNESS_SECONDS"),
            ..services::Settings::default()
        };
        let restore_settings = restore::Settings {
            enabled: valid && on("PORTAL_GAME_RESTORE_NOTICE_ENABLED"),
            source_authority: get("PORTAL_GAME_RESTORE_NOTICE_SOURCE_AUTHORITY")
                .unwrap_or_default(),
            // Reopened intents are answerable only where the command routes may run.
            exchange_available: qualified,
            ..restore::Settings::default()
        };
        let characters = oteryn_web::internal_character_services_router(
            services::Services::new(pool.clone(), service_settings),
            restore_settings,
            command_settings,
            identities.clone(),
        )
        .await;

        let bootstrap = bootstrap::router(
            bootstrap::Reader::new(
                pool.clone(),
                bootstrap::Settings {
                    enabled: valid && on("GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED"),
                },
                Arc::new(overview::Accounts(pool.clone())),
            ),
            identities.clone(),
        );

        let accounts = Arc::new(loyalty::stream::http::DatabaseAccounts);
        let mut premium_settings = premium::http::Settings::from_lookup(get);
        premium_settings.enabled &= valid;
        let premium = premium::http::router(
            premium::repository::Ledger::new(
                pool.clone(),
                accounts.clone(),
                premium_settings.enabled,
            ),
            premium_settings,
            identities.clone(),
        );
        let mut loyalty_settings = loyalty::http::Settings::from_lookup(get);
        loyalty_settings.enabled &= valid;
        let loyalty = loyalty::http::router(
            loyalty::Ledger::new(pool.clone(), accounts, loyalty_settings.enabled),
            loyalty_settings,
            identities.clone(),
        );

        let mut store_settings = purchase::Settings::from_lookup(get);
        store_settings.delivery_enabled &= valid;
        // The factory adds the Store identity itself and refuses any other in its group.
        let others = bindings
            .into_iter()
            .filter(|binding| binding.group != delivery::http::GROUP)
            .collect();
        let delivery = delivery::http::router(
            delivery::Delivery::new(pool.clone(), store_settings),
            others,
        );

        let mut journal_settings = journal::Settings::from_lookup(get);
        journal_settings.items_ingest &= valid && qualified;
        journal_settings.progress_ingest &= valid && qualified;
        let journal =
            journal::configured_router(pool.clone(), &journal_settings, identities.clone());

        let stats = oteryn_stats::online::router(
            pool,
            identities,
            oteryn_stats::online::Config {
                enabled: valid
                    && on("PORTAL_PUBLIC_PROJECTIONS_ENABLED")
                    && matches!(
                        values.get("APP_ENV").map(String::as_str),
                        Some("testing" | "preproduction")
                    ),
                intervals_enabled: on("PORTAL_STATS_ONLINE_INTERVALS_ENABLED"),
                world_totals_enabled: on("PORTAL_STATS_WORLD_TOTALS_ENABLED"),
                owner_rankings_enabled: false,
                source_authority: get("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default(),
            },
        );
        // The shared v2 watermark path is already served by the community
        // dispatcher; `mount` diverts the stats families to this router.
        let online = Router::new()
            .route(ONLINE_SESSION_PATH, post_service(stats.clone()))
            .route(WORLD_DAILY_KILLS_PATH, post_service(stats.clone()));

        Self {
            routes: characters
                .merge(bootstrap)
                .merge(premium)
                .merge(loyalty)
                .merge(delivery)
                .merge(journal)
                .merge(online),
            stats,
            identities_valid: valid,
        }
    }

    pub fn mount(self, app: Router) -> Router {
        app.merge(self.routes)
            .layer(middleware::from_fn_with_state(self.stats, watermark))
    }
}

/// Bindings for every internal identity variable. Dedicated variables get the
/// group their route checks; the rest keep their variable name, so they take
/// part in the overlap check without reaching any route here.
fn bindings(values: &Values) -> Result<Vec<Binding>, ConfigError> {
    let mut bindings = Vec::new();
    for (key, raw) in values {
        if key.ends_with("_IDENTITIES") {
            let Value::Object(members) =
                canonical::decode(raw.as_bytes(), 131072, 4).map_err(|_| ConfigError)?
            else {
                return Err(ConfigError);
            };
            let own = key == CHARACTER_COMMAND_IDENTITIES;
            for (san, value) in members {
                if own && (!dns_name(&san) || !matches!(value, Value::Array(ref a) if a.is_empty()))
                {
                    return Err(ConfigError);
                }
                bindings.push(Binding {
                    identity: Identity::san(&san)?,
                    group: if own {
                        commands::GROUP.into()
                    } else {
                        key.clone()
                    },
                });
            }
        } else if key.ends_with("_IDENTITY") || key.ends_with("_SAN") {
            let group = match key.as_str() {
                RESTORE_IDENTITY => restore::GROUP,
                SERVICES_IDENTITY => services::PROJECTION_GROUP,
                BOOTSTRAP_IDENTITY => bootstrap::GROUP,
                PREMIUM_IDENTITY => premium::http::GROUP,
                LOYALTY_IDENTITY => loyalty::http::GROUP,
                STORE_IDENTITY => delivery::http::GROUP,
                PUBLIC_IDENTITY => PUBLIC_GROUP,
                journal::ITEMS_IDENTITY => journal::ITEMS_GROUP,
                journal::PROGRESS_IDENTITY => journal::PROGRESS_GROUP,
                other => other,
            };
            bindings.push(Binding {
                identity: Identity::san(raw)?,
                group: group.into(),
            });
        }
    }
    IdentityMap::new(bindings.clone())?;
    Ok(bindings)
}

/// The internal listener reads only the leaf certificate's DNS SAN.
fn dns_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name.split('.').all(|label| {
            (1..=63).contains(&label.len())
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

#[derive(serde::Deserialize)]
struct Peek {
    family: Option<String>,
}

/// Sends a v2 watermark of a G-8 statistics family to the stats router; every
/// other request, including other families, continues unchanged.
async fn watermark(State(stats): State<Router>, request: Request, next: Next) -> Response {
    if request.method() != Method::POST
        || request.uri().path() != oteryn_community::v2_watermark::PATH
    {
        return next.run(request).await;
    }
    let (parts, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, WATERMARK_BUFFER).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let family = serde_json::from_slice::<Peek>(&bytes)
        .ok()
        .and_then(|peek| peek.family);
    let request = Request::from_parts(parts, Body::from(bytes));
    let statistics = [
        oteryn_stats::online::Family::CharacterOnlineSessions,
        oteryn_stats::online::Family::WorldDailyKills,
    ];
    if statistics
        .iter()
        .any(|known| family.as_deref() == Some(known.name()))
    {
        match stats.oneshot(request).await {
            Ok(response) => response,
            Err(never) => match never {},
        }
    } else {
        next.run(request).await
    }
}

#[cfg(test)]
#[path = "game_routes_tests.rs"]
mod tests;
