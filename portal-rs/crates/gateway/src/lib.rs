//! Stateless, opt-in native login gateway. No database or signing configuration is loaded.
pub mod config;
pub mod errors;
pub mod issuer;
pub mod limiter;
pub mod wire;

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use errors::{Code, Refusal};
use issuer::IssuerClient;
use limiter::WindowLimiter;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct StateData {
    issuer: IssuerClient,
    enabled: bool,
    version: String,
    start: Instant,
    sources: Mutex<WindowLimiter>,
    attempts: Mutex<WindowLimiter>,
}
/// Constructs the same public HTTP surface as the Go gateway, with native login off unless enabled.
pub fn router(issuer: IssuerClient, enabled: bool, version: String) -> Router {
    let state = Arc::new(StateData {
        issuer,
        enabled,
        version,
        start: Instant::now(),
        sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
        attempts: Mutex::new(WindowLimiter::new(6, Duration::from_secs(120), 100_000)),
    });
    Router::new()
        .route(
            "/health",
            get(|| async { json(200, &serde_json::json!({"status":"ok"})) }),
        )
        .route("/ready", get(ready))
        .route("/version", get(version_info))
        .route("/v1/login", post(login))
        .layer(middleware::from_fn(request_log))
        .with_state(state)
}
async fn ready(State(s): State<Arc<StateData>>) -> Response {
    if s.issuer.ready().await {
        json(200, &serde_json::json!({"status":"ready"}))
    } else {
        json(503, &serde_json::json!({"status":"not_ready"}))
    }
}
async fn version_info(State(s): State<Arc<StateData>>) -> Response {
    json(
        200,
        &serde_json::json!({"service":"oteryn-game-gateway", "version":s.version}),
    )
}

async fn login(State(s): State<Arc<StateData>>, req: Request) -> Response {
    let source = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|i| i.0.ip().to_string())
        .unwrap_or_default();
    let query = req.uri().query().is_some_and(|q| !q.is_empty());
    let mut response = if query {
        invalid()
    } else {
        match to_bytes(req.into_body(), 16 * 1024).await {
            Ok(b) if s.enabled && wire::is_native(&b) => native(&s, &source, &b).await,
            _ => invalid(),
        }
    };
    for (k, v) in [
        (
            "cache-control",
            "no-store, no-cache, must-revalidate, private",
        ),
        ("pragma", "no-cache"),
        ("expires", "0"),
    ] {
        response
            .headers_mut()
            .insert(k, HeaderValue::from_static(v));
    }
    response
}
fn invalid() -> Response {
    json(400, &serde_json::json!({"error":"invalid_request"}))
}
fn allow(l: &Mutex<WindowLimiter>, key: &[u8], now: Duration) -> Result<(), Refusal> {
    l.lock()
        .map_err(|_| Refusal::from(Code::Unavailable))?
        .allow(key, now)
        .map_err(|retry_after| Refusal {
            code: Code::RateLimited,
            retry_after,
        })
}
async fn native(s: &StateData, source: &str, body: &[u8]) -> Response {
    let echo = wire::attempt_echo(body);
    let request = match prepare(s, source, body, s.start.elapsed()) {
        Ok(request) => request,
        Err(r) => return refuse(r, echo.as_deref()),
    };
    match s.issuer.admit(&request).await {
        Ok(success) => {
            tracing::info!(
                attempt_ref = request.attempt_ref,
                code = "OK",
                "native_login_result"
            );
            json(200, &success)
        }
        Err(r) => refuse(r, echo.as_deref()),
    }
}
fn prepare(
    s: &StateData,
    source: &str,
    body: &[u8],
    now: Duration,
) -> Result<wire::Request, Refusal> {
    allow(&s.sources, source.as_bytes(), now)?;
    let request = wire::parse_request(body).map_err(Refusal::from)?;
    let mut digest = Sha256::new();
    digest.update(request.attempt_ref.as_bytes());
    digest.update([0]);
    digest.update(request.game_login_ticket.as_bytes());
    allow(&s.attempts, &digest.finalize(), now)?;
    Ok(request)
}

fn refuse(r: Refusal, echo: Option<&str>) -> Response {
    tracing::info!(
        attempt_ref = echo.unwrap_or(""),
        code = r.code.name(),
        "native_login_result"
    );
    let mut response = json(r.code.status(), &r.code.body(echo));
    if r.code == Code::RateLimited
        && let Ok(value) = HeaderValue::from_str(&r.retry_after.clamp(1, 30).to_string())
    {
        response.headers_mut().insert("retry-after", value);
    }
    response
}
fn json(status: u16, value: &impl Serialize) -> Response {
    match wire::encode(value) {
        Ok(b) => {
            // Public protocol 2 values are ASCII identifiers; version is operator supplied.
            // Match the outer Go encoder's HTML escaping for that diagnostic string.
            let mut b = String::from_utf8_lossy(&b)
                .replace('&', "\\u0026")
                .replace('<', "\\u003c")
                .replace('>', "\\u003e")
                .into_bytes();
            b.push(b'\n');
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                [("content-type", "application/json")],
                Body::from(b),
            )
                .into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "").into_response(),
    }
}
async fn request_log(req: Request, next: Next) -> Response {
    let candidate = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let id = if !candidate.is_empty()
        && candidate.len() <= 64
        && candidate
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        candidate.to_owned()
    } else {
        uuid::Uuid::new_v4().simple().to_string()
    };
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let start = Instant::now();
    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert("x-request-id", value);
    }
    tracing::info!(request_id = id, %method, %path, status = response.status().as_u16(), duration_ms = start.elapsed().as_millis(), "http_request");
    response
}

#[cfg(test)]
mod golden;
