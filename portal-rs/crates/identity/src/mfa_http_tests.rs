#[path = "mfa_test_db.rs"]
mod database;
use super::*;
use tower::ServiceExt;

#[test]
fn challenge_links_localized_retry_guidance_to_the_code_field() -> Result<(), askama::Error> {
    for (language, message) in [
        (Language::En, "The code could not be verified."),
        (Language::Pl, "Nie udało się potwierdzić kodu."),
    ] {
        let mut page = Page {
            lang: language.code(),
            language,
            csrf: String::new(),
            mode: "challenge",
            failed: true,
            passkeys_enabled: false,
            secret: String::new(),
            codes: vec![],
            devices: vec![],
        };
        let rendered = page.render()?;
        assert!(rendered.contains(&format!("role=\"alert\" id=\"mfa-error\">{message}")));
        assert!(rendered.contains("aria-describedby=\"mfa-error\" aria-invalid=\"true\""));
        page.failed = false;
        let rendered = page.render()?;
        assert!(!rendered.contains("id=\"mfa-error\""));
        assert!(!rendered.contains("aria-invalid"));
    }
    Ok(())
}

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
            .env("OTERYN_ACCOUNT_PASSKEYS", "true")
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
    let public = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/en/login")
                .header(header::HOST, "portal.example.test")
                .body(axum::body::Body::empty())?,
        )
        .await?;
    assert_eq!(public.status(), StatusCode::OK);
    let public = axum::body::to_bytes(public.into_body(), 32768).await?;
    assert!(std::str::from_utf8(&public)?.contains("href=\"/en/account/passkeys\""));

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
    assert!(!body.contains("/account/passkeys"));
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
