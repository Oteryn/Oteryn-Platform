#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! §19.6.2 wire and §19.6.3 acceptance against the protocol 3 fixtures.
use super::*;
use crate::runtime::tests::{Database, TestResult, request};
use axum::body::to_bytes;
use oteryn_internal_api::{canonical, guard::Identity};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/contracts/fixtures/native-gateway-protocol3-v1/"
);
const WORLD: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70";
const CHANNEL: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71";
const ACCOUNT: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d";
const CHARACTER: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURES}{name}")).unwrap()
}
fn report() -> String {
    fixture("locator-report.json")
}
fn decode(raw: &str) -> Result<Report, Failure> {
    let value = canonical::decode(raw.as_bytes(), MAX_REPORT_BYTES, 2).map_err(|_| bad())?;
    Report::decode(&value)
}
fn settings() -> Settings {
    Settings {
        enabled: true,
        peers: vec![Peer {
            name: "node".into(),
            identity: Identity::san("node").unwrap(),
            scopes: vec![(WORLD.parse().unwrap(), CHANNEL.parse().unwrap())],
        }],
        projection_authority: "test-source".into(),
        clock_uncertainty_seconds: 0,
        requests_per_minute: 600,
    }
}
fn identities() -> Arc<IdentityMap> {
    Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san("node").unwrap(),
                group: GROUP.into(),
            },
            Binding {
                identity: Identity::san("runtime").unwrap(),
                group: "native-runtime-status".into(),
            },
        ])
        .unwrap(),
    )
}
async fn send(app: &Router, identity: &str, body: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(request(PATH, Some(identity), body.as_bytes().to_vec()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[test]
fn report_fixture_decodes_and_debug_hides_private_evidence() {
    let decoded = decode(&report()).unwrap();
    assert_eq!(decoded.world_id.to_string(), WORLD);
    assert_eq!(decoded.account_id.to_string(), ACCOUNT);
    assert_eq!(decoded.character_id.to_string(), CHARACTER);
    assert!(decoded.present);
    assert_eq!(
        (
            decoded.assignment_epoch,
            decoded.ownership_generation,
            decoded.locator_revision
        ),
        (1, 1, 1)
    );
    assert_eq!(decoded.committed_at, 1_791_763_200);
    let debug = format!("{decoded:?}");
    assert!(!debug.contains("assignment_epoch") && !debug.contains(ACCOUNT));
    // Member order is free; the member set is exact.
    let value = canonical::decode(report().as_bytes(), MAX_REPORT_BYTES, 2).unwrap();
    let Value::Object(mut members) = value else {
        panic!()
    };
    members.reverse();
    assert_eq!(Report::decode(&Value::Object(members)).unwrap(), decoded);
}

#[test]
fn bad_reports_are_malformed() {
    let good = report();
    let cases = [
        good.replacen('{', "{\"extra\":1,", 1),
        good.replace(",\"present\":true", ""),
        good.replace("\"present\":true", "\"present\":null"),
        good.replace("\"present\":true", "\"present\":\"true\""),
        good.replace("\"contract_version\":1", "\"contract_version\":2"),
        good.replace("ReportRecoveryLocatorV1", "ReportRuntimeStatusV1"),
        good.replace("\"assignment_epoch\":1", "\"assignment_epoch\":0"),
        good.replace("\"assignment_epoch\":1", "\"assignment_epoch\":\"1\""),
        good.replace(
            "\"scope_ownership_generation\":1",
            "\"scope_ownership_generation\":-1",
        ),
        good.replace("\"locator_revision\":1", "\"locator_revision\":1.5"),
        good.replace(ACCOUNT, &ACCOUNT.to_uppercase()),
        good.replace(CHARACTER, "0192b3c4-5d6e-4f80-8a1b-2c3d4e5f6a7c"),
        good.replace(WORLD, "00000000-0000-0000-0000-000000000000"),
        good.replace("oteryn:game:primary", ""),
        good.replace("oteryn:game:primary", "bad authority"),
        good.replace("2026-10-12T00:00:00Z", "2026-10-12T00:00:00.000Z"),
        good.replace("2026-10-12T00:00:00Z", "2026-10-12T00:00:00+00:00"),
        good.replace("2026-10-12T00:00:00Z", "2026-02-29T00:00:00Z"),
        good.replace("2026-10-12T00:00:00Z", "2026-10-12T24:00:00Z"),
        good.replace("2026-10-12T00:00:00Z", "2026-10-12t00:00:00Z"),
        good.replace(
            "\"committed_at\":\"2026-10-12T00:00:00Z\"",
            "\"committed_at\":1791763200",
        ),
        "[]".into(),
    ];
    for raw in cases {
        assert_ne!(raw, good);
        assert_eq!(decode(&raw).unwrap_err(), bad(), "{raw}");
    }
}

#[test]
fn utc_seconds_is_exact() {
    assert_eq!(utc_seconds("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(utc_seconds("2000-02-29T23:59:59Z"), Some(951_868_799));
    assert_eq!(utc_seconds("1969-12-31T23:59:59Z"), Some(-1));
    for bad in [
        "2100-02-29T00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-00-01T00:00:00Z",
        "2026-01-01T00:60:00Z",
        "+026-01-01T00:00:00Z",
        "2026-01-01T00:00:00",
    ] {
        assert_eq!(utc_seconds(bad), None, "{bad}");
    }
}

#[tokio::test]
async fn route_is_off_by_default_and_keeps_identities_apart() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")
        .unwrap();
    let off: Router = runtime::configured_routes(pool.clone(), |_| None);
    assert_eq!(
        send(&off, "node", &report()).await,
        (StatusCode::SERVICE_UNAVAILABLE, String::new())
    );
    let scope = format!("{{\"node\":[\"{WORLD}/{CHANNEL}\"]}}");
    let runtime_scope = format!("{{\"runtime\":[\"{WORLD}/{CHANNEL}\"]}}");
    let lookup = |key: &str| match key {
        "OTERYN_ENV" => Some("testing".into()),
        "PORTAL_NATIVE_PROTOCOL3_ENABLED"
        | "PORTAL_NATIVE_RECOVERY_ENABLED"
        | "GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED"
        | "OTERYN_LCFA_ENABLED" => Some("true".into()),
        "OTERYN_LCFA_SOURCE_AUTHORITY" => Some("test-source".into()),
        IDENTITIES => Some(scope.clone()),
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(runtime_scope.clone()),
        _ => None,
    };
    let (_, _, locator, _) = runtime::configuration(lookup).unwrap();
    assert!(locator.enabled && locator.valid());
    assert_eq!(locator.projection_authority, "test-source");
    let app: Router = runtime::configured_routes(pool.clone(), lookup);
    // Another purpose's identity is refused here, a locator identity elsewhere.
    assert_eq!(
        send(&app, "runtime", &report()).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&app, "stranger", &report()).await.0,
        StatusCode::UNAUTHORIZED
    );
    let runtime_route = app
        .clone()
        .oneshot(
            request(
                "/internal/v1/game-auth/native-runtime-status",
                Some("node"),
                b"{}".to_vec(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(runtime_route.status(), StatusCode::UNAUTHORIZED);
    // Outside its scope list.
    let other = report().replace(CHANNEL, "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a72");
    assert_eq!(send(&app, "node", &other).await.0, StatusCode::UNAUTHORIZED);
    // A report over 1024 bytes.
    let large = report().replacen('{', &format!("{{\"pad\":\"{}\",", "a".repeat(700)), 1);
    assert_eq!(
        send(&app, "node", &large).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Each switch is required; preproduction allows it, production never.
    for (key, value) in [
        ("PORTAL_NATIVE_PROTOCOL3_ENABLED", "false"),
        ("PORTAL_NATIVE_RECOVERY_ENABLED", "false"),
        ("OTERYN_ENV", "production"),
    ] {
        let (_, _, locator, _) = runtime::configuration(|k| {
            if k == key {
                Some(value.into())
            } else {
                lookup(k)
            }
        })
        .unwrap();
        assert!(!locator.enabled, "{key}");
    }
    let (_, _, locator, _) = runtime::configuration(|k| {
        if k == "OTERYN_ENV" {
            Some("preprod".into())
        } else {
            lookup(k)
        }
    })
    .unwrap();
    assert!(locator.enabled);
    // A locator certificate shared with another purpose refuses the whole map.
    assert!(
        runtime::configuration(|k| match k {
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(scope.clone()),
            _ => lookup(k),
        })
        .is_none()
    );
    assert!(
        runtime::configuration(|k| match k {
            "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY" => Some("node".into()),
            _ => lookup(k),
        })
        .is_none()
    );
    assert!(
        runtime::configuration(|k| match k {
            IDENTITIES => Some(format!("{{\"node\":[\"{WORLD}\"]}}")),
            _ => lookup(k),
        })
        .is_none()
    );
}

async fn seed(db: &Database) -> TestResult {
    sqlx::query("INSERT INTO native_scope_assignments (world_id,channel_id,assignment_epoch,ownership_generation,node_identity,assigned_at) VALUES ($1,$2,1,1,'node',0)")
        .bind(WORLD.parse::<Uuid>()?).bind(CHANNEL.parse::<Uuid>()?)
        .execute(&db.pool).await?;
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test-source',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)")
        .bind(ACCOUNT.parse::<Uuid>()?).bind(vec![0u8; 32]).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','UNAVAILABLE')")
        .bind(ACCOUNT.parse::<Uuid>()?).bind(CHARACTER.parse::<Uuid>()?).bind(WORLD.parse::<Uuid>()?)
        .execute(&db.pool).await?;
    Ok(())
}
/// (epoch, revision, present, conflicted) of the Character's entry.
async fn entry(db: &Database) -> Option<(String, String, bool, bool)> {
    sqlx::query_as("SELECT assignment_epoch::text, locator_revision::text, present, conflicted FROM native_recovery_locators WHERE character_id=$1")
        .bind(CHARACTER.parse::<Uuid>().unwrap())
        .fetch_optional(&db.pool).await.unwrap()
}
fn revision(raw: &str, n: u64) -> String {
    raw.replace(
        "\"locator_revision\":1",
        &format!("\"locator_revision\":{n}"),
    )
}

#[tokio::test]
async fn db_locator_acceptance_ordering_and_refusals() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    seed(&db).await?;
    let app: Router = router(db.pool.clone(), settings(), identities());
    let accepted = fixture("locator-accepted.json");
    let duplicate = fixture("locator-duplicate.json");
    let superseded = fixture("locator-superseded.json");
    let good = report();

    // Rule 5 first: an unverifiable read model refuses and stores nothing.
    sqlx::query("UPDATE account_character_snapshots SET invalid=true")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        send(&app, "node", &good).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let blind = Settings {
        projection_authority: String::new(),
        ..settings()
    };
    sqlx::query("UPDATE account_character_snapshots SET invalid=false")
        .execute(&db.pool)
        .await?;
    let dark: Router = router(db.pool.clone(), blind, identities());
    assert_eq!(
        send(&dark, "node", &good).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(entry(&db).await, None);
    // Not the owner the read model lists: without a valid snapshot of that
    // account ownership is unverifiable; with one, it lists no such Character.
    let other = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e";
    let stranger = good.replace(ACCOUNT, other);
    assert_eq!(
        send(&app, "node", &stranger).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'test-source','1','1',0,$2)")
        .bind(other.parse::<Uuid>()?).bind(vec![0u8; 32]).execute(&db.pool).await?;
    assert_eq!(send(&app, "node", &stranger).await.0, StatusCode::CONFLICT);
    // Owner listed in another World.
    sqlx::query("UPDATE account_characters SET world_id=$1")
        .bind(Uuid::now_v7())
        .execute(&db.pool)
        .await?;
    assert_eq!(send(&app, "node", &good).await.0, StatusCode::CONFLICT);
    sqlx::query("UPDATE account_characters SET world_id=$1")
        .bind(WORLD.parse::<Uuid>()?)
        .execute(&db.pool)
        .await?;
    // Rule 4: a future committed_at.
    let future = good.replace("2026-10-12T00:00:00Z", "2999-01-01T00:00:00Z");
    assert_eq!(send(&app, "node", &future).await.0, StatusCode::BAD_REQUEST);
    // Rule 2: an assignment mismatch.
    let stale = good.replace(
        "\"scope_ownership_generation\":1",
        "\"scope_ownership_generation\":2",
    );
    assert_eq!(send(&app, "node", &stale).await.0, StatusCode::CONFLICT);
    // Rule 1: a deposed owner in scope is refused.
    sqlx::query("UPDATE native_scope_assignments SET node_identity='successor'")
        .execute(&db.pool)
        .await?;
    assert_eq!(send(&app, "node", &good).await.0, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE native_scope_assignments SET node_identity='node'")
        .execute(&db.pool)
        .await?;
    assert_eq!(entry(&db).await, None);

    // Ordering by (assignment_epoch, locator_revision).
    assert_eq!(
        send(&app, "node", &good).await,
        (StatusCode::OK, accepted.clone())
    );
    assert_eq!(
        send(&app, "node", &good).await,
        (StatusCode::OK, duplicate.clone())
    );
    assert_eq!(
        send(&app, "node", &revision(&good, 2)).await,
        (StatusCode::OK, accepted.clone())
    );
    assert_eq!(
        send(&app, "node", &good).await,
        (StatusCode::OK, superseded)
    );
    let absent = revision(&good, 2).replace("\"present\":true", "\"present\":false");
    assert_eq!(send(&app, "node", &absent).await.0, StatusCode::CONFLICT);
    assert_eq!(entry(&db).await, Some(("1".into(), "2".into(), true, true)));
    assert_eq!(
        send(&app, "node", &revision(&good, 2)).await,
        (StatusCode::OK, duplicate)
    );
    assert_eq!(
        send(
            &app,
            "node",
            &revision(&good, 3).replace("\"present\":true", "\"present\":false")
        )
        .await,
        (StatusCode::OK, accepted.clone())
    );
    assert_eq!(
        entry(&db).await,
        Some(("1".into(), "3".into(), false, false))
    );
    // A higher epoch replaces a higher revision of a lower epoch.
    sqlx::query("UPDATE native_scope_assignments SET assignment_epoch=2")
        .execute(&db.pool)
        .await?;
    assert_eq!(send(&app, "node", &good).await.0, StatusCode::CONFLICT);
    let next = good.replace("\"assignment_epoch\":1", "\"assignment_epoch\":2");
    assert_eq!(send(&app, "node", &next).await, (StatusCode::OK, accepted));
    assert_eq!(
        entry(&db).await,
        Some(("2".into(), "1".into(), true, false))
    );
    db.close().await
}

#[tokio::test]
async fn db_concurrent_reports_keep_the_highest_revision() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    seed(&db).await?;
    let reports: Vec<Report> = (1..=6)
        .map(|n| decode(&revision(&report(), n)).unwrap())
        .collect();
    let results = futures_join(&db.pool, reports).await;
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert_eq!(
        entry(&db).await,
        Some(("1".into(), "6".into(), true, false))
    );
    db.close().await
}
async fn futures_join(pool: &PgPool, reports: Vec<Report>) -> Vec<Result<&'static str, Failure>> {
    let mut tasks = tokio::task::JoinSet::new();
    for report in reports {
        let pool = pool.clone();
        tasks.spawn(async move { ingest(&pool, "test-source", "node", &report).await });
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap());
    }
    results
}
