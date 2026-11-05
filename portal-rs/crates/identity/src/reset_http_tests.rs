use super::*;
use tower::ServiceExt;
#[path = "reset_test_db.rs"]
pub(crate) mod db;
#[tokio::test]
async fn browser_forms_csrf_and_limits() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let app = router(
        db.pool.clone(),
        DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?,
        SessionLayer::new(db.pool.clone()),
    )
    .layer(Extension(ConnectInfo(
        "127.0.0.1:1234".parse::<SocketAddr>()?,
    )));
    for lang in ["en", "pl"] {
        let r = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/{lang}/forgot-password"))
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::OK);
        let cookie = r.headers()["set-cookie"]
            .to_str()?
            .split(';')
            .next()
            .ok_or("missing field")?
            .to_owned();
        let body = String::from_utf8(axum::body::to_bytes(r.into_body(), 100000).await?.to_vec())?;
        assert!(body.contains("type=\"email\""));
        let csrf = body
            .split("name=\"_csrf\" value=\"")
            .nth(1)
            .ok_or("missing field")?
            .split('"')
            .next()
            .ok_or("missing field")?;
        let r = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/{lang}/forgot-password"))
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("email=absent%40example.test"))?,
            )
            .await?;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        for attempt in 0..4 {
            let r = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(format!("/{lang}/forgot-password"))
                        .header("cookie", &cookie)
                        .header("content-type", "application/x-www-form-urlencoded")
                        .body(axum::body::Body::from(format!(
                            "_csrf={csrf}&email=absent%40example.test"
                        )))?,
                )
                .await?;
            assert_eq!(
                r.status(),
                if attempt < 3 && lang == "en" {
                    StatusCode::FOUND
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
    }
    db.close().await
}

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn post(path: &str, cookie: &str, body: String) -> axum::http::Request<axum::body::Body> {
    let mut request = axum::http::Request::new(axum::body::Body::from(body));
    *request.method_mut() = axum::http::Method::POST;
    if let Ok(uri) = path.parse() {
        *request.uri_mut() = uri;
    }
    request.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    if let Ok(value) = cookie.parse() {
        request.headers_mut().insert("cookie", value);
    }
    request
}
#[tokio::test]
async fn browser_reset_success_and_bad_input() -> TestResult {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let key = DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?;
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES ('browser@example.test','old','en','active',now()) RETURNING id").fetch_one(&db.pool).await?;
    let app = router(
        db.pool.clone(),
        key.clone(),
        SessionLayer::new(db.pool.clone()),
    )
    .layer(Extension(ConnectInfo(
        "127.0.0.2:2345".parse::<SocketAddr>()?,
    )));
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/forgot-password")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    let cookie = response.headers()["set-cookie"]
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 100000)
            .await?
            .to_vec(),
    )?;
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("csrf")?;
    let response = app
        .clone()
        .oneshot(post(
            "/en/forgot-password",
            &cookie,
            format!("_csrf={csrf}&email=browser%40example.test"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM outbox WHERE kind='email.password-reset'")
            .fetch_one(&db.pool)
            .await?;
    let token = key
        .open(&serde_json::from_value::<Vec<u8>>(
            payload["ciphertext"].clone(),
        )?)
        .ok_or("ciphertext")?;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/en/reset-password/{token}"))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 100000)
            .await?
            .to_vec(),
    )?;
    assert!(html.contains("autocomplete=\"new-password\""));
    assert!(html.contains(&token));
    for password in ["short", "a+long+new+password"] {
        let response=app.clone().oneshot(post("/en/reset-password",&cookie,format!("_csrf={csrf}&email=browser%40example.test&token={token}&password={password}&password_confirmation={password}"))).await?;
        assert_eq!(response.status(), StatusCode::FOUND);
        if password == "short" {
            assert_eq!(
                response.headers()["location"],
                format!("/en/reset-password/{token}")
            );
        } else {
            assert_eq!(response.headers()["location"], "/en/login");
            assert!(
                response.headers()["set-cookie"]
                    .to_str()?
                    .contains("Max-Age=0")
            );
        }
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert!(oteryn_identity::password::verify(
        "a long new password",
        &hash
    ));
    db.close().await
}
#[tokio::test]
async fn missing_source_and_cross_origin_deny() -> TestResult {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let app = router(
        db.pool.clone(),
        DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?,
        SessionLayer::new(db.pool.clone()),
    );
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/pl/forgot-password")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    let cookie = response.headers()["set-cookie"]
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 100000)
            .await?
            .to_vec(),
    )?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("csrf")?;
    let response = app
        .clone()
        .oneshot(post(
            "/pl/forgot-password",
            &cookie,
            format!("_csrf={csrf}&email=a%40example.test"),
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let mut request = post("/pl/reset-password", &cookie, format!("_csrf={csrf}"));
    request.headers_mut().insert(
        "origin",
        axum::http::HeaderValue::from_static("https://untrusted.example"),
    );
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='identity.password_reset_browser_refused'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, 1);
    db.close().await
}

#[tokio::test]
async fn reset_and_source_limits_and_authenticated_refusal() -> TestResult {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let app = router(
        db.pool.clone(),
        DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?,
        SessionLayer::new(db.pool.clone()),
    );
    for (source, path, limit) in [
        ("127.0.0.3:1234", "/en/reset-password", 5),
        ("127.0.0.4:1234", "/en/forgot-password", 10),
    ] {
        let app = app
            .clone()
            .layer(Extension(ConnectInfo(source.parse::<SocketAddr>()?)));
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(path)
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        let cookie = response.headers()["set-cookie"]
            .to_str()?
            .split(';')
            .next()
            .ok_or("cookie")?
            .to_owned();
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 100000)
                .await?
                .to_vec(),
        )?;
        let csrf = html
            .split("name=\"_csrf\" value=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .ok_or("csrf")?;
        for n in 0..=limit {
            let email = if limit == 5 {
                "rate%40example.test"
            } else {
                "bad"
            };
            let response = app
                .clone()
                .oneshot(post(path, &cookie, format!("_csrf={csrf}&email={email}")))
                .await?;
            assert_eq!(
                response.status(),
                if n < limit {
                    StatusCode::FOUND
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
    }
    let app = app.layer(Extension(ConnectInfo(
        "127.0.0.5:1234".parse::<SocketAddr>()?,
    )));
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/forgot-password")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    let cookie = response.headers()["set-cookie"]
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 100000)
            .await?
            .to_vec(),
    )?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("csrf")?;
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status) VALUES ('signed@example.test','unused','en','active') RETURNING id").fetch_one(&db.pool).await?;
    let raw = cookie.split_once('=').ok_or("cookie value")?.1;
    sqlx::query("UPDATE web_sessions SET account_id=$1 WHERE id_hash=$2")
        .bind(id)
        .bind(oteryn_identity::verify::token_hash(raw))
        .execute(&db.pool)
        .await?;
    for path in ["/en/forgot-password", "/en/reset-password"] {
        let response = app
            .clone()
            .oneshot(post(
                path,
                &cookie,
                format!("_csrf={csrf}&email=signed%40example.test"),
            ))
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()["location"], "/en/");
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='identity.password_reset_authenticated_refused'").fetch_one(&db.pool).await?;
    assert_eq!(count, 2);
    db.close().await
}
