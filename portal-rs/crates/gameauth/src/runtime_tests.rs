use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[derive(Debug)]
pub(crate) struct TestError(String);
impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for TestError {}
macro_rules! convert_errors { ($($kind:ty),*) => { $(impl From<$kind> for TestError { fn from(error:$kind)->Self { Self(error.to_string()) } })* }; }
convert_errors!(
    sqlx::migrate::MigrateError,
    StatusCode,
    sqlx::Error,
    canonical::Error,
    guard::ConfigError,
    sqlx::types::uuid::Error,
    axum::Error,
    axum::http::Error,
    std::io::Error,
    std::convert::Infallible,
    &str,
    tokio::sync::TryAcquireError,
    Box<dyn std::error::Error>
);
pub(crate) type TestResult = Result<(), TestError>;
pub(crate) const WORLD: &str = "019c0000-0000-7000-8000-000000000001";
pub(crate) const CHANNEL: &str = "019c0000-0000-7000-8000-000000000002";
pub(crate) const ACCOUNT: &str = "019c0000-0000-7000-8000-000000000003";
pub(crate) fn value(revision: &str, observed: i64) -> Result<Value, Failure> {
    let mut fields = vec![
        ("contract_version", Value::Unsigned(1)),
        ("operation", string("ReportRuntimeStatusV1")),
        ("source_authority", string("game")),
        ("world_id", string(WORLD)),
        ("channel_id", string(CHANNEL)),
        ("node_id", string("019c0000-0000-4000-8000-000000000004")),
        ("assignment_epoch", string("1")),
        ("scope_ownership_generation", string("1")),
        ("source_revision", string(revision)),
        ("decision_identity", string("publication")),
        ("ready", Value::Bool(true)),
        ("published_at", string(10)),
        ("protocol_major", Value::Unsigned(1)),
        ("transport_profile", Value::Unsigned(1)),
    ];
    for key in [
        "route_revision",
        "runtime_observation_revision",
        "ruleset_revision",
        "content_revision",
        "map_revision",
        "world_policy_revision",
        "offer_revision",
    ] {
        fields.push((key, string("r1")));
    }
    fields.push(("observed_at", string(observed)));
    Ok(object(fields))
}
pub(crate) fn settings() -> Result<Settings, TestError> {
    Ok(Settings {
        enabled: true,
        peers: vec![Peer {
            name: "node".into(),
            identity: Identity::san("node")?,
            scopes: vec![(WORLD.parse()?, CHANNEL.parse()?)],
        }],
        ..Settings::default()
    })
}
pub(crate) fn identities() -> Result<Arc<IdentityMap>, TestError> {
    Ok(Arc::new(IdentityMap::new(vec![
        Binding {
            identity: Identity::san("node")?,
            group: "native-runtime-status".into(),
        },
        Binding {
            identity: Identity::san("evidence")?,
            group: "native-evidence".into(),
        },
    ])?))
}
pub(crate) fn request(
    path: &str,
    identity: Option<&str>,
    bytes: Vec<u8>,
) -> Result<Request<Body>, TestError> {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))?;
    if let Some(identity) = identity {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                identity,
            )?));
    }
    Ok(request)
}

pub(crate) struct Database {
    pub pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    pub(crate) async fn new() -> Result<Option<Self>, TestError> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("t19b_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |conn, _| {
                let path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    pub(crate) async fn assignments(&self) -> TestResult {
        // Fixture-only authority. Production assignment schema and writers belong to T20.
        sqlx::raw_sql("CREATE TABLE native_assignment_epoch_locks (id smallint PRIMARY KEY); INSERT INTO native_assignment_epoch_locks VALUES (1); CREATE TABLE native_scope_assignments (world_id uuid, channel_id uuid, assignment_epoch numeric(20,0), ownership_generation numeric(20,0), node_identity text, PRIMARY KEY(world_id,channel_id));").execute(&self.pool).await?;
        sqlx::query("INSERT INTO native_scope_assignments VALUES ($1,$2,1,1,'node')")
            .bind(WORLD.parse::<Uuid>()?)
            .bind(CHANNEL.parse::<Uuid>()?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub(crate) async fn close(self) -> TestResult {
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
fn runtime_wire_rejects_bad_types_ids_versions_and_fields() -> TestResult {
    let good = value("18446744073709551615", 20)?;
    Report::decode(&good)?;
    for (field, bad_value) in [
        ("source_revision", string("18446744073709551616")),
        ("assignment_epoch", string("01")),
        ("ready", string("true")),
        ("world_id", string("00000000-0000-0000-0000-000000000000")),
        ("channel_id", string("019C0000-0000-7000-8000-000000000002")),
        ("published_at", string(21)),
        ("observed_at", string("9223372036854775808")),
        ("operation", string("ReportScopeAssignmentV1")),
        ("contract_version", Value::Unsigned(2)),
    ] {
        let mut changed = good.clone();
        if let Value::Object(members) = &mut changed {
            let item = members
                .iter_mut()
                .find(|(k, _)| k == field)
                .ok_or("missing test field")?;
            item.1 = bad_value;
        }
        assert!(Report::decode(&changed).is_err(), "{field}");
    }
    Ok(())
}
#[tokio::test]
async fn runtime_guard_refuses_disabled_wrong_identity_bad_json_and_size() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let path = "/internal/v1/game-auth/native-runtime-status";
    let app: Router = router(pool.clone(), Settings::default(), identities()?);
    assert_eq!(
        app.oneshot(request(
            path,
            Some("node"),
            canonical::encode(&value("1", 20)?)?
        )?)
        .await?
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let app: Router = router(pool, settings()?, identities()?);
    for peer in [None, Some("evidence"), Some("unknown")] {
        let response = app
            .clone()
            .oneshot(request(path, peer, b"{}".to_vec())?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(to_bytes(response.into_body(), 1024).await?.is_empty());
    }
    for bytes in [
        b"{\"contract_version\":1,\"contract_version\":1}".to_vec(),
        b"[]".to_vec(),
        b"{\"x\":{}}".to_vec(),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(path, Some("node"), bytes)?)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        app.oneshot(request(path, Some("node"), vec![b' '; 2049])?)
            .await?
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    Ok(())
}
#[tokio::test]
async fn runtime_throttle_counts_semantic_refusals_and_is_per_peer() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let mut options = settings()?;
    options.requests_per_minute = 1;
    let app: Router = router(pool, options, identities()?);
    let path = "/internal/v1/game-auth/native-runtime-status";
    assert_eq!(
        app.clone()
            .oneshot(request(path, Some("node"), b"{}".to_vec())?)
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let response = app
        .oneshot(request(path, Some("node"), b"{}".to_vec())?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    let throttle = Throttle::default();
    assert!(throttle.admit("a", 1).is_ok());
    assert!(throttle.admit("a", 1).is_err());
    assert!(throttle.admit("b", 1).is_ok());
    Ok(())
}
#[tokio::test]
async fn db_runtime_replay_refresh_conflict_recovery_and_freshness() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.assignments().await?;
    let report = Report::decode(&value("1", 20)?)?;
    assert_eq!(ingest(&db.pool, "node", &report).await?, "accepted");
    assert_eq!(ingest(&db.pool, "node", &report).await?, "superseded");
    assert_eq!(
        ingest(&db.pool, "node", &Report::decode(&value("1", 21)?)?).await?,
        "refreshed"
    );
    let options = settings()?;
    assert_eq!(
        read(&db.pool, &options, report.world_id, report.channel_id, 30)
            .await?
            .state,
        EvidenceState::Fresh
    );
    assert_eq!(
        read(&db.pool, &options, report.world_id, report.channel_id, 40)
            .await?
            .state,
        EvidenceState::Stale
    );
    let mut conflict = value("1", 22)?;
    if let Value::Object(members) = &mut conflict {
        members
            .iter_mut()
            .find(|(k, _)| k == "ready")
            .ok_or("ready")?
            .1 = Value::Bool(false);
    }
    assert_eq!(
        ingest(&db.pool, "node", &Report::decode(&conflict)?).await,
        Err(StatusCode::CONFLICT)
    );
    assert_eq!(
        ingest(&db.pool, "node", &report).await,
        Err(StatusCode::CONFLICT)
    );
    assert_eq!(
        read(&db.pool, &options, report.world_id, report.channel_id, 23)
            .await?
            .state,
        EvidenceState::Invalid
    );
    assert_eq!(
        ingest(&db.pool, "node", &Report::decode(&value("2", 24)?)?).await?,
        "accepted"
    );
    assert_eq!(ingest(&db.pool, "node", &report).await?, "superseded");
    assert_eq!(
        read(&db.pool, &options, report.world_id, report.channel_id, 25)
            .await?
            .state,
        EvidenceState::Fresh
    );
    db.close().await
}
#[tokio::test]
async fn db_runtime_rejects_missing_stale_owner_epoch_scope_and_future() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.assignments().await?;
    let report = Report::decode(&value("1", 20)?)?;
    assert_eq!(
        ingest(&db.pool, "other", &report).await,
        Err(StatusCode::CONFLICT)
    );
    sqlx::query("UPDATE native_scope_assignments SET ownership_generation=2")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        ingest(&db.pool, "node", &report).await,
        Err(StatusCode::CONFLICT)
    );
    sqlx::query("UPDATE native_scope_assignments SET ownership_generation=1,assignment_epoch=2")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        ingest(&db.pool, "node", &report).await,
        Err(StatusCode::CONFLICT)
    );
    sqlx::query("DELETE FROM native_scope_assignments")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        ingest(&db.pool, "node", &report).await,
        Err(StatusCode::CONFLICT)
    );
    let app: Router = router(db.pool.clone(), settings()?, identities()?);
    let path = "/internal/v1/game-auth/native-runtime-status";
    let future = value("1", now()? + 6)?;
    assert_eq!(
        app.clone()
            .oneshot(request(path, Some("node"), canonical::encode(&future)?)?)
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut other = value("1", 20)?;
    if let Value::Object(members) = &mut other {
        members
            .iter_mut()
            .find(|(k, _)| k == "channel_id")
            .ok_or("channel")?
            .1 = string(ACCOUNT);
    }
    assert_eq!(
        app.oneshot(request(path, Some("node"), canonical::encode(&other)?)?)
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    db.close().await
}
#[tokio::test]
async fn db_runtime_concurrent_revisions_and_restart_keep_latest() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.assignments().await?;
    let a = Report::decode(&value("1", 20)?)?;
    let b = Report::decode(&value("2", 21)?)?;
    let (left, right) = tokio::join!(ingest(&db.pool, "node", &a), ingest(&db.pool, "node", &b));
    left?;
    right?;
    let observation = read(&db.pool, &settings()?, a.world_id, a.channel_id, 22).await?;
    assert_eq!(
        text(&observation.content.ok_or("content")?, "source_revision")?,
        "2"
    );
    assert_eq!(ingest(&db.pool, "node", &b).await?, "superseded");
    db.close().await
}

#[tokio::test]
async fn configured_mount_is_off_by_default_and_refuses_overlapping_or_bad_config() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let path = "/internal/v1/game-auth/native-runtime-status";
    let app: Router = configured_routes(pool.clone(), |_| None);
    assert_eq!(
        app.oneshot(request(path, Some("node"), b"{}".to_vec())?)
            .await?
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    for rate in ["0", "601", "01", "true", ""] {
        let app: Router = configured_routes(pool.clone(), |key| match key {
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED" => Some("true".into()),
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_REQUESTS_PER_MINUTE" => Some(rate.into()),
            _ => None,
        });
        assert_eq!(
            app.oneshot(request(path, Some("node"), b"{}".to_vec())?)
                .await?
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    let raw = format!("{{\"node\":[\"{WORLD}/{CHANNEL}\"]}}");
    assert!(
        configuration(|key| match key {
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(raw.clone()),
            "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY" => Some("node".into()),
            _ => None,
        })
        .is_none()
    );
    Ok(())
}
#[tokio::test]
async fn native_http_header_bounds_and_content_length_are_enforced() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let app: Router = router(pool, settings()?, identities()?);
    let path = "/internal/v1/game-auth/native-runtime-status";
    let mut oversized = request(path, Some("node"), b"{}".to_vec())?;
    oversized.headers_mut().insert(
        "x-large",
        header::HeaderValue::from_str(&"a".repeat(2048)).map_err(|_| "header")?,
    );
    assert_eq!(
        app.clone().oneshot(oversized).await?.status(),
        StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
    );
    let mut many = request(path, Some("node"), b"{}".to_vec())?;
    for i in 0..32 {
        many.headers_mut().insert(
            axum::http::HeaderName::from_bytes(format!("x-{i}").as_bytes())
                .map_err(|_| "header")?,
            header::HeaderValue::from_static("v"),
        );
    }
    assert_eq!(
        app.clone().oneshot(many).await?.status(),
        StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
    );
    let mut mismatch = request(path, Some("node"), b"{}".to_vec())?;
    mismatch.headers_mut().insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_static("1"),
    );
    assert_eq!(
        app.oneshot(mismatch).await?.status(),
        StatusCode::BAD_REQUEST
    );
    Ok(())
}
#[tokio::test]
async fn db_runtime_http_success_replay_and_global_epoch_reset() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.assignments().await?;
    let app: Router = router(db.pool.clone(), settings()?, identities()?);
    let path = "/internal/v1/game-auth/native-runtime-status";
    for expected in ["accepted", "superseded"] {
        let mut req = request(path, Some("node"), canonical::encode(&value("1", 20)?)?)?;
        req.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("Application/JSON; charset=UTF-8"),
        );
        let response = app.clone().oneshot(req).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 256).await?.as_ref(),
            format!("{{\"contract_version\":1,\"result\":\"{expected}\"}}").as_bytes()
        );
    }
    sqlx::query("INSERT INTO native_scope_assignments VALUES ($1,$2,2,1,'node')")
        .bind(ACCOUNT.parse::<Uuid>()?)
        .bind(ACCOUNT.parse::<Uuid>()?)
        .execute(&db.pool)
        .await?;
    let report = Report::decode(&value("2", 21)?)?;
    assert_eq!(
        ingest(&db.pool, "node", &report).await,
        Err(StatusCode::CONFLICT)
    );
    assert_eq!(
        read(
            &db.pool,
            &settings()?,
            report.world_id,
            report.channel_id,
            22
        )
        .await?
        .state,
        EvidenceState::Invalid
    );
    db.close().await
}
