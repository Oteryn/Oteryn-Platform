//! Production composition regressions. Child processes isolate startup-only env.
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use sha2::{Digest, Sha256};
use std::{error::Error, sync::Arc, time::Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tower::ServiceExt;
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

async fn smtp_session<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: S,
    greet: bool,
    messages: Arc<std::sync::Mutex<Vec<String>>>,
) -> Result {
    let mut io = BufReader::new(stream);
    if greet {
        io.get_mut().write_all(b"220 localhost ESMTP\r\n").await?;
    }
    let mut data = false;
    let mut message = String::new();
    loop {
        let mut line = String::new();
        if io.read_line(&mut line).await? == 0 {
            return Ok(());
        }
        if data {
            if line == ".\r\n" {
                messages
                    .lock()
                    .map_err(|_| "capture lock")?
                    .push(std::mem::take(&mut message));
                data = false;
                io.get_mut().write_all(b"250 accepted\r\n").await?;
            } else {
                message.push_str(&line);
            }
            continue;
        }
        let reply: &[u8] = if line.starts_with("EHLO") {
            b"250-localhost\r\n250 AUTH PLAIN\r\n"
        } else if line.starts_with("AUTH") {
            b"235 authenticated\r\n"
        } else if line.starts_with("DATA") {
            data = true;
            b"354 send data\r\n"
        } else if line.starts_with("QUIT") {
            io.get_mut().write_all(b"221 bye\r\n").await?;
            return Ok(());
        } else {
            b"250 OK\r\n"
        };
        io.get_mut().write_all(reply).await?;
    }
}
async fn fixture(name: &str, mode: &str) -> Result {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let temp = tempfile::tempdir()?;
    let ca = temp.path().join("smtp.pem");
    std::fs::write(&ca, cert.cert.pem())?;
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(vec![cert.cert.der().clone()], key.into())?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let messages = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = messages.clone();
    let implicit = mode == "implicit";
    let server = tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            let messages = capture.clone();
            tokio::spawn(async move {
                let result: Result = async {
                    if implicit {
                        smtp_session(acceptor.accept(socket).await?, true, messages).await
                    } else {
                        let mut io = BufReader::new(socket);
                        io.get_mut().write_all(b"220 localhost ESMTP\r\n").await?;
                        let mut line = String::new();
                        io.read_line(&mut line).await?;
                        if !line.starts_with("EHLO") {
                            return Err("expected EHLO".into());
                        }
                        io.get_mut()
                            .write_all(b"250-localhost\r\n250 STARTTLS\r\n")
                            .await?;
                        line.clear();
                        io.read_line(&mut line).await?;
                        if line != "STARTTLS\r\n" {
                            return Err("expected STARTTLS".into());
                        }
                        io.get_mut().write_all(b"220 start TLS\r\n").await?;
                        smtp_session(acceptor.accept(io.into_inner()).await?, false, messages).await
                    }
                }
                .await;
                if let Err(error) = result {
                    eprintln!("SMTP fixture: {error}");
                }
            });
        }
    });
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .args(["--exact", &format!("audit_l9_tests::{name}"), "--nocapture"])
        .env("L9_CHILD", "true")
        .env("OTERYN_ENV", "testing")
        .env(
            "OTERYN_TERMS_VERSION",
            oteryn_identity::consents::DEFAULT_LEGAL_VERSION,
        )
        .env(
            "OTERYN_PRIVACY_VERSION",
            oteryn_identity::consents::DEFAULT_LEGAL_VERSION,
        )
        .env("TRUSTED_PROXIES", "10.0.0.1")
        .env("OTERYN_SMTP_HOST", "localhost")
        .env("OTERYN_SMTP_PORT", port.to_string())
        .env("OTERYN_SMTP_TLS_MODE", mode)
        .env("OTERYN_SMTP_USERNAME", "fixture")
        .env("OTERYN_SMTP_PASSWORD", "fixture")
        .env("OTERYN_MAIL_FROM", "portal@example.test")
        .env("OTERYN_SMTP_FROM", "portal@example.test")
        .env("SSL_CERT_FILE", ca)
        .env("OTERYN_PUBLIC_ORIGIN", "https://portal.example.test")
        .env("OTERYN_ACCOUNT_CREDENTIALS", "true")
        .env("OTERYN_ACCOUNT_BADGES_ENABLED", "true")
        .env("OTERYN_ACCOUNT_SESSIONS", "false")
        .env("OTERYN_ACCOUNT_LOGIN", "false")
        .env("OTERYN_ACCOUNT_PASSKEYS", "false");
    if matches!(
        name,
        "production_registration_account_exists" | "production_legal_signup_refusals"
    ) {
        command
            .env("OTERYN_ENV", "production")
            .env_remove("OTERYN_TERMS_VERSION")
            .env_remove("OTERYN_PRIVACY_VERSION");
    }
    if name == "missing_mail_refuses_admission" {
        command.env_remove("OTERYN_SMTP_HOST");
    }
    if name == "production_smtp_senders_both_tls_modes" {
        command
            .env("OTERYN_ACCOUNT_SESSIONS", "true")
            .env("OTERYN_ACCOUNT_PASSWORD_RESET", "true")
            .env("OTERYN_RESET_DELIVERY_KEY", "11".repeat(32));
    }
    if name == "production_suspended_report_notification" {
        command
            .env("OTERYN_ACCOUNT_SESSIONS", "true")
            .env("OTERYN_MFA_KEY", "07".repeat(32))
            .env("OTERYN_ADMIN_PANEL", "true")
            .env("OTERYN_ADMIN_HOST", "staff.test")
            .env("OTERYN_REPORTS_ENABLED", "true")
            .env("OTERYN_NOTIFY_UNSUBSCRIBE_KEY", "07".repeat(32))
            .env(
                "OTERYN_NOTIFY_UNSUBSCRIBE_URL",
                "https://portal.example.test/unsubscribe",
            );
    }
    let output = tokio::task::spawn_blocking(move || command.output()).await??;
    server.abort();
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "production child failed: {name} ({mode})"
    );
    let emails = messages.lock().map_err(|_| "capture lock")?;
    let count = emails.len();
    if name == "production_registration_account_exists" {
        assert!(
            emails
                .iter()
                .any(|email| email.contains("existing@example.test")
                    && email.contains("Account already exists"))
        );
    }
    if name == "production_credentials_notification" {
        assert!(
            emails
                .iter()
                .any(|email| email.contains("password@example.test"))
        );
    }
    if name == "production_suspended_report_notification" {
        assert!(
            emails
                .iter()
                .any(|email| email.contains("reporter@example.test") && !email.contains("Private"))
        );
    }
    if name == "production_smtp_senders_both_tls_modes" {
        for fragment in ["/verify-email?token=", "/reset-password/", "123456"] {
            assert!(
                emails
                    .iter()
                    .any(|email| email.replace("=\r\n", "").contains(fragment)),
                "missing production message: {fragment}"
            );
        }
    }

    if name.contains("registration") || name.contains("credentials") || name.contains("suspended") {
        assert_eq!(count, 1);
    }
    if name == "production_smtp_senders_both_tls_modes" {
        assert_eq!(count, 4);
    }
    Ok(())
}
async fn empty_database() -> Result<crate::reset_http::tests::db::Database> {
    crate::reset_http::tests::db::Database::open()
        .await?
        .ok_or_else(|| "PORTAL_TEST_DATABASE_URL required; no skipped audit tests".into())
}
async fn database() -> Result<crate::reset_http::tests::db::Database> {
    let db = empty_database().await?;
    oteryn_db::test_support::seed_final_signup_editions(
        &db.pool,
        oteryn_identity::consents::DEFAULT_LEGAL_VERSION,
    )
    .await?;
    Ok(db)
}
fn app(pool: sqlx::PgPool) -> axum::Router {
    oteryn_web::register::configured(oteryn_web::router(
        oteryn_web::AppState { db: pool.clone() },
        "static",
    ))
    .merge(crate::credentials_http::configured_router(pool))
}
async fn account(pool: &sqlx::PgPool, email: &str) -> Result<uuid::Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at,created_at) VALUES($1,$2,'en','active',now(),now()-interval '3 years') RETURNING id")
        .bind(email).bind(oteryn_identity::password::hash("old password")?).fetch_one(pool).await?)
}
async fn browser(pool: &sqlx::PgPool, id: Option<uuid::Uuid>) -> Result<(String, String)> {
    let raw = uuid::Uuid::now_v7().simple().to_string().repeat(2);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,authorization_generation) VALUES($1,$2,$3,'audit',now()+interval '1 hour',now()+interval '1 day',(SELECT native_security_generation FROM accounts WHERE id=$2))")
        .bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind(&csrf).execute(pool).await?;
    Ok((format!("{}={raw}", oteryn_web::session::COOKIE_NAME), csrf))
}
fn request(
    method: &str,
    path: &str,
    browser: &(String, String),
    peer: &str,
    source: &str,
    body: &str,
) -> Result<Request<Body>> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, &browser.0)
        .header("x-csrf-token", &browser.1)
        .header("x-forwarded-for", source)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))?;
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<std::net::SocketAddr>()?));
    Ok(request)
}
async fn registration_body(
    app: &axum::Router,
    pool: &sqlx::PgPool,
    browser: &(String, String),
    email: &str,
    source: &str,
) -> Result<String> {
    // Complete, empty fixture range for the exact test password's SHA-1 prefix.
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES($1,ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id()) ON CONFLICT(prefix) DO NOTHING")
        .bind("AEB30").execute(pool).await?;
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/en/register/abuse/challenge",
            browser,
            "10.0.0.1:1234",
            source,
            "",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let challenge: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    let token = challenge["token"].as_str().ok_or("challenge token")?;
    let nonce = (0..10_000_000_u64)
        .map(|n| n.to_string())
        .find(|nonce| oteryn_identity::abuse::valid_solution(token, nonce))
        .ok_or("proof search exhausted")?;
    let consent = oteryn_identity::consents::ConsentConfig::default();
    Ok(serde_urlencoded::to_string([
        ("_csrf", browser.1.as_str()),
        ("work_token", token),
        ("work_nonce", &nonce),
        ("email", email),
        ("password", "valid password"),
        ("password_confirmation", "valid password"),
        ("terms", &consent.terms_version),
        ("privacy", &consent.privacy_version),
    ])?)
}
async fn drain(pool: &sqlx::PgPool) -> Result {
    let registry = crate::worker::configured(pool.clone()).await?;
    assert!(registry.batch(pool).await? > 0);
    let failed: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox WHERE state <> 'done'")
        .fetch_one(pool)
        .await?;
    assert_eq!(failed, 0);
    Ok(())
}
#[tokio::test]
async fn production_proxy_budgets() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_proxy_budgets", "starttls").await;
    }
    let start = Instant::now();
    let db = database().await?;
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    let app = app(db.pool.clone());
    let browser = browser(&db.pool, None).await?;
    for i in 0..5 {
        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(request(
                    "POST",
                    "/en/register",
                    &browser,
                    "10.0.0.1:1234",
                    &format!("192.0.2.{}", i + 1),
                    "",
                )?)
                .await?;
            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
    }
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/en/register",
                &browser,
                "10.0.0.1:1234",
                "192.0.2.1",
                ""
            )?)
            .await?
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    for i in 0..5 {
        assert_ne!(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/pl/register",
                    &browser,
                    "198.51.100.9:1234",
                    &format!("203.0.113.{i}"),
                    ""
                )?)
                .await?
                .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/pl/register",
                &browser,
                "198.51.100.9:1234",
                "203.0.113.99",
                ""
            )?)
            .await?
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let body = registration_body(
        &app,
        &db.pool,
        &browser,
        "new-client@example.test",
        "192.0.2.99",
    )
    .await?;
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/en/register",
                &browser,
                "10.0.0.1:1234",
                "192.0.2.99",
                &body
            )?)
            .await?
            .status(),
        StatusCode::SEE_OTHER
    );
    db.close().await?;
    eprintln!(
        "PG production_proxy_budgets {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn production_legal_signup_refusals() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_legal_signup_refusals", "starttls").await;
    }
    assert_eq!(std::env::var("OTERYN_ENV")?, "production");
    use oteryn_db::test_support::{APPROVED_SIGNUP_EDITION, seed_signup_editions};
    use oteryn_identity::consents::{ConsentConfig, DEFAULT_LEGAL_VERSION};

    let legacy = (APPROVED_SIGNUP_EDITION.0, APPROVED_SIGNUP_EDITION.1, false);
    let fictional = ("Tax ID (NIP) (000) 000-00-00", "NIP (000) 000-00-00", true);
    let fictional_image = (
        "Service terms.\n\n![Tax ID (NIP) 000-000-00-00](/missing-image.png)",
        "Warunki świadczenia usług.\n\n<img src='/missing-image.png' alt='NIP 000-000-00-00'>",
        true,
    );
    let invalid_nip = ("Tax ID (NIP) PL 123-456-78-03", "NIP 123-456-78-03", true);
    let blank = ("<!-- Draft terms -->", "<!-- Projekt regulaminu -->", true);
    let invisible = ("&#xfe0f;", "&#x034f;", true);
    let allow_placeholders = ConsentConfig::published(
        DEFAULT_LEGAL_VERSION.into(),
        DEFAULT_LEGAL_VERSION.into(),
        true,
    );
    for (reason, refused, has_visible_body) in [
        ("fictional operator", fictional, true),
        (
            "fictional operator in image alternative",
            fictional_image,
            true,
        ),
        ("invalid operator NIP", invalid_nip, true),
        ("unapproved legacy snapshot", legacy, true),
        ("empty sanitized body", blank, false),
        ("invisible sanitized body", invisible, false),
    ] {
        for slug in ["terms", "privacy"] {
            let (terms, privacy) = if slug == "terms" {
                (refused, APPROVED_SIGNUP_EDITION)
            } else {
                (APPROVED_SIGNUP_EDITION, refused)
            };
            let db = empty_database().await?;
            seed_signup_editions(&db.pool, DEFAULT_LEGAL_VERSION, terms, privacy).await?;
            sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
                .execute(&db.pool)
                .await?;
            let app = app(db.pool.clone());
            let browser = browser(&db.pool, None).await?;
            for locale in ["en", "pl"] {
                if has_visible_body {
                    // Both live rows match their immutable snapshots. Only the
                    // production approval gate can reject these editions.
                    allow_placeholders
                        .check_publication(&db.pool, locale)
                        .await?;
                }
                let path = format!("/{locale}/register");
                let peer = "10.0.0.1:1234";
                let source = "192.0.2.1";
                let page = app
                    .clone()
                    .oneshot(request("GET", &path, &browser, peer, source, "")?)
                    .await?;
                assert_eq!(
                    page.status(),
                    StatusCode::SERVICE_UNAVAILABLE,
                    "{locale} {slug} {reason}"
                );
                let body =
                    registration_body(&app, &db.pool, &browser, "new@example.test", source).await?;
                let response = app
                    .clone()
                    .oneshot(request("POST", &path, &browser, peer, source, &body)?)
                    .await?;
                assert_eq!(
                    response.status(),
                    StatusCode::SERVICE_UNAVAILABLE,
                    "{locale} {slug} {reason}"
                );
                let counts: (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM accounts),
                            (SELECT count(*) FROM account_consents),
                            (SELECT count(*) FROM outbox)",
                )
                .fetch_one(&db.pool)
                .await?;
                assert_eq!(counts, (0, 0, 0), "{locale} {slug} {reason}");
            }
            db.close().await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn production_registration_account_exists() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_registration_account_exists", "starttls").await;
    }
    assert_eq!(std::env::var("OTERYN_ENV")?, "production");
    let start = Instant::now();
    let db = database().await?;
    account(&db.pool, "existing@example.test").await?;
    let browser = browser(&db.pool, None).await?;
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    let app = app(db.pool.clone());
    let body = registration_body(
        &app,
        &db.pool,
        &browser,
        "existing@example.test",
        "192.0.2.1",
    )
    .await?;
    let response = app
        .oneshot(request(
            "POST",
            "/en/register",
            &browser,
            "10.0.0.1:1234",
            "192.0.2.1",
            &body,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers()[header::LOCATION],
        "/en/register/check-inbox"
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox WHERE kind='email.account_exists'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    drain(&db.pool).await?;
    db.close().await?;
    eprintln!(
        "PG production_registration_account_exists {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn production_credentials_notification() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_credentials_notification", "starttls").await;
    }
    let start = Instant::now();
    let db = database().await?;
    let id = account(&db.pool, "password@example.test").await?;
    let browser = browser(&db.pool, Some(id)).await?;
    // Complete, empty breach-cache range for "next password" (SHA-1 prefix E6AC2).
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES('E6AC2',ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id()) ON CONFLICT(prefix) DO NOTHING").execute(&db.pool).await?;
    let response=app(db.pool.clone()).oneshot(request("POST","/en/password/change",&browser,"10.0.0.1:1234","192.0.2.1","current_password=old+password&password=next+password&password_confirmation=next+password")?).await?;
    assert_eq!(response.status(), StatusCode::FOUND);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM email_outbox WHERE category='security'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(count, 1);
    drain(&db.pool).await?;
    db.close().await?;
    eprintln!(
        "PG production_credentials_notification {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn production_badge_redirects() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_badge_redirects", "starttls").await;
    }
    let start = Instant::now();
    let db = database().await?;
    let id = account(&db.pool, "badges@example.test").await?;
    let browser = browser(&db.pool, Some(id)).await?;
    let app = app(db.pool.clone());
    for lang in ["en", "pl"] {
        let mut submission = request(
            "POST",
            "/account/badges",
            &browser,
            "10.0.0.1:1234",
            "192.0.2.1",
            "badge=account-age",
        )?;
        submission
            .headers_mut()
            .insert(header::ACCEPT_LANGUAGE, lang.parse()?);
        let response = app.clone().oneshot(submission).await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let target = response.headers()[header::LOCATION].to_str()?.to_owned();
        assert_eq!(target, format!("/{lang}/account/badges"));
        let response = app
            .clone()
            .oneshot(request(
                "GET",
                &target,
                &browser,
                "10.0.0.1:1234",
                "192.0.2.1",
                "",
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
        assert!(html.contains("account-age"));
    }
    db.close().await?;
    eprintln!(
        "PG production_badge_redirects {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn production_suspended_report_notification() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture("production_suspended_report_notification", "starttls").await;
    }
    let start = Instant::now();
    let db = database().await?;
    let owner = account(&db.pool, "reporter@example.test").await?;
    let staff = account(&db.pool, "staff@example.test").await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,'support',true,true)").bind(owner).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('support','moderator')").bind(staff).execute(&db.pool).await?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(staff)
        .fetch_one(&db.pool)
        .await?;
    let grant = oteryn_identity::mfa::random_token()?;
    let generation:uuid::Uuid=sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true) RETURNING generation").bind(staff).fetch_one(&db.pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '1 hour')").bind(oteryn_identity::mfa::digest(&grant)).bind(staff).bind(generation).bind(oteryn_identity::mfa::digest(&hash)).execute(&db.pool).await?;
    let browser = browser(&db.pool, Some(staff)).await?;
    let raw = browser.0.split('=').nth(1).ok_or("cookie")?;
    sqlx::query("UPDATE web_sessions SET data=$2 WHERE id_hash=$1")
        .bind(Sha256::digest(raw.as_bytes()).to_vec())
        .bind(serde_json::json!({"mfa_grant":grant}))
        .execute(&db.pool)
        .await?;
    let report:uuid::Uuid=sqlx::query_scalar("INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference) VALUES($1,$2,'player','conduct','Reported player') RETURNING id").bind(owner).bind(uuid::Uuid::now_v7()).fetch_one(&db.pool).await?;
    let app = oteryn_support::tickets::configured_staff_router(db.pool.clone())?;
    let mut request = request(
        "POST",
        &format!("/admin/moderation/reports/{report}"),
        &browser,
        "10.0.0.1:1234",
        "192.0.2.1",
        "lock_version=1&action=moderate&status=rejected&public_outcome=Reviewed&moderator_notes=Private",
    )?;
    request
        .headers_mut()
        .insert(header::HOST, "staff.test".parse()?);
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM email_outbox WHERE category='support' AND text_body NOT LIKE '%Private%'").fetch_one(&db.pool).await?;
    assert_eq!(count, 1);
    drain(&db.pool).await?;
    use oteryn_notify::{AccountState, Category};
    let resolver = oteryn_identity::sessions::NotificationAccounts(db.pool.clone());
    assert!(
        resolver
            .recipient(owner, Category::Security)
            .await
            .map_err(|e| e.to_string())?
            .is_none()
    );
    assert!(
        resolver
            .recipient(owner, Category::Support)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
    );
    sqlx::query("UPDATE accounts SET status='closed' WHERE id=$1")
        .bind(owner)
        .execute(&db.pool)
        .await?;
    assert!(
        resolver
            .recipient(owner, Category::Support)
            .await
            .map_err(|e| e.to_string())?
            .is_none()
    );
    db.close().await?;
    eprintln!(
        "PG production_suspended_report_notification {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn production_smtp_senders_both_tls_modes() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        fixture("production_smtp_senders_both_tls_modes", "starttls").await?;
        return fixture("production_smtp_senders_both_tls_modes", "implicit").await;
    }
    let start = Instant::now();
    let db = database().await?;
    let active = account(&db.pool, "smtp@example.test").await?;
    let pending:uuid::Uuid=sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status) VALUES('verify@example.test','unused','en','pending_verification') RETURNING id").fetch_one(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    oteryn_identity::verify::issue(&mut tx, pending, "en").await?;
    let notifications = oteryn_identity::sessions::security_notifications(db.pool.clone())
        .map_err(|e| e.to_string())?;
    notifications
        .enqueue(
            &mut tx,
            active,
            &oteryn_notify::EmailTemplate::PasswordChanged,
        )
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await?;
    let key = oteryn_identity::reset::DeliveryKey::parse(&"11".repeat(32)).ok_or("reset key")?;
    oteryn_identity::reset::request(&db.pool, "smtp@example.test", &key).await?;
    drain(&db.pool).await?;
    crate::mfa_http::send_email(
        "localhost",
        "portal@example.test",
        "smtp@example.test",
        "MFA code".into(),
        "123456".into(),
    )
    .await
    .map_err(|_| "MFA send failed")?;
    db.close().await?;
    eprintln!(
        "PG production_smtp_senders_both_tls_modes {} {:.3}s",
        std::env::var("OTERYN_SMTP_TLS_MODE")?,
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn missing_mail_refuses_admission() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        fixture("missing_mail_refuses_admission", "starttls").await?;
        // Restart with complete delivery configuration and dispatch through real TLS SMTP.
        return fixture("production_registration_account_exists", "starttls").await;
    }
    let start = Instant::now();
    let db = database().await?;
    let browser = browser(&db.pool, None).await?;
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    let app = crate::registration_delivery::admission(app(db.pool.clone()), db.pool.clone());
    let body = registration_body(
        &app,
        &db.pool,
        &browser,
        "delivery-unavailable@example.test",
        "192.0.2.90",
    )
    .await?;
    assert_eq!(
        app.oneshot(request(
            "POST",
            "/en/register",
            &browser,
            "10.0.0.1:1234",
            "192.0.2.90",
            &body
        )?)
        .await?
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!((accounts, jobs), (0, 0));
    assert!(crate::worker::configured(db.pool.clone()).await.is_err());
    db.close().await?;
    eprintln!(
        "PG missing_mail_refuses_admission {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn browser_account_source_budgets_separate_proxy_clients() -> Result {
    if std::env::var("L9_CHILD").is_err() {
        return fixture(
            "browser_account_source_budgets_separate_proxy_clients",
            "starttls",
        )
        .await;
    }
    let start = Instant::now();
    let db = database().await?;
    let id = account(&db.pool, "budget@example.test").await?;
    let browser = browser(&db.pool, Some(id)).await?;
    let app = app(db.pool.clone());
    for _ in 0..5 {
        let response = app.clone().oneshot(request("POST", "/en/password/change", &browser, "10.0.0.1:1234", "192.0.2.1", "current_password=incorrect&password=next+password&password_confirmation=next+password")?).await?;
        assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    for (source, expected) in [("192.0.2.1", true), ("192.0.2.2", false)] {
        let response = app.clone().oneshot(request("POST", "/en/password/change", &browser, "10.0.0.1:1234", source, "current_password=incorrect&password=next+password&password_confirmation=next+password")?).await?;
        assert_eq!(
            response.status() == StatusCode::TOO_MANY_REQUESTS,
            expected,
            "{source}"
        );
    }
    // Untrusted sockets cannot split their budget by changing forwarding headers.
    for n in 0..5 {
        let response = app.clone().oneshot(request("POST", "/en/password/change", &browser, "198.51.100.7:1234", &format!("203.0.113.{n}"), "current_password=incorrect&password=next+password&password_confirmation=next+password")?).await?;
        assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    assert_eq!(
        app.oneshot(request(
            "POST",
            "/en/password/change",
            &browser,
            "198.51.100.7:1234",
            "203.0.113.99",
            "current_password=incorrect&password=next+password&password_confirmation=next+password"
        )?)
        .await?
        .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    db.close().await?;
    eprintln!(
        "PG browser_account_source_budgets_separate_proxy_clients {:.3}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
