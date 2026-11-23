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
async fn seed_session(db: &database::Database, account: Option<Uuid>) {
    // Safe synthetic browser material, never a real credential.
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at)
         VALUES (sha256($1::bytea),$2,'test',$3,now()+interval '1 hour',now()+interval '1 day')",
    )
        .bind(COOKIE.as_bytes()).bind(account).bind(CSRF).execute(&db.pool).await.unwrap();
}
async fn app(db: &database::Database, account: Option<Uuid>) -> Router {
    seed_session(db, account).await;
    test_router(db)
}
fn test_router(db: &database::Database) -> Router {
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
    for _ in 0..4 {
        let response = app
            .clone()
            .oneshot(request("authenticate/start", "{}", "", true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='passkey.http_refused'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "anonymous passkeys audit rows");
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
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='passkey.http_refused'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
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
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='passkey.refused' AND actor=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(count, 3);
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

#[tokio::test]
async fn postgres_passkey_navigation_matches_route_eligibility() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
         VALUES ('navigation@example.test','unused','active',now(),'en') RETURNING id",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let mut registry = oteryn_ui::Registry::default();
    registry.account = true;
    registry.section(
        true,
        "account-passkeys",
        "shell-account-passkeys",
        "account/passkeys",
        oteryn_ui::Group::Account,
        false,
    );
    let app = test_router(&db)
        .route(
            "/en/",
            get(
                |Extension(chrome): Extension<oteryn_ui::Chrome>| async move {
                    oteryn_ui::home(&chrome, &oteryn_ui::HomeData::default())
                },
            ),
        )
        .layer(axum::middleware::from_fn_with_state(
            oteryn_web::chrome::Shell {
                registry,
                sessions: SessionLayer::new(db.pool.clone()),
            },
            oteryn_web::chrome::middleware,
        ));
    let route = "/en/account/passkeys";
    let request = |uri| {
        Request::get(uri)
            .header("cookie", format!("{}={COOKIE}", session::COOKIE_NAME))
            .body(Body::empty())
            .unwrap()
    };
    for (status, verified, mfa_required, expected) in [
        ("active", true, false, StatusCode::OK),
        ("active", true, true, StatusCode::FORBIDDEN),
        ("active", false, false, StatusCode::FORBIDDEN),
        ("suspended", true, false, StatusCode::FORBIDDEN),
    ] {
        sqlx::query(
            "UPDATE accounts SET status=$2,
             email_verified_at=CASE WHEN $3 THEN now() ELSE NULL END,
             login_mfa_required=$4 WHERE id=$1",
        )
        .bind(account)
        .bind(status)
        .bind(verified)
        .bind(mfa_required)
        .execute(&db.pool)
        .await
        .unwrap();
        // Account state transitions can revoke sessions. Give each fixture a valid session.
        seed_session(&db, Some(account)).await;
        let passkeys = app.clone().oneshot(request(route)).await.unwrap();
        assert_eq!(
            passkeys.status(),
            expected,
            "{status}, {verified}, {mfa_required}"
        );
        let home = app.clone().oneshot(request("/en/")).await.unwrap();
        assert_eq!(home.status(), StatusCode::OK);
        let html = to_bytes(home.into_body(), 1024 * 1024).await.unwrap();
        let html = std::str::from_utf8(&html).unwrap();
        assert!(html.contains("data-account-link"), "valid signed-in shell");
        assert_eq!(
            html.contains("href=\"/en/account/passkeys\""),
            expected == StatusCode::OK,
            "{status}, {verified}, {mfa_required}",
        );
        sqlx::query("DELETE FROM web_sessions WHERE id_hash=sha256($1::bytea)")
            .bind(COOKIE.as_bytes())
            .execute(&db.pool)
            .await
            .unwrap();
    }
    db.close().await.unwrap();
}

async fn finish_browser_login(db: &database::Database) -> (Uuid, serde_json::Value) {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as b64};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES ('passkey@example.test','unused','active',now(),'en') RETURNING id")
        .fetch_one(&db.pool).await.unwrap();
    let pair = rcgen::KeyPair::generate().unwrap();
    let public = pair.public_key_raw();
    let signing = rustls::crypto::ring::sign::any_supported_type(
        &rustls::pki_types::PrivatePkcs8KeyDer::from(pair.serialize_der()).into(),
    )
    .unwrap()
    .choose_scheme(&[rustls::SignatureScheme::ECDSA_NISTP256_SHA256])
    .unwrap();
    let id = account.as_bytes();
    let registered = json!({"cred": {
        "cred_id": b64.encode(id), "cred": {"type_":"ES256","key":{"EC_EC2":{
            "curve":"SECP256R1","x":b64.encode(&public[1..33]),"y":b64.encode(&public[33..])}}},
        "counter":0,"user_verified":true,"backup_eligible":false,"backup_state":false,
        "registration_policy":"required","extensions":{},"attestation":{"data":"None","metadata":"None"},
        "attestation_format":"none"
    }});
    sqlx::query(
        "INSERT INTO account_passkeys(account_id,credential_id,credential) VALUES ($1,$2,$3)",
    )
    .bind(account)
    .bind(id.as_slice())
    .bind(registered)
    .execute(&db.pool)
    .await
    .unwrap();
    let app = app(db, None).await;
    sqlx::query("UPDATE web_sessions SET data=jsonb_build_object('oauth_return',$1::text,'mfa_pending','previous challenge','mfa_grant','previous grant')")
        .bind(RETURN_TARGET).execute(&db.pool).await.unwrap();
    let start = app
        .clone()
        .oneshot(request("authenticate/start", "{}", CSRF, true))
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::OK);
    let start: serde_json::Value =
        serde_json::from_slice(&to_bytes(start.into_body(), 65536).await.unwrap()).unwrap();
    let client = json!({"type":"webauthn.get","origin":"https://example.test",
        "challenge":start["options"]["publicKey"]["challenge"]})
    .to_string();
    let mut data = Sha256::digest(b"example.test").to_vec();
    data.extend([5, 0, 0, 0, 1]); // User present and verified, signature counter 1.
    let mut signed = data.clone();
    signed.extend(Sha256::digest(client.as_bytes()));
    let finish = json!({"token":start["token"],"credential":{
        "type":"public-key","id":b64.encode(id),"rawId":b64.encode(id),"extensions":{},
        "response":{"userHandle":b64.encode(id),"authenticatorData":b64.encode(data),
        "clientDataJSON":b64.encode(client),"signature":b64.encode(signing.sign(&signed).unwrap())}
    }});
    let response = app
        .oneshot(request(
            "authenticate/finish",
            &finish.to_string(),
            CSRF,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    (
        account,
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap(),
    )
}

const RETURN_TARGET: &str = "/oauth/authorize?response_type=code&client_id=c&state=s";

#[tokio::test]
async fn passkey_login_discards_pending_mfa_and_grant() {
    let db = database::Database::open()
        .await
        .unwrap()
        .expect("PostgreSQL required");
    let (account, _) = finish_browser_login(&db).await;
    let data: serde_json::Value =
        sqlx::query_scalar("SELECT data FROM web_sessions WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    for key in ["mfa_pending", "mfa_grant"] {
        assert!(
            data[key].as_str().unwrap_or_default().is_empty(),
            "{key} survived login"
        );
    }
    db.close().await.unwrap();
}

#[tokio::test]
async fn passkey_login_consumes_oauth_return() {
    let db = database::Database::open()
        .await
        .unwrap()
        .expect("PostgreSQL required");
    let (account, result) = finish_browser_login(&db).await;
    assert_eq!(result["location"], RETURN_TARGET);
    let stored: Option<String> =
        sqlx::query_scalar("SELECT data->>'oauth_return' FROM web_sessions WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(stored, None);
    db.close().await.unwrap();
}
