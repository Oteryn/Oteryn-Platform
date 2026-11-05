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
    // Earlier credential checks can span fixed windows under hashing load. Seed
    // this email/source through the real key derivation, without password work
    // between charges. Keep a small database-clock margin before the HTTP check.
    let policy = NamedPolicy::IdentityLogin;
    let remaining: f64 = sqlx::query_scalar(
        "SELECT ($1::bigint - mod(extract(epoch FROM statement_timestamp()), $1::bigint))::double precision",
    )
    .bind(policy.window_seconds())
    .fetch_one(&pool)
    .await?;
    if remaining < 3.0 {
        tokio::time::sleep(std::time::Duration::from_secs_f64(remaining)).await;
    }
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
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
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
