use super::*;
use crate::verify::token_hash;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const CURRENT: &str = "current test password";
const NEW: &str = "replacement test password";
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED recovery-key PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("recovery_key_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
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
        // The fixture replacement password has a cached complete, clean range.
        crate::breach::tests::allow(&pool, NEW).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    fn keys(&self) -> RecoveryKeys {
        let mut keys = RecoveryKeys::new(self.pool.clone());
        keys.enabled = Enabled(true);
        keys
    }
    async fn account(&self) -> Result<AccountId> {
        let id = self.pending_account().await?;
        sqlx::query("UPDATE accounts SET status='active',email_verified_at=now() WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }
    async fn pending_account(&self) -> Result<AccountId> {
        let mut conn = self.pool.acquire().await?;
        Ok(crate::accounts::Accounts::create(
            &mut conn,
            "owner@example.test",
            &password::hash(CURRENT)?,
            "en",
        )
        .await?
        .ok_or("missing account")?
        .id)
    }
    async fn key(&self, id: AccountId) -> Result<String> {
        let key = self.keys().generate(id, CURRENT).await?;
        self.keys().confirm(id, CURRENT, &key).await?;
        Ok(key)
    }
    fn app(&self) -> Router {
        oteryn_web::router(
            oteryn_web::AppState {
                db: self.pool.clone(),
            },
            "static",
        )
        .layer(axum::Extension(FeatureFlags::from([FEATURE.to_owned()])))
        .layer(axum::Extension(axum::extract::ConnectInfo(
            std::net::SocketAddr::from(([127, 0, 0, 1], 4321)),
        )))
    }
    async fn session(&self, id: Option<AccountId>) -> Result<String> {
        let raw = "a".repeat(64);
        sqlx::query("INSERT INTO web_sessions (id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,'test',now()+interval '1 hour',now()+interval '2 hours')").bind(token_hash(&raw)).bind(id).bind("b".repeat(64)).execute(&self.pool).await?;
        Ok(format!("__Host-oteryn_session={raw}"))
    }
    /// Budgets are fixed windows on the database clock: wait for a fresh one
    /// when less than a minute is left, so filling a budget cannot straddle a rollover.
    async fn fresh_window(&self, policy: oteryn_web::ratelimit::NamedPolicy) -> Result {
        loop {
            let left: f64 = sqlx::query_scalar("SELECT ($1::numeric - mod(extract(epoch FROM statement_timestamp()), $1::numeric))::float8")
                .bind(policy.window_seconds())
                .fetch_one(&self.pool)
                .await?;
            if left > 60.0 {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_secs_f64(left)).await;
        }
    }
    async fn close(self) -> Result {
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
#[tokio::test]
async fn normalization_and_input() -> Result {
    assert_eq!(normalize(" oteryn-ab cd_12! "), "OTERYNABCD12");
    assert!(matches!(validate_key(""), Err(Error::InvalidInput)));
    assert!(matches!(
        validate_key(&"x".repeat(161)),
        Err(Error::InvalidInput)
    ));
    assert!(matches!(validate_key("---"), Err(Error::Rejected)));
    assert!(!matches_digest(None, "anything").await?);
    Ok(())
}
#[tokio::test]
async fn disabled_by_default() -> Result {
    let keys = RecoveryKeys::new(oteryn_db::lazy_pool(
        "postgres://portal@127.0.0.1:1/unreachable",
    )?);
    assert!(matches!(
        keys.generate(uuid::Uuid::now_v7(), CURRENT).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        keys.recover("a@b.test", "key", NEW, NEW).await,
        Err(Error::Disabled)
    ));
    let app = oteryn_web::router(oteryn_web::AppState { db: keys.pool }, "static");
    let response = app
        .oneshot(Request::get("/en/recovery-key").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}
#[tokio::test]
async fn postgres_breached_or_unchecked_password_keeps_the_key() -> Result {
    use crate::breach::tests::{Mock, Reply, range};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("DELETE FROM registration_password_ranges")
        .execute(&db.pool)
        .await?;
    let id = db.account().await?;
    let key = db.key(id).await?;
    db.session(Some(id)).await?;
    let hit = Mock::start(Reply::Ok(range(NEW, true))).await?;
    let mut keys = db.keys();
    keys.breach = hit.checker(&db.pool);
    assert!(matches!(
        keys.recover("owner@example.test", &key, NEW, NEW).await,
        Err(Error::Breached)
    ));
    sqlx::query("DELETE FROM registration_password_ranges")
        .execute(&db.pool)
        .await?;
    let down = Mock::start(Reply::Status(500)).await?;
    keys.breach = down.checker(&db.pool);
    assert!(matches!(
        keys.recover("owner@example.test", &key, NEW, NEW).await,
        Err(Error::Unavailable)
    ));
    let (hash, sessions, used): (String, i64, bool) = sqlx::query_as("SELECT a.password_hash,(SELECT count(*) FROM web_sessions WHERE account_id=a.id),(SELECT used_at IS NOT NULL FROM account_recovery_keys WHERE account_id=a.id) FROM accounts a WHERE a.id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(password::verify(CURRENT, &hash));
    assert_eq!((sessions, used), (1, false));
    let clean = Mock::start(Reply::Ok(range(NEW, false))).await?;
    keys.breach = clean.checker(&db.pool);
    assert_eq!(
        keys.recover("owner@example.test", &key, NEW, NEW).await?,
        id
    );
    assert_eq!((hit.hits(), down.hits(), clean.hits()), (1, 1, 1));
    db.close().await
}
#[tokio::test]
async fn postgres_success_consumes_key_and_revokes_sessions() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    assert_eq!(key.len(), 51);
    db.session(Some(id)).await?;
    sqlx::query("INSERT INTO oauth_token_families(id,account_id,client_id,credential_hash) VALUES(uuidv7(),$1,'test',decode(repeat('00',32),'hex'))").bind(id).execute(&db.pool).await?;
    sqlx::query("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES(uuidv7(),$1,uuidv7(),'test',1,1,9999999999,9999999999)").bind(id).execute(&db.pool).await?;
    let handle = crate::native_recovery_test_support::handle(&db.pool, id).await?;
    assert_eq!(
        db.keys()
            .recover(" OWNER@EXAMPLE.TEST ", &key.to_lowercase(), NEW, NEW)
            .await?,
        id
    );
    assert!(crate::native_recovery_test_support::revoked(&db.pool, &handle).await?);
    let account = crate::accounts::Accounts {
        pool: db.pool.clone(),
    }
    .find_by_id(id)
    .await?
    .ok_or("account missing")?;
    assert!(password::verify(NEW, &account.password_hash));
    assert!(!password::verify(CURRENT, &account.password_hash));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    for statement in [
        "SELECT count(*) FROM oauth_token_families WHERE account_id=$1 AND revoked_at IS NULL",
        "SELECT count(*) FROM device_sessions WHERE account_id=$1 AND revoked_at IS NULL",
    ] {
        let live: i64 = sqlx::query_scalar(statement)
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
        assert_eq!(live, 0);
    }
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    // Verification (account setup) and recovery each advance it once.
    assert_eq!(generation, 3);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_recovery_key_events WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 3);
    assert!(matches!(
        db.keys()
            .recover("owner@example.test", &key, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    db.close().await
}
#[tokio::test]
async fn postgres_pending_replaced_revoked_and_bad_password_refused() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let handle = crate::native_recovery_test_support::handle(&db.pool, id).await?;
    assert!(matches!(
        db.keys().generate(id, "wrong").await,
        Err(Error::Rejected)
    ));
    let old = db.keys().generate(id, CURRENT).await?;
    assert!(matches!(
        db.keys()
            .recover("owner@example.test", &old, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys().confirm(id, CURRENT, "wrong").await,
        Err(Error::Rejected)
    ));
    db.keys().confirm(id, CURRENT, &old).await?;
    let key = db.key(id).await?;
    assert_ne!(old, key);
    assert!(matches!(
        db.keys()
            .recover("owner@example.test", &old, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys()
            .recover("missing@example.test", &key, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys().revoke(id, "wrong").await,
        Err(Error::Rejected)
    ));
    db.keys().revoke(id, CURRENT).await?;
    assert!(matches!(
        db.keys()
            .recover("owner@example.test", &key, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys().revoke(id, CURRENT).await,
        Err(Error::Rejected)
    ));
    // Refused recoveries change no credential and keep the recovery handles.
    assert!(!crate::native_recovery_test_support::revoked(&db.pool, &handle).await?);
    db.close().await
}
#[tokio::test]
async fn postgres_bad_input_and_suspended_account_refused() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    for (email, pass, confirmation) in [
        ("invalid", NEW, NEW),
        ("owner@example.test", "short", "short"),
        ("owner@example.test", NEW, "different"),
    ] {
        assert!(matches!(
            db.keys().recover(email, &key, pass, confirmation).await,
            Err(Error::InvalidInput)
        ));
    }
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        db.keys()
            .recover("owner@example.test", &key, NEW, NEW)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys().generate(id, CURRENT).await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        db.keys().revoke(id, CURRENT).await,
        Err(Error::Rejected)
    ));
    db.close().await
}
#[tokio::test]
async fn postgres_concurrent_consumption_only_one_succeeds() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    let service = db.keys();
    let (a, b) = tokio::join!(
        service.recover("owner@example.test", &key, NEW, NEW),
        service.recover("owner@example.test", &key, NEW, NEW)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    db.close().await
}
fn failed_reset(conn: &mut PgConnection, id: AccountId) -> ResetFuture<'_> {
    Box::pin(async move {
        reset_base(conn, id).await?;
        Err(Error::Unavailable)
    })
}
#[tokio::test]
async fn postgres_reset_failure_rolls_back_everything() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    db.session(Some(id)).await?;
    let mut service = db.keys();
    service.reset = failed_reset;
    assert!(matches!(
        service.recover("owner@example.test", &key, NEW, NEW).await,
        Err(Error::Unavailable)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    // Only the verification at account setup advanced it.
    assert_eq!(generation, 2);
    let account = crate::accounts::Accounts {
        pool: db.pool.clone(),
    }
    .find_by_id(id)
    .await?
    .ok_or("missing account")?;
    assert!(password::verify(CURRENT, &account.password_hash));
    assert!(
        db.keys()
            .recover("owner@example.test", &key, NEW, NEW)
            .await
            .is_ok()
    );
    db.close().await
}
#[tokio::test]
async fn postgres_unintegrated_security_schema_denies() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    let mut keys = db.keys();
    keys.reset = deny_unintegrated_security;
    assert!(matches!(
        keys.recover("owner@example.test", &key, NEW, NEW).await,
        Err(Error::Unavailable)
    ));
    db.close().await
}
fn post(path: &str, cookie: &str, form: &str) -> Result<Request<Body>> {
    Ok(Request::post(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(form.to_owned()))?)
}
#[tokio::test]
async fn postgres_browser_csrf_authorization_locales_and_rate_limit() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let cookie = db.session(Some(id)).await?;
    for (locale, title) in [
        ("en", "Account recovery key"),
        ("pl", "Klucz odzyskiwania konta"),
    ] {
        let response = db
            .app()
            .oneshot(
                Request::get(format!("/{locale}/recovery-key"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(html.contains(title));
        assert!(html.contains("for=\"key\""));
        assert!(html.contains("name=\"viewport\""));
        assert!(html.contains("op-shell"));
        assert!(html.contains("data-theme=\"dark\""));
    }
    let response = db
        .app()
        .oneshot(post(
            "/en/account/recovery-key",
            &cookie,
            "current_password=wrong",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let form = format!("_csrf={}&current_password=wrong", "b".repeat(64));
    db.fresh_window(oteryn_web::ratelimit::NamedPolicy::IdentityRecoveryKeyManage)
        .await?;
    for _ in 0..5 {
        let response = db
            .app()
            .oneshot(post("/en/account/recovery-key", &cookie, &form)?)
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
    }
    let response = db
        .app()
        .oneshot(post("/en/account/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    sqlx::query("UPDATE web_sessions SET account_id=NULL")
        .execute(&db.pool)
        .await?;
    let response = db
        .app()
        .oneshot(post("/en/account/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await
}
#[tokio::test]
async fn postgres_browser_recovery_success_bad_input_and_no_secret_echo() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let key = db.key(id).await?;
    let cookie = db.session(Some(id)).await?;
    let form = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("email", "owner@example.test".into()),
        ("recovery_key", key),
        ("password", NEW.into()),
        ("password_confirmation", "different".into()),
    ])?;
    let response = db
        .app()
        .oneshot(post("/en/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(!html.contains(NEW));
    assert!(!html.contains("owner@example.test"));
    let form = form.replace("different", "replacement+test+password");
    let response = db
        .app()
        .oneshot(post("/en/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    db.close().await
}

#[tokio::test]
async fn recovery_preserves_other_account_session() -> Result {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = db.account().await?;
    let key = db.key(id).await?;
    let other: AccountId = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES('other@example.test','unused','en','active',now()) RETURNING id").fetch_one(&db.pool).await?;
    let cookie = db.session(Some(other)).await?;
    let form = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("email", "owner@example.test".into()),
        ("recovery_key", key),
        ("password", NEW.into()),
        ("password_confirmation", NEW.into()),
    ])?;
    let response = db
        .app()
        .oneshot(post("/en/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    let alive: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT FROM web_sessions WHERE account_id=$1)")
            .bind(other)
            .fetch_one(&db.pool)
            .await?;
    assert!(alive, "recovery must preserve the unrelated session");
    db.close().await
}

#[tokio::test]
async fn postgres_browser_generate_confirm_and_revoke() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.account().await?;
    let cookie = db.session(Some(id)).await?;
    let form = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("current_password", CURRENT.to_owned()),
    ])?;
    let response = db
        .app()
        .oneshot(post("/en/account/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    let raw = html
        .split("<p class=\"op-code\">")
        .nth(1)
        .and_then(|s| s.split("</p>").next())
        .ok_or("generated key missing")?;
    assert_eq!(raw.len(), 51);
    let stored: String =
        sqlx::query_scalar("SELECT key_hash FROM account_recovery_keys WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert!(stored.starts_with("$argon2id$"));
    assert!(password::verify(&normalize(raw), &stored));
    let confirm = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("current_password", CURRENT.to_owned()),
        ("recovery_key", raw.to_owned()),
    ])?;
    let response = db
        .app()
        .oneshot(post("/en/account/recovery-key/confirm", &cookie, &confirm)?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(!html.contains(raw));
    let response = db
        .app()
        .oneshot(post("/en/account/recovery-key/revoke", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response.headers()[header::LOCATION],
        "/en/account/recovery-key"
    );
    assert!(matches!(
        db.keys().recover("owner@example.test", raw, NEW, NEW).await,
        Err(Error::Rejected)
    ));
    db.close().await
}

#[tokio::test]
async fn audit02_varied_email_recovery_exhausts_aggregate_source_budget() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let cookie = db.session(None).await?;
    let app = db.app();
    db.fresh_window(oteryn_web::ratelimit::NamedPolicy::IdentityRecoveryKeyUse)
        .await?;
    for n in 0..7 {
        let form = serde_urlencoded::to_string([
            ("_csrf", "b".repeat(64)),
            ("email", format!("varied-{n}@example.test")),
            ("recovery_key", "unknown-key".into()),
            ("password", NEW.into()),
            ("password_confirmation", NEW.into()),
        ])?;
        let response = app
            .clone()
            .oneshot(post("/en/recovery-key", &cookie, &form)?)
            .await?;
        assert_eq!(
            response.status(),
            if n < 5 {
                StatusCode::FOUND
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    db.close().await
}

#[tokio::test(flavor = "current_thread")]
async fn recovery_argon2_keeps_single_worker_heartbeat_alive() -> Result {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = ticks.clone();
    let heartbeat = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            observed.fetch_add(1, Ordering::SeqCst);
        }
    });
    let raw = "OTERYN-12345678-12345678-12345678-12345678-12345678";
    let normalized = normalize(raw);
    let digest = expensive(move || password::hash(&normalized)).await??;
    let before = ticks.load(Ordering::SeqCst);
    assert!(before > 2, "hashing blocked executor heartbeat");
    assert!(matches_digest(Some(digest.clone()), raw).await?);
    assert!(
        ticks.load(Ordering::SeqCst) > before + 2,
        "verification blocked executor heartbeat"
    );
    let (a, b, c) = tokio::join!(
        matches_digest(Some(digest.clone()), raw),
        matches_digest(Some(digest.clone()), raw),
        matches_digest(Some(digest), raw)
    );
    assert!(a? && b? && c?);
    heartbeat.abort();
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pg_recovery_requests_keep_single_worker_timers_responsive() -> Result {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let hash = expensive(|| password::hash(CURRENT)).await??;
    let mut ids = Vec::new();
    for index in 0..3 {
        let mut conn = db.pool.acquire().await?;
        let account = crate::accounts::Accounts::create(
            &mut conn,
            &format!("heartbeat-{index}@example.test"),
            &hash,
            "en",
        )
        .await?
        .ok_or("account missing")?;
        sqlx::query("UPDATE accounts SET status='active',email_verified_at=now() WHERE id=$1")
            .bind(account.id)
            .execute(&db.pool)
            .await?;
        ids.push(account.id);
    }
    let max_gap = Arc::new(AtomicU64::new(0));
    let observed = max_gap.clone();
    let heartbeat = tokio::spawn(async move {
        let mut last = std::time::Instant::now();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            observed.fetch_max(last.elapsed().as_millis() as u64, Ordering::SeqCst);
            last = std::time::Instant::now();
        }
    });
    tokio::task::yield_now().await;
    let keys = db.keys();
    let (a, b, c) = tokio::join!(
        keys.generate(ids[0], CURRENT),
        keys.generate(ids[1], CURRENT),
        keys.generate(ids[2], CURRENT)
    );
    let raw = [a?, b?, c?];
    let (a, b, c) = tokio::join!(
        keys.confirm(ids[0], CURRENT, &raw[0]),
        keys.confirm(ids[1], CURRENT, &raw[1]),
        keys.confirm(ids[2], CURRENT, &raw[2])
    );
    a?;
    b?;
    c?;
    let (a, b, c) = tokio::join!(
        keys.recover("heartbeat-0@example.test", &raw[0], NEW, NEW),
        keys.recover("heartbeat-1@example.test", &raw[1], NEW, NEW),
        keys.recover("heartbeat-2@example.test", &raw[2], NEW, NEW)
    );
    assert_eq!([a?, b?, c?], [ids[0], ids[1], ids[2]]);
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    heartbeat.abort();
    assert!(
        max_gap.load(Ordering::SeqCst) < 100,
        "Argon2 stalled single-worker timers: {}ms",
        max_gap.load(Ordering::SeqCst)
    );
    db.close().await?;
    eprintln!(
        "PG pg_recovery_requests_keep_single_worker_timers_responsive duration={:.3}s max_heartbeat_gap={}ms",
        started.elapsed().as_secs_f64(),
        max_gap.load(Ordering::SeqCst)
    );
    Ok(())
}

#[tokio::test]
async fn audit3_revoked_session_cannot_confirm_after_account_lock() -> Result {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL is required for this regression")?;
    let id = db.account().await?;
    let key = db.keys().generate(id, CURRENT).await?;
    let cookie = db.session(Some(id)).await?;
    let form = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("current_password", CURRENT.to_owned()),
        ("recovery_key", key),
    ])?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    let app = db.app();
    let request = post("/en/account/recovery-key/confirm", &cookie, &form)?;
    let dispatched = tokio::spawn(async move { app.oneshot(request).await });
    // Observe the dispatched SQL actually waiting on our account lock.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid WHERE a.datname=current_database() AND a.wait_event_type='Lock' AND l.relation='accounts'::regclass AND a.query LIKE 'SELECT * FROM accounts WHERE id=%FOR UPDATE%')").fetch_one(&db.pool).await?;
            if waiting { break Ok::<(), sqlx::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await??;
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&mut *lock)
        .await?;
    lock.commit().await?;
    let response = dispatched.await??;
    assert_ne!(response.status(), StatusCode::OK);
    let confirmed: bool = sqlx::query_scalar(
        "SELECT confirmed_at IS NOT NULL FROM account_recovery_keys WHERE account_id=$1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    assert!(
        !confirmed,
        "revoked browser session confirmed a recovery key"
    );
    db.close().await?;
    eprintln!(
        "PG audit3_revoked_session_cannot_confirm_after_account_lock: {:?}",
        started.elapsed()
    );
    Ok(())
}

/// Review 5: a pending account may belong to whoever registered the address,
/// so it cannot hold a recovery key, and a key from before verification
/// cannot recover the verified account.
#[tokio::test]
async fn r5_recovery_key_requires_a_verified_owner() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.pending_account().await?;
    assert!(matches!(
        db.keys().generate(id, CURRENT).await,
        Err(Error::Rejected)
    ));
    // Verification (also bumped again by every confirmed email change)
    // unlocks generation without invalidating keys made since.
    sqlx::query("UPDATE accounts SET status='active',email_verified_at=now() WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    let key = db.key(id).await?;
    sqlx::query("UPDATE accounts SET email_verified_at=now()+interval '1 minute' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await?;
    assert!(
        db.keys()
            .recover("owner@example.test", &key, NEW, NEW)
            .await
            .is_ok()
    );
    db.close().await
}

#[tokio::test]
async fn r10_recovery_charges_shared_source_once() -> Result {
    use oteryn_web::ratelimit::NamedPolicy;
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let cookie = db.session(None).await?;
    let form = serde_urlencoded::to_string([
        ("_csrf", "b".repeat(64)),
        ("email", "someone@example.test".into()),
        ("recovery_key", "unknown-key".into()),
        ("password", NEW.into()),
        ("password_confirmation", NEW.into()),
    ])?;
    let response = db
        .app()
        .oneshot(post("/en/recovery-key", &cookie, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let count: i64 =
        sqlx::query_scalar("SELECT sum(count)::bigint FROM web_rate_limits WHERE limiter_name=$1")
            .bind(NamedPolicy::IdentityAnonymousSource.name())
            .fetch_one(&db.pool)
            .await?;
    db.close().await?;
    assert_eq!(count, 1);
    Ok(())
}
