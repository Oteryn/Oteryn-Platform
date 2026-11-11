#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
const NONCE: &str = "4f1c2a9be07d4c3a8b6e5d4c3b2a1908";
const BUILD: &str = "0123456789abcdef0123456789abcdef01234567";
fn known() -> Uuid {
    Uuid::parse_str("01890f4e-7c00-7000-8000-000000000001").unwrap()
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
            eprintln!("SKIPPED loyalty database test: PORTAL_TEST_DATABASE_URL is unset");
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
    fn ledger(&self, enabled: bool) -> Ledger {
        Ledger::new(
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
fn identity_map() -> std::sync::Arc<oteryn_internal_api::guard::IdentityMap> {
    use oteryn_internal_api::guard::*;
    std::sync::Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::san("loyalty.test").unwrap(),
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
    format!(r#"{{"schema":"oteryn.loyalty_snapshot_request.v1","account_id":"{account}","nonce":"{NONCE}"}}"#).into_bytes()
}
fn settings() -> http::Settings {
    http::Settings {
        enabled: true,
        producer_revision: BUILD.into(),
        requests_per_minute: 1200,
    }
}

#[tokio::test]
async fn route_guard_bad_input_and_disabled_without_database() {
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let ledger = Ledger::new(pool, std::sync::Arc::new(TestAccounts), true);
    for (config, peer, bytes, status) in [
        (
            http::Settings::default(),
            Some("loyalty.test"),
            body(known()),
            503,
        ),
        (
            http::Settings {
                enabled: true,
                producer_revision: "bad".into(),
                requests_per_minute: 1200,
            },
            Some("loyalty.test"),
            body(known()),
            503,
        ),
        (settings(), None, body(known()), 401),
        (settings(), Some("store.test"), body(known()), 401),
        (settings(), Some("loyalty.test"), b"{}".to_vec(), 400),
        (settings(), Some("loyalty.test"), vec![b' '; 257], 400),
        (
            settings(),
            Some("loyalty.test"),
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

    for (config, peer, bytes, status) in [
        (
            http::Settings::default(),
            Some("loyalty.test"),
            body(account),
            503,
        ),
        (settings(), Some("store.test"), body(account), 401),
        (settings(), Some("loyalty.test"), vec![b' '; 257], 400),
    ] {
        let response = http::router(ledger.clone(), config, identity_map())
            .oneshot(request(bytes, peer))
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        assert_eq!(response.headers()["cache-control"], "no-store, private");
        assert!(
            axum::body::to_bytes(response.into_body(), 512)
                .await
                .unwrap()
                .is_empty()
        );
    }
    let router = http::router(ledger, settings(), identity_map());
    let response = router
        .clone()
        .oneshot(request(body(account), Some("loyalty.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    let bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert!(bytes.len() <= 512);
    let snapshot: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot["loyalty_points"], 0);
    assert_eq!(snapshot["loyalty_revision"], 1);
    assert_eq!(snapshot["nonce"], NONCE);
    assert_eq!(snapshot["account_id"], account.to_string());
    let mut missing = *account.as_bytes();
    missing[15] = 0;
    let missing = Uuid::from_bytes(missing);
    let response = router
        .clone()
        .oneshot(request(body(missing), Some("loyalty.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    // Malformed but authenticated calls consume the same peer budget.
    for _ in 0..1198 {
        let response = router
            .clone()
            .oneshot(request(b"{}".to_vec(), Some("loyalty.test")))
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
    }
    let response = router
        .oneshot(request(body(account), Some("loyalty.test")))
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    assert_eq!(response.headers()["retry-after"], "60");
    db.close().await;
}

#[test]
fn contract_golden_and_wire_bounds() {
    // SOURCE: docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md,
    // successful response member order and ranges; fixed local example values.
    let expected = r#"{"schema":"oteryn.loyalty_snapshot.v1","producer_revision":"0123456789abcdef0123456789abcdef01234567","nonce":"4f1c2a9be07d4c3a8b6e5d4c3b2a1908","account_id":"01890f4e-7c00-7000-8000-000000000001","loyalty_points":2,"loyalty_revision":7,"computed_at":"1970-01-01T00:00:00Z"}"#;
    assert_eq!(
        snapshot(2, 7, 0, known(), NONCE, BUILD).unwrap(),
        expected.as_bytes()
    );
    assert!(
        snapshot(i32::MAX, MAX_REVISION, 253402300799, known(), NONCE, BUILD)
            .unwrap()
            .len()
            <= 512
    );
    for (points, authority, nonce) in [(-1, 1, NONCE), (0, 0, NONCE), (0, 1, "INVALID")] {
        assert!(snapshot(points, authority, 0, known(), nonce, BUILD).is_err());
    }
    for raw in [
        b"{}".as_slice(),
        b"[]",
        b"{",
        br#"{"schema":"a","schema":"b"}"#,
    ] {
        assert!(http::decode_bytes(raw).is_err());
    }
    let valid = body(known());
    assert!(http::decode_bytes(&valid).is_ok());
    for bad in [
        String::from_utf8(valid.clone())
            .unwrap()
            .replace("000001", "00000A"),
        String::from_utf8(valid).unwrap().replace(NONCE, "bad"),
    ] {
        assert!(http::decode_bytes(bad.as_bytes()).is_err());
    }
    assert!(!http::Settings::default().enabled);
}

#[tokio::test]
async fn db_consumed_intervals_shared_revision_and_rollback() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    let account = known();
    ledger.snapshot(account, NONCE, BUILD).await.unwrap();
    let now: OffsetDateTime = sqlx::query_scalar("SELECT date_trunc('second',clock_timestamp())")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let entitlement = Uuid::now_v7();
    // Two lifecycle events for one interval: revocation clips to 1.5 days.
    // Restarted and independent intervals add fractions before whole-day flooring.
    for (revision, start, end) in [
        (1, -4 * 86400, 4 * 86400),
        (2, -4 * 86400, -216000),
        (3, -86400, 86400),
        (4, 86400, 172800),
    ] {
        sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) VALUES ($1,$2,$3,'GRANT',1,$4,$5,$6,0,$7,$8)")
            .bind(Uuid::now_v7()).bind(account).bind(Uuid::now_v7()).bind(vec![0u8;32]).bind(entitlement).bind(i64::from(revision))
            .bind(now+time::Duration::seconds(start)).bind(now+time::Duration::seconds(end)).execute(&db.pool).await.unwrap();
    }
    let bytes = ledger.snapshot(account, NONCE, BUILD).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["loyalty_points"], 2);
    assert_eq!(value["loyalty_revision"], 2);
    let premium =
        crate::premium::repository::Ledger::new(db.pool.clone(), Arc::new(TestAccounts), true);
    // Premium rejects the deliberately incomplete fixture's lifecycle totals;
    // loyalty uses the independent interval grouping required by its contract.
    assert!(premium.snapshot(account, NONCE, BUILD).await.is_err());
    assert!(ledger.snapshot(account, "bad", BUILD).await.is_err());
    let row: (i32, i64) = sqlx::query_as(
        "SELECT loyalty_points,loyalty_revision FROM loyalty_state WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(row, (2, 2));
    assert_eq!(
        db.ledger(false).snapshot(account, NONCE, BUILD).await,
        Err(Error::Disabled)
    );
    sqlx::query("UPDATE premium_authority SET authority_revision=$2 WHERE account_id=$1")
        .bind(account)
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

#[tokio::test]
async fn db_premium_and_loyalty_reads_share_ordering() {
    let Some(db) = Database::create().await else {
        return;
    };
    let loyalty = db.ledger(true);
    let premium =
        crate::premium::repository::Ledger::new(db.pool.clone(), Arc::new(TestAccounts), true);
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..12 {
        let loyalty = loyalty.clone();
        let premium = premium.clone();
        tasks.spawn(async move {
            let bytes = if index % 2 == 0 {
                loyalty.snapshot(known(), NONCE, BUILD).await
            } else {
                premium.snapshot(known(), NONCE, BUILD).await
            }
            .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            value[if index % 2 == 0 {
                "loyalty_revision"
            } else {
                "authority_revision"
            }]
            .as_i64()
            .unwrap()
        });
    }
    let mut revisions = Vec::new();
    while let Some(result) = tasks.join_next().await {
        revisions.push(result.unwrap());
    }
    revisions.sort();
    assert_eq!(revisions, (1..=12).collect::<Vec<_>>());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_ledger")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
}

#[tokio::test]
async fn db_projection_failure_rolls_back_authority() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    sqlx::raw_sql("CREATE FUNCTION reject_loyalty() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test projection failure'; END; $$; CREATE TRIGGER reject_loyalty BEFORE INSERT ON loyalty_state FOR EACH ROW EXECUTE FUNCTION reject_loyalty();").execute(&db.pool).await.unwrap();
    assert_eq!(
        ledger.snapshot(known(), NONCE, BUILD).await,
        Err(Error::Unavailable)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_authority")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.close().await;
}

#[tokio::test]
async fn db_points_saturate_and_unknown_allocates_nothing() {
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.ledger(true);
    let unknown = Uuid::parse_str("01890f4e-7c00-7000-8000-000000000000").unwrap();
    assert_eq!(
        ledger.snapshot(unknown, NONCE, BUILD).await,
        Err(Error::UnknownAccount)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM premium_authority")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    ledger.snapshot(known(), NONCE, BUILD).await.unwrap();
    sqlx::query("INSERT INTO premium_ledger(request_id,account_id,actor_id,event_type,duration_days,reason_sha256,entitlement_id,lifecycle_revision,delta_seconds,effective_from,effective_until) SELECT gen_random_uuid(),$1,$1,'GRANT',1,$2,gen_random_uuid(),n,0,'0001-01-01T00:00:00Z','2001-01-01T00:00:00Z' FROM generate_series(1,3000) n")
        .bind(known()).bind(vec![0u8;32]).execute(&db.pool).await.unwrap();
    let bytes = ledger.snapshot(known(), NONCE, BUILD).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["loyalty_points"], i32::MAX);
    assert!(bytes.len() <= 512);
    db.close().await;
}

#[test]
fn settings_fail_closed_and_identity_is_dedicated() {
    for rate in ["bad", "0", "6001"] {
        let settings = http::Settings::from_lookup(|key| match key {
            "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED" => Some("true".into()),
            "PLATFORM_BUILD_REVISION" => Some(BUILD.into()),
            "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_REQUESTS_PER_MINUTE" => Some(rate.into()),
            _ => None,
        });
        assert!(!(1..=6000).contains(&settings.requests_per_minute));
    }
    assert!(http::Settings::identity_binding(|_| None).is_err());
    let binding = http::Settings::identity_binding(|_| Some("loyalty.test".into())).unwrap();
    assert_eq!(binding.group, http::GROUP);
    assert!(
        oteryn_internal_api::guard::IdentityMap::new(vec![
            binding.clone(),
            oteryn_internal_api::guard::Binding {
                identity: binding.identity,
                group: crate::premium::http::GROUP.into()
            }
        ])
        .is_err()
    );
}
