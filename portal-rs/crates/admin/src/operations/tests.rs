#![allow(clippy::expect_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;

const ROUTES: &[(&str, &str, &str)] = &[
    (
        "/admin/operations/statistics",
        "days=1",
        "Account registrations",
    ),
    ("/admin/operations/rate-limits", "", "identity-login"),
    (
        "/admin/operations/rate-limits.csv",
        "",
        "saturated_key_windows_last_hour",
    ),
    ("/admin/operations/flags", "", "OTERYN_ADMIN_PANEL"),
    ("/admin/operations/health", "", "Worker jobs due"),
];

async fn send(app: Router, path: &str, query: &str) -> Response {
    app.oneshot(
        Request::builder()
            .uri(format!("{path}?{query}"))
            .header(header::HOST, "staff.test")
            .header(header::ACCEPT_LANGUAGE, "en")
            .body(Body::empty())
            .expect("test request"),
    )
    .await
    .expect("infallible router")
}

#[tokio::test]
async fn reads_success_and_bad_input() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Set PORTAL_TEST_DATABASE_URL to run operations database tests");
        return;
    };
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to disposable PostgreSQL");
    oteryn_db::migrate(&pool)
        .await
        .expect("migrate disposable PostgreSQL");
    let mut tx = statistics_transaction(&pool)
        .await
        .expect("statistics transaction");
    let timeout: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&mut *tx)
        .await
        .expect("transaction-local statement timeout");
    assert_eq!(timeout, "2500ms");
    // Tiny fixtures legitimately prefer sequential scans. Inspect the range
    // conditions PostgreSQL can apply to each date-leading index.
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *tx)
        .await
        .expect("enable index plan on tiny fixture");
    let plan =
        sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(format!("EXPLAIN {STATISTICS_SQL}")))
            .bind(1_i32)
            .fetch_all(&mut *tx)
            .await
            .expect("one-day statistics plan");
    for index in [
        "accounts_operations_created_at",
        "security_events_operations_login_created_at",
    ] {
        let conditions = plan
            .iter()
            .skip_while(|line| !line.contains(index))
            .nth(1)
            .expect("timestamp conditions for statistics index");
        assert!(
            conditions.contains("Index Cond:")
                && conditions.contains("created_at >=")
                && conditions.contains("created_at <="),
            "{index}: {conditions}"
        );
    }
    tx.commit().await.expect("read-only commit");
    let mut lock = pool.begin().await.expect("lock transaction");
    sqlx::query("LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .expect("lock registrations table");
    let mut reader = statistics_transaction(&pool).await.expect("bounded reader");
    let blocked = sqlx::query(STATISTICS_SQL)
        .bind(1_i32)
        .fetch_all(&mut *reader)
        .await
        .expect_err("PostgreSQL cancels a blocked statistics statement");
    assert_eq!(
        blocked
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("57014")
    );
    reader.rollback().await.expect("cancelled reader rollback");
    lock.rollback().await.expect("release registrations lock");
    let account = uuid::Uuid::now_v7();
    let source = std::net::Ipv6Addr::from(account.as_u128());
    let mut keys = oteryn_web::ratelimit::Keys::source(source.into());
    keys.account = Some(account.to_string());
    let limiter = oteryn_web::ratelimit::RateLimiter::new(pool.clone());
    let policy = NamedPolicy::IdentityTermination;
    let saturated = |rows: Rows| {
        rows.into_iter()
            .find(|row| row[0] == policy.name())
            .and_then(|row| row[3].parse::<i64>().ok())
            .expect("numeric counter for the policy")
    };
    let before = saturated(rate_rows(&pool).await.expect("initial aggregate"));
    for _ in 0..policy.limit() + 2 {
        // Extra denied charges must still contribute only one saturated window.
        let _ = limiter.check(policy, std::slice::from_ref(&keys)).await;
    }
    let rate = rate_rows(&pool)
        .await
        .expect("aggregate rate-limit counters");
    assert_eq!(saturated(rate), before + 1);
    sqlx::query(
        "INSERT INTO outbox(kind, payload, idempotency_key, state, attempts, max_attempts, last_error)
         VALUES ('fixture', '{}'::jsonb, $1, 'dead', 1, 5, 'private fixture failure detail')",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .execute(&pool)
    .await
    .expect("permanently rejected job fixture");
    let app = Router::new()
        .route(ROUTES[0].0, get(statistics))
        .route(ROUTES[1].0, get(rate_limits))
        .route(ROUTES[2].0, get(rate_export))
        .route(ROUTES[3].0, get(effective_flags))
        .route(ROUTES[4].0, get(system_health))
        .with_state(pool)
        .layer(Extension(IngestAvailability(false)));
    for &(path, query, expected) in ROUTES {
        let response = send(app.clone(), path, query).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("response body");
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains(expected), "{path}");
        if path == "/admin/operations/health" {
            assert!(html.contains("Worker job failed permanently"));
            assert!(!html.contains("private fixture failure detail"));
        }
        assert_eq!(
            send(app.clone(), path, "account=another").await.status(),
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
}

#[tokio::test]
async fn every_read_refuses_anonymous_sessions() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .expect("lazy pool");
    let app = operations_router(pool, IngestAvailability(false), &|key| match key {
        "OTERYN_ADMIN_PANEL" => Some("true".into()),
        "OTERYN_ADMIN_HOST" => Some("staff.test".into()),
        _ => None,
    })
    .expect("panel router");
    for &(path, query, _) in ROUTES {
        assert_eq!(
            send(app.clone(), path, query).await.status(),
            StatusCode::SEE_OTHER,
            "{path}"
        );
    }
}

#[test]
fn range_and_flag_allowlist() {
    assert_eq!(range(None), Ok(7));
    assert_eq!(range(Some("days=30")), Ok(30));
    for invalid in [
        "days=0",
        "days=31",
        "days=1&days=1",
        "days=-1",
        "days=garbage",
    ] {
        assert_eq!(range(Some(invalid)), Err(StatusCode::BAD_REQUEST));
    }
    let flags = flag_rows(|key| key == "OTERYN_ADMIN_PANEL");
    assert_eq!(flags.iter().filter(|row| row[1] == "true").count(), 1);
    assert!(
        flags
            .iter()
            .all(|row| row[0].starts_with("OTERYN_") && !row[0].contains("URL"))
    );
}
