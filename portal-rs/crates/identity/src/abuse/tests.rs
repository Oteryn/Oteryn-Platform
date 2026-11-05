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
        let path = schema.clone();
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
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
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
        sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at) VALUES($1,$2,now()+interval '1 hour') ON CONFLICT(prefix) DO UPDATE SET suffixes=$2,expires_at=now()+interval '1 hour'")
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
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Disabled)
    );
    db.enable().await?;
    assert_eq!(check_password(&db.pool, "short").await, Err(Error::Invalid));
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Unavailable)
    );
    db.range(password, false).await?;
    assert_eq!(check_password(&db.pool, password).await, Ok(()));
    db.range(password, true).await?;
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Breached)
    );
    sqlx::query("UPDATE registration_password_ranges SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Unavailable)
    );
    sqlx::query("UPDATE registration_password_ranges SET suffixes=ARRAY['bad'],expires_at=now()+interval '1 hour'").execute(&db.pool).await?;
    assert_eq!(
        check_password(&db.pool, password).await,
        Err(Error::Unavailable)
    );
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
    );
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
        ("terms", "2026-11-05"),
        ("privacy", "2026-11-05"),
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
    let body = axum::body::to_bytes(response.into_body(), 4096).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("data breach"));
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
    );
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
