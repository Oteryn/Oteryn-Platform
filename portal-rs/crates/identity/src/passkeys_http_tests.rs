#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
#[path = "passkeys_test_db.rs"]
mod database;
const CSRF: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COOKIE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
async fn app(db: &database::Database, account: Option<Uuid>) -> Router {
    // Safe synthetic browser material, never a real credential.
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES (sha256($1::bytea),$2,'test',$3,now()+interval '1 hour',now()+interval '1 day')")
        .bind(COOKIE.as_bytes()).bind(account).bind(CSRF).execute(&db.pool).await.unwrap();
    let service = Passkeys::new(db.pool.clone(), "example.test", "https://example.test").unwrap();
    let mut sessions = SessionLayer::new(db.pool.clone());
    sessions.csrf.trusted_origins = vec!["https://example.test".into()];
    router(service, sessions)
}
fn request(path: &str, body: &str, csrf: &str, peer: bool) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/en/account/passkeys/{path}"))
        .header("content-type", "application/json")
        .header("cookie", format!("{}={COOKIE}", session::COOKIE_NAME))
        .header("origin", "https://example.test")
        .header("x-csrf-token", csrf)
        .body(Body::from(body.to_owned()))
        .unwrap();
    if peer {
        request
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>().unwrap()));
    }
    request
}
#[tokio::test]
async fn postgres_browser_csrf_peer_input_and_limiter_refusals() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let app = app(&db, None).await;
    assert_eq!(
        app.clone()
            .oneshot(request("authenticate/start", "{}", "wrong", true))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request("authenticate/start", "{}", CSRF, false))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        app.clone()
            .oneshot(request("register/start", "{", CSRF, true))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "register/start",
                r#"{"password":"wrong"}"#,
                CSRF,
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "authenticate/start",
                r#"{"unexpected":1}"#,
                CSRF,
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(request("authenticate/start", "{}", CSRF, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        value["options"]["publicKey"]["allowCredentials"]
            .as_array()
            .is_none_or(|v| v.is_empty())
    );
    assert!(value.get("state").is_none());
    let mut limited = false;
    for _ in 0..21 {
        if app
            .clone()
            .oneshot(request("authenticate/start", "{}", CSRF, true))
            .await
            .unwrap()
            .status()
            == StatusCode::TOO_MANY_REQUESTS
        {
            limited = true;
            break;
        }
    }
    assert!(limited);
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_password_confirmation_and_localized_page() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let hash = oteryn_identity::password::hash("test-only-password").unwrap();
    let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES ('test@example.test',$1,'active',now(),'en') RETURNING id").bind(hash).fetch_one(&db.pool).await.unwrap();
    let app = app(&db, Some(account)).await;
    for path in [
        "register/start",
        "00000000-0000-0000-0000-000000000000/remove",
        "bad/remove",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(path, r#"{"password":"wrong"}"#, CSRF, true))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let response = app
        .clone()
        .oneshot(request(
            "register/start",
            r#"{"password":"test-only-password"}"#,
            CSRF,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        app.clone()
            .oneshot(request("authenticate/start", "{}", CSRF, true))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/pl/account/passkeys")
                .header("cookie", format!("{}={COOKIE}", session::COOKIE_NAME))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Klucze dostępu"));
    assert!(body.contains("/static/passkeys.js"));
    db.close().await.unwrap();
}
