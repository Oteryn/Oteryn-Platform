#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "sessions_test_db.rs"]
mod database;
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn request(method: &str, path: &str, cookie: &str, body: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo("192.0.2.8:1234".parse::<SocketAddr>().unwrap()));
    req
}
async fn fixture(pool: &PgPool, agent: &str) -> (uuid::Uuid, String, String, uuid::Uuid) {
    let account = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES($1,'unused','active',now(),'en') RETURNING id").bind(format!("{}@example.invalid",uuid::Uuid::now_v7())).fetch_one(pool).await.unwrap();
    let raw = "a".repeat(64);
    let csrf = "b".repeat(64);
    let id = sqlx::query_scalar("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,$4,now()+interval '1 hour',now()+interval '2 hours') RETURNING session_id")
        .bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(account).bind(agent).bind(&csrf).fetch_one(pool).await.unwrap();
    (account, format!("{}={raw}", session::COOKIE_NAME), csrf, id)
}
#[tokio::test]
async fn postgres_session_page_csrf_owner_bad_input_and_signout() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let (account, cookie, csrf, id) = fixture(&db.pool, "<script>alert(1)</script>").await;
    let app =
        router(db.pool.clone(), SessionLayer::new(db.pool.clone())).layer(Extension(Language::En));
    let response = app
        .clone()
        .oneshot(request("GET", "/en/account/sessions", &cookie, ""))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    assert!(html.contains("This session"));
    assert!(html.contains("&"));
    assert!(!html.contains("<script>"));
    let localized = app
        .clone()
        .oneshot(request("GET", "/pl/account/sessions", &cookie, ""))
        .await?;
    assert_eq!(localized.headers()[header::CONTENT_LANGUAGE], "pl");
    let localized = String::from_utf8(to_bytes(localized.into_body(), 100_000).await?.to_vec())?;
    assert!(localized.contains("Ta sesja"));
    assert!(html.contains("name=\"_csrf\""));
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/en/account/sessions/all", &cookie, ""))
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/en/account/sessions/invalid",
                &cookie,
                &format!("_csrf={csrf}")
            ))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let foreign = uuid::Uuid::now_v7();
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/en/account/sessions/{foreign}"),
            &cookie,
            &format!("_csrf={csrf}"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/en/account/sessions/{id}"),
            &cookie,
            &format!("_csrf={csrf}"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()[header::LOCATION], "/en/login");
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .contains("Max-Age=0")
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    assert_eq!(
        app.clone()
            .oneshot(request("GET", "/en/account/sessions", &cookie, ""))
            .await?
            .status(),
        StatusCode::FOUND
    );
    db.close().await?;
    Ok(())
}
#[test]
fn cookie_validation_denies_ambiguous_and_malformed_input() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        format!("{}={}", session::COOKIE_NAME, "a".repeat(64))
            .parse()
            .unwrap(),
    );
    assert!(cookie_hash(&headers).is_some());
    headers.append(
        header::COOKIE,
        format!("{}={}", session::COOKIE_NAME, "b".repeat(64))
            .parse()
            .unwrap(),
    );
    assert!(cookie_hash(&headers).is_none());
    assert!(!valid_cookie("bad"));
    assert!(!valid_cookie(&"X".repeat(64)));
}

#[tokio::test]
async fn postgres_login_observer_commits_before_cookie_and_denies_enqueue_failure() -> TestResult {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let (account, cookie, _, _) = fixture(&db.pool, "Observer browser").await;
    // Exercise the exact public rotation API used by T04, behind its session middleware.
    async fn sign_in(
        Extension(session): Extension<BrowserSession>,
        Extension(account): Extension<uuid::Uuid>,
    ) -> Result<Response, StatusCode> {
        session::rotate(&session, session::PrivilegeChange::Login(account))
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(redirect("/en/"))
    }
    let app = Router::new()
        .route("/en/login", get(|| async { StatusCode::OK }).post(sign_in))
        .route_layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            session::middleware,
        ))
        .layer(Extension(account))
        .layer(axum::middleware::from_fn_with_state(
            db.pool.clone(),
            observe_login,
        ));
    let token: String =
        sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/login",
            &cookie,
            &format!("_csrf={token}"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let rotated = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_owned();
    assert!(rotated.starts_with(session::COOKIE_NAME));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    // A malformed mailbox is a notification refusal; no authenticated cookie escapes.
    sqlx::query("UPDATE accounts SET email='invalid mailbox' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    // The mailbox change revokes the account's sessions (credential-change trigger), so a
    // fresh browser session for the same account observes the refusal.
    let raw = "c".repeat(64);
    let token = "d".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,'Changed browser',$3,now()+interval '1 hour',now()+interval '2 hours')")
        .bind(Sha256::digest(raw.as_bytes()).to_vec())
        .bind(account)
        .bind(&token)
        .execute(&db.pool)
        .await?;
    let new_cookie = format!("{}={raw}", session::COOKIE_NAME);
    let response = app
        .oneshot(request(
            "POST",
            "/en/login",
            &new_cookie,
            &format!("_csrf={token}"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await?;
    Ok(())
}
