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
        let db = Self::empty_legal().await?;
        if let Some(db) = &db {
            oteryn_db::test_support::seed_final_signup_editions(
                &db.pool,
                oteryn_identity::consents::DEFAULT_LEGAL_VERSION,
            )
            .await?;
        }
        Ok(db)
    }
    async fn empty_legal() -> Result<Option<Self>> {
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
        sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES ($1,ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id())")
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
        .layer(axum::Extension(axum::extract::ConnectInfo(
            std::net::SocketAddr::from(([192, 0, 2, 60], 1234)),
        )))
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
        terms: Some(oteryn_identity::consents::DEFAULT_LEGAL_VERSION.into()),
        privacy: Some(oteryn_identity::consents::DEFAULT_LEGAL_VERSION.into()),
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
fn session_cookie(response: &axum::response::Response) -> Result<&str> {
    response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or_else(|| "missing cookie".into())
}
async fn session_row(
    db: &Database,
    cookie: &str,
) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>, String)>> {
    let (_, value) = cookie.split_once('=').ok_or("invalid cookie")?;
    Ok(sqlx::query_as(
        "SELECT session_id, account_id, csrf_token FROM web_sessions WHERE id_hash=$1",
    )
    .bind(sha2::Sha256::digest(value.as_bytes()).to_vec())
    .fetch_optional(&db.pool)
    .await?)
}
async fn assert_anonymous_rotation(
    db: &Database,
    old: &str,
    old_csrf: &str,
    response: &axum::response::Response,
) -> Result {
    let new = session_cookie(response)?;
    assert_ne!(new, old);
    assert!(session_row(db, old).await?.is_none());
    let (_, account, csrf) = session_row(db, new)
        .await?
        .ok_or("missing rotated session")?;
    assert!(account.is_none());
    assert_ne!(csrf, old_csrf);
    Ok(())
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
        ("terms", oteryn_identity::consents::DEFAULT_LEGAL_VERSION),
        ("privacy", oteryn_identity::consents::DEFAULT_LEGAL_VERSION),
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
    assert_anonymous_rotation(&db, &cookie, &csrf, &response).await?;
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
    assert_eq!(authenticated, 0);
    db.close().await
}
/// Whether a response sets a session cookie, whether it replaces the request's
/// session, and the cookie attributes.
fn issuance(
    response: &axum::response::Response,
    request_cookie: &str,
) -> Result<Vec<(bool, String)>> {
    let mut issued = Vec::new();
    for value in response.headers().get_all(header::SET_COOKIE) {
        let value = value.to_str()?;
        let (pair, attributes) = value.split_once(';').unwrap_or((value, ""));
        issued.push((pair != request_cookie, attributes.to_owned()));
    }
    Ok(issued)
}
#[tokio::test]
async fn duplicate_email_has_same_response_without_authenticating_existing_account() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (first_request_cookie, csrf, _) = credentials(&app).await?;
    let first = submit(&app, &first_request_cookie, &csrf, &[]).await?;
    let first_issuance = issuance(&first, &first_request_cookie)?;
    assert_anonymous_rotation(&db, &first_request_cookie, &csrf, &first).await?;
    let (cookie, csrf, _) = credentials(&app).await?;
    let duplicate = submit(&app, &cookie, &csrf, &[("email", "new@EXAMPLE.test")]).await?;
    assert_anonymous_rotation(&db, &cookie, &csrf, &duplicate).await?;
    let mut inbox_bodies = Vec::new();
    for response in [&first, &duplicate] {
        let issued = session_cookie(response)?;
        for path in ["/en/register", "/en/register/check-inbox"] {
            let page = app
                .clone()
                .oneshot(
                    Request::get(path)
                        .header(header::COOKIE, issued)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(page.status(), StatusCode::OK);
            if path.ends_with("check-inbox") {
                inbox_bodies.push(to_bytes(page.into_body(), 1024 * 1024).await?);
            }
        }
    }
    assert_eq!(inbox_bodies[0], inbox_bodies[1]);
    // Session-cookie issuance must not reveal whether the address exists.
    assert_eq!(first_issuance, issuance(&duplicate, &cookie)?);
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
    assert_eq!(count, 0);
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
    let preview = app
        .clone()
        .oneshot(Request::get(format!("/en/verify-email?token={raw}")).body(Body::empty())?)
        .await?;
    let cookie = session_cookie(&preview)?.to_owned();
    let html = String::from_utf8(to_bytes(preview.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains(&format!("action=\"/en/verify-email?token={raw}\"")));
    let csrf = html
        .split_once("name=\"_csrf\" value=\"")
        .and_then(|(_, s)| s.split_once('"'))
        .ok_or("missing confirmation CSRF")?
        .0;
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/en/verify-email?token={raw}"))
                .header(header::COOKIE, cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))?,
        )
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
        assert!(verify::consume(&db.pool, &raw).await?.is_none());
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
async fn resend_replaces_expired_token_and_requires_anonymous_csrf() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let id = create(&db).await?;
    let old = token(&db).await?;
    assert!(!verify::resend(&db.pool, id).await?);
    sqlx::query("UPDATE account_email_tokens SET created_at=clock_timestamp()-interval '61 seconds',updated_at=clock_timestamp()-interval '61 seconds',expires_at=clock_timestamp()-interval '1 second' WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    let response = app
        .clone()
        .oneshot(Request::get("/en/verify-email/resend").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = session_cookie(&response)?.to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    assert!(html.contains("name=\"email\""));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("csrf missing")?;
    let (_, foreign_csrf, _) = credentials(&app).await?;
    for invalid in [None, Some(foreign_csrf.as_str())] {
        let mut fields = vec![("email", "NEW@example.test")];
        if let Some(invalid) = invalid {
            fields.push(("_csrf", invalid));
        }
        let response = app
            .clone()
            .oneshot(
                Request::post("/en/verify-email/resend")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(serde_urlencoded::to_string(fields)?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(token(&db).await?, old);
    let response = app
        .oneshot(
            Request::post("/en/verify-email/resend")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(serde_urlencoded::to_string([
                    ("email", "NEW@example.test"),
                    ("_csrf", csrf),
                ])?))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(verify::consume(&db.pool, &old).await?.is_none());
    let new = token(&db).await?;
    assert_ne!(old, new);
    assert_eq!(verify::consume(&db.pool, &new).await?, Some(id));
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

#[tokio::test]
async fn verification_keeps_pending_referral_code_in_fresh_session() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, _) = credentials(&app).await?;
    let registered = submit(&app, &cookie, &csrf, &[]).await?;
    assert_eq!(registered.status(), StatusCode::SEE_OTHER);
    let cookie = session_cookie(&registered)?;
    let (registration_id, _, registration_csrf) = session_row(&db, cookie)
        .await?
        .ok_or("missing registration session")?;
    sqlx::query("UPDATE web_sessions SET data=data||jsonb_build_object('referral.code','invite-code') WHERE session_id=$1")
        .bind(registration_id)
        .execute(&db.pool)
        .await?;
    let raw = token(&db).await?;
    let response = app
        .oneshot(
            Request::post(format!("/en/verify-email?token={raw}"))
                .header("x-csrf-token", &registration_csrf)
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_anonymous_rotation(&db, cookie, &registration_csrf, &response).await?;
    let carried: Vec<Option<String>> =
        sqlx::query_scalar("SELECT data->>'referral.code' FROM web_sessions WHERE session_id<>$1")
            .bind(registration_id)
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(carried, [Some("invite-code".to_owned())]);
    db.close().await
}
#[tokio::test]
async fn verification_from_registration_session_rotates_revoked_session() -> Result {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = db.app();
    let (cookie, csrf, _) = credentials(&app).await?;
    let registered = submit(&app, &cookie, &csrf, &[]).await?;
    assert_eq!(registered.status(), StatusCode::SEE_OTHER);
    assert_anonymous_rotation(&db, &cookie, &csrf, &registered).await?;
    let cookie = session_cookie(&registered)?;
    let (registration_id, _, registration_csrf) = session_row(&db, cookie)
        .await?
        .ok_or("missing registration session")?;
    let raw = token(&db).await?;
    let response = app
        .oneshot(
            Request::post(format!("/en/verify-email?token={raw}"))
                .header("x-csrf-token", &registration_csrf)
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_anonymous_rotation(&db, cookie, &registration_csrf, &response).await?;
    let revoked: bool =
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM web_sessions WHERE session_id=$1)")
            .bind(registration_id)
            .fetch_one(&db.pool)
            .await?;
    assert!(revoked);
    let authenticated: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(authenticated, 0);
    let body = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    assert!(body.contains("Your e-mail has been verified"));
    assert!(!body.contains(&raw));
    db.close().await
}

#[tokio::test]
async fn verification_preview_preserves_pending_account() -> Result {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = create(&db).await?;
    let raw = token(&db).await?;
    let app = db.app();
    for method in ["HEAD", "GET"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!("/en/verify-email?token={raw}"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM accounts a JOIN account_email_tokens t ON t.account_id=a.id WHERE a.id=$1 AND a.status='pending_verification' AND t.used_at IS NULL)").bind(id).fetch_one(&db.pool).await?;
        assert!(
            pending,
            "preview must preserve the unused token and pending account"
        );
    }
    let response = app
        .oneshot(Request::post(format!("/en/verify-email?token={raw}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await
}

#[tokio::test]
async fn resend_queues_existing_unexpired_link() -> Result {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let id = create(&db).await?;
    let original = token(&db).await?;
    sqlx::query("UPDATE account_email_tokens SET created_at=clock_timestamp()-interval '61 seconds',updated_at=clock_timestamp()-interval '61 seconds' WHERE account_id=$1").bind(id).execute(&db.pool).await?;
    assert!(verify::resend(&db.pool, id).await?);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox WHERE kind='email.verify' AND payload->>'token'=$1",
    )
    .bind(original)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 2, "both deliveries must carry the existing link");
    assert!(!verify::resend(&db.pool, id).await?);
    assert_eq!(
        verify::consume(&db.pool, &token(&db).await?).await?,
        Some(id)
    );
    db.close().await
}

#[tokio::test]
async fn registration_requires_current_localized_legal_versions_and_refuses_live_placeholders()
-> Result {
    use oteryn_identity::consents::PLACEHOLDER_LEGAL_VERSION;

    let Some(db) = Database::empty_legal().await? else {
        return Ok(());
    };
    assert_eq!(
        db.app()
            .oneshot(Request::get("/en/register").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        register::register(&db.pool, input(), &ConsentConfig::default(), "en", None)
            .await
            .is_err()
    );

    let version = PLACEHOLDER_LEGAL_VERSION;
    let placeholder_fields = [("terms", version), ("privacy", version)];
    let mut placeholder_input = input();
    placeholder_input.terms = Some(version.into());
    placeholder_input.privacy = Some(version.into());
    let config = |terms: &str, allow_placeholders| {
        ConsentConfig::published(terms.into(), version.into(), allow_placeholders)
    };
    let app = db.app().layer(axum::Extension(config(version, true)));
    let load = |app: Router, locale| async move {
        app.oneshot(Request::get(format!("/{locale}/register")).body(Body::empty())?)
            .await
            .map_err(Into::<Box<dyn std::error::Error>>::into)
    };
    assert_eq!(
        load(app.clone(), "en").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    for slug in ["terms-of-service", "privacy-policy"] {
        let page: uuid::Uuid =
            sqlx::query_scalar("INSERT INTO pages(id,slug) VALUES(uuidv7(),$1) RETURNING id")
                .bind(slug)
                .fetch_one(&db.pool)
                .await?;
        let source: i64 = sqlx::query_scalar(
            "INSERT INTO editorial_pages(page_id,locale,title,body,legal_version,legal_effective_date,published_at)
             VALUES($1,'en',$2,'Draft policy',$3,'2026-11-22',now()) RETURNING revision",
        )
        .bind(page)
        .bind(slug)
        .bind(version)
        .fetch_one(&db.pool)
        .await?;
        sqlx::query(
            "INSERT INTO editorial_pages(page_id,locale,title,body,legal_version,legal_effective_date,published_at,source_revision)
             VALUES($1,'pl',$2,'Draft policy',$3,'2026-11-22',now(),$4)",
        )
        .bind(page)
        .bind(slug)
        .bind(version)
        .bind(source)
        .execute(&db.pool)
        .await?;
    }
    assert_eq!(load(app.clone(), "pl").await?.status(), StatusCode::OK);
    let (cookie, csrf, html) = credentials(&app).await?;
    for name in ["terms", "privacy"] {
        assert!(html.contains(&format!("name=\"{name}\" value=\"{version}\"")));
    }
    let mismatched = db
        .app()
        .layer(axum::Extension(config("different-version", true)));
    assert_eq!(
        load(mismatched, "en").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let live = db.app().layer(axum::Extension(config(version, false)));
    assert_eq!(
        load(live.clone(), "en").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        submit(&live, &cookie, &csrf, &placeholder_fields)
            .await?
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        register::register(
            &db.pool,
            placeholder_input,
            &config(version, false),
            "en",
            None
        )
        .await
        .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_consents")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);

    sqlx::query(
        "UPDATE editorial_pages SET source_revision=0
         WHERE locale='pl' AND page_id=(SELECT id FROM pages WHERE slug='terms-of-service')",
    )
    .execute(&db.pool)
    .await?;
    assert_eq!(
        load(app.clone(), "pl").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        submit(&app, &cookie, &csrf, &placeholder_fields)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    let records: Vec<(String, String)> = sqlx::query_as(
        "SELECT document_kind,document_version FROM account_consents ORDER BY document_kind",
    )
    .fetch_all(&db.pool)
    .await?;
    assert_eq!(
        records,
        [
            ("privacy".to_owned(), version.to_owned()),
            ("terms".to_owned(), version.to_owned())
        ]
    );
    sqlx::query(
        "UPDATE editorial_pages SET legal_version='relabeled-v1', body='KRS 0000000000'
         WHERE locale='en'",
    )
    .execute(&db.pool)
    .await?;
    sqlx::query(
        "INSERT INTO page_legal_versions(page_id,locale,version,content)
         SELECT page_id,locale,legal_version,jsonb_build_object(
             'title',title,'body',body,'effective',legal_effective_date::text,'final_approved',true)
         FROM editorial_pages WHERE locale='en'",
    )
    .execute(&db.pool)
    .await?;
    let relabeled = db.app().layer(axum::Extension(ConsentConfig::published(
        "relabeled-v1".into(),
        "relabeled-v1".into(),
        false,
    )));
    assert_eq!(
        load(relabeled, "en").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );

    let final_config = ConsentConfig::published("final-v1".into(), "final-v1".into(), false);
    let live = db.app().layer(axum::Extension(final_config.clone()));
    sqlx::query(
        "UPDATE editorial_pages SET legal_version='final-v1',body='Approved legal text'
         WHERE locale='en'",
    )
    .execute(&db.pool)
    .await?;
    assert_eq!(
        load(live.clone(), "en").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query(
        "INSERT INTO page_legal_versions(page_id,locale,version,content)
         SELECT page_id,locale,legal_version,jsonb_build_object(
             'title',title,'body',body,'effective',legal_effective_date::text,'final_approved',true)
         FROM editorial_pages WHERE locale='en'",
    )
    .execute(&db.pool)
    .await?;
    assert_eq!(load(live, "en").await?.status(), StatusCode::OK);
    let mut final_input = input();
    final_input.email = "Final@Example.test".into();
    final_input.terms = Some("final-v1".into());
    final_input.privacy = Some("final-v1".into());
    let id = register::register(&db.pool, final_input, &final_config, "en", None)
        .await?
        .ok_or("account missing")?;
    let versions: Vec<String> =
        sqlx::query_scalar("SELECT document_version FROM account_consents WHERE account_id=$1")
            .bind(id)
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(versions, ["final-v1", "final-v1"]);
    db.close().await
}
