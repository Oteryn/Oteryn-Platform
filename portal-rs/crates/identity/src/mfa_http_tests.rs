#[path = "mfa_test_db.rs"]
mod database;
use super::*;
use tower::ServiceExt;
#[tokio::test]
async fn postgres_mfa_browser_csrf_limiter_and_login_challenge()
-> Result<(), Box<dyn std::error::Error>> {
    // Schema creation and migrations are setup, not the guarded flow; in a full
    // parallel test binary they contend with every other migrating test.
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), browser_flow(&db, false)).await??;
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn grant_write_failure_withholds_login() -> Result<(), Box<dyn std::error::Error>> {
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    browser_flow(&db, true).await?;
    db.close().await?;
    Ok(())
}
async fn browser_flow(
    db: &database::Database,
    reject_grant: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let app = compose(Router::new(), db.pool.clone(), mfa.clone(), layer, false);
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
    // A stored authorize target survives the challenge rotation and is the
    // post-MFA destination, consumed once.
    let target = "/oauth/authorize?response_type=code&client_id=c&state=s";
    sqlx::query("UPDATE web_sessions SET data=data||jsonb_build_object('oauth_return',$1::text,'referral.code','invite-code') WHERE account_id IS NULL")
        .bind(target)
        .execute(&db.pool)
        .await?;
    let stored = || {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT string_agg(data->>'oauth_return', ',') FROM web_sessions",
        )
        .fetch_one(&db.pool)
    };
    for _ in 0..4 {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::post("/en/login")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='mfa.http_refused'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 0, "anonymous mfa audit rows");
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
    assert_eq!(stored().await?.as_deref(), Some(target));
    let refused = app.clone().oneshot(challenge_request("bad")?).await?;
    assert_eq!(refused.status(), StatusCode::FOUND);
    assert_eq!(stored().await?.as_deref(), Some(target));
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='mfa.refused'",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(audited, 1);
    if reject_grant {
        sqlx::query("ALTER TABLE web_sessions ADD CHECK (COALESCE(data->>'mfa_grant','') = '')")
            .execute(&db.pool)
            .await?;
        let response = app.oneshot(challenge_request(&codes[0])?).await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let authenticated: i64 =
            sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
                .bind(id)
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(
            authenticated, 0,
            "failed grant must leave no authenticated session"
        );
        for value in response.headers().get_all(header::SET_COOKIE) {
            assert!(
                value.to_str()?.contains("Max-Age=0"),
                "failed login must withhold new cookies"
            );
        }
        return Ok(());
    }
    let accepted = app.clone().oneshot(challenge_request(&codes[0])?).await?;
    assert_eq!(accepted.status(), StatusCode::FOUND);
    assert_eq!(
        accepted.headers().get(header::LOCATION).ok_or("location")?,
        target
    );
    assert_eq!(stored().await?, None);
    assert!(
        accepted
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| v.starts_with(TRUSTED) && v.contains("Secure; HttpOnly; SameSite=Lax"))
    );
    let referral: Option<String> =
        sqlx::query_scalar("SELECT data->>'referral.code' FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(
        referral.as_deref(),
        Some("invite-code"),
        "pending referral after MFA login"
    );
    let grant: String =
        sqlx::query_scalar("SELECT data->>'mfa_grant' FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    assert!(mfa.satisfied(id, &grant).await?);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM security_events WHERE account_id=$1 AND kind='login_succeeded'"
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM email_outbox WHERE account_id=$1 AND category='security'"
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?,
        1
    );
    let authenticated_cookie = accepted
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with(session::COOKIE_NAME) && !v.contains("Max-Age=0"))
        .and_then(|v| v.split(';').next())
        .ok_or("authenticated cookie")?;
    let authenticated_csrf: String =
        sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?;
    let device = mfa.devices(id).await?.pop().ok_or("remembered device")?;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/en/mfa/devices")
                .header(header::COOKIE, authenticated_cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .extension(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>()?))
                .body(axum::body::Body::from(format!(
                    "_csrf={authenticated_csrf}&device={device}"
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(mfa.devices(id).await?.is_empty());
    let settings = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 32768)
            .await?
            .to_vec(),
    )?;
    assert!(
        !settings.contains("action=\"/en/mfa/enroll\""),
        "device removal offered enrollment"
    );
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
    Ok(())
}

#[tokio::test]
async fn postgres_mfa_limits_and_missing_transport() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let context = Context {
        native_enabled: false,
        mfa: Mfa::new(db.pool.clone(), [7; 32]),
        limiter: RateLimiter::new(db.pool.clone()),
    };
    let peer = Some(Extension(ConnectInfo(
        "127.0.0.1:4321".parse::<SocketAddr>()?,
    )));
    for _ in 0..5 {
        assert!(
            charge(
                &context.limiter,
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
        &context.limiter,
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
        &context.limiter,
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
        &context.limiter,
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
        false,
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
        false,
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
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM security_events WHERE account_id=$1 AND kind='login_succeeded'"
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM email_outbox WHERE account_id=$1 AND category='security'"
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await?,
        1
    );
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_mfa_http_refusal_records_only_the_session_account()
-> Result<(), Box<dyn std::error::Error>> {
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let app = compose(
        Router::new(),
        db.pool.clone(),
        Mfa::new(db.pool.clone(), [7; 32]),
        SessionLayer::new(db.pool.clone()),
        false,
    );
    let post = || {
        axum::http::Request::post("/en/mfa/devices")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
    };
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(post().body(axum::body::Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let rows = || {
        sqlx::query_scalar::<_, Option<uuid::Uuid>>(
            "SELECT actor FROM audit_log WHERE action='mfa.http_refused'",
        )
        .fetch_all(&db.pool)
    };
    assert!(rows().await?.is_empty());

    // The session is signed in and has valid CSRF, but lacks the MFA grant for device removal.
    let account: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,locale) VALUES('audit@example.test','unused','active','en') RETURNING id",
    ).fetch_one(&db.pool).await?;
    let cookie = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at)
         VALUES (sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')",
    ).bind(cookie.as_bytes()).bind(account).bind(&csrf).execute(&db.pool).await?;
    let response = app
        .oneshot(
            post()
                .header(header::COOKIE, format!("{}={cookie}", session::COOKIE_NAME))
                .extension(ConnectInfo("127.0.0.1:4321".parse::<SocketAddr>()?))
                .body(axum::body::Body::from(format!("_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let actors = rows().await?;
    assert_eq!(actors.len(), 1, "account-bound MFA HTTP refusal rows");
    assert_eq!(actors[0], Some(account));
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn r9_admin_origin_login_reaches_challenge() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("R9_ADMIN_LOGIN_TEST").is_none() {
        let result = std::process::Command::new(std::env::current_exe()?)
            .args(["r9_admin_origin_login_reaches_challenge", "--nocapture"])
            .env("R9_ADMIN_LOGIN_TEST", "1")
            .env("OTERYN_ACCOUNT_MFA", "true")
            .env("OTERYN_ACCOUNT_LOGIN", "true")
            .env("OTERYN_ADMIN_PANEL", "true")
            .env("OTERYN_ADMIN_HOST", "admin.example.test")
            .env("OTERYN_PUBLIC_ORIGIN", "https://portal.example.test")
            .env("OTERYN_MFA_KEY", "07".repeat(32))
            .status()?;
        assert!(result.success());
        return Ok(());
    }
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let app = oteryn_web::register::configured(configured(Router::new(), db.pool.clone())?);
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let id:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('browser@example.test',$1,'active','en',now()) RETURNING id").bind(oteryn_identity::password::hash("test password")?).fetch_one(&db.pool).await?;
    let enrollment = mfa.enroll(id, "test password").await?.ok_or("enrollment")?;
    let secret = totp_rs::Secret::Encoded(enrollment.secret).to_bytes()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let code = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, secret)?.generate(now);
    let _codes = mfa
        .confirm(id, "test password", &code)
        .await?
        .ok_or("confirm")?;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/login")
                .header(header::HOST, "admin.example.test")
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
            .uri("/login")
            .header(header::HOST, "admin.example.test")
            .header(header::COOKIE, &cookie)
            .header(header::ORIGIN, "https://admin.example.test")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .extension(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>()?))
            .body(axum::body::Body::from(format!(
                "_csrf={token}&email=browser%40example.test&password=test+password"
            )))?)
    };
    let mut wrong_host = request(csrf)?;
    wrong_host
        .headers_mut()
        .insert(header::HOST, "portal.example.test".parse()?);
    assert_eq!(
        app.clone().oneshot(wrong_host).await?.status(),
        StatusCode::FORBIDDEN
    );
    let response = app.oneshot(request(csrf)?).await?;
    assert!(
        response.status().is_redirection(),
        "expected challenge redirect, got {}",
        response.status()
    );
    assert!(
        response.headers()["location"]
            .to_str()?
            .contains("/mfa/challenge")
    );
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn authenticated_challenge_routes_redirect_home() -> Result<(), Box<dyn std::error::Error>> {
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let account: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale) VALUES('authenticated@example.test','unused','active','en') RETURNING id")
        .fetch_one(&db.pool).await?;
    let cookie = "c".repeat(64);
    let csrf = "d".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES (sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
        .bind(cookie.as_bytes()).bind(account).bind(&csrf).execute(&db.pool).await?;
    let app = compose(
        Router::new(),
        db.pool.clone(),
        Mfa::new(db.pool.clone(), [7; 32]),
        SessionLayer::new(db.pool.clone()),
        false,
    );
    for (method, path) in [
        ("GET", "challenge"),
        ("POST", "challenge"),
        ("POST", "email"),
    ] {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(format!("/en/mfa/{path}"))
                    .header(header::COOKIE, format!("{}={cookie}", session::COOKIE_NAME))
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .extension(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>()?))
                    .body(axum::body::Body::from(format!("_csrf={csrf}")))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers()[header::LOCATION], "/en/");
    }
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn panel_mfa_management_checks_live_owner_and_proof() -> Result<(), Box<dyn std::error::Error>>
{
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let password = "panel test password";
    let account: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('panel-mfa@example.test',$1,'active','en',now()) RETURNING id")
        .bind(oteryn_identity::password::hash(password)?).fetch_one(&db.pool).await?;
    let mfa = Mfa::new(db.pool.clone(), [7; 32]);
    let original = mfa.enroll(account, password).await?.ok_or("enrollment")?;
    let factor = |secret| -> Result<totp_rs::TOTP, Box<dyn std::error::Error>> {
        let bytes = totp_rs::Secret::Encoded(secret).to_bytes()?;
        Ok(totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            bytes,
        )?)
    };
    let original = factor(original.secret)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    mfa.confirm(account, password, &original.generate(now - 30))
        .await?
        .ok_or("confirmation")?;
    let app = compose(
        Router::new(),
        db.pool.clone(),
        mfa.clone(),
        SessionLayer::new(db.pool.clone()),
        false,
    );
    let create_session = |cookie: String, csrf: String| async {
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
            .bind(cookie.into_bytes()).bind(account).bind(csrf).execute(&db.pool).await
    };
    let send =
        |path: &'static str, cookie: String, csrf: String, fields: Vec<(&'static str, String)>| {
            let app = app.clone();
            async move {
                let mut fields = fields;
                fields.push(("_csrf", csrf));
                let body = serde_urlencoded::to_string(fields)?;
                let request = axum::http::Request::post(path)
                    .header(header::COOKIE, format!("{}={cookie}", session::COOKIE_NAME))
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .extension(ConnectInfo("127.0.0.1:4322".parse::<SocketAddr>()?))
                    .body(axum::body::Body::from(body))?;
                Ok::<_, Box<dyn std::error::Error>>(app.oneshot(request).await?)
            }
        };
    let manage = "/en/mfa/manage";
    let confirm = "/en/mfa/replace/confirm";
    let cookie = "a".repeat(64);
    let csrf = "b".repeat(64);
    create_session(cookie.clone(), csrf.clone()).await?;
    let proof = |pass: &str, code: String| vec![("password", pass.to_owned()), ("code", code)];
    let action = |name: &str, pass: &str, code: String| {
        let mut fields = proof(pass, code);
        fields.push(("action", name.to_owned()));
        fields
    };
    assert_eq!(
        send(
            manage,
            cookie.clone(),
            "x".repeat(64),
            action("replace", password, original.generate(now))
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            manage,
            cookie.clone(),
            csrf.clone(),
            action("replace", password, "bad".into())
        )
        .await?
        .status(),
        StatusCode::BAD_REQUEST
    );
    let refusal = send(
        manage,
        cookie.clone(),
        csrf.clone(),
        action("replace", "wrong", original.generate(now)),
    )
    .await?;
    assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(refusal.into_body(), 32768).await?;
    let html = std::str::from_utf8(&body)?;
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("href=\"/en/mfa\">Back to security settings"));
    let response = send(
        manage,
        cookie.clone(),
        csrf.clone(),
        action("replace", password, original.generate(now)),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 32768)
            .await?
            .to_vec(),
    )?;
    let replacement = factor(
        body.split("class=\"op-prose\">")
            .nth(1)
            .and_then(|tail| tail.split('<').next())
            .ok_or("new authenticator")?
            .into(),
    )?;
    assert_eq!(mfa.remaining_codes(account).await?, 8);
    assert_eq!(
        send(
            confirm,
            cookie.clone(),
            csrf.clone(),
            proof(password, "bad".into())
        )
        .await?
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            confirm,
            cookie.clone(),
            csrf.clone(),
            proof("wrong", replacement.generate(now - 30))
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            confirm,
            cookie.clone(),
            csrf.clone(),
            proof(password, replacement.generate(now - 30))
        )
        .await?
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        send(
            manage,
            cookie,
            csrf,
            action("rotate_codes", password, replacement.generate(now))
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );

    sqlx::raw_sql(
        "CREATE SEQUENCE rotation_delete_attempts;
         CREATE FUNCTION forbid_second_session_delete() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             IF nextval('rotation_delete_attempts') > 1 THEN
                 RAISE EXCEPTION 'session deletion already committed';
             END IF;
             RETURN NULL;
         END $$;
         CREATE TRIGGER forbid_second_session_delete BEFORE DELETE ON web_sessions
           FOR EACH STATEMENT EXECUTE FUNCTION forbid_second_session_delete();",
    )
    .execute(&db.pool)
    .await?;
    let cookie = "c".repeat(64);
    let csrf = "d".repeat(64);
    create_session(cookie.clone(), csrf.clone()).await?;
    let response = send(
        manage,
        cookie,
        csrf,
        action("rotate_codes", password, replacement.generate(now)),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .contains("Max-Age=0")
    );
    let html = axum::body::to_bytes(response.into_body(), 32768).await?;
    assert_eq!(std::str::from_utf8(&html)?.matches("<textarea ").count(), 8);
    let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM rotation_delete_attempts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(attempts, 1);
    assert_eq!(mfa.remaining_codes(account).await?, 8);
    sqlx::query("DROP TRIGGER forbid_second_session_delete ON web_sessions")
        .execute(&db.pool)
        .await?;
    let cookie = "e".repeat(64);
    let csrf = "f".repeat(64);
    create_session(cookie.clone(), csrf.clone()).await?;
    assert_eq!(
        send(
            manage,
            cookie,
            csrf,
            action("remove", password, replacement.generate(now + 30))
        )
        .await?
        .status(),
        StatusCode::FOUND
    );
    assert!(!mfa.enrolled(account).await?);
    assert_eq!(mfa.remaining_codes(account).await?, 0);
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn panel_native_revoke_checks_recent_session_and_family_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let db = database::Database::open()
        .await?
        .ok_or("PostgreSQL required")?;
    let accounts: Vec<uuid::Uuid> = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('native-owner@example.test','unused','active','en',now()),('native-other@example.test','unused','active','en',now()) RETURNING id").fetch_all(&db.pool).await?;
    let cookie = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256($1::bytea),$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
        .bind(cookie.as_bytes()).bind(accounts[0]).bind(&csrf).execute(&db.pool).await?;
    let families = [uuid::Uuid::now_v7(), uuid::Uuid::now_v7()];
    for (account, family) in accounts.iter().zip(families) {
        sqlx::query("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES($1,$2,uuidv7(),$3,0,1,extract(epoch FROM now())::bigint+3600,extract(epoch FROM now())::bigint+1800)")
            .bind(family).bind(account).bind(family.to_string()).execute(&db.pool).await?;
    }
    let app = compose(
        Router::new(),
        db.pool.clone(),
        Mfa::new(db.pool.clone(), [7; 32]),
        SessionLayer::new(db.pool.clone()),
        true,
    );
    let send = |family: String, token: &str| {
        let app = app.clone();
        let cookie = cookie.clone();
        let token = token.to_owned();
        async move {
            let request = axum::http::Request::post("/en/mfa/native-devices")
                .header(header::COOKIE, format!("{}={cookie}", session::COOKIE_NAME))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .extension(ConnectInfo("127.0.0.1:4323".parse::<SocketAddr>()?))
                .body(axum::body::Body::from(format!(
                    "_csrf={token}&family={family}"
                )))?;
            Ok::<_, Box<dyn std::error::Error>>(app.oneshot(request).await?)
        }
    };
    assert_eq!(
        send(families[0].to_string(), &csrf).await?.status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query(
        "UPDATE web_sessions SET sudo_until=now()+interval '5 minutes' WHERE account_id=$1",
    )
    .bind(accounts[0])
    .execute(&db.pool)
    .await?;
    assert_eq!(
        send(families[0].to_string(), "bad").await?.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send("malformed".into(), &csrf).await?.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        send(families[1].to_string(), &csrf).await?.status(),
        StatusCode::FORBIDDEN
    );
    let response = send(families[0].to_string(), &csrf).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(response.headers().contains_key(header::SET_COOKIE));
    let revoked: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM device_sessions WHERE revoked_at IS NOT NULL")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(revoked, [families[0]]);
    assert_eq!(
        send(families[0].to_string(), &csrf).await?.status(),
        StatusCode::FORBIDDEN
    );
    db.close().await?;
    Ok(())
}
