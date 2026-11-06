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
#[test]
fn normalization_and_input() {
    assert_eq!(normalize(" oteryn-ab cd_12! "), "OTERYNABCD12");
    assert!(matches!(validate_key(""), Err(Error::InvalidInput)));
    assert!(matches!(
        validate_key(&"x".repeat(161)),
        Err(Error::InvalidInput)
    ));
    assert!(matches!(validate_key("---"), Err(Error::Rejected)));
    assert!(!matches_digest(None, "anything"));
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
    assert_eq!(
        db.keys()
            .recover(" OWNER@EXAMPLE.TEST ", &key.to_lowercase(), NEW, NEW)
            .await?,
        id
    );
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
    assert_eq!(generation, 2);
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
    assert_eq!(generation, 1);
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
