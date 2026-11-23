use super::{
    http::{router, session_cookie},
    *,
};
use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, header},
};
use std::net::SocketAddr;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn ambiguous_cookies_deny() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        axum::http::HeaderValue::from_static("__Host-oteryn_session=a; __Host-oteryn_session=b"),
    );
    assert_eq!(session_cookie(&headers), None);
}
#[tokio::test]
async fn disabled_and_bad_input() -> TestResult {
    let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    assert_eq!(
        stream_download(&pool, &ExportConfig::default(), "")
            .await
            .err(),
        Some(StatusCode::NOT_FOUND)
    );
    assert_eq!(
        stream_download(&pool, &ExportConfig { enabled: true }, "bad")
            .await
            .err(),
        Some(StatusCode::UNAUTHORIZED)
    );
    Ok(())
}
#[tokio::test]
async fn postgres_owner_export_and_refusals() -> TestResult {
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED export PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("export_test_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(PRODUCER_DEADLINE + std::time::Duration::from_secs(3))
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
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES ('owner@example.invalid','private-password-hash','active','en',now()) RETURNING id").fetch_one(&pool).await?;
    sqlx::query("INSERT INTO accounts(email,password_hash,status,locale) VALUES ('other@example.invalid','other-private-hash','active','pl')").execute(&pool).await?;
    let token = "a".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until,data) VALUES ($1,$2,$3,'test browser',now()+interval '1 hour',now()+interval '1 day',now()+interval '5 minutes','{\"secret\":\"private-session-state\"}')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(id).bind("c".repeat(64)).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) SELECT $1,id,$2,'other-private-device',now()+interval '1 hour',now()+interval '1 day' FROM accounts WHERE email='other@example.invalid'")
        .bind(Sha256::digest("b".repeat(64).as_bytes()).to_vec()).bind("d".repeat(64)).execute(&pool).await?;
    sqlx::query("INSERT INTO account_consents(account_id,document_kind,document_version) VALUES ($1,'terms','v1')").bind(id).execute(&pool).await?;
    sqlx::query("INSERT INTO account_consents(account_id,document_kind,document_version) SELECT $1,'privacy','batch-' || n FROM generate_series(1,193) n").bind(id).execute(&pool).await?;
    let config = ExportConfig { enabled: true };
    let unread = stream_download(&pool, &config, &token)
        .await
        .map_err(|s| s.to_string())?;
    let released = tokio::time::timeout(
        PRODUCER_DEADLINE + std::time::Duration::from_secs(2),
        pool.acquire(),
    )
    .await;
    assert!(
        released.is_ok(),
        "unread export retained its transaction past deadline"
    );
    drop(released??);
    assert!(
        axum::body::to_bytes(unread, usize::MAX).await.is_err(),
        "deadline must produce a body error after queued data"
    );
    let cancelled = stream_download(&pool, &config, &token)
        .await
        .map_err(|s| s.to_string())?;
    drop(cancelled);
    drop(tokio::time::timeout(std::time::Duration::from_secs(2), pool.acquire()).await??);
    let body = stream_download(&pool, &config, &token)
        .await
        .map_err(|s| s.to_string())?;
    let data: Value = serde_json::from_slice(&axum::body::to_bytes(body, usize::MAX).await?)?;
    assert_eq!(data["account"]["id"], id.to_string());
    assert_eq!(data["sessions"].as_array().map(Vec::len), Some(1));
    use tower::ServiceExt;
    let app = router(pool.clone(), config.clone());
    for locale in ["pl", "en"] {
        let target = format!("/{locale}/account/privacy/export");
        let request = |method| -> Result<_, Box<dyn std::error::Error>> {
            axum::http::Request::builder()
                .method(method)
                .uri(&target)
                .header(header::COOKIE, format!("__Host-oteryn_session={token}"))
                .extension(ConnectInfo("192.0.2.1:1000".parse::<SocketAddr>()?))
                .body(axum::body::Body::empty())
                .map_err(Into::into)
        };
        let count = || {
            sqlx::query_scalar::<_, i64>(
                "SELECT coalesce(sum(count),0)::bigint FROM web_rate_limits",
            )
            .fetch_one(&pool)
        };
        let before = count().await?;
        for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
            let response = app.clone().oneshot(request(method)?).await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(!response.headers().contains_key(header::CONTENT_DISPOSITION));
            assert!(
                response.headers()[header::CONTENT_SECURITY_POLICY]
                    .to_str()?
                    .split(';')
                    .any(|directive| directive.trim() == "frame-ancestors 'none'")
            );
            assert_eq!(count().await?, before);
        }
        let mut post = request(axum::http::Method::POST)?;
        let missing = app
            .clone()
            .oneshot(request(axum::http::Method::POST)?)
            .await?;
        assert_eq!(missing.status(), StatusCode::FORBIDDEN);
        assert_eq!(count().await?, before);

        post.headers_mut()
            .insert("x-csrf-token", "c".repeat(64).parse()?);
        let response = app.clone().oneshot(post).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, private"
        );
        assert_eq!(
            response.headers()[header::CONTENT_DISPOSITION],
            "attachment; filename=\"account-data.json\""
        );
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
        let exported: Value = serde_json::from_slice(&body)?;
        // The browser middleware refreshes the current session's last-seen time.
        let mut expected = data.clone();
        expected["sessions"][0]["last_seen_at"] =
            sqlx::query_scalar("SELECT to_jsonb(last_seen_at) FROM web_sessions WHERE id_hash=$1")
                .bind(Sha256::digest(token.as_bytes()).to_vec())
                .fetch_one(&pool)
                .await?;
        assert_eq!(exported, expected);
        assert_eq!(count().await?, before + 2);
    }
    assert_eq!(data["consents"].as_array().map(Vec::len), Some(194));
    assert!(include_str!("export.rs").contains("FETCH FORWARD 64"));
    let encoded = data.to_string();
    for forbidden in [
        "private-password-hash",
        "other@example.invalid",
        "private-session-state",
        "other-private-device",
        "csrf_token",
        "id_hash",
        &token,
    ] {
        assert!(!encoded.contains(forbidden));
    }
    for (query, expected) in [
        ("UPDATE web_sessions SET sudo_until=NULL", Ok(None)),
        (
            "UPDATE web_sessions SET sudo_until=now()+interval '1 minute',idle_expires_at=now()-interval '1 second'",
            Err(StatusCode::FORBIDDEN),
        ),
        (
            "UPDATE web_sessions SET idle_expires_at=now()+interval '1 hour'; UPDATE accounts SET status='suspended'",
            Err(StatusCode::FORBIDDEN),
        ),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(query))
            .execute(&pool)
            .await?;
        assert_eq!(export_confirmation(&pool, &config, &token).await, expected);
        assert_eq!(
            stream_download(&pool, &config, &token).await.err(),
            Some(StatusCode::FORBIDDEN)
        );
    }
    eprintln!(
        "postgres_owner_export_and_refusals duration: {:?}",
        started.elapsed()
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}

use crate::privacy::test_support as deadline_database;

#[tokio::test]
async fn audit3_unread_export_delivers_terminal_error() -> TestResult {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let (pool, admin, schema) = deadline_database::database(&url).await?;
    let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('deadline@example.test',$1,'active','en',now()) RETURNING id")
        .bind(crate::password::hash("deadline test password")?).fetch_one(&pool).await?;
    let raw = "e".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until) VALUES($1,$2,$3,'deadline test',now()+interval '1 hour',now()+interval '1 day',now()+interval '5 minutes')")
        .bind(Sha256::digest(raw.as_bytes()).to_vec()).bind(id).bind("f".repeat(64)).execute(&pool).await?;
    // Exercise the real producer with a bounded test deadline; the owner-export
    // regression above also exercises the production 30-second deadline.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(200);
    let body = prepare_download(&pool, &ExportConfig { enabled: true }, &raw, deadline)
        .await
        .map_err(|s| s.to_string())?;
    tokio::time::sleep_until(deadline + std::time::Duration::from_millis(50)).await;
    assert!(
        axum::body::to_bytes(body, usize::MAX).await.is_err(),
        "unread export ended successfully after producer timeout"
    );
    deadline_database::close(pool, admin, schema).await?;
    eprintln!(
        "PG audit3_unread_export_delivers_terminal_error: {:?}",
        started.elapsed()
    );
    Ok(())
}
