#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    Router,
    body::{Bytes, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{any, get},
};
use oteryn_gateway::{
    config::Config,
    errors::Code,
    issuer::{ISSUER_PATH, IssuerClient},
    wire::{self, Request as LoginRequest},
};
use serde_json::Value;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::ServiceExt;

fn request() -> LoginRequest {
    include_str!("fixtures/go_vectors.jsonl")
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .find_map(|v| {
            (v["kind"] == "request" && v.get("code").is_none())
                .then(|| wire::parse_request(v["input"].as_str().unwrap().as_bytes()).unwrap())
        })
        .unwrap()
}
struct Server {
    url: String,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn serve(app: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Server { url, task }
}
fn client(url: &str) -> IssuerClient {
    IssuerClient::new(
        url,
        "synthetic-gateway-service-token".into(),
        Duration::from_millis(150),
        Duration::from_millis(150),
    )
    .unwrap()
}
#[derive(Clone)]
struct Reply {
    status: u16,
    body: String,
    calls: Arc<AtomicUsize>,
    expected: Vec<u8>,
}
async fn reply(State(s): State<Reply>, r: Request<axum::body::Body>) -> impl IntoResponse {
    s.calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(r.method(), "POST");
    assert_eq!(r.uri().path(), ISSUER_PATH);
    assert!(r.uri().query().is_none());
    assert_eq!(
        r.headers()["authorization"],
        "Bearer synthetic-gateway-service-token"
    );
    assert_eq!(r.headers()["content-type"], "application/json");
    assert_eq!(r.headers()["accept"], "application/json");
    assert_eq!(
        to_bytes(r.into_body(), 2048).await.unwrap().as_ref(),
        s.expected
    );
    (
        StatusCode::from_u16(s.status).unwrap(),
        [("retry-after", "17")],
        s.body,
    )
}
#[tokio::test]
async fn all_go_issuer_reply_vectors_over_real_http() {
    let mut count = 0;
    for v in include_str!("fixtures/go_vectors.jsonl")
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .filter(|v| v["kind"] == "success" || v["kind"] == "error")
    {
        let r: LoginRequest = serde_json::from_value(v["request"].clone()).unwrap();
        let status = if v["kind"] == "success" {
            200
        } else {
            v["status"].as_u64().unwrap() as u16
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let server = serve(
            Router::new()
                .route(ISSUER_PATH, any(reply))
                .with_state(Reply {
                    status,
                    body: v["input"].as_str().unwrap().to_owned(),
                    calls: calls.clone(),
                    expected: wire::encode_request(&r).unwrap(),
                }),
        )
        .await;
        let result = client(&server.url).admit(&r).await;
        if v["valid"] == true && status == 200 {
            let s = result.unwrap();
            assert_eq!(s.attempt_ref, r.attempt_ref);
        } else {
            let expected = if v["valid"] == true {
                v["code"].as_str().unwrap()
            } else if status == 200 {
                Code::ReconciliationRequired.name()
            } else {
                Code::Unavailable.name()
            };
            let refusal = result.unwrap_err();
            assert_eq!(refusal.code.name(), expected, "{v}");
            if refusal.code == Code::RateLimited {
                assert_eq!(refusal.retry_after, 17);
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        count += 1;
    }
    println!("{count} Go issuer replies verified over real HTTP");
}
#[tokio::test]
async fn upstream_down_is_unavailable_but_written_timeout_requires_reconciliation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    assert_eq!(
        client(&url).admit(&request()).await.unwrap_err().code,
        Code::Unavailable
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let c = calls.clone();
    let server = serve(Router::new().route(
        ISSUER_PATH,
        any(move |body: Bytes| {
            let c = c.clone();
            async move {
                assert!(!body.is_empty());
                c.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(2)).await;
                ""
            }
        }),
    ))
    .await;
    assert_eq!(
        client(&server.url)
            .admit(&request())
            .await
            .unwrap_err()
            .code,
        Code::ReconciliationRequired
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn redirects_are_not_followed_and_error_size_is_bounded() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = calls.clone();
    let target = serve(Router::new().fallback(move || {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            ""
        }
    }))
    .await;
    let location = format!("{}{ISSUER_PATH}", target.url);
    let redirect = serve(Router::new().route(
        ISSUER_PATH,
        any(move || {
            let l = location.clone();
            async move { (StatusCode::TEMPORARY_REDIRECT, [("location", l)]) }
        }),
    ))
    .await;
    assert_eq!(
        client(&redirect.url)
            .admit(&request())
            .await
            .unwrap_err()
            .code,
        Code::Unavailable
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let oversized = serve(Router::new().route(
        ISSUER_PATH,
        any(|| async { (StatusCode::SERVICE_UNAVAILABLE, "x".repeat(4097)) }),
    ))
    .await;
    assert_eq!(
        client(&oversized.url)
            .admit(&request())
            .await
            .unwrap_err()
            .code,
        Code::Unavailable
    );
}
#[tokio::test]
async fn outer_router_preserves_gating_and_cache_headers() {
    let upstream = serve(Router::new().route("/health", get(|| async { "ok" }))).await;
    for enabled in [false, true] {
        let app = oteryn_gateway::router(client(&upstream.url), enabled, "v1.2.3".into());
        for (path, expected) in [
            ("/health", "{\"status\":\"ok\"}\n"),
            ("/ready", "{\"status\":\"ready\"}\n"),
            (
                "/version",
                "{\"service\":\"oteryn-game-gateway\",\"version\":\"v1.2.3\"}\n",
            ),
        ] {
            let r = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("x-request-id", "safe-id")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(r.status(), 200);
            assert_eq!(r.headers()["x-request-id"], "safe-id");
            assert_eq!(
                to_bytes(r.into_body(), 8192).await.unwrap().as_ref(),
                expected.as_bytes()
            );
        }
        for input in [
            b"".to_vec(),
            b"not json".to_vec(),
            b"{\"protocol_version\":2.0}".to_vec(),
            b"x".repeat(16385),
            wire::encode_request(&request()).unwrap(),
        ] {
            let r = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/login?ticket=not-forwarded")
                        .body(axum::body::Body::from(input))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(r.status(), 400);
            assert_eq!(
                r.headers()["cache-control"],
                "no-store, no-cache, must-revalidate, private"
            );
            assert_eq!(r.headers()["pragma"], "no-cache");
            assert_eq!(r.headers()["expires"], "0");
            assert_eq!(
                to_bytes(r.into_body(), 8192).await.unwrap().as_ref(),
                b"{\"error\":\"invalid_request\"}\n"
            );
        }
        if !enabled {
            let r = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/login")
                        .body(axum::body::Body::from(
                            wire::encode_request(&request()).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(r.status(), 400);
        }
    }
}
#[test]
fn config_is_default_off_and_refuses_bad_upstreams_flags_and_timeouts() {
    let base = [
        ("OTERYN_PLATFORM_BASE_URL", "http://127.0.0.1:8080"),
        ("OTERYN_PLATFORM_SERVICE_TOKEN_FILE", "/tmp/synthetic-token"),
    ];
    let config = |extra: Option<(&str, &str)>| {
        Config::from_lookup(|key| {
            extra
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| v.to_owned())
                .or_else(|| {
                    base.iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| v.to_string())
                })
        })
    };
    let c = config(None).unwrap();
    assert!(!c.enabled);
    assert_eq!(c.timeout, Duration::from_secs(5));
    assert!(
        config(Some(("GATEWAY_NATIVE_LOGIN_ENABLED", "true")))
            .unwrap()
            .enabled
    );
    for pair in [
        ("OTERYN_PLATFORM_BASE_URL", "http://example.test"),
        ("OTERYN_PLATFORM_BASE_URL", "https://user:pass@example.test"),
        ("OTERYN_PLATFORM_BASE_URL", "https://example.test?q=1"),
        ("OTERYN_PLATFORM_BASE_URL", "https://example.test#f"),
        ("GATEWAY_NATIVE_LOGIN_ENABLED", "1"),
        ("GATEWAY_NATIVE_ADMISSION_TIMEOUT", "9s"),
        ("GATEWAY_REQUEST_TIMEOUT", "0s"),
        ("GATEWAY_REQUEST_TIMEOUT", "-1s"),
    ] {
        assert!(config(Some(pair)).is_err(), "{pair:?}");
    }
    assert!(oteryn_gateway::config::validate_url("http://[::1]:8080").is_ok());
    assert!(oteryn_gateway::config::validate_url("https://example.test/prefix").is_ok());
}

#[tokio::test]
async fn health_stays_ok_when_upstream_readiness_fails() {
    let app = oteryn_gateway::router(client("http://127.0.0.1:1"), false, "test".into());
    for (path, status, body) in [
        ("/health", 200, "{\"status\":\"ok\"}\n"),
        ("/ready", 503, "{\"status\":\"not_ready\"}\n"),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), status);
        assert_eq!(
            to_bytes(r.into_body(), 8192).await.unwrap().as_ref(),
            body.as_bytes()
        );
    }
}

#[tokio::test]
async fn version_uses_the_go_outer_encoder_escaping() {
    let app = oteryn_gateway::router(client("http://127.0.0.1:1"), false, "<v>&".into());
    let r = app
        .oneshot(
            Request::builder()
                .uri("/version")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        to_bytes(r.into_body(), 8192).await.unwrap().as_ref(),
        b"{\"service\":\"oteryn-game-gateway\",\"version\":\"\\u003cv\\u003e\\u0026\"}\n"
    );
}
