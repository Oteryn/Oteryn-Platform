use super::*;
use axum::body::to_bytes;
use oteryn_internal_api::guard::VerifiedClient;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const REQUEST: &[u8] =
    include_bytes!("../../../../docs/contracts/fixtures/security-revocation-v1/request.json");
const RESPONSE: &[u8] =
    include_bytes!("../../../../docs/contracts/fixtures/security-revocation-v1/page.json");
const EMPTY: &[u8] =
    include_bytes!("../../../../docs/contracts/fixtures/security-revocation-v1/empty.json");
const SAN: &str = "revocations.game.example.invalid";
fn map() -> TestResult<IdentityMap> {
    Ok(identities(vec![(IDENTITY_KEY.into(), SAN.into())])?)
}
fn request(body: impl Into<Body>, identity: Option<&str>) -> TestResult<Request> {
    let mut req = Request::builder()
        .method("POST")
        .uri(PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.into())?;
    if let Some(san) = identity {
        req.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                san,
            )?));
    }
    Ok(req)
}
fn lazy_pool() -> TestResult<PgPool> {
    Ok(PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?)
}
fn app(pool: PgPool, enabled: bool) -> TestResult<Router> {
    Ok(router(pool, Settings { enabled }, Arc::new(map()?)))
}
#[tokio::test]
async fn security_revocation_export_disabled_503() -> TestResult {
    assert!(!Settings::from_lookup(|_| None).enabled);
    assert!(!Settings::from_lookup(|_| Some("1".into())).enabled);
    let response = app(lazy_pool()?, false)?
        .oneshot(request(REQUEST, Some(SAN))?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    assert!(to_bytes(response.into_body(), 256).await?.is_empty());
    let response = router(
        lazy_pool()?,
        Settings { enabled: true },
        Arc::new(IdentityMap::default()),
    )
    .oneshot(request(REQUEST, None)?)
    .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    for vars in [
        vec![],
        vec![(IDENTITY_KEY.into(), "bad name".into())],
        vec![(IDENTITY_KEY.into(), "UPPER.example".into())],
        vec![
            (IDENTITY_KEY.into(), "peer.example".into()),
            ("OTHER_MTLS_CLIENT_IDENTITY".into(), "peer.example".into()),
        ],
        vec![
            (IDENTITY_KEY.into(), "peer.example".into()),
            ("OTHER_IDENTITIES".into(), r#"{"peer.example":[]}"#.into()),
        ],
    ] {
        assert!(identities(vars).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn security_revocation_export_wrong_identity_refused() -> TestResult {
    for identity in [None, Some("other.game.example.invalid")] {
        let mut req = request(REQUEST, identity)?;
        req.headers_mut()
            .insert("x-client-san", header::HeaderValue::from_static(SAN));
        let response = app(lazy_pool()?, true)?.oneshot(req).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(to_bytes(response.into_body(), 256).await?.is_empty());
    }
    Ok(())
}
#[tokio::test]
async fn security_revocation_export_bad_body_400() -> TestResult {
    let app = app(lazy_pool()?, true)?;
    for body in [
        "{}",
        "[]",
        r#"{"schema":"other","after":null,"limit":1}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":1}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":0}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":501}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":"v1:0","limit":1}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":"v1:01","limit":1}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":"v2:1","limit":1}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":"v1:9223372036854775808","limit":1}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":1.0}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":1e0}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":1,"limit":2}"#,
        r#"{"schema":"oteryn.security_revocation_stream_request.v1","after":null,"limit":1,"extra":true}"#,
        &" ".repeat(257),
    ] {
        let response = app
            .clone()
            .oneshot(request(body.to_owned(), Some(SAN))?)
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
        assert!(to_bytes(response.into_body(), 256).await?.is_empty());
    }
    let mut query = request(REQUEST, Some(SAN))?;
    *query.uri_mut() = format!("{PATH}?after=1").parse()?;
    assert_eq!(app.oneshot(query).await?.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
#[tokio::test]
async fn security_revocation_export_rate_limit_retry_after() -> TestResult {
    let app = app(lazy_pool()?, true)?;
    for _ in 0..1200 {
        assert_eq!(
            app.clone()
                .oneshot(request("{}", Some(SAN))?)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let response = app.oneshot(request("{}", Some(SAN))?).await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    Ok(())
}
#[tokio::test]
async fn security_revocation_export_reused_internal_identities_503() -> TestResult {
    for key in [
        "PORTAL_SANCTION_EXPORT_IDENTITY",
        "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY",
        "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY",
        "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY",
        "OTERYN_GATEWAY_SAN",
        "PORTAL_GAME_COINS_IDENTITIES",
    ] {
        let other = if key.ends_with("_IDENTITIES") {
            format!("{{\"{SAN}\":[]}}")
        } else {
            SAN.into()
        };
        let config = identities(vec![(IDENTITY_KEY.into(), SAN.into()), (key.into(), other)]);
        assert!(config.is_err(), "{key}");
        let response = router(
            lazy_pool()?,
            Settings {
                enabled: config.is_ok(),
            },
            Arc::new(config.unwrap_or_default()),
        )
        .oneshot(request(REQUEST, Some(SAN))?)
        .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{key}");
    }
    Ok(())
}
#[test]
fn security_revocation_export_full_page_byte_bound() -> TestResult {
    let raw = serde_json::from_slice::<serde_json::Value>(RESPONSE)?;
    let mut item = raw["items"][0].clone();
    item["native_security_generation"] = i64::MAX.into();
    item["cause"] = 4.into();
    item["effective_at_ms"] = i64::MAX.into();
    let value = canonical::decode(&serde_json::to_vec(&item)?, 1024, 4)?;
    let bytes = encode_page((1..=500).map(|n| (n, value.clone())).collect())?;
    assert!(bytes.len() <= MAX_RESPONSE);
    let page: serde_json::Value = serde_json::from_slice(&bytes)?;
    let items = page["items"].as_array().ok_or("items must be an array")?;
    assert_eq!(items.len(), 500);
    assert_eq!(page["next_cursor"], "v1:500");
    let empty = encode_page(Vec::new())?;
    assert_eq!(empty, EMPTY);
    Ok(())
}
#[test]
fn security_revocation_request_fixture_decodes() -> TestResult {
    assert_eq!(
        Page::decode(&canonical::decode(REQUEST, 256, 4)?)?,
        Page {
            after: None,
            limit: 1
        }
    );
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
            eprintln!(
                "SKIPPED security revocation PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("security_revocations_{}", Uuid::now_v7().simple());
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
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn account(&self) -> TestResult<Uuid> {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES ($1,$2,'synthetic-hash','active','en',now())")
            .bind(id).bind(format!("{id}@example.invalid")).execute(&self.pool).await?;
        Ok(id)
    }
    async fn generation(&self, account: Uuid) -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&self.pool)
                .await?,
        )
    }
    /// (position, account, generation, cause) in stream order.
    async fn records(&self) -> TestResult<Vec<(i64, Uuid, i64, i16)>> {
        Ok(sqlx::query_as("SELECT position,account_id,native_security_generation,cause FROM identity_security_revocations ORDER BY position")
            .fetch_all(&self.pool).await?)
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
async fn page(db: &Database, after: Option<i64>, limit: i64) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(
        &read_page(&db.pool, Page { after, limit }).await?,
    )?)
}
#[tokio::test]
async fn security_revocation_export_fixtures_match() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(
        read_page(
            &db.pool,
            Page {
                after: None,
                limit: 1
            }
        )
        .await?,
        EMPTY
    );
    sqlx::query("INSERT INTO identity_security_revocations(position,revocation_id,account_id,native_security_generation,cause,effective_at_ms) VALUES (1,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000002',2,1,1700000000000)")
        .execute(&db.pool).await?;
    let response = app(db.pool.clone(), true)?
        .oneshot(request(REQUEST, Some(SAN))?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    assert_eq!(
        to_bytes(response.into_body(), MAX_RESPONSE).await?.as_ref(),
        RESPONSE
    );
    for statement in [
        "UPDATE identity_security_revocations SET cause=2",
        "DELETE FROM identity_security_revocations",
    ] {
        assert!(sqlx::query(statement).execute(&db.pool).await.is_err());
    }
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_cause_credential() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    // Unrelated account edits publish nothing.
    sqlx::query("UPDATE accounts SET locale='pl',public_status_visible=true WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    assert!(db.records().await?.is_empty());
    sqlx::query("UPDATE accounts SET password_hash='synthetic-hash-2' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE accounts SET email=$2 WHERE id=$1")
        .bind(account)
        .bind(format!("new-{account}@example.invalid"))
        .execute(&db.pool)
        .await?;
    // Confirmed TOTP enrolment advances through the nested MFA fence trigger.
    sqlx::query(
        "INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES ($1,'\\x00'::bytea,true)",
    )
    .bind(account)
    .execute(&db.pool)
    .await?;
    sqlx::query("UPDATE accounts SET login_mfa_required=true WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    let records = db.records().await?;
    assert_eq!(
        records,
        vec![
            (1, account, 2, 1),
            (2, account, 3, 1),
            (3, account, 4, 1),
            (4, account, 5, 1)
        ]
    );
    assert_eq!(db.generation(account).await?, 5);
    let result = page(&db, None, 500).await?;
    let items = result["items"].as_array().ok_or("items must be an array")?;
    assert_eq!(items.len(), 4);
    for item in items {
        let id = Uuid::parse_str(item["revocation_id"].as_str().ok_or("id")?)?;
        assert_eq!(id.get_version_num(), 7);
        assert!(item["effective_at_ms"].as_i64().ok_or("time")? > 1_700_000_000_000);
        let members: Vec<_> = item.as_object().ok_or("object")?.keys().cloned().collect();
        assert_eq!(members.len(), 5);
    }
    assert_eq!(result["next_cursor"], "v1:4");
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_cause_status() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let suspended = db.account().await?;
    let recovered = db.account().await?;
    let closed = db.account().await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(suspended)
        .execute(&db.pool)
        .await?;
    // Recovery-key use and termination advance the generation alone.
    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(recovered)
    .execute(&db.pool)
    .await?;
    // Termination changes status and credentials together: status wins.
    sqlx::query("UPDATE accounts SET status='closed',password_hash='!closed',email=$2,email_verified_at=NULL WHERE id=$1")
        .bind(closed).bind(format!("closed-{closed}@example.invalid")).execute(&db.pool).await?;
    assert_eq!(
        db.records().await?,
        vec![
            (1, suspended, 2, 2),
            (2, recovered, 2, 2),
            (3, closed, 2, 2)
        ]
    );
    // A non-UUIDv7 account holds no Game session: the change commits, no record.
    let legacy = Uuid::new_v4();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,'legacy@example.invalid','synthetic-hash','active','en')")
        .bind(legacy).execute(&db.pool).await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(legacy)
        .execute(&db.pool)
        .await?;
    assert_eq!(db.generation(legacy).await?, 2);
    assert_eq!(db.records().await?.len(), 3);
    db.cleanup().await
}
async fn suspension(
    db: &Database,
    account: Uuid,
    actor: Uuid,
    status: &str,
    effective: &str,
) -> TestResult<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO sanctions(id,account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,$2,'suspension',$3,'Public suspension',{effective},clock_timestamp()+interval '1 day',$4)")))
        .bind(id).bind(account).bind(status).bind(actor).execute(&db.pool).await?;
    Ok(id)
}
#[tokio::test]
async fn security_revocation_cause_sanction() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let actor = db.account().await?;
    let banned = db.account().await?;
    let typed = db.account().await?;
    let approver = db.account().await?;
    // Warnings, restrictions, pending and expired suspensions publish nothing.
    for (category, status) in [
        ("warning", "active"),
        ("restriction", "active"),
        ("suspension", "pending"),
        ("suspension", "expired"),
    ] {
        sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,$2,$3,'Public reason',clock_timestamp()-interval '1 hour',clock_timestamp()+interval '1 day',$4)")
            .bind(banned).bind(category).bind(status).bind(actor).execute(&db.pool).await?;
    }
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,'suspension','active','Public reason',clock_timestamp()-interval '2 days',clock_timestamp()-interval '1 day',$2)")
        .bind(banned).bind(actor).execute(&db.pool).await?;
    assert!(db.records().await?.is_empty());
    // Approval of a pending suspension makes it effective: one record, and the
    // generation does not change.
    let pending = suspension(
        &db,
        banned,
        actor,
        "pending",
        "clock_timestamp()-interval '1 minute'",
    )
    .await?;
    sqlx::query("UPDATE sanctions SET status='active',approved_by=$2 WHERE id=$1")
        .bind(pending)
        .bind(approver)
        .execute(&db.pool)
        .await?;
    // Later edits and the lift publish nothing.
    sqlx::query("UPDATE sanctions SET acknowledged_at=now() WHERE id=$1")
        .bind(pending)
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE sanctions SET status='revoked' WHERE id=$1")
        .bind(pending)
        .execute(&db.pool)
        .await?;
    assert_eq!(db.records().await?, vec![(1, banned, 1, 3)]);
    assert_eq!(db.generation(banned).await?, 1);
    // Typed ban: the effective revision publishes; an extension and the unban do not.
    let ban = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES ($1,$2,'ban')",
    )
    .bind(ban)
    .bind(typed)
    .execute(&db.pool)
    .await?;
    for (revision, active, expires) in [
        (1, true, "clock_timestamp()+interval '1 hour'"),
        (2, true, "clock_timestamp()+interval '2 hours'"),
        (3, false, "NULL"),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) VALUES ($1,$2,$3,'{{}}',$4,clock_timestamp(),{expires})")))
            .bind(typed).bind(revision).bind(ban).bind(active).execute(&db.pool).await?;
    }
    assert_eq!(
        db.records().await?,
        vec![(1, banned, 1, 3), (2, typed, 1, 3)]
    );
    // A typed namelock is not account-level and publishes nothing.
    let namelock = Uuid::now_v7();
    sqlx::query("INSERT INTO support_sanction_identities(sanction_id,account_id,kind,character_id) VALUES ($1,$2,'namelock',$3)")
        .bind(namelock).bind(typed).bind(Uuid::now_v7()).execute(&db.pool).await?;
    sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) VALUES ($1,4,$2,'{}',true,clock_timestamp(),NULL)")
        .bind(typed).bind(namelock).execute(&db.pool).await?;
    assert_eq!(db.records().await?.len(), 2);
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_future_suspension_published_when_effective() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let actor = db.account().await?;
    let account = db.account().await?;
    sqlx::query("UPDATE accounts SET password_hash='synthetic-hash-2' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    suspension(
        &db,
        account,
        actor,
        "active",
        "clock_timestamp()+interval '300 milliseconds'",
    )
    .await?;
    assert_eq!(db.records().await?, vec![(1, account, 2, 1)]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let result = page(&db, Some(1), 500).await?;
        if result["next_cursor"] == "v1:2" {
            assert_eq!(result["items"][0]["cause"], 3);
            assert_eq!(result["items"][0]["native_security_generation"], 2);
            break;
        }
        assert!(Instant::now() < deadline, "suspension never published");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Reconciliation is once per sanction.
    assert_eq!(
        page(&db, Some(2), 500).await?["next_cursor"],
        serde_json::Value::Null
    );
    assert_eq!(db.records().await?.len(), 2);
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_cause_staff_kick() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    let mut tx = db.pool.begin().await?;
    assert_eq!(staff_kick(&mut tx, account).await?, Some(2));
    // The cause hint is cleared: a later generation-only advance in the same
    // transaction is not attributed to the kick.
    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(account)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    assert_eq!(
        db.records().await?,
        vec![(1, account, 2, 4), (2, account, 3, 2)]
    );
    let mut tx = db.pool.begin().await?;
    assert_eq!(staff_kick(&mut tx, Uuid::now_v7()).await?, None);
    assert!(matches!(
        staff_kick(&mut tx, Uuid::nil()).await,
        Err(Error::Invalid)
    ));
    tx.rollback().await?;
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_rollback_publishes_nothing() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let actor = db.account().await?;
    let account = db.account().await?;
    let mut tx = db.pool.begin().await?;
    sqlx::query("UPDATE accounts SET password_hash='rolled-back' WHERE id=$1")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    staff_kick(&mut tx, account).await?;
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,'suspension','active','Public suspension',clock_timestamp()-interval '1 minute',clock_timestamp()+interval '1 day',$2)")
        .bind(account).bind(actor).execute(&mut *tx).await?;
    tx.rollback().await?;
    assert!(db.records().await?.is_empty());
    assert_eq!(db.generation(account).await?, 1);
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    // The counter increment rolled back too: the next record is position 1.
    assert_eq!(db.records().await?, vec![(1, account, 2, 2)]);
    db.cleanup().await
}
#[tokio::test]
async fn security_revocation_paging_cursor_stable_and_ordered() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.account().await?;
    let second = db.account().await?;
    let third = db.account().await?;
    sqlx::query("UPDATE accounts SET password_hash='committed' WHERE id=$1")
        .bind(first)
        .execute(&db.pool)
        .await?;
    // An uncommitted publisher holds the counter: readers see only committed
    // history and a later writer cannot overtake its position.
    let mut pending = db.pool.begin().await?;
    sqlx::query("UPDATE accounts SET password_hash='pending' WHERE id=$1")
        .bind(second)
        .execute(&mut *pending)
        .await?;
    let visible = page(&db, None, 500).await?;
    assert_eq!(
        visible["items"]
            .as_array()
            .ok_or("items must be an array")?
            .len(),
        1
    );
    assert_eq!(visible["next_cursor"], "v1:1");
    let pool = db.pool.clone();
    let mut later = tokio::spawn(async move {
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(third)
            .execute(&pool)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut later)
            .await
            .is_err()
    );
    pending.commit().await?;
    later.await??;
    let two = page(&db, Some(1), 1).await?;
    let three = page(&db, Some(2), 1).await?;
    assert_eq!(two["items"][0]["account_id"], second.to_string());
    assert_eq!(two["next_cursor"], "v1:2");
    assert_eq!(three["items"][0]["account_id"], third.to_string());
    assert_eq!(three["next_cursor"], "v1:3");
    // Replaying a cursor returns the identical page.
    assert_eq!(page(&db, Some(1), 1).await?, two);
    assert_eq!(
        page(&db, Some(3), 500).await?["next_cursor"],
        serde_json::Value::Null
    );
    db.cleanup().await
}
