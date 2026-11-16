#[path = "login_test_db.rs"]
mod database;
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, header},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn request(
    method: &str,
    path: &str,
    cookie: &str,
    body: &str,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))?;
    req.extensions_mut()
        .insert(ConnectInfo("192.0.2.44:1234".parse::<SocketAddr>()?));
    Ok(req)
}
fn cookie(response: &Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("missing cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("empty cookie")?
        .to_owned())
}
async fn body(response: Response) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 100_000).await?.to_vec(),
    )?)
}
fn csrf(html: &str) -> Result<&str, Box<dyn std::error::Error>> {
    html.split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or_else(|| "missing CSRF token".into())
}
#[tokio::test]
async fn disabled_has_no_routes() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        router(pool.clone(), false, SessionLayer::new(pool))
            .oneshot(request("GET", "/en/login", "", "")?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}
#[tokio::test]
async fn postgres_browser_login_logout_csrf_and_limits() -> TestResult {
    let Some(database) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = database.pool.clone();
    let email = format!("browser-{}@example.test", uuid::Uuid::now_v7());
    let hash = oteryn_identity::password::hash("test password")?;
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(&email).bind(hash).fetch_one(&pool).await?;
    let app = router(pool.clone(), true, SessionLayer::new(pool.clone()));
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", "", "")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let guest = cookie(&response)?;
    let html = body(response).await?;
    assert!(html.contains("Log in"));
    let token = csrf(&html)?;
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("email={email}&password=test+password"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email=bad&password="),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email={email}&password=wrong"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        body(
            app.clone()
                .oneshot(request("GET", "/en/login", &guest, "")?)
                .await?
        )
        .await?
        .contains("The provided credentials are invalid.")
    );
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email={email}&password=test+password"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let authenticated = cookie(&response)?;
    assert_ne!(guest, authenticated);
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", &authenticated, "")?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let rotated_csrf: String =
        sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_ne!(token, rotated_csrf);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/logout",
            &authenticated,
            &format!("_csrf={token}"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/logout",
            &authenticated,
            &format!("_csrf={}", rotated_csrf),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .contains("Max-Age=0")
    );
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", &authenticated, "")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(remaining, 0);
    // Rotation invalidates the guest token too; acquire a fresh anonymous session.
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", "", "")?)
        .await?;
    let guest = cookie(&response)?;
    let html = body(response).await?;
    let token = csrf(&html)?;
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/en/login",
                &guest,
                &format!("_csrf={token}&email={email}&password=wrong"),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
    }
    // The schema's statement clock stays frozen throughout seeding and the
    // HTTP probe, including any password hashing work.
    let policy = NamedPolicy::IdentityLogin;
    // Each test has its own schema; discard only the earlier email quota.
    sqlx::query("DELETE FROM web_rate_limits WHERE limiter_name = $1")
        .bind(policy.name())
        .execute(&pool)
        .await?;
    let limiter = RateLimiter::new(pool.clone());
    let mut keys = Keys::source("192.0.2.44".parse()?);
    keys.email = Some(email.clone());
    for _ in 0..policy.limit() {
        limiter.check(policy, std::slice::from_ref(&keys)).await?;
    }
    let response = app
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email={email}&password=wrong"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    sqlx::query("DELETE FROM web_rate_limits WHERE limiter_name IN ('identity-login','identity-login-source')").execute(&pool).await?;
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    // Security history is durable; isolated-schema teardown owns cleanup.
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM security_events WHERE account_id=$1 AND kind='login_succeeded'"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?
            > 0
    );
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM email_outbox WHERE account_id=$1 AND category='security'"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?
            > 0
    );
    database.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_source_limit_bad_input_and_missing_transport() -> TestResult {
    let Some(database) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = database.pool.clone();
    let app = router(pool.clone(), true, SessionLayer::new(pool));
    let response = app
        .clone()
        .oneshot(request("GET", "/pl/login", "", "")?)
        .await?;
    let guest = cookie(&response)?;
    let html = body(response).await?;
    assert!(html.contains("Logowanie"));
    assert!(html.contains("autocomplete=\"current-password\""));
    let token = csrf(&html)?;
    let form = format!("_csrf={token}&email=bad&password=");
    let mut no_peer = request("POST", "/pl/login", &guest, &form)?;
    no_peer.extensions_mut().remove::<ConnectInfo<SocketAddr>>();
    assert_eq!(
        app.clone().oneshot(no_peer).await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    // The frozen statement clock keeps all 21 requests in one source bucket.
    for _ in 0..20 {
        let response = app
            .clone()
            .oneshot(request("POST", "/pl/login", &guest, &form)?)
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
    }
    let response = app
        .oneshot(request("POST", "/pl/login", &guest, &form)?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    database.close().await?;
    Ok(())
}
#[tokio::test]
async fn postgres_login_returns_to_stored_authorize_target_once() -> TestResult {
    let Some(database) = database::Database::open().await? else {
        return Ok(());
    };
    let pool = database.pool.clone();
    let email = format!("return-{}@example.test", uuid::Uuid::now_v7());
    let hash = oteryn_identity::password::hash("test password")?;
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id").bind(&email).bind(hash).fetch_one(&pool).await?;
    let app = router(pool.clone(), true, SessionLayer::new(pool.clone()));
    let target = "/oauth/authorize?response_type=code&client_id=c&state=s";
    let store = "UPDATE web_sessions SET data=data||jsonb_build_object('oauth_return',$1::text) WHERE account_id IS NULL";
    let stored = "SELECT data->>'oauth_return' FROM web_sessions WHERE account_id=$1";
    // An unacceptable target is consumed and ignored: home, never elsewhere.
    for bad in ["//evil.example/oauth/authorize?a=1", "/oauth/authorize?"] {
        let response = app
            .clone()
            .oneshot(request("GET", "/en/login", "", "")?)
            .await?;
        let guest = cookie(&response)?;
        let token = csrf(&body(response).await?)?.to_owned();
        sqlx::query(store).bind(bad).execute(&pool).await?;
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/en/login",
                &guest,
                &format!("_csrf={token}&email={email}&password=test+password"),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()[header::LOCATION], "/en/");
        let left: Option<String> = sqlx::query_scalar(stored).bind(id).fetch_one(&pool).await?;
        assert_eq!(left, None);
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", "", "")?)
        .await?;
    let guest = cookie(&response)?;
    let token = csrf(&body(response).await?)?.to_owned();
    sqlx::query(store).bind(target).execute(&pool).await?;
    // A refused login keeps the target for the next attempt.
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email={email}&password=wrong"),
        )?)
        .await?;
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    let kept: Option<String> = sqlx::query_scalar(
        "SELECT data->>'oauth_return' FROM web_sessions WHERE account_id IS NULL",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(kept.as_deref(), Some(target));
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &guest,
            &format!("_csrf={token}&email={email}&password=test+password"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], target);
    let left: Option<String> = sqlx::query_scalar(stored).bind(id).fetch_one(&pool).await?;
    assert_eq!(left, None);
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    // Security history is durable; isolated-schema teardown owns cleanup.
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM security_events WHERE account_id=$1 AND kind='login_succeeded'"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?
            > 0
    );
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM email_outbox WHERE account_id=$1 AND category='security'"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?
            > 0
    );
    database.close().await?;
    Ok(())
}

#[tokio::test]
async fn audit02_paused_password_completion_refuses_reset_even_after_anonymous_get() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('paused-login@example.test',$1,'active','en',now()) RETURNING id").bind(oteryn_identity::password::hash("test password")?).fetch_one(&db.pool).await?;
    let proof = login::authenticate_authorized(
        &db.pool,
        Credentials {
            email: "paused-login@example.test".into(),
            password: "test password".into(),
        },
    )
    .await?
    .ok_or("proof")?;
    let app = router(db.pool.clone(), true, SessionLayer::new(db.pool.clone()));
    let response = app
        .clone()
        .oneshot(request("GET", "/en/login", "", "")?)
        .await?;
    let guest = cookie(&response)?;
    let html = body(response).await?;
    let csrf = csrf(&html)?.to_owned();
    let key = oteryn_identity::reset::DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?;
    oteryn_identity::reset::request(&db.pool, "paused-login@example.test", &key).await?;
    let payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM outbox WHERE kind='email.password-reset'")
            .fetch_one(&db.pool)
            .await?;
    let token = key
        .open(&serde_json::from_value::<Vec<u8>>(
            payload["ciphertext"].clone(),
        )?)
        .ok_or("ciphertext")?;
    assert!(
        oteryn_identity::reset::complete(
            &db.pool,
            oteryn_identity::reset::Input {
                email: "paused-login@example.test".into(),
                token,
                password: "replacement password".into(),
                password_confirmation: "replacement password".into()
            }
        )
        .await?
    );
    // A real ordinary GET refreshes the same anonymous session after revocation.
    assert_eq!(
        app.oneshot(request("GET", "/en/login", &guest, "")?)
            .await?
            .status(),
        StatusCode::OK
    );
    let proof = std::sync::Arc::new(tokio::sync::Mutex::new(Some(proof)));
    let pool = db.pool.clone();
    let completion = Router::new()
        .route(
            "/resume",
            post(move |Extension(session): Extension<BrowserSession>| {
                let proof = proof.clone();
                let pool = pool.clone();
                async move {
                    let proof = proof.lock().await.take().ok_or(StatusCode::UNAUTHORIZED)?;
                    establish(&pool, Language::En, &session, proof).await
                }
            }),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            session::middleware,
        ));
    let response = completion
        .oneshot(request(
            "POST",
            "/resume",
            &guest,
            &format!("_csrf={csrf}"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn logout_delete_failure_keeps_deletion_cookie_through_error_renderer() -> TestResult {
    use sha2::{Digest, Sha256};
    let started = std::time::Instant::now();
    let database = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let pool = database.pool.clone();
    let account: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES('logout-failure@example.test','unused','active',now(),'en') RETURNING id").fetch_one(&pool).await?;
    let raw = "e".repeat(64);
    let csrf = "f".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at,account_id) VALUES($1,'logout regression',$2,now()+interval '1 hour',now()+interval '1 day',$3)").bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(&csrf).bind(account).execute(&pool).await?;
    sqlx::raw_sql("CREATE FUNCTION refuse_logout_delete() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test storage deletion failure'; END $$; CREATE TRIGGER refuse_logout_delete BEFORE DELETE ON web_sessions FOR EACH ROW EXECUTE FUNCTION refuse_logout_delete()").execute(&pool).await?;
    let app = router(pool.clone(), true, SessionLayer::new(pool.clone()));
    for path in ["/logout", "/pl/logout", "/en/logout"] {
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                path,
                &format!("{}={raw}", session::COOKIE_NAME),
                &format!("_csrf={csrf}"),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            response
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .any(|v| v.to_str().is_ok_and(|s| s.contains("Max-Age=0"))),
            "{path}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM web_sessions")
                .fetch_one(&pool)
                .await?,
            1
        );
    }
    database.close().await?;
    eprintln!(
        "logout_delete_failure_keeps_deletion_cookie_through_error_renderer duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn production_proxy_policy_isolates_registration_verification_login_and_search() -> TestResult
{
    use oteryn_web::{
        AppState,
        ratelimit::{NamedPolicy, TrustedProxies},
    };
    let started = std::time::Instant::now();
    let database = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let pool = database.pool.clone();
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&pool)
        .await?;
    let app = oteryn_web::router(AppState { db: pool.clone() }, "static")
        .merge(router(pool.clone(), true, SessionLayer::new(pool.clone())))
        .merge(oteryn_web::search::routes(
            oteryn_web::search::SearchState {
                pool: pool.clone(),
                world: None,
                characters: None,
            },
        ));
    assert!(
        include_str!("../../../../deploy/oracle/Caddyfile")
            .contains("header_up X-Forwarded-For {client_ip}")
    );
    assert!(
        include_str!("../../../../deploy/oracle/public-platform.compose.yml")
            .contains("TRUSTED_PROXIES: 172.18.0.10\n")
    );
    let app = crate::with_proxy_policy(app, pool.clone(), TrustedProxies::parse("172.18.0.10")?);
    let response = app
        .clone()
        .oneshot(request("GET", "/en/register", "", "")?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = cookie(&response)?;
    let html = body(response).await?;
    let csrf = csrf(&html)?;
    for (policy, path, method, expected) in [
        (
            NamedPolicy::IdentityRegistration,
            "/en/register",
            "POST",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            NamedPolicy::IdentityVerificationSource,
            "/en/verify-email?token=invalid",
            "GET",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            NamedPolicy::IdentityLoginSource,
            "/en/login",
            "POST",
            StatusCode::FOUND,
        ),
        (
            NamedPolicy::WikiSearch,
            "/en/search?q=budget",
            "GET",
            StatusCode::OK,
        ),
    ] {
        for n in 0..=policy.limit() + 1 {
            let client = if n <= policy.limit() {
                "192.0.2.10"
            } else {
                "192.0.2.11"
            };
            let mut req = request(method, path, &cookie, &format!("_csrf={csrf}"))?;
            req.headers_mut().insert("x-forwarded-for", client.parse()?);
            req.extensions_mut()
                .insert(ConnectInfo("172.18.0.10:8080".parse::<SocketAddr>()?));
            let response = app.clone().oneshot(req).await?;
            assert_eq!(
                response.status(),
                if n == policy.limit() {
                    StatusCode::TOO_MANY_REQUESTS
                } else {
                    expected
                },
                "{} request {n}",
                policy.name()
            );
        }
    }
    database.close().await?;
    eprintln!(
        "production_proxy_policy_isolates_registration_verification_login_and_search duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
