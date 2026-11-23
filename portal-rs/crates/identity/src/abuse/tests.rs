use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!(
                "SKIPPED registration abuse PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset"
            );
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("abuse_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = format!("{schema}, pg_catalog");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        oteryn_db::test_support::seed_final_signup_editions(
            &pool,
            crate::consents::DEFAULT_LEGAL_VERSION,
        )
        .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    // Keep both identity's clock_timestamp budget and the web route's
    // statement_timestamp budget in the same controlled minute. These functions
    // exist only in this test's schema, ahead of pg_catalog on its search path.
    async fn freeze_budget_clock(&self) -> TestResult {
        sqlx::query("CREATE TABLE test_budget_clock AS SELECT clock_timestamp() AS instant")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE FUNCTION clock_timestamp() RETURNS timestamptz LANGUAGE sql AS 'SELECT instant FROM test_budget_clock'")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE FUNCTION statement_timestamp() RETURNS timestamptz LANGUAGE sql AS 'SELECT instant FROM test_budget_clock'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    async fn enable(&self) -> TestResult {
        sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    async fn range(&self, password: &str, breached: bool) -> TestResult {
        let digest = hex(&sha1::Sha1::digest(password.as_bytes()));
        let suffixes: Vec<String> = if breached {
            vec![digest[5..].into()]
        } else {
            Vec::new()
        };
        sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES($1,$2,now()+interval '1 hour',true,pg_current_xact_id()) ON CONFLICT(prefix) DO UPDATE SET suffixes=$2,expires_at=now()+interval '1 hour',complete=true,complete_xact=pg_current_xact_id()")
            .bind(&digest[..5]).bind(suffixes).execute(&self.pool).await?;
        Ok(())
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
fn solve(token: &str) -> String {
    let mut nonce = 0_u64;
    loop {
        let value = nonce.to_string();
        if valid_solution(token, &value) {
            return value;
        }
        nonce += 1;
    }
}
const CSRF: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
#[test]
fn malformed_work_is_rejected() {
    for (token, nonce) in [
        ("bad".into(), "1"),
        ("A".repeat(64), ""),
        ("A".repeat(64), "-1"),
        ("A".repeat(64), "11111111111111111"),
    ] {
        assert!(!valid_solution(&token, nonce));
    }
    assert!(form_hash("").is_err());
}
#[tokio::test]
async fn work_is_bound_expiring_and_single_use_under_concurrency() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(issue(&db.pool, CSRF).await, Err(Error::Disabled));
    db.enable().await?;
    assert_eq!(issue(&db.pool, "bad").await, Err(Error::Invalid));
    let token = issue(&db.pool, CSRF).await?;
    let nonce = solve(&token);
    assert_eq!(
        consume(&db.pool, &"F".repeat(64), &token, &nonce).await,
        Err(Error::Invalid)
    );
    let (a, b) = tokio::join!(
        consume(&db.pool, CSRF, &token, &nonce),
        consume(&db.pool, CSRF, &token, &nonce)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(
        consume(&db.pool, CSRF, &token, &nonce).await,
        Err(Error::Invalid)
    );
    let token = issue(&db.pool, CSRF).await?;
    sqlx::query("UPDATE registration_work SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        consume(&db.pool, CSRF, &token, &solve(&token)).await,
        Err(Error::Invalid)
    );
    let old = issue(&db.pool, CSRF).await?;
    let _new = issue(&db.pool, CSRF).await?;
    assert_eq!(
        consume(&db.pool, CSRF, &old, &solve(&old)).await,
        Err(Error::Invalid)
    );
    db.close().await
}
#[tokio::test]
async fn breached_missing_stale_and_malformed_ranges_deny() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let password = "a locally generated test passphrase";
    // No cached range and an unreachable range API: never accepted.
    let down = crate::breach::Checker::new(
        db.pool.clone(),
        "http://127.0.0.1:1/range/",
        crate::breach::TIMEOUT,
    );
    let check = |password: &'static str| check_password_with(&db.pool, &down, password);
    assert_eq!(check(password).await, Err(Error::Disabled));
    db.enable().await?;
    assert_eq!(check("short").await, Err(Error::Invalid));
    assert_eq!(check(password).await, Err(Error::Unavailable));
    db.range(password, false).await?;
    assert_eq!(check(password).await, Ok(()));
    assert_eq!(check_password(&db.pool, password).await, Ok(()));
    db.range(password, true).await?;
    assert_eq!(check(password).await, Err(Error::Breached));
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Breached)
    );
    sqlx::query("UPDATE registration_password_ranges SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(check(password).await, Err(Error::Unavailable));
    sqlx::query("UPDATE registration_password_ranges SET suffixes=ARRAY['bad'],expires_at=now()+interval '1 hour'").execute(&db.pool).await?;
    assert_eq!(check(password).await, Err(Error::Unavailable));
    // An incomplete operator-import row (counts >= 100 only) never decides a lookup.
    sqlx::query("UPDATE registration_password_ranges SET suffixes=ARRAY[]::text[],complete=false")
        .execute(&db.pool)
        .await?;
    assert_eq!(check(password).await, Err(Error::Unavailable));
    sqlx::query("DELETE FROM registration_abuse_policy")
        .execute(&db.pool)
        .await?;
    assert_eq!(enabled(&db.pool).await, Err(Error::Unavailable));
    db.close().await
}

async fn browser_form(
    db: &Database,
) -> Result<(axum::Router, String, String), Box<dyn std::error::Error>> {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, header},
    };
    use tower::ServiceExt;
    let app = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .layer(axum::Extension(axum::extract::ConnectInfo(
        "192.0.2.50:1234".parse::<std::net::SocketAddr>()?,
    )));
    let response = app
        .clone()
        .oneshot(Request::get("/en/register").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing cookie")?
        .to_string();
    let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    assert!(html.contains("/register/abuse.js"));
    let head = html
        .split_once("</head>")
        .ok_or("missing registration head")?
        .0;
    assert!(head.contains(
        r#"<link rel="preload" as="font" type="font/woff2" crossorigin href="/static/thais/alegreya-56505fe0.woff2">"#
    ));
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("missing CSRF")?
        .to_string();
    Ok((app, cookie, csrf))
}
async fn browser_submit(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    password: &str,
    valid: bool,
) -> Result<axum::response::Response, Box<dyn std::error::Error>> {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    let response = app
        .clone()
        .oneshot(
            Request::post("/register/abuse/challenge")
                .header(header::COOKIE, cookie)
                .header("x-csrf-token", csrf)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let challenge: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    let token = challenge["token"].as_str().ok_or("token missing")?;
    let nonce = if valid { solve(token) } else { "bad".into() };
    let fields = [
        ("_csrf", csrf),
        ("work_token", token),
        ("work_nonce", &nonce),
        ("email", "abuse@example.test"),
        ("password", password),
        ("password_confirmation", password),
        ("terms", crate::consents::DEFAULT_LEGAL_VERSION),
        ("privacy", crate::consents::DEFAULT_LEGAL_VERSION),
    ];
    Ok(app
        .clone()
        .oneshot(
            Request::post("/en/register")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(serde_urlencoded::to_string(fields)?))?,
        )
        .await?)
}
#[tokio::test]
async fn browser_registration_accepts_proof_and_safe_password() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enable().await?;
    db.range("a fresh registration passphrase", false).await?;
    let (app, cookie, csrf) = browser_form(&db).await?;
    let response = browser_submit(
        &app,
        &cookie,
        &csrf,
        "a fresh registration passphrase",
        true,
    )
    .await?;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers()[axum::http::header::LOCATION],
        "/en/register/check-inbox"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await
}
#[tokio::test]
async fn browser_registration_refuses_bad_proof_and_breached_password() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enable().await?;
    let password = "a compromised registration passphrase";
    db.range(password, true).await?;
    let (app, cookie, csrf) = browser_form(&db).await?;
    assert_eq!(
        browser_submit(&app, &cookie, &csrf, password, false)
            .await?
            .status(),
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    let response = browser_submit(&app, &cookie, &csrf, password, true).await?;
    assert_eq!(
        response.status(),
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("data breach"));
    assert!(html.contains("class=\"op-theme op-shell\""));
    assert!(!html.contains(password));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[tokio::test]
async fn challenge_requires_csrf_and_disabled_registration_denies() -> TestResult {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .layer(axum::Extension(axum::extract::ConnectInfo(
        "192.0.2.50:1234".parse::<std::net::SocketAddr>()?,
    )));
    assert_eq!(
        app.clone()
            .oneshot(Request::get("/en/register").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        app.oneshot(Request::post("/register/abuse/challenge").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    db.close().await
}

#[tokio::test]
async fn audit02_challenge_locales_share_source_budget_without_more_writes() -> TestResult {
    use tower::ServiceExt;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    let app = http::routes::<()>(db.pool.clone());
    for n in 0..7 {
        let path = if n % 2 == 0 {
            "/en/register/abuse/challenge"
        } else {
            "/pl/register/abuse/challenge"
        };
        let request = axum::http::Request::post(path)
            .header("x-csrf-token", format!("{n:064x}"))
            .extension(axum::extract::ConnectInfo(
                "192.0.2.90:1234".parse::<std::net::SocketAddr>()?,
            ))
            .body(axum::body::Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        assert_eq!(
            response.status(),
            if n < 5 {
                axum::http::StatusCode::OK
            } else {
                axum::http::StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM registration_work")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 5);
    db.close().await
}

#[tokio::test]
async fn challenge_budget_is_keyed_by_resolved_client_ip_not_proxy_peer() -> TestResult {
    use tower::ServiceExt;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE registration_abuse_policy SET enabled=true")
        .execute(&db.pool)
        .await?;
    let app = http::routes::<()>(db.pool.clone());
    let proxy = "10.0.0.1:443".parse::<std::net::SocketAddr>()?;
    let first: std::net::IpAddr = "203.0.113.7".parse()?;
    let second: std::net::IpAddr = "203.0.113.8".parse()?;
    for (n, (client, expected)) in [
        (first, axum::http::StatusCode::OK),
        (first, axum::http::StatusCode::OK),
        (first, axum::http::StatusCode::OK),
        (first, axum::http::StatusCode::OK),
        (first, axum::http::StatusCode::OK),
        (first, axum::http::StatusCode::TOO_MANY_REQUESTS),
        // Another client behind the same proxy keeps its own budget.
        (second, axum::http::StatusCode::OK),
    ]
    .into_iter()
    .enumerate()
    {
        let request = axum::http::Request::post("/en/register/abuse/challenge")
            .header("x-csrf-token", format!("{n:064x}"))
            .extension(axum::extract::ConnectInfo(proxy))
            .extension(http::ClientIp(client))
            .body(axum::body::Body::empty())?;
        assert_eq!(
            app.clone().oneshot(request).await?.status(),
            expected,
            "{n}"
        );
    }
    db.close().await
}

#[tokio::test]
async fn audit02_registration_dispatch_charges_before_work_across_fresh_sessions() -> TestResult {
    use tower::ServiceExt;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enable().await?;
    db.range("a fresh registration passphrase", false).await?;
    db.freeze_budget_clock().await?;
    // Solving work and hashing passwords may span a wall-clock minute under
    // load. Keep the fixed-window budgets on the schema-local database clock.
    let mut forms = Vec::new();
    for _ in 0..7 {
        let (app, cookie, csrf) = browser_form(&db).await?;
        let token = issue(&db.pool, &csrf).await.map_err(|e| e.to_string())?;
        let nonce = solve(&token);
        let body = serde_urlencoded::to_string([
            ("_csrf", csrf.as_str()),
            ("work_token", token.as_str()),
            ("work_nonce", nonce.as_str()),
            ("email", "registration-budget@example.test"),
            ("password", "a fresh registration passphrase"),
            ("password_confirmation", "a fresh registration passphrase"),
            ("terms", crate::consents::DEFAULT_LEGAL_VERSION),
            ("privacy", crate::consents::DEFAULT_LEGAL_VERSION),
        ])?;
        forms.push((app, cookie, body));
    }
    let (reset_app, reset_cookie, reset_body) = forms[5].clone();
    for (n, (app, cookie, body)) in forms.into_iter().enumerate() {
        let response = app
            .oneshot(
                axum::http::Request::post(if n % 2 == 0 {
                    "/en/register"
                } else {
                    "/pl/register"
                })
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(body))?,
            )
            .await?;
        assert_eq!(
            response.status(),
            if n < 5 {
                axum::http::StatusCode::SEE_OTHER
            } else {
                axum::http::StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM outbox WHERE kind='email.verify'")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    let budgets: Vec<(String, i64)> =
        sqlx::query_as("SELECT limiter_name,count FROM web_rate_limits ORDER BY limiter_name")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(
        budgets,
        vec![
            ("identity-registration".into(), 5),
            ("identity-registration-attempt".into(), 6),
        ]
    );
    // Refusals precede work consumption. Both fixed windows reset at the next
    // database minute; the very same refused form then reaches registration.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM registration_work")
            .fetch_one(&db.pool)
            .await?,
        2
    );
    sqlx::query("UPDATE test_budget_clock SET instant=instant+interval '1 minute'")
        .execute(&db.pool)
        .await?;
    let response = reset_app
        .oneshot(
            axum::http::Request::post("/en/register")
                .header("cookie", reset_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(reset_body))?,
        )
        .await?;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM registration_work")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.close().await
}

#[tokio::test]
async fn audit3_english_challenge_refusal_uses_requested_language() -> TestResult {
    use tower::ServiceExt;
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL is required for this regression")?;
    db.enable().await?;
    let (app, cookie, csrf) = browser_form(&db).await?;
    sqlx::query("UPDATE registration_abuse_policy SET enabled=false")
        .execute(&db.pool)
        .await?;
    let request = || {
        axum::http::Request::post("/en/register/abuse/challenge")
            .header(axum::http::header::COOKIE, &cookie)
            .header("x-csrf-token", &csrf)
            .body(axum::body::Body::empty())
    };
    let response = app.oneshot(request()?).await?;
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(body.contains("Registration is currently unavailable."));
    assert!(!body.contains("Rejestracja jest obecnie"));

    // The shared application error shell replaces module links. Exercise the
    // adapter with the same real session/CSRF middleware to inspect its retry link.
    let adapter = http::routes::<()>(db.pool.clone())
        .route_layer(axum::middleware::from_fn_with_state(
            oteryn_web::session::SessionLayer::new(db.pool.clone()),
            oteryn_web::session::middleware,
        ))
        .layer(axum::Extension(axum::extract::ConnectInfo(
            "192.0.2.50:1234".parse::<std::net::SocketAddr>()?,
        )));
    let response = adapter.oneshot(request()?).await?;
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(body.contains("/en/register"));
    assert!(body.contains("lang=\"en\""));
    assert!(body.contains("Registration is currently unavailable."));
    assert!(!body.contains("/pl/register"));
    db.close().await?;
    eprintln!(
        "PG audit3_english_challenge_refusal_uses_requested_language: {:?}",
        started.elapsed()
    );
    Ok(())
}
