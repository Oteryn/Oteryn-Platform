use super::http::{GROUP, SNAPSHOT_PATH, WATERMARK_PATH};
use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use oteryn_core::Environment;
use oteryn_internal_api::{
    canonical::{self, Value},
    fixtures,
    guard::{Binding, Identity, IdentityMap, VerifiedClient},
};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ACCOUNT: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6";
const OTHER: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c7";
const NOW: i64 = 1_790_000_005;
// Producer vectors preserved by the Laravel contract test in this repository.
const SNAPSHOT: &str = r#"{"contract_version":1,"operation":"PublishAccountCharactersV1","source_authority":"oteryn:character-authority:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","projection_revision":"42","source_observed_at":"1790000000","characters":[{"character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","availability":"AVAILABLE"}]}"#;
const WATERMARK: &str = r#"{"contract_version":1,"operation":"PublishProjectionWatermarkV1","source_authority":"oteryn:character-authority:primary","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}"#;
const ACCEPTED: &[u8] = br#"{"contract_version":1,"result":"accepted"}"#;
const SUPERSEDED: &[u8] = br#"{"contract_version":1,"result":"superseded"}"#;

#[derive(Debug)]
struct TestAccounts(AccountState);
impl AccountStateLookup for TestAccounts {
    fn state(&self, _: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async { self.0 })
    }
}
fn settings() -> Settings {
    Settings {
        enabled: true,
        environment: Environment::Testing,
        source_authority: "oteryn:character-authority:primary".into(),
        ..Settings::default()
    }
}
fn identities() -> Result<Arc<IdentityMap>, Box<dyn std::error::Error>> {
    Ok(Arc::new(IdentityMap::new(vec![
        Binding {
            identity: Identity::san("character-projection")?,
            group: GROUP.into(),
        },
        Binding {
            identity: Identity::san("runtime-status")?,
            group: "runtime-status".into(),
        },
        Binding {
            identity: Identity::san("character-projection-b")?,
            group: GROUP.into(),
        },
    ])?))
}
fn request(
    path: &str,
    raw: impl Into<Body>,
    peer: Option<&str>,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(raw.into())?;
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                peer,
            )?));
    }
    Ok(request)
}
async fn body(response: Response) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, no-cache, must-revalidate, private"
    );
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    assert_eq!(response.headers()[header::EXPIRES], "0");
    Ok(to_bytes(response.into_body(), 16384).await?.to_vec())
}
fn revision(raw: &str, revision: &str) -> String {
    raw.replace(
        "\"projection_revision\":\"42\"",
        &format!("\"projection_revision\":\"{revision}\""),
    )
}
fn epoch(raw: &str, epoch: &str) -> String {
    raw.replace(
        "\"projection_epoch\":\"1\"",
        &format!("\"projection_epoch\":\"{epoch}\""),
    )
}

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED LCFA database test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("lcfa_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        // Identifier contains only a fixed prefix and generated hexadecimal UUID.
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |connection, _| {
                let query = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(&query)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105145653_t18_account_characters.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            admin,
            pool,
            schema,
        }))
    }
    fn projection(&self) -> Projection {
        self.with_account(AccountState::Active)
    }
    fn with_account(&self, state: AccountState) -> Projection {
        Projection::new(self.pool.clone(), settings(), Arc::new(TestAccounts(state)))
    }
    fn app(&self) -> Result<Router, Box<dyn std::error::Error>> {
        Ok(router(self.projection(), identities()?))
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}

#[test]
fn golden_codecs_and_fixture_harness() -> TestResult {
    Snapshot::decode(SNAPSHOT.as_bytes())?;
    Watermark::decode(WATERMARK.as_bytes())?;
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/contracts/fixtures");
    for fixture in fixtures::load(root)? {
        match fixture.value.get("operation") {
            Some(Value::String(operation)) if operation == "PublishAccountCharactersV1" => {
                Snapshot::decode(&fixture.canonical)?;
            }
            Some(Value::String(operation)) if operation == "PublishProjectionWatermarkV1" => {
                Watermark::decode(&fixture.canonical)?;
            }
            _ => {}
        }
    }
    assert_eq!(
        canonical::encode(&canonical::decode(SNAPSHOT.as_bytes(), 16384, 4)?)?,
        SNAPSHOT.as_bytes()
    );
    // Laravel allows canonical compact objects in any member order.
    let reordered = SNAPSHOT.replace(
        "\"contract_version\":1,\"operation\":\"PublishAccountCharactersV1\"",
        "\"operation\":\"PublishAccountCharactersV1\",\"contract_version\":1",
    );
    assert_eq!(
        Snapshot::decode(reordered.as_bytes())?.digest,
        Snapshot::decode(SNAPSHOT.as_bytes())?.digest
    );
    Ok(())
}

#[test]
fn malformed_envelopes_identifiers_names_and_uint64_are_refused() {
    for raw in [
        format!("{SNAPSHOT}\n"),
        SNAPSHOT.replace(",\"operation\"", ", \"operation\""),
        SNAPSHOT.replace("\"characters\":", "\"extra\":1,\"characters\":"),
        SNAPSHOT.replace(
            "\"characters\":",
            "\"projection_revision\":\"42\",\"characters\":",
        ),
        SNAPSHOT.replace(ACCOUNT, &ACCOUNT.to_uppercase()),
        SNAPSHOT.replace("7d5e", "4d5e"),
        revision(SNAPSHOT, "0"),
        revision(SNAPSHOT, "01"),
        revision(SNAPSHOT, "18446744073709551616"),
        SNAPSHOT.replace("\"42\"", "42"),
        SNAPSHOT.replace("\"42\"", "null"),
        SNAPSHOT.replace("AVAILABLE", "ONLINE"),
        SNAPSHOT.replace("contract_version\":1", "contract_version\":2"),
        SNAPSHOT.replace("1790000000", "9223372036854775808"),
    ] {
        assert!(Snapshot::decode(raw.as_bytes()).is_err(), "{raw}");
    }
    for name in [
        "",
        ".",
        "..",
        "Al/dric",
        "Al\"dric",
        "Al\\dric",
        "Al\ndric",
        "Al\u{202e}dric",
        "Al\u{2066}dric",
        "Al\u{200b}dric",
        "Alde\u{301}ric",
        &"N".repeat(65),
    ] {
        assert!(!super::wire::name_valid(name), "{name:?}");
    }
    for name in ["Al Dric", "Þórr", "Al.", "Al\u{2028}dric"] {
        assert!(super::wire::name_valid(name));
    }
    assert!(Snapshot::decode(revision(SNAPSHOT, "18446744073709551615").as_bytes()).is_ok());
    assert!(Watermark::decode(WATERMARK.replace("1790000000", "1790000004").as_bytes()).is_err());
}
fn sized_snapshot(count: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut value = canonical::decode(SNAPSHOT.as_bytes(), 16384, 4)?;
    let Value::Object(ref mut members) = value else {
        return Err("object required".into());
    };
    let entry = members
        .iter()
        .find(|(key, _)| key == "characters")
        .ok_or("characters")?
        .1
        .clone();
    let Value::Array(entries) = entry else {
        return Err("array required".into());
    };
    let template = canonical::encode(&entries[0])?;
    let template = String::from_utf8(template)?;
    let mut characters = Vec::new();
    for index in 0..count {
        let raw = template.replace(
            "3b1122334401\",\"world_id",
            &format!("{index:012x}\",\"world_id"),
        );
        characters.push(canonical::decode(raw.as_bytes(), 1024, 4)?);
    }
    for (key, value) in members {
        if key == "characters" {
            *value = Value::Array(characters.clone());
        }
    }
    Ok(canonical::encode(&value)?)
}
#[test]
fn list_bound_and_strict_order() -> TestResult {
    assert_eq!(Snapshot::decode(&sized_snapshot(64)?)?.characters.len(), 64);
    assert_eq!(
        Snapshot::decode(&sized_snapshot(65)?).err(),
        Some(Error::BadInput)
    );
    let duplicate = SNAPSHOT.replace(
        "}]}",
        &format!(
            "}},{}]}}",
            SNAPSHOT
                .split("\"characters\":[")
                .nth(1)
                .ok_or("characters")?
                .trim_end_matches("]}")
        ),
    );
    assert!(Snapshot::decode(duplicate.as_bytes()).is_err());
    Ok(())
}

#[tokio::test]
async fn peer_flag_and_transport_failures_need_no_database() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let projection = Projection::new(pool, settings(), Arc::new(DenyAccounts));
    let app = router(projection.clone(), identities()?);
    for peer in [None, Some("runtime-status"), Some("unknown")] {
        let response = app
            .clone()
            .oneshot(request(SNAPSHOT_PATH, SNAPSHOT, peer)?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(body(response).await?.is_empty());
    }
    for (key, value, status) in [
        (header::CONTENT_TYPE, "text/plain", StatusCode::BAD_REQUEST),
        (
            header::TRANSFER_ENCODING,
            "chunked",
            StatusCode::BAD_REQUEST,
        ),
        (header::CONTENT_ENCODING, "gzip", StatusCode::BAD_REQUEST),
        (
            header::CONTENT_LENGTH,
            "16385",
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let mut request = request(SNAPSHOT_PATH, SNAPSHOT, Some("character-projection"))?;
        request.headers_mut().insert(key, value.parse()?);
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), status);
        assert!(body(response).await?.is_empty());
    }
    let response = app
        .oneshot(request(
            WATERMARK_PATH,
            "x".repeat(513),
            Some("character-projection"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    for environment in [Environment::Local, Environment::Production] {
        let mut projection = projection.clone();
        projection.settings.environment = environment;
        let response = router(projection, identities()?)
            .oneshot(request(
                SNAPSHOT_PATH,
                SNAPSHOT,
                Some("character-projection"),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(body(response).await?.is_empty());
    }
    let mut projection = projection;
    projection.settings = Settings::default();
    let response = router(projection, identities()?)
        .oneshot(request(
            SNAPSHOT_PATH,
            SNAPSHOT,
            Some("character-projection"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

#[tokio::test]
async fn db_golden_http_acknowledgements_and_bad_input() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let app = db.app()?;
    for (path, raw) in [(SNAPSHOT_PATH, SNAPSHOT), (WATERMARK_PATH, WATERMARK)] {
        let response = app
            .clone()
            .oneshot(request(path, raw, Some("character-projection"))?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await?, ACCEPTED);
    }
    let response = app
        .clone()
        .oneshot(request(
            WATERMARK_PATH,
            WATERMARK,
            Some("character-projection"),
        )?)
        .await?;
    assert_eq!(body(response).await?, SUPERSEDED);
    let response = app
        .clone()
        .oneshot(request(
            SNAPSHOT_PATH,
            revision(SNAPSHOT, "41"),
            Some("character-projection"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await?, SUPERSEDED);
    for raw in [
        sized_snapshot(65)?,
        SNAPSHOT.replace("primary", "foreign").into_bytes(),
        SNAPSHOT
            .replace("1790000000", "9223372036854775807")
            .into_bytes(),
    ] {
        let response = app
            .clone()
            .oneshot(request(
                SNAPSHOT_PATH,
                raw.clone(),
                Some("character-projection"),
            )?)
            .await?;
        assert_eq!(
            response.status(),
            if raw.windows(7).any(|s| s == b"foreign") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            }
        );
        assert!(body(response).await?.is_empty());
    }
    let mut req = request(SNAPSHOT_PATH, SNAPSHOT, Some("character-projection"))?;
    req.headers_mut().insert(
        header::CONTENT_TYPE,
        "Application/JSON; charset=utf-8".parse()?,
    );
    req.headers_mut()
        .insert(header::CONTENT_ENCODING, "Identity".parse()?);
    assert_eq!(app.clone().oneshot(req).await?.status(), StatusCode::OK);
    let mut req = request(SNAPSHOT_PATH, SNAPSHOT, Some("character-projection"))?;
    req.headers_mut()
        .insert(header::CONTENT_LENGTH, "1".parse()?);
    assert_eq!(app.oneshot(req).await?.status(), StatusCode::BAD_REQUEST);
    db.close().await
}

#[tokio::test]
async fn db_conflict_persists_and_higher_revision_repairs_atomically() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    let account = AccountId::parse(ACCOUNT)?;
    p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW)
        .await?;
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Ready
    );
    p.snapshot(
        Snapshot::decode(SNAPSHOT.replace("1790000000", "1790000004").as_bytes())?,
        NOW,
    )
    .await?;
    let observed: i64 =
        sqlx::query_scalar("SELECT source_observed_at FROM account_character_snapshots")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(observed, 1_790_000_000);
    assert_eq!(
        p.snapshot(
            Snapshot::decode(SNAPSHOT.replace("Aldric", "Other").as_bytes())?,
            NOW
        )
        .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Invalid
    );
    assert_eq!(
        p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW)
            .await,
        Err(Error::Conflict)
    );
    p.snapshot(
        Snapshot::decode(
            revision(SNAPSHOT, "43")
                .replace("Aldric", "New Name")
                .as_bytes(),
        )?,
        NOW,
    )
    .await?;
    let view = p.list_characters(account, NOW).await?;
    assert_eq!(view.state, ReadState::Ready);
    assert_eq!(view.characters.len(), 1);
    assert_eq!(view.characters[0].name, "New Name");
    let mut empty = Snapshot::decode(revision(SNAPSHOT, "44").as_bytes())?;
    empty.characters.clear();
    empty.digest = vec![0; 32];
    p.snapshot(empty, NOW).await?;
    assert_eq!(p.list_characters(account, NOW).await?.characters.len(), 0);
    db.close().await
}

#[tokio::test]
async fn db_epoch_raise_from_either_publication_invalidates_other_accounts() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    let account = AccountId::parse(ACCOUNT)?;
    let other = AccountId::parse(OTHER)?;
    p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW)
        .await?;
    p.snapshot(
        Snapshot::decode(SNAPSHOT.replace(ACCOUNT, OTHER).as_bytes())?,
        NOW,
    )
    .await?;
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    p.snapshot(Snapshot::decode(epoch(SNAPSHOT, "2").as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Stale
    );
    assert_eq!(
        p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
            .await?,
        Outcome::Superseded
    );
    p.watermark(Watermark::decode(epoch(WATERMARK, "2").as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Ready
    );
    assert_eq!(
        p.list_characters(other, NOW).await?.state,
        ReadState::Invalid
    );
    p.watermark(
        Watermark::decode(epoch(WATERMARK, "18446744073709551615").as_bytes())?,
        NOW,
    )
    .await?;
    assert_eq!(
        p.snapshot(Snapshot::decode(epoch(SNAPSHOT, "2").as_bytes())?, NOW)
            .await?,
        Outcome::Superseded
    );
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Invalid
    );
    p.snapshot(
        Snapshot::decode(epoch(SNAPSHOT, "18446744073709551615").as_bytes())?,
        NOW,
    )
    .await?;
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Ready
    );
    db.close().await
}

#[tokio::test]
async fn db_watermark_freshness_boundaries_missing_and_unknown_accounts() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    let account = AccountId::parse(ACCOUNT)?;
    p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(account, NOW).await?.state,
        ReadState::Stale
    );
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(account, NOW + 24).await?.state,
        ReadState::Ready
    );
    assert_eq!(
        p.list_characters(account, NOW + 25).await?.state,
        ReadState::Stale
    );
    assert_eq!(
        p.list_characters(AccountId::parse(OTHER)?, NOW)
            .await?
            .state,
        ReadState::Missing
    );
    for state in [
        AccountState::Denied,
        AccountState::Unknown,
        AccountState::Ambiguous,
        AccountState::Unavailable,
    ] {
        let view = db.with_account(state).list_characters(account, NOW).await?;
        assert_eq!(view.state, ReadState::Denied);
        assert!(view.characters.is_empty());
    }
    let denied = Projection::new(db.pool.clone(), settings(), Arc::new(DenyAccounts));
    assert_eq!(
        denied.list_characters(account, NOW).await?.state,
        ReadState::Denied
    );
    // A repeated or regressing watermark never moves the freshness fence.
    assert_eq!(
        p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
            .await?,
        Outcome::Superseded
    );
    assert_eq!(
        p.watermark(
            Watermark::decode(WATERMARK.replace("1790000000", "1789999999").as_bytes())?,
            NOW
        )
        .await?,
        Outcome::Superseded
    );
    db.close().await
}

#[tokio::test]
async fn db_concurrent_snapshots_wait_for_feed_lock_and_finish_at_highest_revision() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await?;
    let mut tasks = Vec::new();
    for rev in [43, 46, 41, 45, 44] {
        let p = p.clone();
        tasks.push(tokio::spawn(async move {
            p.snapshot(
                Snapshot::decode(
                    revision(SNAPSHOT, &rev.to_string())
                        .replace("Aldric", &format!("Name {rev}"))
                        .as_bytes(),
                )?,
                NOW,
            )
            .await
        }));
    }
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert!(tasks.iter().all(|task| !task.is_finished()));
    blocker.commit().await?;
    for task in tasks {
        task.await??;
    }
    let row = sqlx::query("SELECT projection_revision, invalid FROM account_character_snapshots")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(row.try_get::<String, _>("projection_revision")?, "46");
    assert!(!row.try_get::<bool, _>("invalid")?);
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    let view = p.list_characters(AccountId::parse(ACCOUNT)?, NOW).await?;
    assert_eq!(view.characters.len(), 1);
    assert_eq!(view.characters[0].name, "Name 46");
    db.close().await
}

#[tokio::test]
async fn db_rate_limit_is_shared_across_routes_and_separate_per_peer() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut p = db.projection();
    p.settings.requests_per_minute = 1;
    let app = router(p, identities()?);
    assert_eq!(
        app.clone()
            .oneshot(request(
                SNAPSHOT_PATH,
                SNAPSHOT,
                Some("character-projection")
            )?)
            .await?
            .status(),
        StatusCode::OK
    );
    let response = app
        .clone()
        .oneshot(request(
            WATERMARK_PATH,
            WATERMARK,
            Some("character-projection"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    assert!(body(response).await?.is_empty());
    assert_eq!(
        app.clone()
            .oneshot(request(
                WATERMARK_PATH,
                WATERMARK,
                Some("character-projection-b")
            )?)
            .await?
            .status(),
        StatusCode::OK
    );
    sqlx::query("UPDATE account_character_publisher_limits SET reset_at=0")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        app.oneshot(request(
            WATERMARK_PATH,
            WATERMARK,
            Some("character-projection")
        )?)
        .await?
        .status(),
        StatusCode::OK
    );
    db.close().await
}

#[tokio::test]
async fn db_failed_replacement_rolls_back_epoch_metadata_and_rows() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW)
        .await?;
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    sqlx::query(
        "ALTER TABLE account_characters ADD CONSTRAINT test_refuse_name CHECK (name <> 'Refused')",
    )
    .execute(&db.pool)
    .await?;
    let replacement = epoch(&revision(SNAPSHOT, "43"), "2").replace("Aldric", "Refused");
    assert_eq!(
        p.snapshot(Snapshot::decode(replacement.as_bytes())?, NOW)
            .await,
        Err(Error::Unavailable)
    );
    let view = p.list_characters(AccountId::parse(ACCOUNT)?, NOW).await?;
    assert_eq!(view.state, ReadState::Ready);
    assert_eq!(view.characters[0].name, "Aldric");
    let highest: String = sqlx::query_scalar("SELECT highest_epoch FROM account_character_feed")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(highest, "1");
    db.close().await
}

#[tokio::test]
async fn db_concurrent_equal_revision_conflict_stays_invalid_until_higher_pair() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = db.projection();
    let (left, right) = tokio::join!(
        p.snapshot(Snapshot::decode(SNAPSHOT.as_bytes())?, NOW),
        p.snapshot(
            Snapshot::decode(SNAPSHOT.replace("Aldric", "Contender").as_bytes())?,
            NOW
        )
    );
    assert!(
        (left == Ok(Outcome::Accepted) && right == Err(Error::Conflict))
            || (right == Ok(Outcome::Accepted) && left == Err(Error::Conflict))
    );
    p.watermark(Watermark::decode(WATERMARK.as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(AccountId::parse(ACCOUNT)?, NOW)
            .await?
            .state,
        ReadState::Invalid
    );
    p.snapshot(Snapshot::decode(revision(SNAPSHOT, "43").as_bytes())?, NOW)
        .await?;
    assert_eq!(
        p.list_characters(AccountId::parse(ACCOUNT)?, NOW)
            .await?
            .state,
        ReadState::Ready
    );
    db.close().await
}
