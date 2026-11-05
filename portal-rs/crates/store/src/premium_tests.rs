#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn interval_lifecycle_and_refusals() {
    let first = transition(None, Change::Grant(2), 100).unwrap();
    let extended = transition(Some(&first), Change::Grant(1), 101).unwrap();
    assert_eq!(extended.from, 100);
    assert_eq!(extended.until, 100 + 3 * 86400);
    assert_eq!(extended.id, first.id);
    assert_eq!(extended.revision, 2);
    let revoked = transition(Some(&extended), Change::Revoke, 100).unwrap();
    assert_eq!(revoked.until, 101);
    assert_eq!(revoked.classify(100), "REVOKED");
    assert_eq!(
        transition(Some(&revoked), Change::Revoke, 102),
        Err(Error::NotActive)
    );
    let regrant = transition(Some(&revoked), Change::Grant(1), 110).unwrap();
    assert_eq!(
        (regrant.id, regrant.revision, regrant.from),
        (first.id, 4, 110)
    );
    assert_eq!(first.classify(first.until), "EXPIRED");
    let restart = transition(Some(&first), Change::Grant(1), first.until).unwrap();
    assert_eq!(restart.from, first.until);
    assert_eq!(transition(None, Change::Revoke, 100), Err(Error::NotActive));
    for days in [0, 367, u16::MAX] {
        assert_eq!(
            transition(None, Change::Grant(days), 100),
            Err(Error::Invalid)
        );
    }
    let mut capped = first.clone();
    capped.until = 100 + 3660 * 86400;
    assert_eq!(
        transition(Some(&capped), Change::Grant(1), 100),
        Err(Error::Horizon)
    );
    capped.revision = MAX_REVISION;
    assert_eq!(
        transition(Some(&capped), Change::Revoke, 100),
        Err(Error::Unavailable)
    );
}

#[test]
fn request_vectors_and_malformed_input() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/premium-snapshot-v1");
    let fixtures = oteryn_internal_api::fixtures::load(root).unwrap();
    for fixture in fixtures {
        if fixture.path.file_name().unwrap() == "request.valid.json" {
            assert!(http::decode_bytes(&fixture.canonical).is_ok());
        }
        if fixture.path.file_name().unwrap() == "manifest.json" {
            if let oteryn_internal_api::canonical::Value::Array(cases) =
                fixture.value.get("invalid_requests").unwrap()
            {
                for case in cases {
                    assert!(http::decode(case.get("body").unwrap()).is_err(), "{case:?}");
                }
            } else {
                panic!("missing cases");
            }
        }
    }
    for body in [
        "{}",
        "[]",
        "{",
        r#"{"schema":"a","schema":"b"}"#,
        r#"{"schema":{},"account_id":[],"nonce":null}"#,
    ] {
        assert!(http::decode_bytes(body.as_bytes()).is_err());
    }
    assert!(http::decode_bytes(&[b' '; 257]).is_err());
    assert!(!http::Settings::default().enabled);
}

const NONCE: &str = "4f1c2a9be07d4c3a8b6e5d4c3b2a1908";
const BUILD: &str = "0123456789abcdef0123456789abcdef01234567";
fn parse_time(value: &str) -> i64 {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .unwrap()
        .unix_timestamp()
}
#[test]
fn every_snapshot_golden_is_byte_identical() {
    use oteryn_internal_api::canonical::Value;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/premium-snapshot-v1");
    let fixtures = oteryn_internal_api::fixtures::load(root).unwrap();
    let mut count = 0;
    for fixture in fixtures {
        let name = fixture.path.file_name().unwrap().to_str().unwrap();
        if !name.starts_with("snapshot.") || name == "snapshot.schema.json" {
            continue;
        }
        let v = &fixture.value;
        let string = |key| match v.get(key).unwrap() {
            Value::String(s) => s.as_str(),
            _ => panic!("string expected"),
        };
        let number = |key| match v.get(key).unwrap() {
            Value::Unsigned(n) => *n as i64,
            Value::Signed(n) => *n,
            _ => panic!("number expected"),
        };
        let entitlement = if string("entitlement_state") == "NONE" {
            None
        } else {
            Some(Entitlement {
                id: Uuid::parse_str(string("entitlement_id")).unwrap(),
                revoked: string("entitlement_state") == "REVOKED",
                revision: number("lifecycle_revision"),
                from: parse_time(string("effective_from")),
                until: parse_time(string("effective_until")),
            })
        };
        let bytes = wire::snapshot(
            entitlement.as_ref(),
            parse_time(string("authority_issued_at")),
            number("authority_revision"),
            Uuid::parse_str(string("account_id")).unwrap(),
            string("nonce"),
            string("producer_revision"),
        )
        .unwrap();
        assert_eq!(bytes, fixture.canonical, "{name}");
        assert!(bytes.len() <= 1024);
        count += 1;
    }
    assert_eq!(count, 5);
}

// SOURCE: Oteryn/Oteryn-Game @ 857f15f376907c592bb28d854fccff18a15cdf87,
// crates/gameplay/social/src/premium/test_producer.rs::snapshot and
// apps/game-server/tests/premium_snapshot_client.rs. Small producer vector only.
// The consumer parses members independent of order; Platform emits contract order.
#[test]
fn game_consumer_producer_vector_and_clipped_one_second() {
    let account = Uuid::parse_str("01890f4e-7c00-7000-8000-000000000001").unwrap();
    let issued = parse_time("2026-09-30T12:00:00Z");
    let mut entitlement = Entitlement {
        id: Uuid::parse_str("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70").unwrap(),
        revoked: false,
        revision: 1,
        from: parse_time("2026-01-01T00:00:00Z"),
        until: parse_time("2099-01-01T00:00:00Z"),
    };
    let expected = r#"{"schema":"oteryn.premium_snapshot.v1","producer_revision":"0123456789abcdef0123456789abcdef01234567","producer_profile":"oteryn.entitlement.profile_b.v1","nonce":"4f1c2a9be07d4c3a8b6e5d4c3b2a1908","account_id":"01890f4e-7c00-7000-8000-000000000001","product_id":"oteryn.premium_time","product_version":1,"entitlement_id":"01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70","entitlement_state":"ACTIVE","lifecycle_revision":1,"authority_revision":7,"effective_from":"2026-01-01T00:00:00Z","effective_until":"2099-01-01T00:00:00Z","authority_issued_at":"2026-09-30T12:00:00Z","authority_valid_until":"2026-09-30T13:00:00Z","refresh_after":"2026-09-30T12:40:00Z"}"#;
    assert_eq!(
        wire::snapshot(Some(&entitlement), issued, 7, account, NONCE, BUILD).unwrap(),
        expected.as_bytes()
    );
    entitlement.until = issued + 1;
    let bytes = wire::snapshot(Some(&entitlement), issued, 8, account, NONCE, BUILD).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["refresh_after"], "2026-09-30T12:00:00Z");
    assert_eq!(value["authority_valid_until"], "2026-09-30T12:00:01Z");
    assert!(wire::snapshot(None, issued, MAX_REVISION + 1, account, NONCE, BUILD).is_err());
}

#[derive(Debug)]
struct TestAccounts;
impl Accounts for TestAccounts {
    fn state<'a>(
        &'a self,
        account: Uuid,
        _: &'a mut sqlx::PgConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AccountState, Error>> + Send + 'a>>
    {
        Box::pin(async move {
            Ok(if account.as_bytes()[15] == 0 {
                AccountState::Unknown
            } else if account.as_bytes()[15] == 255 {
                AccountState::Ambiguous
            } else {
                AccountState::Known
            })
        })
    }
}
struct Database {
    pool: sqlx::PgPool,
    admin: sqlx::PgPool,
    schema: String,
}
impl Database {
    async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED premium database test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = sqlx::PgPool::connect(&url).await.unwrap();
        // Identifiers contain only this fixed prefix and locally generated UUID hex.
        let schema = format!("premium_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let options = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(12)
            .after_connect(move |conn, _| {
                let options = options.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(options)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105160001_t27_premium_ledger.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    fn ledger(&self, enabled: bool) -> repository::Ledger {
        repository::Ledger::new(
            self.pool.clone(),
            std::sync::Arc::new(TestAccounts),
            enabled,
        )
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
fn operator() -> Operator {
    Operator {
        actor: Uuid::now_v7(),
        confirmed_mfa: true,
        permission: "products.premium.manage".into(),
    }
}
fn mutation(account: Uuid, change: Change) -> Mutation {
    Mutation {
        account,
        change,
        request: Uuid::now_v7(),
        reason: "Test operator reason".into(),
    }
}
fn known() -> Uuid {
    let mut bytes = *Uuid::now_v7().as_bytes();
    bytes[15] = 1;
    Uuid::from_bytes(bytes)
}

#[tokio::test]
async fn db_idempotency_and_transactional_refusals() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    let actor = operator();
    let account = known();
    let grant = mutation(account, Change::Grant(3));
    let first = ledger.mutate(&actor, &grant).await.unwrap();
    assert_eq!(ledger.mutate(&actor, &grant).await.unwrap(), first);
    let revoke = mutation(account, Change::Revoke);
    let revoked = ledger.mutate(&actor, &revoke).await.unwrap();
    assert_eq!(revoked.revision, 2);
    assert_eq!(ledger.mutate(&actor, &grant).await.unwrap(), first);
    assert_eq!(
        ledger
            .mutate(&actor, &mutation(account, Change::Revoke))
            .await,
        Err(Error::NotActive)
    );
    let regrant = ledger
        .mutate(&actor, &mutation(account, Change::Grant(1)))
        .await
        .unwrap();
    assert_eq!(regrant.revision, 3);
    let mut conflict = grant.clone();
    conflict.change = Change::Grant(4);
    assert_eq!(ledger.mutate(&actor, &conflict).await, Err(Error::Conflict));
    conflict = grant.clone();
    conflict.account = known();
    assert_eq!(ledger.mutate(&actor, &conflict).await, Err(Error::Conflict));
    let mut denied = actor.clone();
    denied.confirmed_mfa = false;
    assert_eq!(ledger.mutate(&denied, &grant).await, Err(Error::Denied));
    denied = actor.clone();
    denied.permission = "admin".into();
    assert_eq!(ledger.mutate(&denied, &grant).await, Err(Error::Denied));
    let mut bad = grant.clone();
    bad.reason = "short".into();
    assert_eq!(ledger.mutate(&actor, &bad).await, Err(Error::Invalid));
    assert_eq!(
        db.ledger(false).mutate(&actor, &grant).await,
        Err(Error::Disabled)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_ledger")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let audit: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_audit")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!((count, audit), (3, 3));
    assert!(
        sqlx::query("DELETE FROM premium_ledger")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await;
}

#[tokio::test]
async fn db_concurrent_grants_snapshots_and_retries_serialize() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    let actor = operator();
    let account = known();
    let grant = mutation(account, Change::Grant(1));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let l = ledger.clone();
        let a = actor.clone();
        let g = grant.clone();
        tasks.spawn(async move { l.mutate(&a, &g).await.unwrap() });
    }
    while let Some(result) = tasks.join_next().await {
        assert_eq!(result.unwrap().revision, 1);
    }
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let l = ledger.clone();
        let a = actor.clone();
        tasks.spawn(async move {
            l.mutate(&a, &mutation(account, Change::Grant(1)))
                .await
                .unwrap()
        });
    }
    let mut reads = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let l = ledger.clone();
        reads.spawn(async move { l.snapshot(account, NONCE, BUILD).await.unwrap() });
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap());
    }
    results.sort_by_key(|e| e.revision);
    assert_eq!(results.last().unwrap().revision, 9);
    let last = results.last().unwrap();
    assert_eq!(last.until - last.from, 9 * 86400);
    let mut snapshots = Vec::new();
    while let Some(result) = reads.join_next().await {
        snapshots.push(serde_json::from_slice::<serde_json::Value>(&result.unwrap()).unwrap());
    }
    snapshots.sort_by_key(|v| v["authority_revision"].as_i64());
    for (index, value) in snapshots.iter().enumerate() {
        assert_eq!(value["authority_revision"], index as i64 + 1);
        if index > 0 {
            assert!(
                value["lifecycle_revision"].as_i64()
                    >= snapshots[index - 1]["lifecycle_revision"].as_i64()
            );
        }
    }
    db.close().await;
}

#[tokio::test]
async fn db_unknown_ambiguous_and_revision_exhaustion_deny() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    for (last, expected) in [(0, Error::UnknownAccount), (255, Error::Unavailable)] {
        let mut bytes = *known().as_bytes();
        bytes[15] = last;
        let account = Uuid::from_bytes(bytes);
        assert_eq!(ledger.snapshot(account, NONCE, BUILD).await, Err(expected));
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_authority")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let account = known();
    let none = ledger.snapshot(account, NONCE, BUILD).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&none).unwrap();
    assert_eq!(value["entitlement_state"], "NONE");
    sqlx::query("UPDATE premium_authority SET authority_revision=$1")
        .bind(MAX_REVISION)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        ledger.snapshot(account, NONCE, BUILD).await,
        Err(Error::Unavailable)
    );
    db.close().await;
}

fn identity_map() -> std::sync::Arc<oteryn_internal_api::guard::IdentityMap> {
    use oteryn_internal_api::guard::*;
    std::sync::Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san("premium.test").unwrap(),
                group: http::GROUP.into(),
            },
            Binding {
                identity: Identity::san("store.test").unwrap(),
                group: "store".into(),
            },
        ])
        .unwrap(),
    )
}
fn request(body: Vec<u8>, peer: Option<&str>) -> axum::extract::Request {
    let mut request = axum::extract::Request::builder()
        .method("POST")
        .uri(http::PATH)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    if let Some(peer) = peer {
        request.extensions_mut().insert(
            oteryn_internal_api::guard::VerifiedClient::from_verified_tls13_http11(
                oteryn_internal_api::guard::Identity::san(peer).unwrap(),
            ),
        );
    }
    request
}
fn body(account: Uuid) -> Vec<u8> {
    format!(r#"{{"schema":"oteryn.premium_snapshot_request.v1","account_id":"{account}","nonce":"{NONCE}"}}"#).into_bytes()
}
fn settings() -> http::Settings {
    http::Settings {
        enabled: true,
        producer_revision: BUILD.into(),
    }
}

#[tokio::test]
async fn route_guard_bad_input_and_disabled_without_database() {
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let ledger = repository::Ledger::new(pool, std::sync::Arc::new(TestAccounts), true);
    for (config, peer, bytes, status) in [
        (
            http::Settings::default(),
            Some("premium.test"),
            body(known()),
            503,
        ),
        (
            http::Settings {
                enabled: true,
                producer_revision: "bad".into(),
            },
            Some("premium.test"),
            body(known()),
            503,
        ),
        (settings(), None, body(known()), 401),
        (settings(), Some("store.test"), body(known()), 401),
        (settings(), Some("premium.test"), b"{}".to_vec(), 400),
        (settings(), Some("premium.test"), vec![b' '; 257], 400),
        (
            settings(),
            Some("premium.test"),
            b"{\"a\":1,\"a\":2}".to_vec(),
            400,
        ),
    ] {
        let response = http::router(ledger.clone(), config, identity_map())
            .oneshot(request(bytes, peer))
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        assert_eq!(response.headers()["cache-control"], "no-store, private");
        assert!(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .is_empty()
        );
    }
    use oteryn_internal_api::guard::*;
    assert!(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san("same").unwrap(),
                group: http::GROUP.into()
            },
            Binding {
                identity: Identity::san("same").unwrap(),
                group: "store".into()
            }
        ])
        .is_err()
    );
    assert!(Identity::san("").is_err());
    assert!(Identity::san("invalid\n").is_err());
}

#[tokio::test]
async fn db_route_success_unknown_and_rate_limit() {
    use tower::ServiceExt;
    let Some(db) = Database::create().await else {
        return;
    };
    let account = known();
    let ledger = db.ledger(true);
    ledger
        .mutate(&operator(), &mutation(account, Change::Grant(1)))
        .await
        .unwrap();
    let router = http::router(ledger, settings(), identity_map());
    let response = router
        .clone()
        .oneshot(request(body(account), Some("premium.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    let bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert!(bytes.len() <= 1024);
    let snapshot: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot["entitlement_state"], "ACTIVE");
    assert_eq!(snapshot["nonce"], NONCE);
    assert_eq!(snapshot["account_id"], account.to_string());
    let mut missing = *account.as_bytes();
    missing[15] = 0;
    let missing = Uuid::from_bytes(missing);
    let response = router
        .clone()
        .oneshot(request(body(missing), Some("premium.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    // Malformed but authenticated calls consume the same peer budget.
    for _ in 0..1198 {
        let response = router
            .clone()
            .oneshot(request(b"{}".to_vec(), Some("premium.test")))
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
    }
    let response = router
        .oneshot(request(body(account), Some("premium.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    db.close().await;
}

#[tokio::test]
async fn db_audit_failure_rolls_back_and_invalid_durable_state_denies() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    let actor = operator();
    let account = known();
    sqlx::raw_sql("CREATE FUNCTION reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit failure'; END; $$; CREATE TRIGGER reject_audit BEFORE INSERT ON premium_audit FOR EACH ROW EXECUTE FUNCTION reject_audit();").execute(&db.pool).await.unwrap();
    assert_eq!(
        ledger
            .mutate(&actor, &mutation(account, Change::Grant(1)))
            .await,
        Err(Error::Unavailable)
    );
    for query in [
        "SELECT COUNT(*) FROM premium_ledger",
        "SELECT COUNT(*) FROM premium_audit",
        "SELECT COUNT(*) FROM premium_authority",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
    sqlx::query("DROP TRIGGER reject_audit ON premium_audit")
        .execute(&db.pool)
        .await
        .unwrap();
    // A syntactically valid but inconsistent imported ledger must never issue evidence.
    sqlx::query("INSERT INTO premium_authority(account_id) VALUES ($1)")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES ($1,$2,$3,'GRANT',1,$4,$5,1,1,'2026-01-01T00:00:00Z','2026-01-02T00:00:00Z')")
        .bind(Uuid::now_v7()).bind(account).bind(actor.actor).bind(vec![0u8;32]).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    assert_eq!(
        ledger.snapshot(account, NONCE, BUILD).await,
        Err(Error::Unavailable)
    );
    let revision: i64 =
        sqlx::query_scalar("SELECT authority_revision FROM premium_authority WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(revision, 0);
    db.close().await;
}
