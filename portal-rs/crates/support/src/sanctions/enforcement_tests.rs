use super::*;
use axum::body::to_bytes;
use oteryn_internal_api::guard::{Binding, Identity};
use sqlx::postgres::PgPoolOptions;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../../docs/contracts/fixtures/game-enforcement-wire-v1/",
            $name
        ))
    };
}
const STREAM_REQUEST: &[u8] = fixture!("stream-request.json");
const STREAM_PAGE: &[u8] = fixture!("stream-page.json");
const STREAM_EMPTY: &[u8] = fixture!("stream-empty.json");
const COMMAND_APPLY: &[u8] = fixture!("command-apply.json");
const COMMAND_REVOKE: &[u8] = fixture!("command-revoke.json");
const READ_REQUEST: &[u8] = fixture!("read-request.json");
const RESULT_APPLIED: &[u8] = fixture!("result-applied.json");
const RESULT_REVOKED: &[u8] = fixture!("result-revoked.json");
const RESULT_ACK: &[u8] = fixture!("result-ack.json");
const RECONCILE_REQUEST: &[u8] = fixture!("reconcile-request.json");
const RECONCILE_PAGE: &[u8] = fixture!("reconcile-page.json");

const READER: &str = "enforcement-reader.game.example.invalid";
const RESULTS: &str = "enforcement-results.game.example.invalid";
const OTHER: &str = "other.game.example.invalid";

/// Fixture ids `019a0000-0000-7000-8000-000000000NNN`.
fn id(n: u128) -> Uuid {
    Uuid::from_u128(0x019a0000_0000_7000_8000_000000000000 + n)
}
fn apply() -> Command {
    Command {
        operation_id: id(0x101),
        sanction_stream_id: id(0x201),
        intent: Intent::Apply,
        profile: Profile::Mute,
        decision_revision: 1,
        prior_decision_revision: None,
        policy_revision: 1,
        account_id: id(0x002),
        character_id: id(0x301),
        effective_at_ms: Some(1_700_000_000_000),
        expires_at_ms: Some(1_700_003_600_000),
        reason_code: Some(7),
        decided_at_ms: 1_700_000_000_000,
        audit_ref: id(0x401),
        correlation_id: id(0x501),
    }
}
fn revoke() -> Command {
    Command {
        operation_id: id(0x102),
        intent: Intent::Revoke,
        decision_revision: 2,
        prior_decision_revision: Some(1),
        effective_at_ms: None,
        expires_at_ms: None,
        reason_code: Some(12),
        decided_at_ms: 1_700_000_900_000,
        audit_ref: id(0x402),
        correlation_id: id(0x502),
        ..apply()
    }
}
fn mute() -> Profiles {
    Profiles {
        mute: true,
        jail: false,
    }
}
fn identities() -> TestResult<Arc<IdentityMap>> {
    Ok(Arc::new(IdentityMap::new(vec![
        Binding {
            identity: Identity::san(READER)?,
            group: READER_GROUP.into(),
        },
        Binding {
            identity: Identity::san(RESULTS)?,
            group: RESULTS_GROUP.into(),
        },
        Binding {
            identity: Identity::san(OTHER)?,
            group: "OTHER_MTLS_CLIENT_IDENTITY".into(),
        },
    ])?))
}
fn on(limit: u32) -> Settings {
    Settings {
        reader_enabled: true,
        results_enabled: true,
        profiles: mute(),
        requests_per_minute: limit,
    }
}
fn lazy_pool() -> TestResult<PgPool> {
    Ok(PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?)
}
fn request(path: &str, body: impl Into<Body>, identity: Option<&str>) -> TestResult<Request> {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.into())?;
    if let Some(san) = identity {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                san,
            )?));
    }
    Ok(request)
}
fn peer_of(path: &str) -> &'static str {
    if path == RESULTS_PATH {
        RESULTS
    } else {
        READER
    }
}
async fn send(
    app: &Router,
    request: TestResult<Request>,
) -> TestResult<(StatusCode, Vec<u8>, Response)> {
    let response = app.clone().oneshot(request?).await?;
    let status = response.status();
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static("no-store, private"))
    );
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, MAX_PAGE + 1).await?.to_vec();
    if status != StatusCode::OK {
        assert!(bytes.is_empty(), "non-200 body must be empty");
    }
    Ok((status, bytes, Response::from_parts(parts, Body::empty())))
}
async fn status(app: &Router, request: TestResult<Request>) -> TestResult<StatusCode> {
    Ok(send(app, request).await?.0)
}
const ALL: [(&str, &[u8]); 4] = [
    (STREAM_PATH, STREAM_REQUEST),
    (COMMAND_PATH, READ_REQUEST),
    (RESULTS_PATH, RESULT_APPLIED),
    (RECONCILE_PATH, RECONCILE_REQUEST),
];

#[test]
fn command_bytes_and_digests_match_fixtures() -> TestResult {
    assert_eq!(apply().encode()?, COMMAND_APPLY);
    assert_eq!(revoke().encode()?, COMMAND_REVOKE);
    assert_eq!(
        digest(COMMAND_APPLY),
        "54184dd57a532cf775eb9096898f9ca22c2f258e33f5693afc3af40a3511035f"
    );
    assert_eq!(
        digest(COMMAND_REVOKE),
        "4be8fbb95a5db8bba9f01b4dd2db83823d6d645d57b4110b030ead9bd037e9cd"
    );
    assert_eq!(
        digest(RESULT_APPLIED),
        "5f97970a439dafa68c599fd3f7b8bb1384e8330e507781fd58395499ad959f5f"
    );
    // Each result's command_digest is the SHA-256 of its command fixture.
    for (result, command) in [
        (RESULT_APPLIED, COMMAND_APPLY),
        (RESULT_REVOKED, COMMAND_REVOKE),
    ] {
        let value = canonical::decode_canonical(result, MAX_RESULT, 4, &RESULT_SHAPE)?;
        let receipt = Receipt::decode(&value)?;
        assert_eq!(receipt.command_digest, digest(command));
        assert_eq!(canonical::encode(&value)?, result);
    }
    // Every fixture is compact canonical JSON with no terminal newline.
    for bytes in [
        STREAM_REQUEST,
        STREAM_PAGE,
        STREAM_EMPTY,
        READ_REQUEST,
        RESULT_ACK,
        RECONCILE_REQUEST,
        RECONCILE_PAGE,
    ] {
        let value = canonical::decode(bytes, MAX_PAGE, 4)?;
        assert_eq!(canonical::encode(&value)?, bytes);
    }
    assert_eq!(
        encode_page("oteryn.enforcement_command_stream_page.v1", vec![])?,
        STREAM_EMPTY
    );
    assert_eq!(
        encode_page(
            "oteryn.enforcement_command_stream_page.v1",
            vec![(1, COMMAND_APPLY.to_vec()), (2, COMMAND_REVOKE.to_vec())]
        )?,
        STREAM_PAGE
    );
    Ok(())
}

#[test]
fn command_rules_refuse_invalid_combinations() {
    let cases: Vec<Command> = vec![
        Command {
            prior_decision_revision: Some(1),
            ..apply()
        },
        Command {
            expires_at_ms: None,
            ..apply()
        },
        Command {
            expires_at_ms: Some(1_700_000_000_000),
            ..apply()
        },
        Command {
            reason_code: None,
            ..apply()
        },
        Command {
            reason_code: Some(0),
            ..apply()
        },
        Command {
            decision_revision: 1,
            ..revoke()
        },
        Command {
            effective_at_ms: Some(1),
            ..revoke()
        },
        Command {
            intent: Intent::Replace,
            ..revoke()
        },
        Command {
            intent: Intent::Expire,
            ..apply()
        },
        Command {
            account_id: Uuid::nil(),
            ..apply()
        },
        Command {
            character_id: Uuid::new_v4(),
            ..apply()
        },
        Command {
            policy_revision: 0,
            ..apply()
        },
        Command {
            decided_at_ms: 0,
            ..apply()
        },
    ];
    for case in cases {
        assert!(matches!(case.encode(), Err(Error::Invalid)), "{case:?}");
    }
    let expire = Command {
        intent: Intent::Expire,
        prior_decision_revision: None,
        effective_at_ms: None,
        expires_at_ms: None,
        reason_code: None,
        ..apply()
    };
    assert!(expire.encode().is_ok());
}

#[test]
fn settings_default_off_and_refuse_unknown_values() {
    assert_eq!(Settings::from_lookup(|_| None), Settings::default());
    let lookup = |pairs: &'static [(&'static str, &'static str)]| {
        Settings::from_lookup(move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        })
    };
    let enabled = lookup(&[
        ("PORTAL_GAME_ENFORCEMENT_ENABLED", "true"),
        ("PORTAL_GAME_ENFORCEMENT_PROFILES", r#"["MUTE","JAIL"]"#),
    ]);
    assert!(enabled.reader_enabled && enabled.results_enabled);
    assert!(enabled.profiles.enabled(Profile::Mute) && enabled.profiles.enabled(Profile::Jail));
    assert_eq!(enabled.requests_per_minute, 1200);
    assert!(!lookup(&[("PORTAL_GAME_ENFORCEMENT_ENABLED", "TRUE")]).reader_enabled);
    for (key, value) in [
        ("PORTAL_GAME_ENFORCEMENT_PROFILES", r#"["MUTE","BAN"]"#),
        ("PORTAL_GAME_ENFORCEMENT_PROFILES", r#"["MUTE","MUTE"]"#),
        ("PORTAL_GAME_ENFORCEMENT_PROFILES", r#""MUTE""#),
        ("PORTAL_GAME_ENFORCEMENT_REQUESTS_PER_MINUTE", "0"),
        ("PORTAL_GAME_ENFORCEMENT_REQUESTS_PER_MINUTE", "6001"),
        ("PORTAL_GAME_ENFORCEMENT_REQUESTS_PER_MINUTE", "060"),
    ] {
        let settings = Settings::from_lookup(|k| match k {
            "PORTAL_GAME_ENFORCEMENT_ENABLED" => Some("true".into()),
            k if k == key => Some(value.into()),
            _ => None,
        });
        assert!(
            !settings.reader_enabled && !settings.results_enabled,
            "{key}={value}"
        );
        assert_eq!(settings.profiles, Profiles::default());
    }
}

/// Guard order 1: flag off or group not configured is 503 before identity or body.
#[tokio::test]
async fn refusal_flag_off_or_unconfigured_is_503_first() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let off = router(lazy_pool()?, Settings::default(), identities()?);
    let unconfigured = router(lazy_pool()?, on(1200), Arc::new(IdentityMap::default()));
    for app in [&off, &unconfigured] {
        for (path, _) in ALL {
            for identity in [None, Some(peer_of(path)), Some(OTHER)] {
                assert_eq!(
                    status(app, request(path, "not json", identity)).await?,
                    StatusCode::SERVICE_UNAVAILABLE
                );
            }
        }
    }
    // Each group has its own switch.
    let reader_only = router(
        lazy_pool()?,
        Settings {
            results_enabled: false,
            ..on(1200)
        },
        identities()?,
    );
    assert_eq!(
        status(
            &reader_only,
            request(RESULTS_PATH, RESULT_APPLIED, Some(RESULTS))
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        status(&reader_only, request(STREAM_PATH, "{}", Some(READER))).await?,
        StatusCode::BAD_REQUEST
    );
    Ok(())
}

/// Guard order 2: wrong, missing or header-only identity, and the other group's identity.
#[tokio::test]
async fn refusal_wrong_identity_or_scope_is_401() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let app = router(lazy_pool()?, on(1200), identities()?);
    for (path, body) in ALL {
        let foreign = if path == RESULTS_PATH {
            READER
        } else {
            RESULTS
        };
        for identity in [None, Some(OTHER), Some(foreign)] {
            assert_eq!(
                status(&app, request(path, body, identity)).await?,
                StatusCode::UNAUTHORIZED,
                "{path} {identity:?}"
            );
        }
        let mut spoofed = request(path, body, None)?;
        for name in ["x-client-san", "ssl-client-s-dn", "x-forwarded-client-cert"] {
            spoofed
                .headers_mut()
                .insert(name, HeaderValue::from_static(READER));
        }
        assert_eq!(status(&app, Ok(spoofed)).await?, StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

/// Guard order 3: the per-group budget counts malformed bodies and precedes transport.
#[tokio::test]
async fn refusal_budget_precedes_transport_and_counts_malformed() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let app = router(lazy_pool()?, on(1), identities()?);
    assert_eq!(
        status(&app, request(STREAM_PATH, "{", Some(READER))).await?,
        StatusCode::BAD_REQUEST
    );
    let mut bad_transport = request(COMMAND_PATH, "{", Some(READER))?;
    bad_transport
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    let (code, _, response) = send(&app, Ok(bad_transport)).await?;
    assert_eq!(code, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response.headers().get(header::RETRY_AFTER),
        Some(&HeaderValue::from_static("60"))
    );
    // The results group has its own budget; unauthenticated peers spend none.
    assert_eq!(
        status(&app, request(RESULTS_PATH, "x", Some(OTHER))).await?,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(&app, request(RESULTS_PATH, "x", Some(RESULTS))).await?,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status(&app, request(RESULTS_PATH, "x", Some(RESULTS))).await?,
        StatusCode::TOO_MANY_REQUESTS
    );
    Ok(())
}

/// Guard order 4 and §1.4: transport, size limits and malformed bodies are 400.
#[tokio::test]
async fn refusal_transport_size_and_malformed_body_is_400() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let app = router(lazy_pool()?, on(6000), identities()?);
    for (path, body) in ALL {
        let peer = Some(peer_of(path));
        let mut cases = Vec::new();
        let mut typed = request(path, body, peer)?;
        typed.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        cases.push(typed);
        let mut twice = request(path, body, peer)?;
        twice.headers_mut().append(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        cases.push(twice);
        let mut chunked = request(path, body, peer)?;
        chunked.headers_mut().insert(
            header::TRANSFER_ENCODING,
            HeaderValue::from_static("chunked"),
        );
        cases.push(chunked);
        let mut gzip = request(path, body, peer)?;
        gzip.headers_mut()
            .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        cases.push(gzip);
        cases.push(request(&format!("{path}?a=1"), body, peer)?);
        let bound = if path == RESULTS_PATH {
            MAX_RESULT
        } else {
            256
        };
        let mut declared = request(path, vec![b' '; bound + 1], peer)?;
        declared
            .headers_mut()
            .insert(header::CONTENT_LENGTH, HeaderValue::from(bound + 1));
        cases.push(declared);
        cases.push(request(path, vec![b' '; bound + 1], peer)?);
        for case in cases {
            assert_eq!(
                status(&app, Ok(case)).await?,
                StatusCode::BAD_REQUEST,
                "{path}"
            );
        }
    }
    let page_cases = |schema: &str| {
        vec![
            String::new(),
            "[]".into(),
            r#"{"schema":"other","after":null,"limit":1}"#.into(),
            format!(r#"{{"schema":"{schema}","after":null}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":0}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":501}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":1.0}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":1e2}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":"1"}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":1,"limit":2}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":1,"extra":1}}"#),
            format!(r#"{{"schema":"{schema}","after":"v1:0","limit":1}}"#),
            format!(r#"{{"schema":"{schema}","after":"v1:01","limit":1}}"#),
            format!(r#"{{"schema":"{schema}","after":"v2:1","limit":1}}"#),
            format!(r#"{{"schema":"{schema}","after":"v1:9223372036854775808","limit":1}}"#),
            format!(r#"{{"schema":"{schema}","after":1,"limit":1}}"#),
            format!(r#"{{"schema":"{schema}","after":null,"limit":1,"x":{{"y":{{"z":[]}}}}}}"#),
        ]
    };
    for (path, schema) in [
        (STREAM_PATH, "oteryn.enforcement_command_stream_request.v1"),
        (RECONCILE_PATH, "oteryn.enforcement_reconcile_request.v1"),
    ] {
        for body in page_cases(schema) {
            assert_eq!(
                status(&app, request(path, body.clone(), Some(READER))).await?,
                StatusCode::BAD_REQUEST,
                "{body}"
            );
        }
    }
    for body in [
        r#"{"schema":"oteryn.enforcement_command_read_request.v1"}"#,
        r#"{"schema":"oteryn.enforcement_command_read_request.v1","operation_id":"019A0000-0000-7000-8000-000000000101"}"#,
        r#"{"schema":"oteryn.enforcement_command_read_request.v1","operation_id":"019a0000-0000-4000-8000-000000000101"}"#,
        r#"{"schema":"oteryn.enforcement_command_read_request.v1","operation_id":"019a0000000070008000000000000101"}"#,
        r#"{"schema":"oteryn.enforcement_stream_request.v1","operation_id":"019a0000-0000-7000-8000-000000000101"}"#,
    ] {
        assert_eq!(
            status(&app, request(COMMAND_PATH, body, Some(READER))).await?,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    // A3 is canonical: whitespace, member order, escapes and every body rule.
    let applied = std::str::from_utf8(RESULT_APPLIED)?;
    let mut results = vec![
        format!("{applied}\n"),
        applied.replacen(',', ", ", 1),
        applied.replacen("\"state\":\"APPLIED\"", "\"state\":\"\\u0041PPLIED\"", 1),
        applied.replacen(
            r#""receipt_revision":1,"state":"APPLIED""#,
            r#""state":"APPLIED","receipt_revision":1"#,
            1,
        ),
        applied.replacen(
            r#""decided_at_ms":1700000000500"#,
            r#""decided_at_ms":1700000000500,"x":1"#,
            1,
        ),
    ];
    for (from, to) in [
        (
            r#""schema":"oteryn.enforcement_result.v1""#,
            r#""schema":"oteryn.enforcement_result.v2""#,
        ),
        (
            "019a0000-0000-7000-8000-000000000101",
            "019a0000-0000-4000-8000-000000000101",
        ),
        (
            "54184dd57a532cf775eb9096898f9ca22c2f258e33f5693afc3af40a3511035f",
            "54184DD57A532CF775EB9096898F9CA22C2F258E33F5693AFC3AF40A3511035F",
        ),
        (r#""receipt_revision":1"#, r#""receipt_revision":0"#),
        (r#""receipt_revision":1"#, r#""receipt_revision":65536"#),
        (r#""result_code":"OK""#, r#""result_code":"STALE_REVISION""#),
        (r#""state":"APPLIED""#, r#""state":"UNKNOWN_TO_GAME""#),
        (
            r#""state":"APPLIED","result_code":"OK""#,
            r#""state":"REJECTED","result_code":"OK""#,
        ),
        (
            r#""state":"APPLIED","result_code":"OK""#,
            r#""state":"ACCEPTED_PENDING","result_code":"OK""#,
        ),
        (
            r#""accepted_decision_revision":1"#,
            r#""accepted_decision_revision":-1"#,
        ),
        (r#""stream_state":"ACTIVE""#, r#""stream_state":"REVOKED""#),
        (
            r#""effective_until_ms":1700003600000"#,
            r#""effective_until_ms":null"#,
        ),
        (
            r#""runtime_effect":"ONLINE_APPLIED""#,
            r#""runtime_effect":"LATER""#,
        ),
        (
            r#""capability_revision":"enforcement.1""#,
            r#""capability_revision":"Enforcement.1""#,
        ),
        (
            r#""capability_revision":"enforcement.1""#,
            r#""capability_revision":"""#,
        ),
        (r#""recovery_generation":1"#, r#""recovery_generation":0"#),
        (r#""decided_at_ms":1700000000500"#, r#""decided_at_ms":0"#),
    ] {
        assert!(applied.contains(from), "{from}");
        results.push(applied.replacen(from, to, 1));
    }
    for body in results {
        assert_ne!(body.as_bytes(), RESULT_APPLIED);
        assert_eq!(
            status(&app, request(RESULTS_PATH, body.clone(), Some(RESULTS))).await?,
            StatusCode::BAD_REQUEST,
            "{body:?}"
        );
    }
    Ok(())
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> TestResult<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP enforcement wire PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("enforcement_wire_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |conn, _| {
                let path = path.clone();
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
        for migration in [
            include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
            include_str!("../../../../migrations/20261105170000_t10_rbac_audit.sql"),
            include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
            include_str!("../../../../migrations/20261105220235_t34_sanctions.sql"),
            include_str!("../../../../migrations/20261105220236_t34b_appeals.sql"),
            include_str!("../../../../migrations/20261105231600_support_sanction_revisions.sql"),
            include_str!("../../../../migrations/20261111140002_n009_sanction_export.sql"),
            include_str!("../../../../migrations/20261115100001_support_enforcement_commands.sql"),
            include_str!("../../../../migrations/20261122170100_support_enforcement_decisions.sql"),
            include_str!("../../../../migrations/20261122170600_enforcement_expiry_block.sql"),
            include_str!("../../../../migrations/20261122170500_enforcement_overdue_cursor.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn record(&self, command: &Command) -> Result<Option<Recorded>, Error> {
        let mut tx = self.pool.begin().await?;
        let recorded = record(&mut tx, &mute(), command).await?;
        tx.commit().await?;
        Ok(recorded)
    }
    async fn count(&self, table: &str) -> TestResult<i64> {
        Ok(
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&self.pool)
                .await?,
        )
    }
    async fn cleanup(self) -> TestResult {
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
fn result_with(from: &str, to: &str) -> String {
    let applied = String::from_utf8_lossy(RESULT_APPLIED);
    assert!(applied.contains(from), "{from}");
    applied.replacen(from, to, 1)
}

#[tokio::test]
async fn routes_serve_fixture_bytes_on_postgres() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = router(db.pool.clone(), on(6000), identities()?);
    let call = |path: &'static str, body: &'static [u8]| request(path, body, Some(peer_of(path)));

    let (code, bytes, _) = send(&app, call(STREAM_PATH, STREAM_REQUEST)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, STREAM_EMPTY));
    assert_eq!(
        status(&app, call(COMMAND_PATH, READ_REQUEST)).await?,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status(&app, call(RESULTS_PATH, RESULT_APPLIED)).await?,
        StatusCode::NOT_FOUND
    );

    let first = db.record(&apply()).await?.ok_or("not recorded")?;
    assert_eq!(first.position, 1);
    assert_eq!(first.command_digest, digest(COMMAND_APPLY));
    assert_eq!(
        db.record(&revoke()).await?.ok_or("not recorded")?.position,
        2
    );

    let (code, bytes, response) = send(&app, call(STREAM_PATH, STREAM_REQUEST)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, STREAM_PAGE));
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&HeaderValue::from_static("application/json"))
    );
    let (code, bytes, _) = send(&app, call(COMMAND_PATH, READ_REQUEST)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, COMMAND_APPLY));
    // Paging: the cursor is the last returned position; caught up is empty.
    let (_, bytes, _) = send(
        &app,
        request(
            STREAM_PATH,
            r#"{"limit":1,"after":"v1:1","schema":"oteryn.enforcement_command_stream_request.v1"}"#,
            Some(READER),
        ),
    )
    .await?;
    let mut expected =
        br#"{"schema":"oteryn.enforcement_command_stream_page.v1","items":["#.to_vec();
    expected.extend_from_slice(COMMAND_REVOKE);
    expected.extend_from_slice(br#"],"next_cursor":"v1:2"}"#);
    assert_eq!(bytes, expected);
    let (_, bytes, _) = send(
        &app,
        request(
            STREAM_PATH,
            r#"{"schema":"oteryn.enforcement_command_stream_request.v1","after":"v1:2","limit":5}"#,
            Some(READER),
        ),
    )
    .await?;
    assert_eq!(bytes, STREAM_EMPTY);
    // A cursor beyond committed history is 400 on A1 and A4.
    for (path, schema) in [
        (STREAM_PATH, "oteryn.enforcement_command_stream_request.v1"),
        (RECONCILE_PATH, "oteryn.enforcement_reconcile_request.v1"),
    ] {
        let body = format!(r#"{{"schema":"{schema}","after":"v1:3","limit":1}}"#);
        assert_eq!(
            status(&app, request(path, body, Some(READER))).await?,
            StatusCode::BAD_REQUEST
        );
    }

    let (code, bytes, _) = send(&app, call(RESULTS_PATH, RESULT_APPLIED)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, RESULT_ACK));
    let (code, bytes, _) = send(&app, call(RECONCILE_PATH, RECONCILE_REQUEST)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, RECONCILE_PAGE));

    // An identical republication is accepted and writes nothing.
    let (code, bytes, _) = send(&app, call(RESULTS_PATH, RESULT_APPLIED)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, RESULT_ACK));
    assert_eq!(db.count("support_enforcement_results").await?, 1);
    let (code, bytes, _) = send(&app, call(RESULTS_PATH, RESULT_REVOKED)).await?;
    assert_eq!((code, bytes.as_slice()), (StatusCode::OK, RESULT_ACK));
    assert_eq!(db.count("support_enforcement_results").await?, 2);
    db.cleanup().await
}

/// Revision and fencing conflicts on A3: 409, 404 and 400 rules that need the command.
#[tokio::test]
async fn results_refuse_revision_and_fencing_conflicts_on_postgres() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = router(db.pool.clone(), on(6000), identities()?);
    let post = |body: String| request(RESULTS_PATH, body, Some(RESULTS));
    db.record(&apply()).await?;
    db.record(&revoke()).await?;
    let pending = result_with(
        r#""receipt_revision":1,"state":"APPLIED","result_code":"OK""#,
        r#""receipt_revision":1,"state":"ACCEPTED_PENDING","result_code":"PENDING_RUNTIME""#,
    );
    assert_eq!(status(&app, post(pending.clone())).await?, StatusCode::OK);
    // Changed bytes under a stored (operation_id, receipt_revision).
    assert_eq!(
        status(
            &app,
            post(pending.replacen("PENDING_RUNTIME", "PENDING_EXPIRY", 1))
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "PENDING_EXPIRY needs an EXPIRE command"
    );
    let changed = pending.replacen(
        r#""decided_at_ms":1700000000500"#,
        r#""decided_at_ms":1700000000501"#,
        1,
    );
    assert_eq!(status(&app, post(changed)).await?, StatusCode::CONFLICT);
    // A command_digest that is not the stored command's.
    let foreign = result_with(r#""receipt_revision":1"#, r#""receipt_revision":2"#).replacen(
        &digest(COMMAND_APPLY),
        &digest(COMMAND_REVOKE),
        1,
    );
    assert_eq!(status(&app, post(foreign)).await?, StatusCode::CONFLICT);
    // The stream must be the command's; the state must fit the intent.
    let wrong_stream = result_with(
        "019a0000-0000-7000-8000-000000000201",
        "019a0000-0000-7000-8000-000000000299",
    );
    assert_eq!(
        status(&app, post(wrong_stream)).await?,
        StatusCode::BAD_REQUEST
    );
    let wrong_intent = result_with(r#""state":"APPLIED""#, r#""state":"REVOKED""#).replacen(
        r#""stream_state":"ACTIVE","effective_until_ms":1700003600000"#,
        r#""stream_state":"REVOKED","effective_until_ms":null"#,
        1,
    );
    assert_eq!(
        status(&app, post(wrong_intent)).await?,
        StatusCode::BAD_REQUEST
    );
    // Unknown operation.
    let unknown = result_with(
        "019a0000-0000-7000-8000-000000000101",
        "019a0000-0000-7000-8000-000000000199",
    );
    assert_eq!(status(&app, post(unknown)).await?, StatusCode::NOT_FOUND);

    // Terminal at receipt 3.
    let terminal = result_with(r#""receipt_revision":1"#, r#""receipt_revision":3"#);
    assert_eq!(status(&app, post(terminal.clone())).await?, StatusCode::OK);
    // A result above a stored terminal, terminal or not.
    for (state, code) in [("APPLIED", "OK"), ("ACCEPTED_PENDING", "PENDING_RUNTIME")] {
        let above = result_with(
            r#""receipt_revision":1,"state":"APPLIED","result_code":"OK""#,
            &format!(r#""receipt_revision":4,"state":"{state}","result_code":"{code}""#),
        );
        assert_eq!(status(&app, post(above)).await?, StatusCode::CONFLICT);
    }
    // A second terminal below the stored one.
    let second = result_with(
        r#""receipt_revision":1,"state":"APPLIED","result_code":"OK""#,
        r#""receipt_revision":2,"state":"REJECTED","result_code":"STALE_REVISION""#,
    );
    assert_eq!(status(&app, post(second)).await?, StatusCode::CONFLICT);
    // A late non-terminal below the terminal is history and changes nothing.
    let late = pending.replacen(r#""receipt_revision":1"#, r#""receipt_revision":2"#, 1);
    assert_eq!(status(&app, post(late)).await?, StatusCode::OK);
    assert_eq!(db.count("support_enforcement_results").await?, 3);
    let (_, bytes, _) = send(
        &app,
        request(RECONCILE_PATH, RECONCILE_REQUEST, Some(READER)),
    )
    .await?;
    let page: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(page["items"][0]["state"], "APPLIED");
    assert_eq!(page["items"][0]["receipt_revision"], 3);
    assert_eq!(
        page["items"][0]["result_digest"],
        digest(terminal.as_bytes())
    );
    assert_eq!(page["items"][1]["state"], "PENDING");
    // The journal is append only.
    assert!(
        sqlx::query("DELETE FROM support_enforcement_results")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE support_enforcement_commands SET decision_revision=9")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.cleanup().await
}

/// Producer fences: profile flag, stream identity and revision preconditions.
#[tokio::test]
async fn record_fences_profiles_streams_and_revisions_on_postgres() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    // A profile not enabled writes no command (§6.1, E8).
    let mut tx = db.pool.begin().await?;
    assert_eq!(record(&mut tx, &Profiles::default(), &apply()).await?, None);
    let jail = Command {
        profile: Profile::Jail,
        ..apply()
    };
    assert_eq!(record(&mut tx, &mute(), &jail).await?, None);
    tx.commit().await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 0);
    // A later decision on a stream that never produced a command stays local.
    assert_eq!(db.record(&revoke()).await?, None);
    assert_eq!(db.count("support_enforcement_commands").await?, 0);

    db.record(&apply()).await?;
    let fresh = |n: u128| Uuid::from_u128(0x019a0000_0000_7000_8000_000000000900 + n);
    // Reused operation id, a second APPLY, or a changed profile or target.
    let reused = Command {
        sanction_stream_id: fresh(1),
        ..apply()
    };
    assert!(matches!(db.record(&reused).await, Err(Error::Conflict)));
    for case in [
        Command {
            operation_id: fresh(2),
            ..apply()
        },
        Command {
            operation_id: fresh(3),
            profile: Profile::Jail,
            ..revoke()
        },
        Command {
            operation_id: fresh(4),
            character_id: fresh(50),
            ..revoke()
        },
    ] {
        let mut tx = db.pool.begin().await?;
        let both = Profiles::parse(Some(r#"["MUTE","JAIL"]"#))?;
        assert!(matches!(
            record(&mut tx, &both, &case).await,
            Err(Error::Conflict)
        ));
    }
    // Stale precondition.
    let stale = Command {
        operation_id: fresh(5),
        decision_revision: 5,
        prior_decision_revision: Some(4),
        ..revoke()
    };
    assert!(matches!(db.record(&stale).await, Err(Error::Stale)));
    // EXPIRE names the current timed decision, once.
    let expire = |op: u128, revision: i64| Command {
        operation_id: fresh(op),
        intent: Intent::Expire,
        decision_revision: revision,
        prior_decision_revision: None,
        effective_at_ms: None,
        expires_at_ms: None,
        reason_code: None,
        ..apply()
    };
    assert!(matches!(db.record(&expire(6, 2)).await, Err(Error::Stale)));
    assert_eq!(
        db.record(&expire(7, 1))
            .await?
            .ok_or("not recorded")?
            .position,
        2
    );
    assert!(matches!(db.record(&expire(8, 1)).await, Err(Error::Stale)));
    // A gap in decision_revision (display-only N009 revisions) is allowed.
    let replace = Command {
        operation_id: fresh(9),
        intent: Intent::Replace,
        decision_revision: 4,
        prior_decision_revision: Some(1),
        effective_at_ms: Some(1_700_000_000_000),
        expires_at_ms: Some(1_700_007_200_000),
        ..revoke()
    };
    assert_eq!(
        db.record(&replace).await?.ok_or("not recorded")?.position,
        3
    );
    // Refused writes and their rollback leave no position gap.
    let committed: i64 = sqlx::query_scalar("SELECT position FROM support_enforcement_counter")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(committed, 3);
    db.cleanup().await
}

#[tokio::test]
async fn audit_w3_03_terminal_below_pending_refused_on_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    db.record(&apply()).await?;
    let app = router(db.pool.clone(), on(6000), identities()?);
    let post = |body: String| request(RESULTS_PATH, body, Some(RESULTS));
    let pending = result_with(
        r#""receipt_revision":1,"state":"APPLIED","result_code":"OK""#,
        r#""receipt_revision":2,"state":"ACCEPTED_PENDING","result_code":"PENDING_RUNTIME""#,
    );
    assert_eq!(status(&app, post(pending)).await?, StatusCode::OK);
    assert_eq!(
        status(&app, post(String::from_utf8(RESULT_APPLIED.to_vec())?)).await?,
        StatusCode::CONFLICT
    );
    assert_eq!(db.count("support_enforcement_results").await?, 1);
    let terminal = result_with(r#""receipt_revision":1"#, r#""receipt_revision":3"#);
    assert_eq!(status(&app, post(terminal)).await?, StatusCode::OK);
    let (_, bytes, _) = send(
        &app,
        request(RECONCILE_PATH, RECONCILE_REQUEST, Some(READER)),
    )
    .await?;
    let page: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(page["items"][0]["state"], "APPLIED");
    assert_eq!(page["items"][0]["receipt_revision"], 3);
    let late = result_with(
        r#""state":"APPLIED","result_code":"OK""#,
        r#""state":"ACCEPTED_PENDING","result_code":"PENDING_RUNTIME""#,
    );
    assert_eq!(status(&app, post(late)).await?, StatusCode::OK);
    db.cleanup().await?;
    eprintln!(
        "audit_w3_03_terminal_below_pending_refused_on_postgres duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn replacement_after_refusal_fences_on_games_accepted_revision() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.record(&apply()).await?;
    let recorded = db.record(&revoke()).await?.ok_or("not recorded")?;
    let body = std::str::from_utf8(RESULT_REVOKED)?
        .replace("\"state\":\"REVOKED\"", "\"state\":\"REJECTED\"")
        .replace(
            "\"result_code\":\"OK\"",
            "\"result_code\":\"LIFECYCLE_CONFLICT\"",
        )
        .replace(
            "\"accepted_decision_revision\":2",
            "\"accepted_decision_revision\":1",
        )
        .replace(
            "\"stream_state\":\"REVOKED\"",
            "\"stream_state\":\"ACTIVE\"",
        )
        .replace(
            "\"effective_until_ms\":null",
            "\"effective_until_ms\":1700003600000",
        )
        .replace(
            "\"runtime_effect\":\"ONLINE_APPLIED\"",
            "\"runtime_effect\":\"NONE\"",
        );
    let value = canonical::decode(body.as_bytes(), MAX_RESULT, 4)?;
    let receipt = Receipt::decode(&value)?;
    assert_eq!(receipt.command_digest, recorded.command_digest);
    ingest(&db.pool, &receipt, body.as_bytes()).await?;

    let retry = Command {
        operation_id: id(0x103),
        decision_revision: 3,
        ..revoke()
    };
    assert!(matches!(
        db.record(&Command {
            prior_decision_revision: Some(2),
            ..retry.clone()
        })
        .await,
        Err(Error::Stale)
    ));
    assert!(matches!(
        db.record(&Command {
            decision_revision: 2,
            ..retry.clone()
        })
        .await,
        Err(Error::Stale)
    ));
    assert_eq!(db.record(&retry).await?.ok_or("not recorded")?.position, 3);
    db.cleanup().await
}

async fn platform_apply(db: &Database) -> TestResult<Command> {
    let command = apply();
    for account in [command.account_id, id(0x501)] {
        sqlx::query(
            "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
                     VALUES ($1,$2,'fixture','active','en',now())",
        )
        .bind(account)
        .bind(format!("{account}@example.invalid"))
        .execute(&db.pool)
        .await?;
    }
    let mut tx = db.pool.begin().await?;
    assert_eq!(
        persist_platform_sanction(&mut tx, &command, id(0x501))
            .await
            .map_err(|e| format!("{e:?}"))?,
        1,
    );
    persist_typed_decision(&mut tx, &command).await?;
    assert!(record(&mut tx, &mute(), &command).await?.is_some());
    tx.commit().await?;
    Ok(command)
}

async fn receive(db: &Database, bytes: &[u8]) -> TestResult {
    let decoded = canonical::decode(bytes, MAX_RESULT, 4)?;
    let receipt = Receipt::decode(&decoded)?;
    ingest(&db.pool, &receipt, bytes).await?;
    Ok(())
}

#[tokio::test]
async fn rejected_newer_revoke_claim_cannot_confirm_release() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let command = platform_apply(&db).await?;
    receive(&db, RESULT_APPLIED).await?;
    db.record(&revoke()).await?;
    let body = std::str::from_utf8(RESULT_REVOKED)?
        .replace(
            r#""state":"REVOKED","result_code":"OK""#,
            r#""state":"REJECTED","result_code":"STALE_REVISION""#,
        )
        .replace(
            r#""accepted_decision_revision":2"#,
            r#""accepted_decision_revision":3"#,
        )
        .replace(
            r#""runtime_effect":"ONLINE_APPLIED""#,
            r#""runtime_effect":"NONE""#,
        );
    receive(&db, body.as_bytes()).await?;
    let mut tx = db.pool.begin().await?;
    let state = latest(
        &mut tx,
        command.account_id,
        command.character_id,
        BrowserProfile::Mute,
    )
    .await?
    .ok_or("missing projected state")?;
    assert!(matches!(state.confirmed_state(), StateLabel::Applied));
    assert_eq!(state.accepted_revision, Some(3));
    assert_eq!(state.confirmed_revision, Some(1));
    assert!(state.actions().is_empty());
    tx.commit().await?;
    db.cleanup().await
}

#[tokio::test]
async fn overdue_worker_checks_one_bounded_batch_and_ignores_operations_with_any_receipt()
-> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    // Backdated journal fixture, with the normal immutable publication path.
    sqlx::query(
        "ALTER TABLE support_enforcement_commands ALTER COLUMN created_at
                 SET DEFAULT clock_timestamp()-interval '5 minutes'",
    )
    .execute(&db.pool)
    .await?;
    let first = apply();
    db.record(&first).await?;
    receive(&db, RESULT_APPLIED).await?;
    let pending = Command {
        operation_id: id(0x700),
        sanction_stream_id: id(0x701),
        ..apply()
    };
    let record = db
        .record(&pending)
        .await?
        .ok_or("pending command missing")?;
    let receipt = result_with(
        r#""state":"APPLIED","result_code":"OK""#,
        r#""state":"ACCEPTED_PENDING","result_code":"PENDING_RUNTIME""#,
    )
    .replace(
        &first.operation_id.to_string(),
        &pending.operation_id.to_string(),
    )
    .replace(
        &first.sanction_stream_id.to_string(),
        &pending.sanction_stream_id.to_string(),
    )
    .replace(&digest(COMMAND_APPLY), &record.command_digest);
    receive(&db, receipt.as_bytes()).await?;
    let mut missing = Vec::new();
    for offset in 0..127 {
        let command = Command {
            operation_id: id(0x2000 + offset),
            sanction_stream_id: id(0x3000 + offset),
            ..apply()
        };
        db.record(&command).await?.ok_or("command missing")?;
        missing.push(command.operation_id);
    }
    sqlx::query("ALTER TABLE support_enforcement_commands ALTER COLUMN created_at SET DEFAULT clock_timestamp()")
        .execute(&db.pool).await?;
    db.record(&Command {
        operation_id: id(0x4000),
        sanction_stream_id: id(0x4001),
        ..apply()
    })
    .await?;
    let worker = Overdue(db.pool.clone());
    assert_eq!(worker.check().await?, missing[..126]);
    let (left, right) = tokio::join!(worker.check(), worker.check());
    let caught_up: Vec<_> = left?.into_iter().chain(right?).collect();
    assert_eq!(caught_up, missing[126..]);
    assert!(worker.check().await?.is_empty());
    assert_eq!(db.count("support_enforcement_commands").await?, 130);
    assert_eq!(db.count("support_enforcement_results").await?, 2);
    db.cleanup().await
}

#[tokio::test]
async fn refused_successors_keep_the_original_timer_and_never_publish_a_stale_expiry() -> TestResult
{
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    for (action, minutes) in [(Action::Replace, Some(15)), (Action::Revoke, None)] {
        let Some(db) = Database::new().await? else {
            return Ok(());
        };
        let command = platform_apply(&db).await?;
        receive(&db, RESULT_APPLIED).await?;
        let worker = Expiry::new(db.pool.clone(), mute());
        let journal = BrowserJournal {
            profiles: mute(),
            exchange_available: true,
            authority: None,
        };
        let target = Target {
            account: command.account_id,
            character: command.character_id,
            name: "Test character".into(),
        };
        let operation = Uuid::now_v7();
        let timer = || {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT reconcile_at_ms FROM support_enforcement_decisions WHERE operation_id=$1",
            )
            .bind(command.operation_id)
            .fetch_one(&db.pool)
        };
        let mut tx = db.pool.begin().await?;
        journal
            .submit(
                &mut tx,
                &target,
                Decision {
                    actor: id(0x501),
                    profile: BrowserProfile::Mute,
                    action,
                    operation,
                    version: 1,
                    minutes,
                    reason: oteryn_admin::console::enforcement::Reason::RuleViolation,
                },
            )
            .await
            .map_err(|error| format!("{error:?}"))?;
        tx.commit().await?;
        worker.reconcile().await?;
        assert_eq!(timer().await?, command.expires_at_ms);
        assert_eq!(db.count("support_enforcement_commands").await?, 2);

        let bytes = read_command(&db.pool, operation)
            .await?
            .ok_or("missing successor")?;
        let refused = std::str::from_utf8(RESULT_APPLIED)?
            .replace(&command.operation_id.to_string(), &operation.to_string())
            .replace(&digest(COMMAND_APPLY), &digest(&bytes))
            .replace(
                r#""state":"APPLIED","result_code":"OK""#,
                r#""state":"REJECTED","result_code":"LIFECYCLE_CONFLICT""#,
            )
            .replace(
                r#""runtime_effect":"ONLINE_APPLIED""#,
                r#""runtime_effect":"NONE""#,
            );
        receive(&db, refused.as_bytes()).await?;
        assert_eq!(timer().await?, command.expires_at_ms);
        worker.reconcile().await?;
        worker.reconcile().await?;
        assert_eq!(timer().await?, None);
        assert_eq!(db.count("support_enforcement_commands").await?, 2);

        let mut tx = db.pool.begin().await?;
        let rows = journal
            .restrictions(&mut tx, &target)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert!(!rows[0].expiry_pending);
        assert!(rows[0].expiry_blocked);
        assert!(matches!(rows[0].state, StateLabel::Refused));
        assert!(matches!(rows[0].current, StateLabel::Applied));
        assert_eq!(rows[0].actions, &[Action::Revoke]);
        let resolved = Uuid::now_v7();
        journal
            .submit(
                &mut tx,
                &target,
                Decision {
                    actor: id(0x501),
                    profile: BrowserProfile::Mute,
                    action: Action::Revoke,
                    operation: resolved,
                    version: rows[0].version,
                    minutes: None,
                    reason: oteryn_admin::console::enforcement::Reason::RuleViolation,
                },
            )
            .await
            .map_err(|e| format!("{e:?}"))?;
        tx.commit().await?;
        let bytes = read_command(&db.pool, resolved)
            .await?
            .ok_or("missing reconciliation revoke")?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes)?["decision_revision"],
            3
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes)?["prior_decision_revision"],
            1
        );
        let revoked = std::str::from_utf8(RESULT_REVOKED)?
            .replace(&revoke().operation_id.to_string(), &resolved.to_string())
            .replace(&digest(COMMAND_REVOKE), &digest(&bytes))
            .replace(
                r#""accepted_decision_revision":2"#,
                r#""accepted_decision_revision":3"#,
            );
        receive(&db, revoked.as_bytes()).await?;
        let mut tx = db.pool.begin().await?;
        let rows = journal
            .restrictions(&mut tx, &target)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert!(!rows[0].expiry_blocked);
        assert!(matches!(rows[0].current, StateLabel::Revoked));
        tx.commit().await?;
        db.cleanup().await?;
    }
    Ok(())
}

#[tokio::test]
async fn expiry_pages_past_due_timers_with_pending_successors() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let initial = platform_apply(&db).await?;
    for n in 0..9 {
        let command = if n == 0 {
            initial.clone()
        } else {
            let command = Command {
                operation_id: id(0x600 + n),
                sanction_stream_id: id(0x201 + n),
                ..initial.clone()
            };
            let mut tx = db.pool.begin().await?;
            let revision = persist_platform_sanction(&mut tx, &command, id(0x501))
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(revision, 1);
            persist_typed_decision(&mut tx, &command).await?;
            assert!(record(&mut tx, &mute(), &command).await?.is_some());
            tx.commit().await?;
            command
        };
        let confirmed = std::str::from_utf8(RESULT_APPLIED)?
            .replace(
                &initial.operation_id.to_string(),
                &command.operation_id.to_string(),
            )
            .replace(
                &initial.sanction_stream_id.to_string(),
                &command.sanction_stream_id.to_string(),
            )
            .replace(&digest(COMMAND_APPLY), &digest(&command.encode()?));
        receive(&db, confirmed.as_bytes()).await?;
        if n < 8 {
            let successor = Command {
                operation_id: id(0x700 + n),
                sanction_stream_id: command.sanction_stream_id,
                ..revoke()
            };
            assert!(db.record(&successor).await?.is_some());
        }
    }
    let worker = Expiry::new(db.pool.clone(), mute());
    worker.reconcile().await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 17);
    worker.reconcile().await?;
    let expired: Uuid = sqlx::query_scalar(
        "SELECT sanction_stream_id FROM support_enforcement_commands WHERE intent='EXPIRE'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(expired, id(0x209));
    assert_eq!(db.count("support_enforcement_commands").await?, 18);
    db.cleanup().await
}

#[tokio::test]
async fn worker_expires_latest_confirmed_decision_once_and_waits_for_game() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let command = platform_apply(&db).await?;
    let worker = Expiry::new(db.pool.clone(), mute());
    worker.reconcile().await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 1);
    receive(&db, RESULT_APPLIED).await?;
    let (first, duplicate) = tokio::join!(worker.reconcile(), worker.reconcile());
    first?;
    duplicate?;
    worker.reconcile().await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 2);
    let (operation, revision, command_digest): (Uuid, i64, String) = sqlx::query_as(
        "SELECT operation_id,decision_revision,command_digest FROM support_enforcement_commands WHERE intent='EXPIRE'",
    ).fetch_one(&db.pool).await?;
    assert_eq!(revision, 1);
    let mut tx = db.pool.begin().await?;
    let state = latest(
        &mut tx,
        command.account_id,
        command.character_id,
        BrowserProfile::Mute,
    )
    .await?
    .ok_or("missing projected state")?;
    assert!(matches!(state.confirmed_state(), StateLabel::Applied));
    assert!(state.actions().is_empty());
    tx.commit().await?;

    let body = std::str::from_utf8(RESULT_REVOKED)?
        .replace(&revoke().operation_id.to_string(), &operation.to_string())
        .replace(&digest(COMMAND_REVOKE), &command_digest)
        .replace(r#""state":"REVOKED""#, r#""state":"EXPIRED""#)
        .replace(r#""stream_state":"REVOKED""#, r#""stream_state":"EXPIRED""#)
        .replace(
            r#""accepted_decision_revision":2"#,
            r#""accepted_decision_revision":1"#,
        );
    let app = router(db.pool.clone(), on(6000), identities()?);
    for false_expiry in [
        body.replace(
            r#""stream_state":"EXPIRED","effective_until_ms":null"#,
            r#""stream_state":"ACTIVE","effective_until_ms":1700003600000"#,
        ),
        body.replace(
            r#""accepted_decision_revision":1"#,
            r#""accepted_decision_revision":2"#,
        ),
    ] {
        assert_ne!(false_expiry, body);
        assert_eq!(
            status(&app, request(RESULTS_PATH, false_expiry, Some(RESULTS))).await?,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(db.count("support_enforcement_results").await?, 1);
        let stored_status: String = sqlx::query_scalar("SELECT status FROM sanctions WHERE id=$1")
            .bind(command.sanction_stream_id)
            .fetch_one(&db.pool)
            .await?;
        assert_eq!(stored_status, "active");
    }
    receive(&db, body.as_bytes()).await?;
    let mut tx = db.pool.begin().await?;
    let state = latest(
        &mut tx,
        command.account_id,
        command.character_id,
        BrowserProfile::Mute,
    )
    .await?
    .ok_or("missing projected state")?;
    assert!(matches!(state.confirmed_state(), StateLabel::Expired));
    assert_eq!(state.actions(), &[Action::Apply]);
    assert_eq!(state.platform_status.as_deref(), Some("expired"));
    tx.commit().await?;
    worker.reconcile().await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 2);
    db.cleanup().await
}

#[tokio::test]
async fn accepted_appeal_publishes_revoke_at_the_n009_revision() -> TestResult {
    use crate::appeals::{Action as Appeal, Appeals, Status};
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let command = platform_apply(&db).await?;
    receive(&db, RESULT_APPLIED).await?;
    let mut tx = db.pool.begin().await?;
    let appeals = Appeals { enabled: true };
    let stream = command.sanction_stream_id;
    appeals
        .change(
            &mut tx,
            command.account_id,
            stream,
            1,
            Appeal::Submit("Please review"),
        )
        .await?;
    assert_eq!(db.count("support_enforcement_commands").await?, 1);
    appeals
        .change(
            &mut tx,
            command.account_id,
            stream,
            2,
            Appeal::Decide {
                status: Status::Reviewing,
                outcome: "",
            },
        )
        .await?;
    let revoke = prepare_appeal_revoke(
        &mut tx,
        stream,
        command.account_id,
        command.character_id,
        "MUTE",
    )
    .await
    .map_err(|e| format!("{e:?}"))?
    .ok_or("missing appeal revoke")?;
    let operation = revoke.operation_id;
    appeals
        .change(
            &mut tx,
            command.account_id,
            stream,
            3,
            Appeal::Decide {
                status: Status::Accepted,
                outcome: "Accepted",
            },
        )
        .await?;
    publish_appeal_revoke(&mut tx, id(0x501), revoke, mute())
        .await
        .map_err(|e| format!("{e:?}"))?;
    tx.commit().await?;
    let row: (i64, i64, bool, String) = sqlx::query_as(
        "SELECT c.decision_revision,d.decision_revision,n.revoked,w.status
         FROM support_enforcement_commands c
         JOIN support_enforcement_decisions d USING(operation_id)
         JOIN support_sanction_export_revisions n ON n.sanction_stream_id=c.sanction_stream_id
             AND n.decision_revision=c.decision_revision
         JOIN support_appeal_workflows w ON w.sanction_id=c.sanction_stream_id
         WHERE c.operation_id=$1",
    )
    .bind(operation)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(row, (3, 3, true, "accepted".to_owned()));
    db.cleanup().await
}

#[tokio::test]
async fn moderation_labels_a_refused_apply_separately_from_platform_status() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let command = platform_apply(&db).await?;
    let rejected = std::str::from_utf8(RESULT_APPLIED)?
        .replace(
            r#""state":"APPLIED","result_code":"OK""#,
            r#""state":"REJECTED","result_code":"TARGET_NOT_APPLICABLE""#,
        )
        .replace(
            r#""accepted_decision_revision":1"#,
            r#""accepted_decision_revision":0"#,
        )
        .replace(
            r#""stream_state":"ACTIVE","effective_until_ms":1700003600000"#,
            r#""stream_state":"NONE","effective_until_ms":null"#,
        )
        .replace(
            r#""runtime_effect":"ONLINE_APPLIED""#,
            r#""runtime_effect":"NONE""#,
        );
    receive(&db, rejected.as_bytes()).await?;
    let (platform, game) = moderation_state(&db.pool, command.sanction_stream_id, "MUTE")
        .await?
        .ok_or("missing moderation state")?;
    assert_eq!(platform, "active");
    assert_eq!(game.profile.name(), "MUTE");
    assert!(matches!(game.request, StateLabel::Refused));
    assert!(matches!(game.current, StateLabel::Unconfirmed));
    db.cleanup().await
}
