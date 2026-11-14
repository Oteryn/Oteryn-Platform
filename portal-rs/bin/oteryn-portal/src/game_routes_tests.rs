//! Composition tests: each mounted router answers through the internal-listener
//! adapter only for its own identity, and is unavailable with its flag off.
//! No database is reachable here; requests stop at the guard (400 for a body
//! that is not the contract's JSON) or, for Store delivery, at its database
//! rate budget (503).
use super::*;
use axum::{
    http::{StatusCode, header},
    routing::post,
};
use oteryn_internal_api::guard::VerifiedClient;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const AUTHORITY: &str = "oteryn:character-authority:primary";
const COMMAND_A: &str = "char-authority-a.oteryn.internal";
const COMMAND_B: &str = "char-authority-b.oteryn.internal";
const RESTORE: &str = "restore.oteryn.internal";
const SERVICES: &str = "character-services.oteryn.internal";
const BOOTSTRAP: &str = "bootstrap.oteryn.internal";
const PREMIUM: &str = "premium.oteryn.internal";
const LOYALTY: &str = "loyalty.oteryn.internal";
const STORE: &str = "store.oteryn.internal";
const PUBLIC: &str = "public.oteryn.internal";
const LCFA: &str = "lcfa.oteryn.internal";
const STRANGER: &str = "stranger.oteryn.internal";
const JOURNAL_ITEMS: &str = "journal-items.oteryn.internal";
const JOURNAL_PROGRESS: &str = "journal-progress.oteryn.internal";
const JOURNAL_ITEMS_PATHS: [&str; 2] = [
    "/internal/v1/playerdata/journal/records",
    "/internal/v1/playerdata/journal/cursor",
];
const JOURNAL_PROGRESS_PATHS: [&str; 2] = [
    "/internal/v1/playerdata/progress/record",
    "/internal/v1/playerdata/progress/watermark",
];

const COMMAND_PATHS: [&str; 6] = [
    "/internal/v1/game-auth/character-commands/pending",
    "/internal/v1/game-auth/character-commands/read",
    "/internal/v1/game-auth/character-commands/receipt",
    "/internal/v2/game-auth/character-commands/pending",
    "/internal/v2/game-auth/character-commands/read",
    "/internal/v2/game-auth/character-commands/receipt",
];
const RESTORE_PATH: &str = "/internal/v1/game-auth/game-restore/notice";
const SERVICES_PATHS: [&str; 2] = [
    "/internal/v2/game-auth/character-services/state",
    "/internal/v2/game-auth/character-services/watermark",
];
const BOOTSTRAP_PATH: &str = "/internal/v1/game-auth/character-bootstrap-intents/read";
const PREMIUM_PATH: &str = "/internal/v1/products-entitlements/premium-snapshots/read";
const LOYALTY_PATH: &str = "/internal/v1/products-entitlements/loyalty-snapshots/read";
const STORE_PATHS: [&str; 4] = [
    "/internal/v1/products-entitlements/store/operations/pending",
    "/internal/v1/products-entitlements/store/operations/read",
    "/internal/v1/products-entitlements/store/operations/receipt",
    "/internal/v1/products-entitlements/store/operations/reconcile",
];
const PURCHASE_PATHS: [&str; 3] = [
    "/internal/v1/products-entitlements/store/balance/read",
    "/internal/v1/products-entitlements/store/history/read",
    "/internal/v1/products-entitlements/store/purchases",
];
const LCFA_PATH: &str = "/internal/v1/game-auth/native-account-characters";
const STATS_WATERMARK: &str = r#"{"family":"world_daily_kills"}"#;

/// Every flag on, each surface with its own identity, in a qualified environment.
fn enabled() -> Values {
    [
        ("OTERYN_ENV", "testing"),
        ("APP_ENV", "testing"),
        ("PORTAL_CHARACTER_COMMANDS_ENABLED", "true"),
        ("PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY", AUTHORITY),
        (
            CHARACTER_COMMAND_IDENTITIES,
            r#"{"char-authority-a.oteryn.internal":[],"char-authority-b.oteryn.internal":[]}"#,
        ),
        ("PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED", "true"),
        ("PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED", "true"),
        (
            "PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY",
            "oteryn:game:primary",
        ),
        ("PORTAL_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS", "1"),
        ("PORTAL_CHARACTER_SERVICES_FRESHNESS_SECONDS", "30"),
        (SERVICES_IDENTITY, SERVICES),
        ("PORTAL_GAME_RESTORE_NOTICE_ENABLED", "true"),
        ("PORTAL_GAME_RESTORE_NOTICE_SOURCE_AUTHORITY", AUTHORITY),
        (RESTORE_IDENTITY, RESTORE),
        ("GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED", "true"),
        (BOOTSTRAP_IDENTITY, BOOTSTRAP),
        (
            "PLATFORM_BUILD_REVISION",
            "0123456789abcdef0123456789abcdef01234567",
        ),
        ("PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED", "true"),
        (PREMIUM_IDENTITY, PREMIUM),
        ("PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED", "true"),
        (LOYALTY_IDENTITY, LOYALTY),
        ("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED", "true"),
        (
            "PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY",
            "oteryn:game:primary",
        ),
        (STORE_IDENTITY, STORE),
        ("PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED", "true"),
        ("PORTAL_PUBLIC_PROJECTIONS_ENABLED", "true"),
        ("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY", "oteryn:game:primary"),
        ("PORTAL_STATS_ONLINE_INTERVALS_ENABLED", "true"),
        ("PORTAL_STATS_WORLD_TOTALS_ENABLED", "true"),
        (PUBLIC_IDENTITY, PUBLIC),
        ("PORTAL_LCFA_INTERNAL_IDENTITY", LCFA),
        ("PORTAL_PLAYERDATA_G10_INGEST_ENABLED", "true"),
        (
            "PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY",
            "OTERYN_GAME_JOURNAL",
        ),
        ("PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN", JOURNAL_ITEMS),
        ("PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED", "true"),
        (
            "PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY",
            "oteryn:journal:primary",
        ),
        ("PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN", JOURNAL_PROGRESS),
        ("PORTAL_PLAYERDATA_JOURNAL_FAMILIES", "BANK,ITEMS,stamina"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}

fn pool() -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(300))
        .connect_lazy("postgres://game-routes@127.0.0.1:1/game_routes")
}

/// The internal listener's composition: these routes on top of a stand-in for
/// the community watermark dispatcher already merged on the same router.
async fn app(values: &Values) -> Result<Router, Box<dyn std::error::Error>> {
    let community = Router::new().route(
        oteryn_community::v2_watermark::PATH,
        post(|| async { "community" }),
    );
    let app = Routes::configured(pool()?, values).await.mount(community);
    let lcfa = crate::game_login::lcfa_router(
        pool()?,
        values.get("PORTAL_LCFA_INTERNAL_IDENTITY").cloned(),
        oteryn_characters::lcfa::Settings {
            enabled: true,
            environment: oteryn_core::Environment::Testing,
            source_authority: AUTHORITY.into(),
            ..Default::default()
        },
    )?;
    Ok(match lcfa {
        Some(lcfa) => app.merge(lcfa),
        None => app,
    })
}

/// One request through the internal listener adapter with a verified SAN.
async fn call(
    app: &Router,
    san: &str,
    path: &str,
    body: &str,
) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let client = VerifiedClient::from_verified_tls13_http11(Identity::san(san)?);
    let scoped = crate::internal_tls::scoped(app.clone(), client, None, "127.0.0.1:9".parse()?);
    let request = axum::http::Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))?;
    let response = scoped.oneshot(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4096).await?;
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

async fn status(
    app: &Router,
    san: &str,
    path: &str,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let body = if path == oteryn_community::v2_watermark::PATH {
        STATS_WATERMARK
    } else {
        "not json"
    };
    Ok(call(app, san, path, body).await?.0)
}

type Surface = (
    &'static str,
    &'static str,
    &'static [&'static str],
    StatusCode,
);

/// Route, its own identity, the flags that turn it off and the status a
/// request with the right identity gets once past the identity check. The v1
/// command exchange also opens while the restore notice is enabled, so both
/// flags close it.
fn surfaces() -> Vec<Surface> {
    let mut surfaces: Vec<Surface> = Vec::new();
    for path in &COMMAND_PATHS[..3] {
        surfaces.push((
            *path,
            COMMAND_A,
            &[
                "PORTAL_CHARACTER_COMMANDS_ENABLED",
                "PORTAL_GAME_RESTORE_NOTICE_ENABLED",
            ],
            StatusCode::BAD_REQUEST,
        ));
    }
    for path in &COMMAND_PATHS[3..] {
        surfaces.push((
            *path,
            COMMAND_B,
            &["PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED"],
            StatusCode::BAD_REQUEST,
        ));
    }
    surfaces.push((
        RESTORE_PATH,
        RESTORE,
        &["PORTAL_GAME_RESTORE_NOTICE_ENABLED"],
        StatusCode::BAD_REQUEST,
    ));
    for path in SERVICES_PATHS {
        surfaces.push((
            path,
            SERVICES,
            &["PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED"],
            StatusCode::BAD_REQUEST,
        ));
    }
    surfaces.push((
        BOOTSTRAP_PATH,
        BOOTSTRAP,
        &["GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED"],
        StatusCode::BAD_REQUEST,
    ));
    surfaces.push((
        PREMIUM_PATH,
        PREMIUM,
        &["PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED"],
        StatusCode::BAD_REQUEST,
    ));
    surfaces.push((
        LOYALTY_PATH,
        LOYALTY,
        &["PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED"],
        StatusCode::BAD_REQUEST,
    ));
    for path in STORE_PATHS {
        // Past the identity check the Store budget needs the database, absent here.
        surfaces.push((
            path,
            STORE,
            &["PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED"],
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    for path in PURCHASE_PATHS {
        // Purchases share the Store identity (wire contract §4.1) and budget.
        surfaces.push((
            path,
            STORE,
            &["PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED"],
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    for path in [
        ONLINE_SESSION_PATH,
        WORLD_DAILY_KILLS_PATH,
        oteryn_community::v2_watermark::PATH,
    ] {
        surfaces.push((
            path,
            PUBLIC,
            &["PORTAL_PUBLIC_PROJECTIONS_ENABLED"],
            StatusCode::BAD_REQUEST,
        ));
    }
    for path in JOURNAL_ITEMS_PATHS {
        surfaces.push((
            path,
            JOURNAL_ITEMS,
            &["PORTAL_PLAYERDATA_G10_INGEST_ENABLED"],
            StatusCode::BAD_REQUEST,
        ));
    }
    for path in JOURNAL_PROGRESS_PATHS {
        surfaces.push((
            path,
            JOURNAL_PROGRESS,
            &["PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED"],
            StatusCode::BAD_REQUEST,
        ));
    }
    surfaces
}

#[tokio::test]
async fn each_route_answers_its_identity_refuses_others_and_is_off_without_its_flag() -> TestResult
{
    let app = app(&enabled()).await?;
    for (path, identity, flags, reached) in surfaces() {
        assert_eq!(
            status(&app, identity, path).await?,
            reached,
            "{path} own identity"
        );
        assert_eq!(
            status(&app, STRANGER, path).await?,
            StatusCode::UNAUTHORIZED,
            "{path} unknown identity"
        );
        let mut off = enabled();
        for flag in flags {
            off.remove(*flag);
        }
        let off = app_without_lcfa(&off).await?;
        assert_eq!(
            status(&off, identity, path).await?,
            StatusCode::SERVICE_UNAVAILABLE,
            "{path} with {flags:?} unset"
        );
    }
    Ok(())
}

async fn app_without_lcfa(values: &Values) -> Result<Router, Box<dyn std::error::Error>> {
    let community = Router::new().route(
        oteryn_community::v2_watermark::PATH,
        post(|| async { "community" }),
    );
    Ok(Routes::configured(pool()?, values).await.mount(community))
}

#[tokio::test]
async fn defaults_leave_every_route_unavailable() -> TestResult {
    let app = app_without_lcfa(&Values::new()).await?;
    for (path, identity, _, _) in surfaces() {
        assert_eq!(
            status(&app, identity, path).await?,
            StatusCode::SERVICE_UNAVAILABLE,
            "{path}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn character_command_identities_reach_only_the_command_routes() -> TestResult {
    let app = app(&enabled()).await?;
    for identity in [COMMAND_A, COMMAND_B] {
        for path in COMMAND_PATHS {
            assert_eq!(
                status(&app, identity, path).await?,
                StatusCode::BAD_REQUEST,
                "{path}"
            );
        }
        let mut others: Vec<&str> = surfaces()
            .into_iter()
            .map(|(path, ..)| path)
            .filter(|path| !COMMAND_PATHS.contains(path))
            .collect();
        others.push(LCFA_PATH);
        for path in others {
            assert_eq!(
                status(&app, identity, path).await?,
                StatusCode::UNAUTHORIZED,
                "{identity} on {path}"
            );
        }
    }
    // And no other purpose reaches the command routes.
    for identity in [
        RESTORE, SERVICES, BOOTSTRAP, PREMIUM, LOYALTY, STORE, PUBLIC, LCFA,
    ] {
        for path in COMMAND_PATHS {
            assert_eq!(
                status(&app, identity, path).await?,
                StatusCode::UNAUTHORIZED,
                "{identity} on {path}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn reused_or_malformed_identities_keep_every_route_unavailable() -> TestResult {
    let mut cases = Vec::new();
    // A Character Authority host reused by another purpose, here or elsewhere.
    for key in [
        PREMIUM_IDENTITY,
        "PORTAL_LCFA_INTERNAL_IDENTITY",
        "PORTAL_SANCTION_EXPORT_IDENTITY",
    ] {
        let mut values = enabled();
        values.insert(key.into(), COMMAND_A.into());
        cases.push(values);
    }
    for raw in [
        r#"["char-authority-a.oteryn.internal"]"#,
        r#"{"char-authority-a.oteryn.internal":true}"#,
        r#"{"Char-Authority-A.oteryn.internal":[]}"#,
        r#"{"char-authority-a.oteryn.internal":[],"char-authority-a.oteryn.internal":[]}"#,
        "not json",
    ] {
        let mut values = enabled();
        values.insert(CHARACTER_COMMAND_IDENTITIES.into(), raw.into());
        cases.push(values);
    }
    for values in cases {
        assert!(bindings(&values).is_err());
        let app = app_without_lcfa(&values).await?;
        for (path, identity, _, _) in surfaces() {
            assert_eq!(
                status(&app, identity, path).await?,
                StatusCode::SERVICE_UNAVAILABLE,
                "{path}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn character_commands_stay_off_outside_testing_and_preproduction() -> TestResult {
    let mut values = enabled();
    values.insert("OTERYN_ENV".into(), "production".into());
    let app = app_without_lcfa(&values).await?;
    for path in COMMAND_PATHS.iter().chain(&SERVICES_PATHS) {
        assert_eq!(
            status(&app, COMMAND_A, path).await?,
            StatusCode::SERVICE_UNAVAILABLE,
            "{path}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn v2_watermark_reaches_stats_only_for_its_families() -> TestResult {
    let app = app_without_lcfa(&enabled()).await?;
    let path = oteryn_community::v2_watermark::PATH;
    for family in ["character_online_sessions", "world_daily_kills"] {
        let body = format!(r#"{{"family":"{family}"}}"#);
        let (code, _) = call(&app, PUBLIC, path, &body).await?;
        assert_eq!(code, StatusCode::BAD_REQUEST, "{family}");
        let (code, _) = call(&app, STRANGER, path, &body).await?;
        assert_eq!(code, StatusCode::UNAUTHORIZED, "{family}");
    }
    for body in [
        r#"{"family":"guild"}"#,
        r#"{"family":"world_daily"}"#,
        "{}",
        "not json",
    ] {
        assert_eq!(
            call(&app, PUBLIC, path, body).await?,
            (StatusCode::OK, "community".to_owned()),
            "{body}"
        );
    }
    Ok(())
}

/// T72: each journal purpose answers only its own identity and source, stays
/// off outside testing/preproduction and without its source authority, and
/// with no ownership adapter a valid contract record is 503, never accepted.
#[tokio::test]
async fn journal_ingest_binds_its_purpose_source_and_environment() -> TestResult {
    let app = app_without_lcfa(&enabled()).await?;
    for (own, other, paths, source) in [
        (
            JOURNAL_ITEMS,
            JOURNAL_PROGRESS,
            JOURNAL_ITEMS_PATHS,
            "OTERYN_GAME_JOURNAL",
        ),
        (
            JOURNAL_PROGRESS,
            JOURNAL_ITEMS,
            JOURNAL_PROGRESS_PATHS,
            "oteryn:journal:primary",
        ),
    ] {
        for path in paths {
            let named = format!(r#"{{"source_authority":"{source}"}}"#);
            assert_eq!(
                call(&app, own, path, &named).await?.0,
                StatusCode::BAD_REQUEST
            );
            assert_eq!(
                call(&app, own, path, r#"{"source_authority":"oteryn:other"}"#).await?,
                (StatusCode::UNAUTHORIZED, String::new()),
                "{path}"
            );
            assert_eq!(
                call(&app, other, path, &named).await?.0,
                StatusCode::UNAUTHORIZED,
                "{other} on {path}"
            );
        }
    }
    let bank = concat!(
        r#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","#,
        r#""account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","#,
        r#""character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"#,
        r#""projection_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","#,
        r#""record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","#,
        r#""kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","#,
        r#""event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","#,
        r#""previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}"#
    );
    assert_eq!(
        call(&app, JOURNAL_ITEMS, JOURNAL_ITEMS_PATHS[0], bank).await?,
        (StatusCode::SERVICE_UNAVAILABLE, String::new())
    );
    for (key, value) in [
        ("OTERYN_ENV", Some("production")),
        ("OTERYN_ENV", None),
        ("PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY", None),
        ("PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN", None),
    ] {
        let mut values = enabled();
        match value {
            Some(value) => values.insert(key.into(), value.into()),
            None => values.remove(key),
        };
        let off = app_without_lcfa(&values).await?;
        assert_eq!(
            status(&off, JOURNAL_ITEMS, JOURNAL_ITEMS_PATHS[0]).await?,
            StatusCode::SERVICE_UNAVAILABLE,
            "{key}={value:?}"
        );
    }
    for (key, value) in [
        ("OTERYN_ENV", Some("production")),
        ("OTERYN_ENV", None),
        ("PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY", None),
        ("PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN", None),
    ] {
        let mut values = enabled();
        match value {
            Some(value) => values.insert(key.into(), value.into()),
            None => values.remove(key),
        };
        let off = app_without_lcfa(&values).await?;
        for path in JOURNAL_PROGRESS_PATHS {
            assert_eq!(
                status(&off, JOURNAL_PROGRESS, path).await?,
                StatusCode::SERVICE_UNAVAILABLE,
                "{path} {key}={value:?}"
            );
        }
    }
    // A journal identity reused by another purpose disables every route.
    let mut values = enabled();
    values.insert(PREMIUM_IDENTITY.into(), JOURNAL_PROGRESS.into());
    assert!(bindings(&values).is_err());
    Ok(())
}

#[tokio::test]
async fn store_purchases_stay_off_outside_testing_and_preproduction() -> TestResult {
    // On, a stranger is refused at the identity check: the route is mounted.
    let app = app_without_lcfa(&enabled()).await?;
    for path in PURCHASE_PATHS {
        assert_eq!(
            status(&app, STRANGER, path).await?,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    // Off, the surface answers 503 before any identity check.
    for (key, value) in [
        ("OTERYN_ENV", Some("production")),
        ("OTERYN_ENV", None),
        ("APP_ENV", Some("production")),
        ("APP_ENV", None),
        ("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED", None),
    ] {
        let mut values = enabled();
        match value {
            Some(value) => values.insert(key.into(), value.into()),
            None => values.remove(key),
        };
        let app = app_without_lcfa(&values).await?;
        for path in PURCHASE_PATHS {
            for identity in [STORE, STRANGER] {
                assert_eq!(
                    status(&app, identity, path).await?,
                    StatusCode::SERVICE_UNAVAILABLE,
                    "{path} {identity} with {key}={value:?}"
                );
            }
        }
    }
    Ok(())
}
