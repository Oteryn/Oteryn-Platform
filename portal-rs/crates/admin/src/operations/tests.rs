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
    let app = Router::new()
        .route(ROUTES[0].0, get(statistics))
        .route(ROUTES[1].0, get(rate_limits))
        .route(ROUTES[2].0, get(rate_export))
        .route(ROUTES[3].0, get(effective_flags))
        .route(ROUTES[4].0, get(system_health))
        .with_state(pool);
    for &(path, query, expected) in ROUTES {
        let response = send(app.clone(), path, query).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("response body");
        assert!(String::from_utf8_lossy(&body).contains(expected), "{path}");
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
    let app = operations_router(pool, &|key| match key {
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
