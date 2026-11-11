#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use time::OffsetDateTime;
use tower::ServiceExt;
const ACCOUNT: &str = "01890f4e-7c00-7000-8000-000000000001";
const BUILD: &str = "0123456789abcdef0123456789abcdef01234567";
const NONCE: &str = "4f1c2a9be07d4c3a8b6e5d4c3b2a1908";
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
            Ok(
                if account.to_string() == "01890f4e-7c00-7000-8000-000000000000" {
                    AccountState::Unknown
                } else if account.to_string() == "01890f4e-7c00-7000-8000-0000000000ff" {
                    AccountState::Ambiguous
                } else {
                    AccountState::Known
                },
            )
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
            eprintln!("SKIPPED loyalty stream database test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = sqlx::PgPool::connect(&url).await.unwrap();
        // Identifiers contain only this fixed prefix and locally generated UUID hex.
        let schema = format!("loyalty_test_{}", Uuid::now_v7().simple());
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
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    fn ledger(&self, enabled: bool) -> super::super::Ledger {
        super::super::Ledger::new(
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
impl Database {
    fn stream(&self) -> Stream {
        Stream::new(self.pool.clone(), Arc::new(TestAccounts), true)
    }
    async fn account(&self, id: Uuid) {
        sqlx::query("INSERT INTO premium_authority(account_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(id)
            .execute(&self.pool)
            .await
            .unwrap();
    }
    async fn interval(
        &self,
        id: Uuid,
        entitlement: Uuid,
        revision: i64,
        from: OffsetDateTime,
        until: OffsetDateTime,
    ) {
        sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES ($1,$2,$2,'GRANT',1,$3,$4,$5,0,$6,$7)")
            .bind(Uuid::now_v7()).bind(id).bind(vec![0u8;32]).bind(entitlement).bind(revision).bind(from).bind(until).execute(&self.pool).await.unwrap();
    }
    async fn days(&self, id: Uuid, revision: i64, days: i64) {
        self.account(id).await;
        let now: OffsetDateTime =
            sqlx::query_scalar("SELECT date_trunc('second',clock_timestamp())")
                .fetch_one(&self.pool)
                .await
                .unwrap();
        self.interval(
            id,
            Uuid::now_v7(),
            revision,
            now - time::Duration::days(days) - time::Duration::hours(1),
            now - time::Duration::hours(1),
        )
        .await;
    }
}
fn settings() -> http::Settings {
    http::Settings {
        enabled: true,
        requests_per_minute: 1200,
    }
}
fn identities() -> Arc<oteryn_internal_api::guard::IdentityMap> {
    use oteryn_internal_api::guard::*;
    Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san("loyalty-stream.test").unwrap(),
                group: http::GROUP.into(),
            },
            Binding {
                identity: Identity::san("loyalty-snapshot.test").unwrap(),
                group: super::super::http::GROUP.into(),
            },
            Binding {
                identity: Identity::san("premium.test").unwrap(),
                group: crate::premium::http::GROUP.into(),
            },
        ])
        .unwrap(),
    )
}
fn request(bytes: Vec<u8>, peer: Option<&str>) -> axum::extract::Request {
    let mut request = axum::extract::Request::builder()
        .method("POST")
        .uri(http::PATH)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(bytes))
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
fn body() -> Vec<u8> {
    include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/request.json").to_vec()
}
fn lazy_stream(enabled: bool) -> Stream {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgresql://localhost/unused")
        .unwrap();
    Stream::new(pool, Arc::new(TestAccounts), enabled)
}
async fn assert_failure(app: axum::Router, bytes: Vec<u8>, peer: Option<&str>, status: u16) {
    let response = app.oneshot(request(bytes, peer)).await.unwrap();
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    assert!(
        axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn loyalty_stream_disabled_503() {
    for (enabled, config, map) in [
        (true, http::Settings::default(), identities()),
        (false, settings(), identities()),
        (
            true,
            http::Settings {
                enabled: true,
                requests_per_minute: 0,
            },
            identities(),
        ),
        (
            true,
            http::Settings {
                enabled: true,
                requests_per_minute: 6001,
            },
            identities(),
        ),
        (
            true,
            settings(),
            Arc::new(oteryn_internal_api::guard::IdentityMap::default()),
        ),
    ] {
        assert_failure(
            http::router(lazy_stream(enabled), config, map),
            body(),
            Some("loyalty-stream.test"),
            503,
        )
        .await;
    }
    assert!(!http::Settings::from_lookup(|_| None).enabled);
    for rate in ["bad", "0", "6001"] {
        let config = http::Settings::from_lookup(|key| match key {
            "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED" => Some("true".into()),
            "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_REQUESTS_PER_MINUTE" => Some(rate.into()),
            _ => None,
        });
        assert!(!(1..=6000).contains(&config.requests_per_minute));
    }
}
#[tokio::test]
async fn loyalty_stream_wrong_identity_refused() {
    use oteryn_internal_api::guard::*;
    let app = http::router(lazy_stream(true), settings(), identities());
    for peer in [
        None,
        Some("loyalty-snapshot.test"),
        Some("premium.test"),
        Some("unbound.test"),
    ] {
        assert_failure(app.clone(), body(), peer, 401).await;
    }
    let mut spoofed = request(body(), None);
    spoofed.headers_mut().insert(
        "x-client-identity",
        ::http::HeaderValue::from_static("loyalty-stream.test"),
    );
    assert_eq!(app.oneshot(spoofed).await.unwrap().status(), 401);
    assert!(http::Settings::identity_binding(|_| None).is_err());
    let binding = http::Settings::identity_binding(|_| Some("loyalty-stream.test".into())).unwrap();
    assert_eq!(binding.group, http::GROUP);
    assert!(
        IdentityMap::new(vec![
            binding.clone(),
            Binding {
                identity: binding.identity,
                group: super::super::http::GROUP.into()
            }
        ])
        .is_err()
    );
}
#[tokio::test]
async fn loyalty_stream_bad_body_400() {
    let app = http::router(lazy_stream(true), settings(), identities());
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/loyalty-stream-v1/invalid-requests.json"
    ))
    .unwrap();
    let invalid = cases["cases"].as_array().unwrap();
    for bytes in invalid
        .iter()
        .map(|v| serde_json::to_vec(v).unwrap())
        .chain([
            b"{".to_vec(),
            vec![b' '; 257],
            br#"{"schema":"oteryn.loyalty_stream_request.v1","after":null,"limit":1,"limit":2}"#
                .to_vec(),
            br#"{"schema":"oteryn.loyalty_stream_request.v1","after":null,"limit":1.0}"#.to_vec(),
        ])
    {
        assert_failure(app.clone(), bytes, Some("loyalty-stream.test"), 400).await;
    }
    for (name, value) in [
        ("content-type", "text/plain"),
        ("content-encoding", "gzip"),
        ("transfer-encoding", "chunked"),
    ] {
        let mut req = request(body(), Some("loyalty-stream.test"));
        req.headers_mut().insert(
            ::http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            ::http::HeaderValue::from_str(value).unwrap(),
        );
        assert_eq!(app.clone().oneshot(req).await.unwrap().status(), 400);
    }
}
#[tokio::test]
async fn loyalty_stream_rate_limit_retry_after() {
    let app = http::router(
        lazy_stream(true),
        http::Settings {
            enabled: true,
            requests_per_minute: 1,
        },
        identities(),
    );
    assert_failure(
        app.clone(),
        b"{}".to_vec(),
        Some("loyalty-stream.test"),
        400,
    )
    .await;
    let response = app
        .oneshot(request(body(), Some("loyalty-stream.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    assert_eq!(response.headers()["retry-after"], "60");
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    assert!(
        axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
}
#[test]
fn loyalty_stream_fixtures_match() {
    let value = oteryn_internal_api::canonical::decode(&body(), 256, 4).unwrap();
    assert_eq!(http::decode(&value).unwrap(), (None, 1));
    let page = Page {
        items: vec![Item {
            account_id: Uuid::parse_str(ACCOUNT).unwrap(),
            spent_premium_days: 2,
            loyalty_points: 2,
            platform_revision: 1,
        }],
        next: Cursor::parse("v1:1:0").unwrap(),
    };
    assert_eq!(
        page.encode().unwrap(),
        include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/response.json")
    );
    let credited = Page {
        items: vec![Item {
            account_id: Uuid::parse_str(ACCOUNT).unwrap(),
            spent_premium_days: 2,
            loyalty_points: 3,
            platform_revision: 2,
        }],
        next: Cursor::parse("v1:2:0").unwrap(),
    };
    assert_eq!(
        credited.encode().unwrap(),
        include_bytes!(
            "../../../../docs/contracts/fixtures/loyalty-stream-v1/credit-response.json"
        )
    );
    assert_eq!(
        Page {
            items: vec![],
            next: Cursor::parse("v1:0:0").unwrap()
        }
        .encode()
        .unwrap(),
        include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/empty-response.json")
    );
    for limit in [1, 500] {
        let value=oteryn_internal_api::canonical::decode(format!(r#"{{"limit":{limit},"after":"v1:1:2","schema":"oteryn.loyalty_stream_request.v1"}}"#).as_bytes(),256,4).unwrap();
        assert!(http::decode(&value).is_ok());
    }
}
#[tokio::test]
async fn loyalty_stream_cursor_stable() {
    let Some(db) = Database::create().await else {
        return;
    };
    let accounts: Vec<_> = (0..3).map(|_| Uuid::now_v7()).collect();
    for account in &accounts {
        db.days(*account, 1, 2).await;
    }
    let first = db.stream().read(None, 1).await.unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.next.through, 3);
    let initial = db.stream().read(Some(first.next), 1).await.unwrap();
    db.days(accounts[0], 2, 1).await;
    let added = Uuid::now_v7();
    db.days(added, 1, 4).await;
    let publisher = db.stream();
    tokio::spawn(async move { publisher.read(None, 500).await.unwrap() })
        .await
        .unwrap();
    let second = db.stream().read(Some(first.next), 1).await.unwrap();
    assert_eq!(second.encode().unwrap(), initial.encode().unwrap());
    let third = db.stream().read(Some(second.next), 1).await.unwrap();
    let mut seen = vec![
        first.items[0].account_id,
        second.items[0].account_id,
        third.items[0].account_id,
    ];
    seen.sort();
    let mut expected = accounts.clone();
    expected.sort();
    assert_eq!(seen, expected);
    assert_eq!(third.next, Cursor::parse("v1:3:0").unwrap());
    let tail = db.stream().read(Some(third.next), 500).await.unwrap();
    assert_eq!(tail.items.len(), 2);
    assert!(tail.items.iter().any(|i| i.account_id == accounts[0]
        && i.spent_premium_days == 3
        && i.platform_revision == 2));
    assert!(
        tail.items
            .iter()
            .any(|i| i.account_id == added && i.platform_revision == 1)
    );
    assert!(
        db.stream()
            .read(Some(tail.next), 500)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let bad =
        br#"{"schema":"oteryn.loyalty_stream_request.v1","after":"v1:999:0","limit":1}"#.to_vec();
    assert_failure(
        http::router(db.stream(), settings(), identities()),
        bad,
        Some("loyalty-stream.test"),
        400,
    )
    .await;
    assert_eq!(db.stream().read(None, 0).await, Err(Error::Invalid));
    assert_eq!(db.stream().read(None, 501).await, Err(Error::Invalid));
    db.close().await;
}
#[tokio::test]
async fn loyalty_stream_revision_increases_on_change() {
    let Some(db) = Database::create().await else {
        return;
    };
    let account = Uuid::parse_str(ACCOUNT).unwrap();
    db.account(account).await;
    let stream = db.stream();
    let first = stream.read(None, 500).await.unwrap();
    assert_eq!(
        (
            first.items[0].spent_premium_days,
            first.items[0].platform_revision
        ),
        (0, 1)
    );
    db.days(account, 1, 2).await;
    let second = stream.read(Some(first.next), 500).await.unwrap();
    assert_eq!(
        (
            second.items[0].spent_premium_days,
            second.items[0].platform_revision
        ),
        (2, 2)
    );
    db.ledger(true)
        .snapshot(account, NONCE, BUILD)
        .await
        .unwrap();
    assert!(
        stream
            .read(Some(second.next), 500)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    db.days(account, 2, 1).await;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let stream = stream.clone();
        tasks.spawn(async move { stream.read(Some(second.next), 500).await.unwrap() });
    }
    while let Some(result) = tasks.join_next().await {
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(
            (
                page.items[0].spent_premium_days,
                page.items[0].platform_revision
            ),
            (3, 3)
        );
    }
    let (count, head): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM loyalty_stream_events),position FROM loyalty_stream_head",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!((count, head), (3, 3));
    let (entitlement,from):(Uuid,OffsetDateTime)=sqlx::query_as("SELECT entitlement_id,effective_from FROM premium_ledger WHERE account_id=$1 AND lifecycle_revision=1").bind(account).fetch_one(&db.pool).await.unwrap();
    db.interval(
        account,
        entitlement,
        3,
        from,
        from + time::Duration::days(1),
    )
    .await;
    let correction = stream
        .read(Some(Cursor::parse("v1:3:0").unwrap()), 500)
        .await
        .unwrap();
    assert_eq!(
        (
            correction.items[0].spent_premium_days,
            correction.items[0].platform_revision
        ),
        (2, 4)
    );
    assert_eq!(correction.items[0].loyalty_points, 2);
    db.close().await;
}
#[tokio::test]
async fn loyalty_stream_snapshot_unchanged() {
    let Some(db) = Database::create().await else {
        return;
    };
    let account = Uuid::parse_str(ACCOUNT).unwrap();
    db.days(account, 1, 2).await;
    let app = super::super::http::router_with_stream(
        db.ledger(true),
        super::super::http::Settings {
            enabled: true,
            producer_revision: BUILD.into(),
            requests_per_minute: 1200,
        },
        settings(),
        identities(),
    );
    let response = app
        .clone()
        .oneshot(request(body(), Some("loyalty-stream.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap(),
        include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/response.json")
            .as_slice()
    );
    let authority: i64 =
        sqlx::query_scalar("SELECT authority_revision FROM premium_authority WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(authority, 0);
    let mut req=request(format!(r#"{{"schema":"oteryn.loyalty_snapshot_request.v1","account_id":"{account}","nonce":"{NONCE}"}}"#).into_bytes(),Some("loyalty-snapshot.test"));
    *req.uri_mut() = super::super::http::PATH.parse().unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), 512)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let computed = OffsetDateTime::parse(
        value["computed_at"].as_str().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap()
    .unix_timestamp();
    assert_eq!(
        bytes,
        super::super::snapshot(2, 1, computed, account, NONCE, BUILD).unwrap()
    );
    let second = db
        .ledger(true)
        .snapshot(account, NONCE, BUILD)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&second).unwrap()["loyalty_revision"],
        2
    );
    let all = db.stream().read(None, 500).await.unwrap();
    assert_eq!(all.items.len(), 1);
    assert_eq!(all.items[0].platform_revision, 1);
    db.close().await;
}
#[tokio::test]
async fn loyalty_stream_time_growth_and_lifecycle_computation() {
    let Some(db) = Database::create().await else {
        return;
    };
    let account = Uuid::parse_str(ACCOUNT).unwrap();
    db.account(account).await;
    let now: OffsetDateTime = sqlx::query_scalar("SELECT date_trunc('second',clock_timestamp())")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let entitlement = Uuid::now_v7();
    db.interval(
        account,
        entitlement,
        1,
        now - time::Duration::days(4),
        now + time::Duration::days(4),
    )
    .await;
    db.interval(
        account,
        entitlement,
        2,
        now - time::Duration::days(4),
        now - time::Duration::hours(60),
    )
    .await;
    db.interval(
        account,
        entitlement,
        3,
        now - time::Duration::days(1),
        now + time::Duration::days(1),
    )
    .await;
    db.interval(
        account,
        entitlement,
        4,
        now + time::Duration::days(2),
        now + time::Duration::days(3),
    )
    .await;
    let first = db.stream().read(None, 500).await.unwrap();
    assert_eq!(first.items[0].spent_premium_days, 2);
    let snapshot = db
        .ledger(true)
        .snapshot(account, NONCE, BUILD)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&snapshot).unwrap()["loyalty_points"],
        2
    );
    let timed = Uuid::now_v7();
    db.account(timed).await;
    let now: OffsetDateTime = sqlx::query_scalar("SELECT date_trunc('second',clock_timestamp())")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    db.interval(
        timed,
        Uuid::now_v7(),
        1,
        now - time::Duration::days(1) + time::Duration::seconds(3),
        now + time::Duration::days(1),
    )
    .await;
    let before = db.stream().read(Some(first.next), 500).await.unwrap();
    assert_eq!(
        before
            .items
            .iter()
            .find(|i| i.account_id == timed)
            .unwrap()
            .spent_premium_days,
        0
    );
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    let after = db.stream().read(Some(before.next), 500).await.unwrap();
    let item = after.items.iter().find(|i| i.account_id == timed).unwrap();
    assert_eq!((item.spent_premium_days, item.platform_revision), (1, 2));
    db.close().await;
}
#[tokio::test]
async fn loyalty_stream_refusal_and_failure_roll_back() {
    let Some(db) = Database::create().await else {
        return;
    };
    let unknown = Uuid::parse_str("01890f4e-7c00-7000-8000-000000000000").unwrap();
    db.account(unknown).await;
    assert!(db.stream().read(None, 500).await.unwrap().items.is_empty());
    let account = Uuid::parse_str(ACCOUNT).unwrap();
    db.days(account, 1, 1).await;
    let ambiguous = Uuid::parse_str("01890f4e-7c00-7000-8000-0000000000ff").unwrap();
    db.account(ambiguous).await;
    assert_eq!(db.stream().read(None, 500).await, Err(Error::Unavailable));
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT COUNT(*) FROM loyalty_stream_state),(SELECT COUNT(*) FROM loyalty_stream_events),position FROM loyalty_stream_head").fetch_one(&db.pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    sqlx::query("DELETE FROM premium_authority WHERE account_id=$1")
        .bind(ambiguous)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_stream() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test stream failure'; END; $$; CREATE TRIGGER reject_stream BEFORE INSERT ON loyalty_stream_events FOR EACH ROW EXECUTE FUNCTION reject_stream();").execute(&db.pool).await.unwrap();
    assert_failure(
        http::router(db.stream(), settings(), identities()),
        body(),
        Some("loyalty-stream.test"),
        503,
    )
    .await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loyalty_stream_state")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
}

#[tokio::test]
async fn loyalty_stream_configured_route_uses_account_authority() {
    let Some(db) = Database::create().await else {
        return;
    };
    let account = Uuid::parse_str(ACCOUNT).unwrap();
    db.days(account, 1, 2).await;
    let mut values = std::collections::BTreeMap::from([
        (
            "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED".into(),
            "true".into(),
        ),
        (
            "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY".into(),
            "loyalty-stream.test".into(),
        ),
        (
            "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY".into(),
            "loyalty-snapshot.test".into(),
        ),
    ]);
    let app = http::configured_with_values(db.pool.clone(), &values);
    let response = app
        .clone()
        .oneshot(request(body(), Some("loyalty-stream.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap(),
        include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/empty-response.json")
            .as_slice()
    );
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,'loyalty@example.test','test-only','active','en')")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    let response = app
        .oneshot(request(body(), Some("loyalty-stream.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap(),
        include_bytes!("../../../../docs/contracts/fixtures/loyalty-stream-v1/response.json")
            .as_slice()
    );
    values.insert(
        "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY".into(),
        "loyalty-stream.test".into(),
    );
    assert_failure(
        http::configured_with_values(db.pool.clone(), &values),
        body(),
        Some("loyalty-stream.test"),
        503,
    )
    .await;
    values.insert(
        "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY".into(),
        "invalid\n".into(),
    );
    assert_failure(
        http::configured_with_values(db.pool.clone(), &values),
        body(),
        Some("loyalty-stream.test"),
        503,
    )
    .await;
    values.remove("GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY");
    values.remove("PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY");
    assert_failure(
        http::configured_with_values(db.pool.clone(), &values),
        body(),
        Some("loyalty-stream.test"),
        503,
    )
    .await;
    db.close().await;
}

#[test]
fn loyalty_stream_full_page_under_consumer_body_cap() {
    let page = Page {
        items: (0..500)
            .map(|_| Item {
                account_id: Uuid::now_v7(),
                spent_premium_days: i32::MAX,
                loyalty_points: i32::MAX,
                platform_revision: i64::MAX,
            })
            .collect(),
        next: Cursor::parse("v1:9223372036854775806:9223372036854775807").unwrap(),
    };
    let bytes = page.encode().unwrap();
    assert!(bytes.len() < 256 * 1024);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema"], "oteryn.loyalty_stream_page.v1");
    assert_eq!(value.as_object().unwrap().len(), 3);
    assert_eq!(value["items"].as_array().unwrap().len(), 500);
    let cursor = value["next_cursor"].as_str().unwrap();
    assert!((1..=256).contains(&cursor.len()));
    assert!(cursor.bytes().all(|byte| (0x20..=0x7e).contains(&byte)));
    let empty = Page {
        items: vec![],
        next: page.next,
    }
    .encode()
    .unwrap();
    assert!(serde_json::from_slice::<serde_json::Value>(&empty).unwrap()["next_cursor"].is_null());
}

#[tokio::test]
async fn loyalty_stream_all_identity_reuse_503() {
    for (key, raw) in [
        (
            "PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY",
            "loyalty-stream.test",
        ),
        (
            "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY",
            "loyalty-stream.test",
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY",
            "loyalty-stream.test",
        ),
        (
            "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY",
            "loyalty-stream.test",
        ),
        (
            "PORTAL_GAME_COINS_IDENTITIES",
            r#"{"loyalty-stream.test":[]}"#,
        ),
        (
            "GAME_AUTH_NATIVE_EVIDENCE_IDENTITIES",
            r#"{"loyalty-stream.test":["world-a"]}"#,
        ),
        ("OTHER_INTERNAL_SAN", "loyalty-stream.test"),
        ("PORTAL_SANCTION_EXPORT_IDENTITY", "loyalty-stream.test"),
        (
            "PORTAL_GAME_COINS_IDENTITIES",
            r#"{"coins.test":["invalid"]}"#,
        ),
        ("PORTAL_GAME_COINS_IDENTITIES", "invalid"),
    ] {
        let mut values = std::collections::BTreeMap::from([
            (
                "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED".into(),
                "true".into(),
            ),
            (
                "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY".into(),
                "loyalty-stream.test".into(),
            ),
        ]);
        values.insert(key.into(), raw.into());
        assert_failure(
            http::configured_with_values(lazy_stream(true).pool, &values),
            body(),
            Some("loyalty-stream.test"),
            503,
        )
        .await;
    }
}

#[test]
fn loyalty_stream_identity_shared_with_sanction_export_is_refused() {
    let values = std::collections::BTreeMap::from([
        (
            "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY".to_string(),
            "shared.test".to_string(),
        ),
        (
            "PORTAL_SANCTION_EXPORT_IDENTITY".to_string(),
            "shared.test".to_string(),
        ),
    ]);
    assert!(http::identity_map(&values).is_err());
}
