#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::premium::{Change, Mutation, Operator};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{Binding, Identity, IdentityMap, VerifiedClient},
};
use std::{collections::BTreeMap, sync::Mutex};
use tower::ServiceExt;

const NONCE: &str = "4f1c2a9be07d4c3a8b6e5d4c3b2a1908";
const BUILD: &str = "0123456789abcdef0123456789abcdef01234567";
const B1_PEER: &str = "profile-b-authority.test";
const B2_PEER: &str = "profile-b-changes.test";
const OTHER_PEER: &str = "premium.test";

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/profile-b-authority-v1")
        .join(name);
    std::fs::read(path).unwrap()
}
fn parse_time(value: &str) -> i64 {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .unwrap()
        .unix_timestamp()
}
fn fixture_account() -> Uuid {
    Uuid::parse_str("019a0000-0000-7000-8000-000000000002").unwrap()
}
fn product(id: &str) -> Product {
    Product {
        id: id.into(),
        version: 1,
    }
}
fn active_record() -> Record {
    Record {
        id: Uuid::parse_str("019a0000-0000-7000-8000-000000000601").unwrap(),
        revoked: false,
        revision: 3,
        from: parse_time("2023-11-14T00:00:00Z"),
        until: Some(parse_time("2023-12-14T00:00:00Z")),
        lease_policy: LEASE_POLICY.into(),
    }
}

#[test]
fn authority_fixture_is_byte_identical() {
    let active = product("oteryn.example_active");
    let none = product("oteryn.example_none");
    let record = active_record();
    let bytes = crate::premium::wire::authority(
        &[(&active, Some(&record)), (&none, None)],
        parse_time("2023-11-14T22:13:20Z"),
        7,
        fixture_account(),
        NONCE,
        BUILD,
    )
    .unwrap();
    assert_eq!(bytes, fixture("authority.json"));
}

#[test]
fn empty_change_page_fixture_is_byte_identical() {
    assert_eq!(
        crate::premium::wire::change_page(&[]).unwrap(),
        fixture("changes-empty.json")
    );
}

#[test]
fn request_fixtures_decode_and_malformed_requests_refuse() {
    let b1 = canonical::decode(&fixture("authority-request.json"), 256, 4).unwrap();
    assert_eq!(
        http::decode_authority(&b1).unwrap(),
        (fixture_account(), NONCE.to_owned())
    );
    let b2 = canonical::decode(&fixture("changes-request.json"), 256, 4).unwrap();
    assert_eq!(http::decode_changes(&b2).unwrap(), (None, 200));
    let account = fixture_account();
    for bad in [
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v2","account_id":"{account}","nonce":"{NONCE}"}}"#
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{account}"}}"#
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{account}","nonce":"{NONCE}","x":1}}"#
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{}","nonce":"{NONCE}"}}"#,
            account.to_string().to_uppercase()
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{}","nonce":"{NONCE}"}}"#,
            Uuid::new_v4()
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{account}","nonce":"{}"}}"#,
            NONCE.to_uppercase()
        ),
        format!(
            r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{account}","nonce":1}}"#
        ),
    ] {
        let value = canonical::decode(bad.as_bytes(), 256, 4).unwrap();
        assert_eq!(http::decode_authority(&value), Err(Error::Invalid), "{bad}");
    }
    for bad in [
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":0}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":501}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":"v1:0","limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":"v1:01","limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":"v2:1","limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":"v1:9223372036854775808","limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":1,"limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","limit":1}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":"1"}"#,
        r#"{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":1,"x":1}"#,
    ] {
        let value = canonical::decode(bad.as_bytes(), 256, 4).unwrap();
        assert_eq!(http::decode_changes(&value), Err(Error::Invalid), "{bad}");
    }
}

#[test]
fn product_configuration_and_cutoff_boundaries() {
    assert_eq!(products(None), Some(Vec::new()));
    assert_eq!(
        products(Some(
            r#"["oteryn.example_active@1","oteryn.premium_time@2"]"#
        )),
        Some(vec![
            product("oteryn.example_active"),
            Product {
                id: "oteryn.premium_time".into(),
                version: 2
            }
        ])
    );
    let seventeen: Vec<String> = (0..17).map(|n| format!("\"oteryn.p{n}@1\"")).collect();
    for bad in [
        r#"["oteryn.premium_time@1"]"#.to_owned(),
        r#"["oteryn.a@1","oteryn.a@1"]"#.to_owned(),
        r#"["oteryn.a@0"]"#.to_owned(),
        r#"["oteryn.a@01"]"#.to_owned(),
        r#"["Oteryn.a@1"]"#.to_owned(),
        r#"["oteryn@1"]"#.to_owned(),
        r#"["oteryn.a"]"#.to_owned(),
        r#""oteryn.a@1""#.to_owned(),
        format!("[{}]", seventeen.join(",")),
    ] {
        assert_eq!(products(Some(&bad)), None, "{bad}");
    }
    // A commercial end clips the lease; refresh_after stays before the cutoff.
    let issued = parse_time("2023-11-14T22:13:20Z");
    let mut record = active_record();
    record.until = Some(issued + 1);
    let active = product("oteryn.example_active");
    let bytes = crate::premium::wire::authority(
        &[(&active, Some(&record))],
        issued,
        1,
        fixture_account(),
        NONCE,
        BUILD,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["entitlements"][0]["authority_valid_until"],
        "2023-11-14T22:13:21Z"
    );
    assert_eq!(
        value["entitlements"][0]["refresh_after"],
        "2023-11-14T22:13:20Z"
    );
    // issued == effective_until is EXPIRED, on a full lease.
    record.until = Some(issued);
    let bytes = crate::premium::wire::authority(
        &[(&active, Some(&record))],
        issued,
        1,
        fixture_account(),
        NONCE,
        BUILD,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["entitlements"][0]["entitlement_state"], "EXPIRED");
    assert_eq!(
        value["entitlements"][0]["authority_valid_until"],
        "2023-11-14T23:13:20Z"
    );
    // Open-ended: null end, lease still finite.
    record.until = None;
    let bytes = crate::premium::wire::authority(
        &[(&active, Some(&record))],
        issued,
        1,
        fixture_account(),
        NONCE,
        BUILD,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["entitlements"][0]["entitlement_state"], "ACTIVE");
    assert!(value["entitlements"][0]["effective_until"].is_null());
    assert_eq!(
        value["entitlements"][0]["authority_valid_until"],
        "2023-11-14T23:13:20Z"
    );
    // Sixteen worst-case items stay within the 8192-byte bound.
    let records: Vec<(Product, Record)> = (0..16)
        .map(|n| {
            (
                Product {
                    id: format!("{}.{n:0>31}", "a".repeat(32)),
                    version: MAX_REVISION,
                },
                Record {
                    revision: MAX_REVISION,
                    ..active_record()
                },
            )
        })
        .collect();
    let items: Vec<_> = records.iter().map(|(p, r)| (p, Some(r))).collect();
    let bytes = crate::premium::wire::authority(
        &items,
        issued,
        MAX_REVISION,
        fixture_account(),
        NONCE,
        BUILD,
    )
    .unwrap();
    assert!(bytes.len() <= 8192);
}

#[derive(Debug)]
struct TestAccounts;
impl Accounts for TestAccounts {
    fn state<'a>(
        &'a self,
        account: Uuid,
        _: &'a mut sqlx::PgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<AccountState, Error>> + Send + 'a>> {
        Box::pin(async move {
            Ok(match account.as_bytes()[15] {
                0 => AccountState::Unknown,
                255 => AccountState::Ambiguous,
                _ => AccountState::Known,
            })
        })
    }
}

/// In-memory entitlement source keyed by product id, read inside the B1 transaction.
#[derive(Debug, Default)]
struct TestEntitlements(Mutex<BTreeMap<String, Record>>);
impl Entitlements for TestEntitlements {
    fn read<'a>(&'a self, _: Uuid, product: &'a Product, _: &'a mut PgConnection) -> Read<'a> {
        let record = self.0.lock().unwrap().get(&product.id).cloned();
        Box::pin(async move { Ok(record) })
    }
}

fn identity_map() -> Arc<IdentityMap> {
    let binding = |san: &str, group: &str| Binding {
        identity: Identity::san(san).unwrap(),
        group: group.into(),
    };
    Arc::new(
        IdentityMap::new(vec![
            binding(B1_PEER, http::AUTHORITY_GROUP),
            binding(B2_PEER, http::CHANGES_GROUP),
            binding(OTHER_PEER, crate::premium::http::GROUP),
        ])
        .unwrap(),
    )
}
fn settings(products: &[&str]) -> http::Settings {
    http::Settings {
        authority_enabled: true,
        changes_enabled: true,
        producer_revision: BUILD.into(),
        products: Some(products.iter().map(|id| product(id)).collect()),
        authority_budget: Some(1200),
        changes_budget: Some(1200),
    }
}
fn request(path: &str, body: &[u8], peer: Option<&str>) -> axum::extract::Request {
    let mut request = axum::extract::Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_vec()))
        .unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(peer).unwrap(),
            ));
    }
    request
}
fn b1_body(account: Uuid) -> Vec<u8> {
    format!(r#"{{"schema":"oteryn.entitlement_authority_request.v1","account_id":"{account}","nonce":"{NONCE}"}}"#).into_bytes()
}
fn b2_body(after: Option<i64>, limit: i64) -> Vec<u8> {
    let after = after.map_or("null".to_owned(), |n| format!("\"v1:{n}\""));
    format!(
        r#"{{"schema":"oteryn.entitlement_change_request.v1","after":{after},"limit":{limit}}}"#
    )
    .into_bytes()
}
async fn send(router: &axum::Router, request: axum::extract::Request) -> (u16, Vec<u8>) {
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 300_000)
        .await
        .unwrap()
        .to_vec();
    if status != 200 {
        assert!(bytes.is_empty(), "non-200 body must be empty");
    }
    (status, bytes)
}
fn router_with(
    pool: sqlx::PgPool,
    entitlements: Arc<TestEntitlements>,
    settings: http::Settings,
) -> axum::Router {
    http::router(
        Authority::new(
            pool.clone(),
            Arc::new(TestAccounts),
            entitlements,
            settings.authority_valid(),
        ),
        Changes::new(pool, settings.changes_valid()),
        settings,
        identity_map(),
    )
}

#[tokio::test]
async fn malformed_requests_consume_peer_budget() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    for (path, peer) in [
        (http::AUTHORITY_PATH, B1_PEER),
        (http::CHANGES_PATH, B2_PEER),
    ] {
        for content_type in ["application/json", "text/plain"] {
            let mut config = settings(&[]);
            config.authority_budget = Some(2);
            config.changes_budget = Some(2);
            let router = router_with(pool.clone(), Arc::new(TestEntitlements::default()), config);
            for attempt in 0..3 {
                let body: &[u8] = if content_type == "application/json" {
                    b"not json"
                } else {
                    b"{}"
                };
                let mut malformed = request(path, body, Some(peer));
                malformed
                    .headers_mut()
                    .insert("content-type", content_type.parse().unwrap());
                let response = router.clone().oneshot(malformed).await.unwrap();
                let expected = if attempt < 2 { 400 } else { 429 };
                assert_eq!(
                    response.status().as_u16(),
                    expected,
                    "{path} {content_type} attempt {attempt}"
                );
                assert_eq!(response.headers()["cache-control"], "no-store, private");
                if attempt == 2 {
                    assert_eq!(response.headers()["retry-after"], "60");
                }
                assert!(
                    axum::body::to_bytes(response.into_body(), 256)
                        .await
                        .unwrap()
                        .is_empty()
                );
            }
        }
    }
}

/// Guard order without a database: flag/configuration (503) before identity
/// (401), identity and scope before the body (400).
#[tokio::test]
async fn route_guard_order_without_database() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let entitlements = Arc::new(TestEntitlements::default());
    let off = http::Settings::default();
    let on = settings(&[]);
    let mut bad_revision = settings(&[]);
    bad_revision.producer_revision = "bad".into();
    let mut bad_products = settings(&[]);
    bad_products.products = None;
    let mut bad_budget = settings(&[]);
    bad_budget.authority_budget = None;
    bad_budget.changes_budget = None;
    let account = fixture_account();
    let b1 = http::AUTHORITY_PATH;
    let b2 = http::CHANGES_PATH;
    type Case<'a> = (&'a http::Settings, &'a str, Vec<u8>, Option<&'a str>, u16);
    let cases: Vec<Case> = vec![
        // Flag off or misconfigured: 503 for every peer, before identity.
        (&off, b1, b1_body(account), Some(B1_PEER), 503),
        (&off, b1, b1_body(account), Some(OTHER_PEER), 503),
        (&off, b2, b2_body(None, 1), Some(B2_PEER), 503),
        (&off, b2, b2_body(None, 1), None, 503),
        (&bad_revision, b1, b1_body(account), Some(B1_PEER), 503),
        (&bad_products, b1, b1_body(account), Some(B1_PEER), 503),
        (&bad_budget, b1, b1_body(account), Some(B1_PEER), 503),
        (&bad_budget, b2, b2_body(None, 1), Some(B2_PEER), 503),
        // Wrong identity or scope: 401 before the body is read.
        (&on, b1, b1_body(account), None, 401),
        (&on, b1, b1_body(account), Some(OTHER_PEER), 401),
        (&on, b1, b1_body(account), Some(B2_PEER), 401),
        (&on, b2, b2_body(None, 1), Some(B1_PEER), 401),
        (&on, b1, b"not json".to_vec(), Some(B2_PEER), 401),
        // Malformed or oversized: 400.
        (&on, b1, b"{}".to_vec(), Some(B1_PEER), 400),
        (&on, b1, vec![b' '; 257], Some(B1_PEER), 400),
        (&on, b1, b"{\"a\":1,\"a\":2}".to_vec(), Some(B1_PEER), 400),
        (&on, b1, b"[]".to_vec(), Some(B1_PEER), 400),
        (&on, b2, b2_body(None, 501), Some(B2_PEER), 400),
        (&on, b2, vec![b' '; 257], Some(B2_PEER), 400),
        (
            &on,
            b2,
            br#"{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":1.0}"#
                .to_vec(),
            Some(B2_PEER),
            400,
        ),
    ];
    for (config, path, body, peer, expected) in cases {
        let router = router_with(pool.clone(), entitlements.clone(), config.clone());
        let (status, _) = send(&router, request(path, &body, peer)).await;
        assert_eq!(status, expected, "{path} {peer:?} {config:?}");
    }
    // Query string and a wrong content type are transport refusals.
    let router = router_with(pool.clone(), entitlements.clone(), on.clone());
    let mut query = request(b1, &b1_body(account), Some(B1_PEER));
    *query.uri_mut() = format!("{b1}?x=1").parse().unwrap();
    assert_eq!(send(&router, query).await.0, 400);
    let mut typed = request(b2, &b2_body(None, 1), Some(B2_PEER));
    typed
        .headers_mut()
        .insert("content-type", "text/plain".parse().unwrap());
    assert_eq!(send(&router, typed).await.0, 400);
    // Every authenticated request counts against the budget, then 429.
    let mut tight = on.clone();
    tight.authority_budget = Some(1);
    let router = router_with(pool, entitlements, tight);
    assert_eq!(
        send(&router, request(b1, b"{}", Some(B1_PEER))).await.0,
        400
    );
    let response = router
        .clone()
        .oneshot(request(b1, &b1_body(account), Some(B1_PEER)))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 429);
    assert_eq!(response.headers()["retry-after"], "60");
    // Configuration parsing: only literal `true` enables, and bounds hold.
    let lookup = |pairs: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
        }
    };
    let parsed = http::Settings::from_lookup(lookup(&[
        ("PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED", "TRUE"),
        ("PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED", "true"),
        ("PLATFORM_BUILD_REVISION", BUILD),
        (
            "PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_REQUESTS_PER_MINUTE",
            "6001",
        ),
    ]));
    assert!(!parsed.authority_valid());
    assert!(!parsed.changes_valid());
    let parsed = http::Settings::from_lookup(lookup(&[
        ("PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED", "true"),
        ("PLATFORM_BUILD_REVISION", BUILD),
    ]));
    assert!(parsed.authority_valid());
    assert_eq!(parsed.authority_budget, Some(1200));
    assert_eq!(parsed.products, Some(Vec::new()));
    assert!(!parsed.changes_valid());
}

struct Database {
    pool: sqlx::PgPool,
    admin: sqlx::PgPool,
    schema: String,
}
impl Database {
    async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED profile-b database test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = sqlx::PgPool::connect(&url).await.unwrap();
        // Identifiers contain only this fixed prefix and locally generated UUID hex.
        let schema = format!("profile_b_test_{}", Uuid::now_v7().simple());
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
        for migration in [
            include_str!("../../../migrations/20261105160000_t27_premium_ledger.sql"),
            include_str!("../../../migrations/20261115100002_profile_b_authority_wire.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    /// The journal writer a Profile-B product ledger uses: next position under
    /// the locked head, in one transaction.
    async fn publish(&self, account: Uuid, product: &str, revision: i64) {
        let mut tx = self.pool.begin().await.unwrap();
        let position: i64 = sqlx::query_scalar(
            "UPDATE profile_b_change_head SET position=position+1 WHERE singleton RETURNING position",
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        sqlx::query("INSERT INTO profile_b_changes(position,account_id,product_id,product_version,lifecycle_revision) VALUES ($1,$2,$3,1,$4)")
            .bind(position)
            .bind(account)
            .bind(product)
            .bind(revision)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
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
fn known() -> Uuid {
    let mut bytes = *Uuid::now_v7().as_bytes();
    bytes[15] = 1;
    Uuid::from_bytes(bytes)
}

#[tokio::test]
async fn db_change_pages_match_fixtures_and_cursor_rules() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let router = router_with(
        db.pool.clone(),
        Arc::new(TestEntitlements::default()),
        settings(&[]),
    );
    let b2 = http::CHANGES_PATH;
    let fixture_request = fixture("changes-request.json");
    let (status, bytes) = send(&router, request(b2, &fixture_request, Some(B2_PEER))).await;
    assert_eq!((status, bytes), (200, fixture("changes-empty.json")));
    // A cursor beyond the journal head is invalid.
    assert_eq!(
        send(&router, request(b2, &b2_body(Some(1), 200), Some(B2_PEER)))
            .await
            .0,
        400
    );
    db.publish(fixture_account(), "oteryn.premium_time", 4)
        .await;
    db.publish(fixture_account(), "oteryn.example_active", 3)
        .await;
    let (status, bytes) = send(&router, request(b2, &fixture_request, Some(B2_PEER))).await;
    assert_eq!((status, bytes), (200, fixture("changes-page.json")));
    // The page cursor is the last item; a limit shortens the page.
    let (_, bytes) = send(&router, request(b2, &b2_body(None, 1), Some(B2_PEER))).await;
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["next_cursor"], "v1:1");
    let (_, bytes) = send(&router, request(b2, &b2_body(Some(2), 200), Some(B2_PEER))).await;
    assert_eq!(bytes, fixture("changes-empty.json"));
    // The journal is append-only.
    assert!(
        sqlx::query("DELETE FROM profile_b_changes")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE profile_b_changes SET lifecycle_revision=9")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await;
}

#[tokio::test]
async fn db_premium_lifecycle_changes_publish_hints_in_their_transaction() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger =
        crate::premium::repository::Ledger::new(db.pool.clone(), Arc::new(TestAccounts), true);
    let operator = Operator {
        actor: Uuid::now_v7(),
        confirmed_mfa: true,
        permission: "products.premium.manage".into(),
    };
    let account = known();
    let mutation = |change| Mutation {
        account,
        change,
        request: Uuid::now_v7(),
        reason: "Test operator reason".into(),
    };
    let grant = mutation(Change::Grant(2));
    ledger.mutate(&operator, &grant).await.unwrap();
    // An idempotent replay is not a lifecycle change.
    ledger.mutate(&operator, &grant).await.unwrap();
    ledger
        .mutate(&operator, &mutation(Change::Revoke))
        .await
        .unwrap();
    // A rolled-back change publishes nothing.
    let mut tx = db.pool.begin().await.unwrap();
    ledger
        .mutate_in(&mut tx, &operator, &mutation(Change::Grant(1)))
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let router = router_with(
        db.pool.clone(),
        Arc::new(TestEntitlements::default()),
        settings(&[]),
    );
    let (status, bytes) = send(
        &router,
        request(http::CHANGES_PATH, &b2_body(None, 500), Some(B2_PEER)),
    )
    .await;
    assert_eq!(status, 200);
    let expected = format!(
        r#"{{"schema":"oteryn.entitlement_change_page.v1","items":[{{"account_id":"{account}","product_id":"oteryn.premium_time","product_version":1,"lifecycle_revision":1}},{{"account_id":"{account}","product_id":"oteryn.premium_time","product_version":1,"lifecycle_revision":2}}],"next_cursor":"v1:2"}}"#
    );
    assert_eq!(String::from_utf8(bytes).unwrap(), expected);
    db.close().await;
}

#[tokio::test]
async fn db_authority_issues_advances_and_fences() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let entitlements = Arc::new(TestEntitlements::default());
    let router = router_with(
        db.pool.clone(),
        entitlements.clone(),
        settings(&["oteryn.example_active", "oteryn.example_none"]),
    );
    let b1 = http::AUTHORITY_PATH;
    let account = fixture_account();
    let revision = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(authority_revision),0) FROM profile_b_authority",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap()
    };

    // Unknown account: 404 before any revision is allocated; ambiguous: 503.
    for (last, expected) in [(0_u8, 404), (255, 503)] {
        let mut bytes = *account.as_bytes();
        bytes[15] = last;
        let (status, _) = send(
            &router,
            request(b1, &b1_body(Uuid::from_bytes(bytes)), Some(B1_PEER)),
        )
        .await;
        assert_eq!(status, expected);
    }
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profile_b_authority")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);

    // Explicit NONE for both products, revision 1.
    let (status, bytes) = send(
        &router,
        request(b1, &fixture("authority-request.json"), Some(B1_PEER)),
    )
    .await;
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["authority_revision"], 1);
    assert_eq!(value["nonce"], NONCE);
    assert_eq!(value["account_id"], account.to_string());
    assert_eq!(value["producer_revision"], BUILD);
    for (index, id) in ["oteryn.example_active", "oteryn.example_none"]
        .iter()
        .enumerate()
    {
        let item = &value["entitlements"][index];
        assert_eq!(item["product_id"], *id);
        assert_eq!(item["entitlement_state"], "NONE");
        assert_eq!(item["lifecycle_revision"], 0);
        assert_eq!(item["lease_policy"], LEASE_POLICY);
    }
    let issued = parse_time(value["authority_issued_at"].as_str().unwrap());
    assert_eq!(
        parse_time(
            value["entitlements"][0]["authority_valid_until"]
                .as_str()
                .unwrap()
        ),
        issued + 3600
    );
    assert_eq!(
        parse_time(value["entitlements"][0]["refresh_after"].as_str().unwrap()),
        issued + 2400
    );

    // An entitlement whose revision was never published is advanced: 503,
    // and the revision is not consumed.
    let mut record = active_record();
    record.from = issued - 86400;
    record.until = Some(issued + 30 * 86400);
    entitlements
        .0
        .lock()
        .unwrap()
        .insert("oteryn.example_active".into(), record.clone());
    let (status, _) = send(&router, request(b1, &b1_body(account), Some(B1_PEER))).await;
    assert_eq!(status, 503);
    assert_eq!(revision().await, 1);
    // Published through revision 3: ACTIVE with the record's values.
    for rev in 1..=3 {
        db.publish(account, "oteryn.example_active", rev).await;
    }
    let (status, bytes) = send(&router, request(b1, &b1_body(account), Some(B1_PEER))).await;
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["authority_revision"], 2);
    let item = &value["entitlements"][0];
    assert_eq!(item["entitlement_state"], "ACTIVE");
    assert_eq!(item["entitlement_id"], record.id.to_string());
    assert_eq!(item["lifecycle_revision"], 3);
    assert_eq!(value["entitlements"][1]["entitlement_state"], "NONE");
    // A published revision the source has not caught up with is stale: 503.
    db.publish(account, "oteryn.example_active", 4).await;
    assert_eq!(
        send(&router, request(b1, &b1_body(account), Some(B1_PEER)))
            .await
            .0,
        503
    );
    // A NONE product with published history is stale too.
    record.revision = 4;
    entitlements
        .0
        .lock()
        .unwrap()
        .insert("oteryn.example_active".into(), record.clone());
    db.publish(account, "oteryn.example_none", 1).await;
    assert_eq!(
        send(&router, request(b1, &b1_body(account), Some(B1_PEER)))
            .await
            .0,
        503
    );
    assert_eq!(revision().await, 2);

    // Lease mismatch: a record under another lease policy never issues.
    let other = known();
    let mut leased = active_record();
    leased.from = issued - 86400;
    leased.until = Some(issued + 86400);
    leased.revision = 1;
    leased.lease_policy = "oteryn.profile_b.lease.v2".into();
    let single = Arc::new(TestEntitlements::default());
    single
        .0
        .lock()
        .unwrap()
        .insert("oteryn.example_active".into(), leased.clone());
    db.publish(other, "oteryn.example_active", 1).await;
    let leased_router = router_with(
        db.pool.clone(),
        single.clone(),
        settings(&["oteryn.example_active"]),
    );
    assert_eq!(
        send(&leased_router, request(b1, &b1_body(other), Some(B1_PEER)))
            .await
            .0,
        503
    );
    leased.lease_policy = LEASE_POLICY.into();
    single
        .0
        .lock()
        .unwrap()
        .insert("oteryn.example_active".into(), leased);
    assert_eq!(
        send(&leased_router, request(b1, &b1_body(other), Some(B1_PEER)))
            .await
            .0,
        200
    );

    // Advanced issue time (clock moved back) and an exhausted revision: 503.
    sqlx::query("UPDATE profile_b_authority SET last_issued_at=clock_timestamp()+interval '1 hour' WHERE account_id=$1")
        .bind(other)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&leased_router, request(b1, &b1_body(other), Some(B1_PEER)))
            .await
            .0,
        503
    );
    sqlx::query("UPDATE profile_b_authority SET last_issued_at=NULL, authority_revision=$2 WHERE account_id=$1")
        .bind(other)
        .bind(MAX_REVISION)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&leased_router, request(b1, &b1_body(other), Some(B1_PEER)))
            .await
            .0,
        503
    );
    // The production source fails closed for a configured product.
    let closed = http::router(
        Authority::new(
            db.pool.clone(),
            Arc::new(TestAccounts),
            Arc::new(NoGenericLedger),
            true,
        ),
        Changes::new(db.pool.clone(), true),
        settings(&["oteryn.example_none"]),
        identity_map(),
    );
    assert_eq!(
        send(&closed, request(b1, &b1_body(known()), Some(B1_PEER)))
            .await
            .0,
        503
    );
    db.close().await;
}

#[tokio::test]
async fn db_authority_revision_strictly_increases_under_concurrency() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let router = router_with(
        db.pool.clone(),
        Arc::new(TestEntitlements::default()),
        settings(&["oteryn.example_none"]),
    );
    let account = known();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let router = router.clone();
        tasks.spawn(async move {
            send(
                &router,
                request(http::AUTHORITY_PATH, &b1_body(account), Some(B1_PEER)),
            )
            .await
        });
    }
    let mut revisions = Vec::new();
    while let Some(result) = tasks.join_next().await {
        let (status, bytes) = result.unwrap();
        assert_eq!(status, 200);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        revisions.push(value["authority_revision"].as_i64().unwrap());
    }
    revisions.sort_unstable();
    assert_eq!(revisions, (1..=16).collect::<Vec<_>>());
    db.close().await;
}

/// The value check every 200 body passes: canonical, no escapes, under bound.
#[test]
fn fixtures_are_canonical() {
    for name in [
        "authority-request.json",
        "authority.json",
        "changes-request.json",
        "changes-page.json",
        "changes-empty.json",
    ] {
        let bytes = fixture(name);
        let value: Value = canonical::decode(&bytes, 300_000, 4).unwrap();
        assert_eq!(canonical::encode(&value).unwrap(), bytes, "{name}");
    }
}

#[tokio::test]
async fn db_authority_held_account_lock_sheds_within_mutation_budget() {
    use sqlx::Connection;
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    sqlx::query("CREATE TABLE accounts(id uuid PRIMARY KEY)")
        .execute(&db.pool)
        .await
        .unwrap();
    let account = known();
    sqlx::query("INSERT INTO accounts VALUES($1)")
        .bind(account)
        .execute(&db.pool)
        .await
        .unwrap();
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let mut blocker = sqlx::PgConnection::connect(&url).await.unwrap();
    sqlx::query("BEGIN").execute(&mut blocker).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM {schema}.accounts WHERE id=$1 FOR UPDATE"
    )))
    .bind(account)
    .execute(&mut blocker)
    .await
    .unwrap();

    // The production account adapter takes the row FOR SHARE and arms nothing.
    let authority = Authority::new(
        db.pool.clone(),
        Arc::new(crate::loyalty::stream::http::DatabaseAccounts),
        Arc::new(NoGenericLedger),
        true,
    );
    let started = Instant::now();
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        authority.read(account, NONCE, BUILD, &[]),
    )
    .await;
    let elapsed = started.elapsed();
    sqlx::query("ROLLBACK").execute(&mut blocker).await.unwrap();
    blocker.close().await.unwrap();
    let issued: i64 = sqlx::query_scalar("SELECT count(*) FROM profile_b_authority")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    db.close().await;

    let observed = observed.expect("authority still waiting at 10 s on a held account lock");
    assert_eq!(observed, Err(Error::Unavailable));
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
    assert_eq!(issued, 0);
}
