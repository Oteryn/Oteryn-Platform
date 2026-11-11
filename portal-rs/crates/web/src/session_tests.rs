use super::*;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::Request as HttpRequest,
    routing::{get, post},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED session database test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!(
            "session_test_{}",
            random_token()?.chars().take(32).collect::<String>()
        );
        // The identifier consists solely of a fixed prefix and generated hexadecimal bytes.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(search_path)
                        .execute(connection)
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
    async fn close(self) -> TestResult {
        self.pool.close().await;
        // The schema name was generated above and never includes external input.
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
fn app(pool: PgPool) -> Router {
    let mut layer = SessionLayer::new(pool);
    layer
        .csrf
        .trusted_origins
        .push("https://portal.test".into());
    layer
        .csrf
        .api_allow_list
        .push((axum::http::Method::POST, "/api/exempt".into()));
    Router::new()
        .route(
            "/form",
            get(|Extension(session): Extension<BrowserSession>| async move {
                session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            })
            .post(|body: String| async move { body }),
        )
        .route("/api/exempt", post(|| async { StatusCode::NO_CONTENT }))
        .route("/api/protected", post(|| async { StatusCode::NO_CONTENT }))
        .layer(axum::middleware::from_fn_with_state(layer, middleware))
}
async fn credentials(app: &Router) -> Result<(String, String), Box<dyn std::error::Error>> {
    let response = app
        .clone()
        .oneshot(HttpRequest::get("/form").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE].to_str()?.to_owned();
    for attribute in ["Secure", "HttpOnly", "SameSite=Lax", "Path=/"] {
        assert!(cookie.contains(attribute));
    }
    assert!(!cookie.contains("Domain="));
    let token = String::from_utf8(to_bytes(response.into_body(), 1024).await?.to_vec())?;
    Ok((
        cookie.split(';').next().ok_or("missing cookie")?.into(),
        token,
    ))
}

#[tokio::test]
async fn database_cookie_csrf_and_api_allow_list() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let pool = db.pool.clone();
    let app = app(pool);
    let (cookie, token) = credentials(&app).await?;
    for (supplied, origin, site, expected) in [
        (None, None, None, StatusCode::FORBIDDEN),
        (Some("wrong"), None, None, StatusCode::FORBIDDEN),
        (
            Some(token.as_str()),
            Some("https://evil.test"),
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            Some(token.as_str()),
            Some("null"),
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            Some(token.as_str()),
            Some("https://portal.test"),
            Some("cross-site"),
            StatusCode::FORBIDDEN,
        ),
        (
            Some(token.as_str()),
            Some("https://portal.test"),
            Some("same-origin"),
            StatusCode::OK,
        ),
        (Some(token.as_str()), None, None, StatusCode::OK),
    ] {
        let mut request = HttpRequest::post("/form").header(header::COOKIE, &cookie);
        if let Some(token) = supplied {
            request = request.header("x-csrf-token", token);
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        if let Some(site) = site {
            request = request.header("sec-fetch-site", site);
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty())?)
                .await?
                .status(),
            expected
        );
    }
    for (body, expected) in [
        (
            format!("_csrf={token}&message=hello%20world"),
            StatusCode::OK,
        ),
        (
            format!("_csrf={token}&_csrf={token}"),
            StatusCode::FORBIDDEN,
        ),
        ("message=missing".into(), StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::post("/form")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body.clone()))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            assert_eq!(to_bytes(response.into_body(), 1024).await?, body);
        }
    }
    for (path, content_type, status) in [
        ("/api/exempt", "application/json", StatusCode::NO_CONTENT),
        (
            "/api/exempt",
            "application/x-www-form-urlencoded",
            StatusCode::FORBIDDEN,
        ),
        ("/api/protected", "application/json", StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(
                    HttpRequest::post(path)
                        .header(header::CONTENT_TYPE, content_type)
                        .body(Body::empty())?
                )
                .await?
                .status(),
            status
        );
    }
    db.close().await
}

#[tokio::test]
async fn database_rotation_preserves_typed_data_and_revokes_old_id() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let pool = db.pool.clone();
    let session = BrowserSession::new(
        pool.clone(),
        SessionConfig::default(),
        Some("127.0.0.1".parse()?),
        "agent\nsummary",
    );
    let key = SessionKey::<Vec<String>>::new("flash");
    session.set(key, &vec!["saved".into()]).await?;
    let old = session
        .inner
        .lock()
        .await
        .record
        .as_ref()
        .ok_or("missing record")?
        .id
        .clone();
    let old_csrf = session.csrf_token().await?;
    let account = Uuid::from_u128(42);
    rotate(&session, PrivilegeChange::Login(account)).await?;
    let new = session
        .inner
        .lock()
        .await
        .record
        .as_ref()
        .ok_or("missing record")?
        .id
        .clone();
    assert_ne!(old, new);
    assert_ne!(old_csrf, session.csrf_token().await?);
    assert_eq!(session.account_id().await, Some(account));
    assert_eq!(
        session.get(SessionKey::<Vec<String>>::new("flash")).await?,
        Some(vec!["saved".into()])
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE id_hash=$1")
        .bind(hash(&old))
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let stored: (Vec<u8>, Option<String>, String) = sqlx::query_as(
        "SELECT id_hash, host(ip), user_agent_summary FROM web_sessions WHERE id_hash=$1",
    )
    .bind(hash(&new))
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored.0.len(), 32);
    assert_ne!(stored.0, new.as_bytes());
    assert_eq!(stored.1.as_deref(), Some("127.0.0.1"));
    assert_eq!(stored.2, "agentsummary");
    rotate(&session, PrivilegeChange::Sudo(OffsetDateTime::now_utc())).await?;
    rotate(&session, PrivilegeChange::Logout).await?;
    assert!(session.inner.lock().await.record.is_none());
    assert!(
        session
            .inner
            .lock()
            .await
            .cookie
            .as_ref()
            .is_some_and(|v| v.contains("Max-Age=0"))
    );
    assert_eq!(session.get(SessionKey::<String>::new("flash")).await?, None);
    let anonymous = BrowserSession::new(pool, SessionConfig::default(), None, "");
    assert!(
        rotate(&anonymous, PrivilegeChange::Sudo(OffsetDateTime::now_utc()))
            .await
            .is_err()
    );
    assert!(
        anonymous
            .set(SessionKey::new(""), &"invalid")
            .await
            .is_err()
    );
    db.close().await
}

#[tokio::test]
async fn database_idle_and_absolute_expiry_and_cleanup() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let pool = db.pool.clone();
    let layer = SessionLayer::new(pool.clone());
    for idle in [true, false] {
        let session = BrowserSession::new(pool.clone(), SessionConfig::default(), None, "");
        session.csrf_token().await?;
        let id = session
            .inner
            .lock()
            .await
            .record
            .as_ref()
            .ok_or("missing record")?
            .id
            .clone();
        sqlx::query("UPDATE web_sessions SET idle_expires_at=CASE WHEN $2 THEN now()-interval '1 second' ELSE idle_expires_at END, absolute_expires_at=CASE WHEN $2 THEN absolute_expires_at ELSE now()-interval '1 second' END WHERE id_hash=$1")
        .bind(hash(&id)).bind(idle)
        .execute(&pool)
        .await?;
        let mut request = HttpRequest::get("/")
            .header(header::COOKIE, format!("{COOKIE_NAME}={id}"))
            .body(Body::empty())?;
        assert!(
            run(&layer, &mut request)
                .await?
                .inner
                .lock()
                .await
                .record
                .is_none()
        );
        assert!(
            session
                .set(SessionKey::new("pending"), &true)
                .await
                .is_err()
        );
        assert!(
            rotate(&session, PrivilegeChange::Login(Uuid::from_u128(42)))
                .await
                .is_err()
        );
        oteryn_db::outbox::Cleanup::clean(&SessionCleanup { pool: pool.clone() }).await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE id_hash=$1")
            .bind(hash(&id))
            .fetch_one(&pool)
            .await?;
        assert_eq!(count, 0);
    }
    let session = BrowserSession::new(pool.clone(), SessionConfig::default(), None, "");
    session.csrf_token().await?;
    let id = session
        .inner
        .lock()
        .await
        .record
        .as_ref()
        .ok_or("missing record")?
        .id
        .clone();
    let before: OffsetDateTime =
        sqlx::query_scalar("SELECT absolute_expires_at FROM web_sessions WHERE id_hash=$1")
            .bind(hash(&id))
            .fetch_one(&pool)
            .await?;
    let mut request = HttpRequest::get("/")
        .header(header::COOKIE, format!("{COOKIE_NAME}={id}"))
        .body(Body::empty())?;
    assert!(
        run(&layer, &mut request)
            .await?
            .inner
            .lock()
            .await
            .record
            .is_some()
    );
    rotate(&session, PrivilegeChange::Login(Uuid::from_u128(42))).await?;
    let rotated = session
        .inner
        .lock()
        .await
        .record
        .as_ref()
        .ok_or("missing record")?
        .id
        .clone();
    let after: OffsetDateTime =
        sqlx::query_scalar("SELECT absolute_expires_at FROM web_sessions WHERE id_hash=$1")
            .bind(hash(&rotated))
            .fetch_one(&pool)
            .await?;
    assert_eq!(before, after);
    SessionCleanup { pool: pool.clone() }.clean().await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE id_hash=$1")
        .bind(hash(&rotated))
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    assert!(
        session
            .get(SessionKey::<String>::new("absent"))
            .await?
            .is_none()
    );
    assert!(
        SessionConfig {
            idle_seconds: 0,
            absolute_seconds: 1
        }
        .validate()
        .is_err()
    );
    let mut malformed = HttpRequest::get("/")
        .header(header::COOKIE, format!("{COOKIE_NAME}=bad"))
        .body(Body::empty())?;
    assert!(run(&layer, &mut malformed).await.is_err());
    db.close().await
}

#[tokio::test]
async fn chrome_reads_existing_session_without_refreshing_it() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let session = BrowserSession::new(
        db.pool.clone(),
        SessionConfig::default(),
        None,
        "chrome test",
    );
    rotate(&session, PrivilegeChange::Login(Uuid::from_u128(42))).await?;
    let csrf = session.csrf_token().await?;
    let id = session
        .inner
        .lock()
        .await
        .record
        .as_ref()
        .ok_or("missing record")?
        .id
        .clone();
    let snapshot = |pool: PgPool| async move {
        sqlx::query_scalar::<_, String>("SELECT row_to_json(s)::text FROM web_sessions s LIMIT 1")
            .fetch_one(&pool)
            .await
    };
    let before = snapshot(db.pool.clone()).await?;
    let mut registry = oteryn_ui::Registry::default();
    registry.login = true;
    registry.account = true;
    registry.registration = true;
    let app = Router::new()
        .route(
            "/en/",
            get(
                |Extension(chrome): Extension<oteryn_ui::Chrome>| async move {
                    oteryn_ui::home(&chrome, &oteryn_ui::HomeData::default())
                },
            ),
        )
        .layer(axum::middleware::from_fn_with_state(
            crate::chrome::Shell {
                registry,
                sessions: SessionLayer::new(db.pool.clone()),
            },
            crate::chrome::middleware,
        ));
    for cookie in [None, Some(format!("{COOKIE_NAME}={id}"))] {
        let mut builder = HttpRequest::get("/en/");
        if let Some(cookie) = &cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        let response = app.clone().oneshot(builder.body(Body::empty())?).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert_eq!(html.contains("action=\"/en/logout\""), cookie.is_some());
        assert_eq!(html.contains(&csrf), cookie.is_some());
    }
    assert_eq!(snapshot(db.pool.clone()).await?, before);
    db.close().await
}

#[tokio::test]
async fn nwave_consent_forms_accept_public_origin_and_reject_forgery() -> TestResult {
    let db = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let layer = crate::public_sessions(db.clone(), Some("https://portal.test".into()));
    let session = BrowserSession::new(db, SessionConfig::default(), None, "consent regression");
    let token = "a".repeat(64);
    session.inner.lock().await.record = Some(Record {
        id: "test-session".into(),
        csrf: token.clone(),
        account_id: Some(Uuid::now_v7()),
    });
    for (origin, csrf, granted, expected) in [
        (
            "https://portal.test",
            token.as_str(),
            "&granted=true",
            Ok(()),
        ),
        ("https://portal.test", token.as_str(), "", Ok(())),
        (
            "https://foreign.test",
            token.as_str(),
            "",
            Err(StatusCode::FORBIDDEN),
        ),
        ("null", token.as_str(), "", Err(StatusCode::FORBIDDEN)),
        (
            "https://portal.test",
            "wrong",
            "&granted=true",
            Err(StatusCode::FORBIDDEN),
        ),
    ] {
        let mut request = HttpRequest::post("/en/account/consents")
            .header(header::ORIGIN, origin)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!(
                "_csrf={csrf}&kind=play_session&version=1{granted}"
            )))?;
        assert_eq!(
            crate::csrf::verify(&layer.csrf, &session, &mut request).await,
            expected,
            "origin={origin}, granted={granted}"
        );
    }
    let unconfigured = crate::public_sessions(session.pool.clone(), None);
    assert!(unconfigured.csrf.trusted_origins.is_empty());
    Ok(())
}
