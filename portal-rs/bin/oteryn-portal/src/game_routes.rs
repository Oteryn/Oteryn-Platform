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
//! | `profile-b/authority/read` | `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED` | `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_MTLS_CLIENT_IDENTITY` |
//! | `profile-b/changes/read` | `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED` | `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_MTLS_CLIENT_IDENTITY` |
//! | `store/operations/{pending,read,receipt,reconcile}` | `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED` | `PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY` |
//! | `store/{balance/read,history/read,purchases}` | `PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED` with the delivery flag | `PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY` (one Store identity, store wire contract §4.1) |
//! | v2 `game-public/{character-online-session,world-daily-kills}` and their watermarks | `PORTAL_PUBLIC_PROJECTIONS_ENABLED` with the G-8 family flags | `PORTAL_PUBLIC_PROJECTIONS_SAN` |
//! | v2 `game-public/{character-skills,character-achievements,character-equipment,character-outfit}` and their watermarks (G-4) | `PORTAL_PUBLIC_PROJECTIONS_ENABLED` with `PORTAL_G4_{SKILLS,ACHIEVEMENTS,EQUIPMENT,OUTFIT}_INGEST_ENABLED` | `PORTAL_PUBLIC_PROJECTIONS_SAN` |
//! | `playerdata/journal/{records,cursor}` (G-10) | `PORTAL_PLAYERDATA_G10_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN` |
//! | `playerdata/progress/{record,watermark}` (G-11) | `PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED` | `PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN` |
//! | `support/enforcement-commands/{stream/read,commands/read,reconcile/read}` | `PORTAL_GAME_ENFORCEMENT_ENABLED` | `PORTAL_GAME_ENFORCEMENT_READER_IDENTITY` |
//! | `support/enforcement-commands/results` | `PORTAL_GAME_ENFORCEMENT_ENABLED`, and `PORTAL_GAME_RESTORE_NOTICE_ENABLED` not `true` | `PORTAL_GAME_ENFORCEMENT_RESULTS_IDENTITY` |
//!
//! The enforcement results group stays off while the restore notice is on:
//! the notice does not yet reopen enforcement results (wire contract §6.4, E6).
//! Character commands and both character-service exchanges also require
//! `OTERYN_ENV` `testing` or `preprod`, as the enablement runbook states.
//! So do both journal ingest purposes (T72, G-11 §9 rollout), which also need
//! their source authority and family list (`oteryn_playerdata::journal`).
//! Store purchases require the same, and separately require `APP_ENV` set to
//! `testing` or `preproduction` (unset refuses): a production sale is impossible
//! under the proposed store wire.
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
use base64::Engine;
use oteryn_characters::{bootstrap, commands, overview, restore, services};
use oteryn_community::families;
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{Binding, ConfigError, Identity, IdentityMap},
};
use oteryn_playerdata::journal;
use oteryn_store::{delivery, loyalty, premium, premium::profile_b, purchase};
use oteryn_support::sanctions::enforcement;
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

/// One startup snapshot of every purpose on the shared internal listener.
pub struct Configuration {
    values: Values,
    registry: Result<Vec<Binding>, ConfigError>,
}
impl Configuration {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::new(
            std::env::vars_os()
                .filter_map(|(key, value)| {
                    Some((key.into_string().ok()?, value.into_string().ok()?))
                })
                .collect(),
        )
    }
    fn new(values: Values) -> Result<Self, ConfigError> {
        validate_store_restore(&values)?;
        let registry = bindings(&values);
        Ok(Self { values, registry })
    }
}

pub async fn mount(
    app: Router,
    pool: PgPool,
    environment: oteryn_core::Environment,
    configuration: Configuration,
) -> Router {
    let Configuration { values, registry } = configuration;
    let routes = Routes::with_registry(pool, &values, environment, registry).await;
    if !routes.identities_valid {
        tracing::error!(
            "internal identity configuration is invalid or reuses an identity; game routes unavailable"
        );
    }
    routes.mount(app)
}

/// Restore does not yet reconcile Store receipts or include them in its fence.
fn validate_store_restore(values: &Values) -> Result<(), ConfigError> {
    if values
        .get("PORTAL_GAME_RESTORE_NOTICE_ENABLED")
        .map(String::as_str)
        == Some("true")
        && values
            .get("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED")
            .map(String::as_str)
            == Some("true")
    {
        return Err(ConfigError);
    }
    Ok(())
}

/// Built routers; `stats` and `families` also answer their own families on the
/// shared v2 watermark path.
pub struct Routes {
    routes: Router,
    diverted: Diverted,
    identities_valid: bool,
}
#[derive(Clone)]
struct Diverted {
    stats: Router,
    families: Router,
}

impl Routes {
    #[cfg(test)]
    pub async fn configured(
        pool: PgPool,
        values: &Values,
        environment: oteryn_core::Environment,
    ) -> Self {
        Self::with_registry(pool, values, environment, bindings(values)).await
    }

    async fn with_registry(
        pool: PgPool,
        values: &Values,
        environment: oteryn_core::Environment,
        registry: Result<Vec<Binding>, ConfigError>,
    ) -> Self {
        let get = |key: &str| values.get(key).cloned();
        let on = |key: &str| values.get(key).map(String::as_str) == Some("true");
        let qualified = matches!(
            environment,
            oteryn_core::Environment::Testing | oteryn_core::Environment::Preprod
        );
        let bindings = registry;
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
        let mut profile_b_settings = profile_b::http::Settings::from_lookup(get);
        profile_b_settings.authority_enabled &= valid;
        profile_b_settings.changes_enabled &= valid;
        // No product contract declares Profile B yet (#1467): a configured
        // product answers 503 rather than invented NONE evidence.
        let profile_b = profile_b::http::router(
            profile_b::Authority::new(
                pool.clone(),
                accounts.clone(),
                Arc::new(profile_b::NoGenericLedger),
                profile_b_settings.authority_valid(),
            ),
            profile_b::Changes::new(pool.clone(), profile_b_settings.changes_valid()),
            profile_b_settings,
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
        // Store's domain settings use the contract's legacy environment labels.
        // Adapt the parsed process environment here, never the raw OTERYN_ENV.
        store_settings.environment = match environment {
            oteryn_core::Environment::Local => "local",
            oteryn_core::Environment::Testing => "testing",
            oteryn_core::Environment::Preprod => "preproduction",
            oteryn_core::Environment::Production => "production",
        }
        .into();
        store_settings.delivery_enabled &= valid;
        let mut purchase_settings = store_settings.clone();
        purchase_settings.purchase_enabled &= qualified
            && matches!(
                values.get("APP_ENV").map(String::as_str),
                Some("testing" | "preproduction")
            );
        // As for delivery, the factory adds the shared Store identity itself.
        let purchase_others = bindings
            .iter()
            .filter(|binding| binding.group != purchase::http::GROUP)
            .cloned()
            .collect();
        let purchase = purchase::http::router(
            purchase::database(pool.clone(), purchase_settings),
            purchase_others,
        );
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

        let mut enforcement_settings = enforcement::Settings::from_lookup(get);
        enforcement_settings.reader_enabled &= valid;
        enforcement_settings.results_enabled &= valid && !on("PORTAL_GAME_RESTORE_NOTICE_ENABLED");
        let enforcement =
            enforcement::router(pool.clone(), enforcement_settings, identities.clone());
        // G-4 character families: ingest only on this listener; the T24
        // prerequisite and APP_ENV are checked in `ingest_config`.
        let mut g4 = families::ingest_config(get);
        g4.skills &= valid;
        g4.achievements &= valid;
        g4.equipment &= valid;
        g4.outfit &= valid;
        let g4 = families::ingest_router(pool.clone(), identities.clone(), g4);
        let mut character_families = Router::new();
        for family in families::Family::ALL {
            character_families = character_families.route(
                &format!("/internal/v2/game-public/{}", family.path()),
                post_service(g4.clone()),
            );
        }

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
                .merge(profile_b)
                .merge(delivery)
                .merge(journal)
                .merge(purchase)
                .merge(enforcement)
                .merge(online)
                .merge(character_families),
            diverted: Diverted {
                stats,
                families: g4,
            },
            identities_valid: valid,
        }
    }

    pub fn mount(self, app: Router) -> Router {
        let valid = self.identities_valid;
        app.merge(self.routes)
            .layer(middleware::from_fn_with_state(self.diverted, watermark))
            .layer(middleware::from_fn(
                move |request: Request, next: Next| async move {
                    if !valid && request.uri().path().starts_with("/internal/") {
                        return oteryn_internal_api::guard::failure(
                            StatusCode::SERVICE_UNAVAILABLE,
                        );
                    }
                    next.run(request).await
                },
            ))
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
                profile_b::http::AUTHORITY_IDENTITY => profile_b::http::AUTHORITY_GROUP,
                profile_b::http::CHANGES_IDENTITY => profile_b::http::CHANGES_GROUP,
                STORE_IDENTITY => delivery::http::GROUP,
                PUBLIC_IDENTITY => PUBLIC_GROUP,
                journal::ITEMS_IDENTITY => journal::ITEMS_GROUP,
                journal::PROGRESS_IDENTITY => journal::PROGRESS_GROUP,
                enforcement::READER_IDENTITY => enforcement::READER_GROUP,
                enforcement::RESULTS_IDENTITY => enforcement::RESULTS_GROUP,
                other => other,
            };
            if group.starts_with("support-enforcement-") && !dns_name(raw) {
                return Err(ConfigError);
            }
            bindings.push(Binding {
                identity: Identity::san(raw)?,
                group: group.into(),
            });
        }
    }
    if let Some(raw) = values.get("PORTAL_ADMIN_CONTROL_AGENTS_JSON") {
        let agents: serde_json::Value = serde_json::from_str(raw).map_err(|_| ConfigError)?;
        for agent in agents.as_array().ok_or(ConfigError)? {
            let identity = agent
                .get("identity")
                .and_then(serde_json::Value::as_object)
                .ok_or(ConfigError)?;
            if identity.len() != 1 {
                return Err(ConfigError);
            }
            let identity = if let Some(san) = identity.get("san") {
                Identity::san(san.as_str().ok_or(ConfigError)?)?
            } else if let Some(spki) = identity.get("spki") {
                Identity::spki_sha256(
                    base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(spki.as_str().ok_or(ConfigError)?)
                        .map_err(|_| ConfigError)?
                        .try_into()
                        .map_err(|_| ConfigError)?,
                )
            } else {
                return Err(ConfigError);
            };
            bindings.push(Binding {
                identity,
                group: "admin-control".into(),
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

/// Sends a v2 watermark of a G-8 statistics family to the stats router and one of
/// a G-4 character family to the families router; every other request, including
/// other families, continues unchanged.
async fn watermark(State(diverted): State<Diverted>, request: Request, next: Next) -> Response {
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
    let target = if statistics
        .iter()
        .any(|known| family.as_deref() == Some(known.name()))
    {
        diverted.stats
    } else if family
        .as_deref()
        .and_then(families::Family::from_name)
        .is_some()
    {
        diverted.families
    } else {
        return next.run(request).await;
    };
    match target.oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

#[cfg(test)]
#[path = "game_routes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "game_routes_store_tests.rs"]
mod store_tests;
