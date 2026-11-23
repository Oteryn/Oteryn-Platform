use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request as HttpRequest,
};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn bearer_requires_a_single_matching_value() {
    let expected: [u8; 32] = Sha256::digest(b"fixture").into();
    let mut headers = HeaderMap::new();
    for value in ["Basic fixture", "Bearer", "Bearer different"] {
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
        assert!(!authorized(Some(&expected), &headers));
    }
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer fixture"),
    );
    assert!(!authorized(None, &headers));
    assert!(authorized(Some(&expected), &headers));
    headers.append(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer fixture"),
    );
    assert!(!authorized(Some(&expected), &headers));
}

#[allow(clippy::panic)]
async fn panic_fixture() -> StatusCode {
    tokio::task::yield_now().await;
    panic!("synthetic message")
}

#[tokio::test]
async fn matched_routes_record_errors_and_panics_and_export_metrics() -> TestResult {
    let Some(db) = crate::session::tests::Database::new().await? else {
        return Ok(());
    };
    install_panic_hook();
    let telemetry = Arc::new(Telemetry::new(db.pool.clone(), Some("fixture")));
    let downstream = carry_matched_path(
        Router::new()
            .route("/items/{id}", get(|| async { StatusCode::BAD_GATEWAY }))
            .route("/panic/{id}", get(panic_fixture)),
    );
    let app = observe(
        Router::new().fallback_service(downstream).merge(
            Router::new()
                .route("/metrics", get(metrics))
                .with_state(telemetry.clone()),
        ),
        telemetry.clone(),
        true,
    );
    let send = async |uri: &str| -> Result<Response, Box<dyn std::error::Error>> {
        Ok(app
            .clone()
            .oneshot(HttpRequest::get(uri).body(Body::empty())?)
            .await?)
    };

    let first = send("/items/first").await?;
    assert_eq!(first.status(), StatusCode::BAD_GATEWAY);
    sqlx::query("UPDATE portal_errors SET status = 'ignored'")
        .execute(&db.pool)
        .await?;
    let second = send("/items/second?fixture=private").await?;
    assert_eq!(second.status(), StatusCode::BAD_GATEWAY);
    let request_id: Uuid = second.headers()["x-request-id"].to_str()?.parse()?;
    let group: (String, i64, Uuid, bool, Option<String>) = sqlx::query_as(
        "SELECT status, occurrence_count, last_request_id, first_seen <= last_seen, top_frame
         FROM portal_errors WHERE route = '/items/{id}' AND error_kind = 'http_502'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(group, ("open".into(), 2, request_id, true, None));

    let failed = send("/panic/third").await?;
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(failed.headers()[header::CACHE_CONTROL], "no-store");
    let frame: String = sqlx::query_scalar(
        "SELECT top_frame FROM portal_errors WHERE route = '/panic/{id}' AND error_kind = 'panic'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert!(frame.contains("observability_tests.rs:"));
    assert!(!frame.contains("synthetic message"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT sum(count)::bigint FROM portal_error_rate")
            .fetch_one(&db.pool)
            .await?,
        3
    );
    assert_eq!(send("/missing/path").await?.status(), StatusCode::NOT_FOUND);
    assert_eq!(send("/metrics").await?.status(), StatusCode::NOT_FOUND);

    let internal = observe(Router::new(), telemetry.clone(), false)
        .oneshot(HttpRequest::get("/missing").body(Body::empty())?)
        .await?;
    assert_eq!(internal.status(), StatusCode::NOT_FOUND);
    assert!(!internal.headers().contains_key("x-request-id"));

    let slots = telemetry
        .record_slots
        .acquire_many(telemetry.record_slots.available_permits() as u32)
        .await?;
    assert_eq!(
        send("/items/deferred").await?.status(),
        StatusCode::BAD_GATEWAY
    );
    drop(slots);

    sqlx::query(
        "INSERT INTO outbox(kind, payload, idempotency_key, state)
         VALUES ('email.verify', '{}', 'verify', 'pending'),
                ('email.account_exists', '{}', 'exists', 'pending'),
                ('email.password-reset', '{}', 'reset', 'pending'),
                ('notify.email', '{}', 'notify', 'pending'),
                ('observability.email', '{}', 'operation', 'pending'),
                ('email.verify', '{}', 'delivered', 'done'),
                ('fixture.other', '{}', 'other', 'pending')",
    )
    .execute(&db.pool)
    .await?;

    let scrape = async || -> Result<Response, Box<dyn std::error::Error>> {
        Ok(app
            .clone()
            .oneshot(
                HttpRequest::get("/metrics")
                    .header(header::AUTHORIZATION, "Bearer fixture")
                    .body(Body::empty())?,
            )
            .await?)
    };
    let pending_refresh = telemetry.outbox_snapshot.lock().await;
    let busy = scrape().await?;
    assert_eq!(busy.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(busy.headers()[header::RETRY_AFTER], "2");
    drop(pending_refresh);
    let scrape_response = scrape().await?;
    assert_eq!(scrape_response.status(), StatusCode::OK);
    assert_eq!(
        scrape_response.headers()[header::CONTENT_TYPE],
        "text/plain; version=0.0.4; charset=utf-8"
    );
    let body = String::from_utf8(to_bytes(scrape_response.into_body(), 65536).await?.to_vec())?;
    for sample in [
        "portal_http_requests_total{route=\"/items/{id}\",status_class=\"5xx\"} 3",
        "portal_http_requests_total{route=\"<unmatched>\",status_class=\"4xx\"} 2",
        "portal_http_request_duration_seconds_bucket{route=\"/items/{id}\",le=\"+Inf\"} 3",
        "portal_http_5xx_total 4",
        "portal_http_in_flight_requests 1",
        "portal_error_records_missed_total 1",
        "portal_worker_jobs{state=\"pending\"} 6",
        "portal_worker_jobs{state=\"done\"} 1",
        "portal_outbox_backlog 6",
        "portal_outbox_oldest_age_seconds ",
        "portal_mail_outbox_backlog 5",
        "portal_db_pool_connections{state=\"in_use\"}",
        "portal_metrics_db_up 1",
        "portal_process_uptime_seconds",
        "portal_build_info{version=",
    ] {
        assert!(body.contains(sample), "{sample}");
    }
    for input in [
        "/items/first",
        "/items/second",
        "fixture=private",
        "/missing/path",
    ] {
        assert!(!body.contains(input));
    }
    let snapshot = telemetry.outbox_snapshot.lock().await;
    assert!(
        snapshot
            .as_ref()
            .is_some_and(|(_, output)| output.contains("portal_mail_outbox_backlog 5"))
    );
    db.close().await
}
