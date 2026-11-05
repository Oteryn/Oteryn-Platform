use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use oteryn_identity::{
    abuse,
    accounts::Accounts,
    consents::ConsentConfig,
    password,
    register::{self, Registration},
    verify,
};
use sha1::{Digest, Sha1};
use sqlx::PgPool;
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED identity database test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("identity_test_{}", uuid::Uuid::now_v7().simple());
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
        sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
            .execute(&pool)
            .await?;
        // This fixture password has an authoritative empty breach range.
        let digest = format!("{:X}", Sha1::digest(input().password.as_bytes()));
        sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at) VALUES ($1,ARRAY[]::text[],now()+interval '1 hour')")
            .bind(&digest[..5])
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    fn app(&self) -> Router {
        oteryn_web::router(
            oteryn_web::AppState {
                db: self.pool.clone(),
            },
            "static",
        )
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
fn input() -> Registration {
    Registration {
        email: "New@Example.test".into(),
        password: "registration test password".into(),
        password_confirmation: "registration test password".into(),
        terms: Some("2026-11-05".into()),
        privacy: Some("2026-11-05".into()),
    }
}
async fn create(db: &Database) -> Result<uuid::Uuid> {
    Ok(register::register(
        &db.pool,
        input(),
        &ConsentConfig::default(),
        "en",
        Some("127.0.0.1".parse()?),
    )
    .await?
    .ok_or("account missing")?)
}
async fn token(db: &Database) -> Result<String> {
    Ok(sqlx::query_scalar("SELECT payload->>'token' FROM outbox WHERE kind='email.verify' ORDER BY created_at DESC,id DESC LIMIT 1").fetch_one(&db.pool).await?)
}
async fn credentials(app: &Router) -> Result<(String, String, String)> {
    let response = app
        .clone()
        .oneshot(Request::get("/en/register").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie missing")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("csrf missing")?
        .to_owned();
    Ok((cookie, csrf, html))
}
async fn submit(
    app: &Router,
    cookie: &str,
    csrf: &str,
    changes: &[(&str, &str)],
) -> Result<axum::response::Response> {
    let mut proof = None;
    if !csrf.is_empty() {
        let response = app
            .clone()
            .oneshot(
                Request::post("/register/abuse/challenge")
                    .header(header::COOKIE, cookie)
                    .header("x-csrf-token", csrf)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let challenge: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
        let token = challenge["token"].as_str().ok_or("work token missing")?;
        let nonce = (0..u64::MAX)
            .map(|nonce| nonce.to_string())
            .find(|nonce| abuse::valid_solution(token, nonce))
            .ok_or("work solution missing")?;
        proof = Some((token.to_owned(), nonce));
    }
    let mut fields = vec![
        ("email", "New@Example.test"),
        ("password", "registration test password"),
        ("password_confirmation", "registration test password"),
        ("terms", "2026-11-05"),
        ("privacy", "2026-11-05"),
        ("_csrf", csrf),
    ];
    if let Some((token, nonce)) = &proof {
        fields.push(("work_token", token));
        fields.push(("work_nonce", nonce));
    }
    for (key, value) in changes {
        fields.retain(|(k, _)| k != key);
        if !value.is_empty() {
            fields.push((key, value));
        }
    }
    Ok(app
        .clone()
        .oneshot(
            Request::post("/en/register")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, cookie)
                .body(Body::from(serde_urlencoded::to_string(fields)?))?,
        )
        .await?)
}
#[tokio::test]
async fn registration_creates_account_consents_outbox_and_rotates_session() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, html) = credentials(&app).await?;
    assert!(html.contains("aria-describedby=\"password-hint password-error\""));
    assert!(html.contains("/static/css/tokens.css"));
    let response = submit(&app, &cookie, &csrf, &[]).await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers()[header::LOCATION],
        "/en/register/check-inbox"
    );
    assert_ne!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .split(';')
            .next(),
        Some(cookie.as_str())
    );
    let account = Accounts {
        pool: db.pool.clone(),
    }
    .find_by_email("NEW@example.test")
    .await?
    .ok_or("account missing")?;
    assert_eq!(account.status, "pending_verification");
    assert_eq!(account.id.get_version_num(), 7);
    assert!(password::verify(
        "registration test password",
        &account.password_hash
    ));
    assert!(!account.public_status_visible && !account.public_account_association);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_consents WHERE account_id=$1")
            .bind(account.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 2);
    let row:(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER (WHERE payload->>'account_id'=$1 AND payload->>'locale'='en') FROM outbox WHERE kind='email.verify'").bind(account.id.to_string()).fetch_one(&db.pool).await?;
    assert_eq!(row, (1, 1));
    let raw = token(&db).await?;
    let digest: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM account_email_tokens")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(digest, verify::token_hash(&raw));
    let authenticated: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(account.id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(authenticated, 1);
    db.close().await
}
#[tokio::test]
async fn duplicate_email_has_same_response_without_authenticating_existing_account() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, _) = credentials(&app).await?;
    let first = submit(&app, &cookie, &csrf, &[]).await?;
    let first_cookie = first.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    let first_inbox = app
        .clone()
        .oneshot(
            Request::get("/en/register/check-inbox")
                .header(header::COOKIE, first_cookie)
                .body(Body::empty())?,
        )
        .await?;
    let (cookie, csrf, _) = credentials(&app).await?;
    let duplicate_inbox = app
        .clone()
        .oneshot(
            Request::get("/en/register/check-inbox")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        to_bytes(first_inbox.into_body(), 1024 * 1024).await?,
        to_bytes(duplicate_inbox.into_body(), 1024 * 1024).await?
    );
    let duplicate = submit(&app, &cookie, &csrf, &[("email", "new@EXAMPLE.test")]).await?;
    assert_eq!(first.status(), duplicate.status());
    assert_eq!(
        first.headers()[header::LOCATION],
        duplicate.headers()[header::LOCATION]
    );
    assert_eq!(
        to_bytes(first.into_body(), 1024).await?,
        to_bytes(duplicate.into_body(), 1024).await?
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox WHERE kind='email.account_exists'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    db.close().await
}
#[tokio::test]
async fn invalid_fields_are_rejected_without_side_effects() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, _) = credentials(&app).await?;
    for (key, value, message) in [
        ("email", "bad", "Enter a valid"),
        ("password", "short", "Use at least"),
        ("password_confirmation", "mismatch", "Passwords must match"),
        ("terms", "", "Accept the current"),
        ("privacy", "old-version", "Accept the current"),
    ] {
        let response = submit(&app, &cookie, &csrf, &[(key, value)]).await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
        assert!(html.contains(message));
        assert!(!html.contains("registration test password"));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[tokio::test]
async fn missing_csrf_is_refused() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, _, _) = credentials(&app).await?;
    assert_eq!(
        submit(&app, &cookie, "", &[("_csrf", "")]).await?.status(),
        StatusCode::FORBIDDEN
    );
    db.close().await
}
#[tokio::test]
async fn valid_verification_activates_account_and_used_token_is_generic_error() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = create(&db).await?;
    let raw = token(&db).await?;
    let app = db.app();
    let response = app
        .clone()
        .oneshot(Request::get(format!("/en/verify-email?token={raw}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    assert!(html.contains("Your e-mail has been verified"));
    assert!(!html.contains(&raw));
    let account = Accounts {
        pool: db.pool.clone(),
    }
    .find_by_id(id)
    .await?
    .ok_or("missing account")?;
    assert_eq!(account.status, "active");
    assert!(account.email_verified_at.is_some());
    let response = app
        .oneshot(Request::get(format!("/en/verify-email?token={raw}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    db.close().await
}
#[tokio::test]
async fn expired_unknown_and_malformed_tokens_have_identical_error() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    create(&db).await?;
    let raw = token(&db).await?;
    sqlx::query("UPDATE account_email_tokens SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    for raw in [raw, "f".repeat(64), "bad".into()] {
        assert!(!verify::consume(&db.pool, &raw).await?);
        let response = db
            .app()
            .oneshot(Request::get(format!("/pl/verify-email?token={raw}")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
        assert!(html.contains("Ten link jest nieprawidłowy"));
        assert!(html.contains("/pl/verify-email/resend"));
    }
    db.close().await
}
#[tokio::test]
async fn resend_invalidates_old_token_and_requires_authenticated_csrf() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, _) = credentials(&app).await?;
    let response = submit(&app, &cookie, &csrf, &[]).await?;
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    let old = token(&db).await?;
    let response = app
        .clone()
        .oneshot(
            Request::get("/en/register")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("csrf missing")?;
    let response = app
        .clone()
        .oneshot(
            Request::post("/en/verify-email/resend")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", csrf)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(!verify::consume(&db.pool, &old).await?);
    let new = token(&db).await?;
    assert_ne!(old, new);
    assert!(verify::consume(&db.pool, &new).await?);
    let (cookie, csrf, _) = credentials(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::post("/en/verify-email/resend")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", csrf)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .oneshot(
            Request::post("/en/verify-email/resend")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await
}
#[tokio::test]
async fn concurrent_verification_consumes_token_once() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    create(&db).await?;
    let raw = token(&db).await?;
    let (a, b) = tokio::join!(
        verify::consume(&db.pool, &raw),
        verify::consume(&db.pool, &raw)
    );
    assert_ne!(a?, b?);
    db.close().await
}
#[test]
fn password_hash_verify_and_rehash() -> Result {
    let hash = password::hash("registration test password")?;
    assert!(password::verify("registration test password", &hash));
    assert!(!password::verify("wrong password", &hash));
    assert!(!password::needs_rehash(&hash));
    assert!(password::needs_rehash("invalid"));
    let older = password::PasswordConfig {
        memory_kib: 8192,
        iterations: 1,
        parallelism: 1,
    }
    .hash("registration test password")?;
    assert!(password::needs_rehash(&older));
    assert!(password::verify("registration test password", &older));
    Ok(())
}
#[test]
fn laravel_php_argon2id_hash_is_accepted() {
    // Generated locally by PHP password_hash using PASSWORD_ARGON2ID and its default options.
    let hash = "$argon2id$v=19$m=65536,t=4,p=1$bnZ6bi5yejFrOFdyUDdRbQ$d6+0Uge+PSI4Z07oKA/Qawyn6CK6Qjtut4xsGIvItFE";
    assert!(password::verify("registration test password", hash));
    assert!(!password::verify("incorrect password", hash));
    assert!(password::needs_rehash(hash));
}
#[test]
fn password_rules_count_characters_and_bytes() {
    assert!(!password::valid("123456789"));
    assert!(password::valid("1234567890"));
    assert!(password::valid(&"a".repeat(256)));
    assert!(!password::valid(&"a".repeat(257)));
    assert!(password::valid(&"ą".repeat(10)));
    assert!(!password::valid(&"ą".repeat(129)));
}

#[tokio::test]
async fn verification_query_is_removed_before_request_tracing() -> Result {
    let app = Router::new()
        .route(
            "/en/verify-email",
            axum::routing::get(|request: axum::extract::Request| async move {
                request.uri().to_string()
            }),
        )
        .layer(axum::middleware::from_fn(oteryn_web::verify::protect_token));
    let response = app
        .oneshot(Request::get("/en/verify-email?token=bearer-material").body(Body::empty())?)
        .await?;
    assert_eq!(
        to_bytes(response.into_body(), 1024).await?.as_ref(),
        b"/en/verify-email"
    );
    Ok(())
}
