#[path = "mfa_test_db.rs"]
mod database;
use super::*;
use tower::ServiceExt;
#[tokio::test]
async fn postgres_mfa_browser_csrf_limiter_and_login_challenge()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(std::time::Duration::from_secs(30), browser_flow()).await??;
    Ok(())
}
async fn browser_flow() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('browser@example.test',$1,'active','en',now()) RETURNING id").bind(oteryn_identity::password::hash("test password")?).fetch_one(&db.pool).await?;
    let enrollment = mfa.enroll(id, "test password").await?.ok_or("enrollment")?;
    let secret = totp_rs::Secret::Encoded(enrollment.secret).to_bytes()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let code = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, secret)?.generate(now);
    let codes = mfa
        .confirm(id, "test password", &code)
        .await?
        .ok_or("confirm")?;
    let mut layer = SessionLayer::new(db.pool.clone());
    layer.csrf.trusted_origins = vec!["https://portal.example.test".into()];
    let app = compose(Router::new(), db.pool.clone(), mfa.clone(), layer);
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/login")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let bytes = axum::body::to_bytes(response.into_body(), 32768).await?;
    let body = String::from_utf8(bytes.to_vec())?;
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("csrf")?;
    let request = |token: &str| -> Result<_, Box<dyn std::error::Error>> {
        Ok(axum::http::Request::builder()
            .method("POST")
            .uri("/en/login")
            .header(header::COOKIE, &cookie)
            .header(header::ORIGIN, "https://portal.example.test")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .extension(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>()?))
            .body(axum::body::Body::from(format!(
                "_csrf={token}&email=browser%40example.test&password=test+password"
            )))?)
    };
    let refused = app.clone().oneshot(request("bad")?).await?;
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    let response = app.clone().oneshot(request(csrf)?).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response.headers().get(header::LOCATION).ok_or("location")?,
        "/en/mfa/challenge"
    );
    let cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .rfind(|v| !v.contains("Max-Age=0"))
        .ok_or("rotated")?
        .split(';')
        .next()
        .ok_or("cookie")?;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/mfa/challenge")
                .header(header::COOKIE, cookie)
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .ok_or("cache")?,
        "no-store, private"
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0);
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 32768)
            .await?
            .to_vec(),
    )?;
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("csrf")?;
    let challenge_request = |code: &str| -> Result<_, Box<dyn std::error::Error>> {
        Ok(axum::http::Request::builder()
            .method("POST")
            .uri("/en/mfa/challenge")
            .header(header::COOKIE, cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .extension(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>()?))
            .body(axum::body::Body::from(format!(
                "_csrf={csrf}&code={code}&remember=yes"
            )))?)
    };
    let refused = app.clone().oneshot(challenge_request("bad")?).await?;
    assert_eq!(refused.status(), StatusCode::FOUND);
    let accepted = app.clone().oneshot(challenge_request(&codes[0])?).await?;
    assert_eq!(accepted.status(), StatusCode::FOUND);
    assert!(
        accepted
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| v.starts_with(TRUSTED) && v.contains("Secure; HttpOnly; SameSite=Lax"))
    );
    let grant: String =
        sqlx::query_scalar("SELECT data->>'mfa_grant' FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert!(mfa.satisfied(id, &grant).await?);
    let stale = app.clone().oneshot(challenge_request(&codes[1])?).await?;
    assert_eq!(stale.status(), StatusCode::FORBIDDEN);
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/en/mfa/challenge")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(axum::body::Body::from("code=bad"))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_mfa_limits_and_missing_transport() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let context = Context {
        mfa: Mfa::new(db.pool.clone(), [7; 32]),
        limiter: RateLimiter::new(db.pool.clone()),
    };
    let peer = Some(Extension(ConnectInfo(
        "127.0.0.1:4321".parse::<SocketAddr>()?,
    )));
    for _ in 0..5 {
        assert!(
            charge(
                &context,
                peer,
                &HeaderMap::new(),
                Language::En,
                (None, Some("pending-account".into()), None),
                &[NamedPolicy::IdentityMfaChallenge]
            )
            .await
            .is_ok()
        );
    }
    let response = charge(
        &context,
        peer,
        &HeaderMap::new(),
        Language::En,
        (None, Some("pending-account".into()), None),
        &[NamedPolicy::IdentityMfaChallenge],
    )
    .await
    .err()
    .ok_or("missing denial")?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let response = charge(
        &context,
        None,
        &HeaderMap::new(),
        Language::En,
        (None, None, None),
        &[NamedPolicy::IdentityMfaChallengeSource],
    )
    .await
    .err()
    .ok_or("missing denial")?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let response = charge(
        &context,
        peer,
        &HeaderMap::new(),
        Language::En,
        (None, None, None),
        &[NamedPolicy::IdentityMfaChallenge],
    )
    .await
    .err()
    .ok_or("missing denial")?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn mfa_composition_preserves_unrelated_response_policy()
-> Result<(), Box<dyn std::error::Error>> {
    let pool = oteryn_db::lazy_pool("postgres://localhost/unreachable")?;
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let app = compose(
        app,
        pool.clone(),
        Mfa::new(pool.clone(), [7; 32]),
        SessionLayer::new(pool),
    );
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::CACHE_CONTROL).is_none());
    assert!(
        response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_password_browser_rotation_releases_account_lock()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('password-browser@example.test',$1,'active','en',now()) RETURNING id")
        .bind(oteryn_identity::password::hash("test password")?).fetch_one(&db.pool).await?;
    let app = compose(
        Router::new(),
        db.pool.clone(),
        Mfa::new(db.pool.clone(), [7; 32]),
        SessionLayer::new(db.pool.clone()),
    );
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/login")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie")?
        .to_owned();
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 32768)
            .await?
            .to_vec(),
    )?;
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("csrf")?;
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/en/login")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .extension(ConnectInfo("127.0.0.1:4321".parse::<SocketAddr>()?))
        .body(axum::body::Body::from(format!(
            "_csrf={csrf}&email=password-browser%40example.test&password=test+password"
        )))?;
    let response =
        tokio::time::timeout(std::time::Duration::from_secs(15), app.oneshot(request)).await??;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response.headers().get(header::LOCATION).ok_or("location")?,
        "/en/"
    );
    let rotated = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("rotated cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("rotated cookie")?;
    assert_ne!(rotated, cookie);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await?;
    Ok(())
}
