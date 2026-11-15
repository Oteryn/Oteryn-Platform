//! Stateless, opt-in native login gateway. No database or signing configuration is loaded.
pub mod config;
pub mod errors;
pub mod issuer;
pub mod limiter;
pub mod recover;
pub mod v3;
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
use recover::{RecoverCode, RecoverRefusal};
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
    protocol3: bool,
    version: String,
    start: Instant,
    sources: Mutex<WindowLimiter>,
    attempts: Mutex<WindowLimiter>,
    recovery: RecoveryLimits,
}
/// §19.9: `/v1/recover` per source and per (`attempt_ref`, handle hash),
/// `/v1/recover/revoke` per source; separate from the login budgets.
#[derive(Debug)]
struct RecoveryLimits {
    sources: Mutex<WindowLimiter>,
    attempts: Mutex<WindowLimiter>,
    revoke_sources: Mutex<WindowLimiter>,
}
impl Default for RecoveryLimits {
    fn default() -> Self {
        Self {
            sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
            attempts: Mutex::new(attempt_limiter()),
            revoke_sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
        }
    }
}
/// Public surface switches. Protocol 3 (contract §19.1) is a separate
/// default-off switch that applies only while native login is enabled.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub enabled: bool,
    pub protocol3: bool,
    pub version: String,
}
/// Constructs the same public HTTP surface as the Go gateway, with native login off unless enabled.
pub fn router(issuer: IssuerClient, enabled: bool, version: String) -> Router {
    router_with(
        issuer,
        Options {
            enabled,
            protocol3: false,
            version,
        },
    )
}
/// `router` with the §19 switches.
pub fn router_with(issuer: IssuerClient, options: Options) -> Router {
    let state = Arc::new(StateData {
        issuer,
        enabled: options.enabled,
        protocol3: options.enabled && options.protocol3,
        version: options.version,
        start: Instant::now(),
        sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
        attempts: Mutex::new(attempt_limiter()),
        recovery: RecoveryLimits::default(),
    });
    Router::new()
        .route(
            "/health",
            get(|| async { json(200, &serde_json::json!({"status":"ok"})) }),
        )
        .route("/ready", get(ready))
        .route("/version", get(version_info))
        .route("/v1/login", post(login))
        .route("/v1/recover", post(recover_route))
        .route("/v1/recover/revoke", post(revoke_route))
        .layer(middleware::from_fn(request_log))
        .layer(middleware::from_fn(request_deadline))
        .with_state(state)
}
pub const BODY_DEADLINE: Duration = Duration::from_secs(5);
// Allow the complete body budget plus the issuer's maximum 8 s exchange so its
// existing typed refusals still win for completed requests.
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(14);
pub const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(6);

async fn request_deadline(req: Request, next: Next) -> Response {
    match tokio::time::timeout(REQUEST_DEADLINE, next.run(req)).await {
        Ok(response) => response,
        Err(_) => request_timeout(),
    }
}
fn request_timeout() -> Response {
    let mut response = StatusCode::REQUEST_TIMEOUT.into_response();
    response
        .headers_mut()
        .insert("connection", HeaderValue::from_static("close"));
    no_store(&mut response);
    response
}
/// Bound header reads, then drain or abort tracked connections on shutdown.
pub async fn serve_until(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let (drain, draining) = tokio::sync::watch::channel(false);
    let mut connections = tokio::task::JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => break,
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let app = app.clone().layer(axum::Extension(ConnectInfo(peer)));
                let mut draining = draining.clone();
                connections.spawn(async move {
                    let service = hyper_util::service::TowerToHyperService::new(app);
                    let mut builder = hyper::server::conn::http1::Builder::new();
                    builder.timer(hyper_util::rt::TokioTimer::new()).header_read_timeout(BODY_DEADLINE);
                    let connection = builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service);
                    tokio::pin!(connection);
                    tokio::select! {
                        _ = &mut connection => {},
                        _ = draining.changed() => {
                            connection.as_mut().graceful_shutdown();
                            let _ = connection.await;
                        }
                    }
                });
            }
            _ = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    let _ = drain.send(true);
    if tokio::time::timeout(SHUTDOWN_DEADLINE, async {
        while connections.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    Ok(())
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

fn source_of(req: &Request) -> String {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|i| i.0.ip().to_string())
        .unwrap_or_default()
}
async fn login(State(s): State<Arc<StateData>>, req: Request) -> Response {
    let source = source_of(&req);
    let query = req.uri().query().is_some_and(|q| !q.is_empty());
    let mut response = if query {
        invalid()
    } else {
        let body =
            match tokio::time::timeout(BODY_DEADLINE, to_bytes(req.into_body(), 16 * 1024)).await {
                Ok(body) => body,
                Err(_) => return request_timeout(),
            };
        match body {
            Ok(b) if s.enabled && wire::is_native(&b) => native(&s, &source, &b).await,
            // §19.1: with the protocol 3 switch off every protocol 3 request is
            // NATIVE_LOGIN_UNSUPPORTED_VERSION, in the protocol 2 body.
            Ok(b) if s.enabled && v3::is_v3(&b) && !s.protocol3 => refuse(
                Code::UnsupportedVersion.into(),
                wire::attempt_echo(&b).as_deref(),
            ),
            Ok(b) if s.protocol3 && v3::is_v3(&b) => native3(&s, &source, &b).await,
            _ => invalid(),
        }
    };
    no_store(&mut response);
    response
}
fn no_store(response: &mut Response) {
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
}
/// The recovery routes exist only while native login is on; with protocol 3
/// off every request is `NATIVE_LOGIN_UNSUPPORTED_VERSION` in the protocol 2
/// body (§19.1). Bodies are read up to the login bound and then held to the
/// §19.9 request limit by the parser.
async fn recovery_front(
    s: &StateData,
    req: Request,
    echo_of: fn(&[u8]) -> Option<String>,
) -> Result<(String, Vec<u8>), Response> {
    if !s.enabled {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    let source = source_of(&req);
    let query = req.uri().query().is_some_and(|q| !q.is_empty());
    let body = tokio::time::timeout(BODY_DEADLINE, to_bytes(req.into_body(), 16 * 1024))
        .await
        .map_err(|_| request_timeout())?
        .map(|b| b.to_vec())
        .unwrap_or_default();
    let echo = echo_of(&body);
    if !s.protocol3 {
        return Err(refuse(Code::UnsupportedVersion.into(), echo.as_deref()));
    }
    if query {
        return Err(refuse_recovery(
            RecoverCode::Malformed.into(),
            echo.as_deref(),
        ));
    }
    Ok((source, body))
}
async fn recover_route(State(s): State<Arc<StateData>>, req: Request) -> Response {
    let mut response = match recovery_front(&s, req, wire::attempt_echo).await {
        Ok((source, body)) => recovery(&s, &source, &body).await,
        Err(refused) => refused,
    };
    no_store(&mut response);
    response
}
async fn recovery(s: &StateData, source: &str, body: &[u8]) -> Response {
    let echo = wire::attempt_echo(body);
    let now = s.start.elapsed();
    let result = match prepare_recovery(s, source, body, now) {
        Ok(request) => s.issuer.recover(&request).await,
        Err(refused) => Err(refused),
    };
    match result {
        Ok(success) => {
            tracing::info!(
                attempt_ref = success.attempt_ref,
                code = "OK",
                protocol_version = 3,
                "native_recovery_result"
            );
            json(200, &success)
        }
        Err(refused) => refuse_recovery(refused, echo.as_deref()),
    }
}
/// §19.9 order as on login: the source budget, the parse, then the
/// (`attempt_ref`, handle hash) budget. Only the handle's hash is kept.
fn prepare_recovery(
    s: &StateData,
    source: &str,
    body: &[u8],
    now: Duration,
) -> Result<recover::RecoverRequest, RecoverRefusal> {
    allow(&s.recovery.sources, source.as_bytes(), now).map_err(recover_limit)?;
    let request = recover::parse_request(body)?;
    let mut digest = Sha256::new();
    digest.update(request.attempt_ref.as_bytes());
    digest.update([0]);
    digest.update(Sha256::digest(request.recovery_handle.as_bytes()));
    allow(&s.recovery.attempts, &digest.finalize(), now).map_err(recover_limit)?;
    Ok(request)
}
fn recover_limit(r: Refusal) -> RecoverRefusal {
    RecoverRefusal {
        code: r.code.into(),
        retry_after: r.retry_after,
    }
}
async fn revoke_route(State(s): State<Arc<StateData>>, req: Request) -> Response {
    let mut response = match recovery_front(&s, req, |_| None).await {
        Ok((source, body)) => revocation(&s, &source, &body).await,
        Err(refused) => refused,
    };
    no_store(&mut response);
    response
}
async fn revocation(s: &StateData, source: &str, body: &[u8]) -> Response {
    let result = match allow(
        &s.recovery.revoke_sources,
        source.as_bytes(),
        s.start.elapsed(),
    ) {
        Ok(()) => match recover::parse_revoke(body) {
            Ok(request) => s.issuer.revoke(&request).await,
            Err(code) => Err(code.into()),
        },
        Err(r) => Err(recover_limit(r)),
    };
    match result {
        Ok(()) => {
            tracing::info!(code = "OK", "native_recovery_revoke_result");
            json(200, &recover::REVOKED)
        }
        Err(refused) => refuse_recovery(refused, None),
    }
}
/// §19.10: only the attempt and the code are logged, never the handle, its
/// hash or the grant.
fn refuse_recovery(r: RecoverRefusal, echo: Option<&str>) -> Response {
    tracing::info!(
        attempt_ref = echo.unwrap_or(""),
        code = r.code.name(),
        protocol_version = 3,
        "native_recovery_result"
    );
    let mut response = json(r.code.status(), &recover::body(r.code, echo));
    let seconds = match r.code {
        RecoverCode::RateLimited => Some(r.retry_after.clamp(1, 30)),
        RecoverCode::AttemptPending => Some(r.retry_after.max(1)),
        _ => None,
    };
    if let Some(value) = seconds.and_then(|v| HeaderValue::from_str(&v.to_string()).ok()) {
        response.headers_mut().insert("retry-after", value);
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
async fn native3(s: &StateData, source: &str, body: &[u8]) -> Response {
    let echo = wire::attempt_echo(body);
    let result = match prepare_with(s, source, body, s.start.elapsed(), v3::parse_request) {
        Ok(request) => s.issuer.admit3(&request).await.map(|ok| (request, ok)),
        Err(r) => Err(r.into()),
    };
    match result {
        Ok((request, success)) => {
            tracing::info!(
                attempt_ref = request.attempt_ref,
                code = "OK",
                protocol_version = 3,
                "native_login_result"
            );
            json(200, &success)
        }
        Err(refused) => refuse3(&refused, echo.as_deref()),
    }
}
fn refuse3(refused: &v3::Refusal3, echo: Option<&str>) -> Response {
    let r = refused.refusal;
    tracing::info!(
        attempt_ref = echo.unwrap_or(""),
        code = r.code.name(),
        protocol_version = 3,
        "native_login_result"
    );
    let mut response = json(r.code.status(), &v3::body(refused, echo));
    if r.code == Code::RateLimited
        && let Ok(value) = HeaderValue::from_str(&r.retry_after.clamp(1, 30).to_string())
    {
        response.headers_mut().insert("retry-after", value);
    }
    response
}
fn prepare(
    s: &StateData,
    source: &str,
    body: &[u8],
    now: Duration,
) -> Result<wire::Request, Refusal> {
    prepare_with(s, source, body, now, wire::parse_request)
}
fn prepare_with(
    s: &StateData,
    source: &str,
    body: &[u8],
    now: Duration,
    parse: fn(&[u8]) -> Result<wire::Request, Code>,
) -> Result<wire::Request, Refusal> {
    allow(&s.sources, source.as_bytes(), now)?;
    let request = parse(body).map_err(Refusal::from)?;
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

// Recovery handles have a maximum lifetime of 12 hours (§19.3 U20).
// Starting retention at first use conservatively covers the remaining lifetime;
// login tickets expire sooner. Never evict a live total budget for capacity.
fn attempt_limiter() -> WindowLimiter {
    WindowLimiter::new(6, Duration::from_secs(12 * 60 * 60), 100_000)
}

#[cfg(test)]
mod attempt_budget_tests {
    use super::*;

    #[test]
    fn login_total_budget_survives_old_window() {
        check_total_budget(attempt_limiter());
    }

    #[test]
    fn recovery_total_budget_survives_old_window() -> Result<(), Box<dyn std::error::Error>> {
        check_total_budget(
            RecoveryLimits::default()
                .attempts
                .into_inner()
                .map_err(|_| "poisoned limiter")?,
        );
        Ok(())
    }

    #[test]
    fn recovery_refuses_after_120_seconds_before_issuer_dispatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = StateData {
            issuer: IssuerClient::new(
                "http://127.0.0.1:1",
                "synthetic".into(),
                Duration::from_secs(1),
                Duration::from_secs(1),
            )?,
            enabled: true,
            protocol3: true,
            version: "test".into(),
            start: Instant::now(),
            sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
            attempts: Mutex::new(attempt_limiter()),
            recovery: RecoveryLimits::default(),
        };
        let body = include_bytes!(
            "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/recover-request.json"
        );
        let mut issuer_dispatches = 0;
        for _ in 0..6 {
            if prepare_recovery(&state, "127.0.0.1", body, Duration::ZERO).is_ok() {
                issuer_dispatches += 1;
            }
        }
        let result = prepare_recovery(&state, "127.0.0.1", body, Duration::from_secs(121));
        assert!(result.is_err());
        let refusal = result.err().ok_or("exhausted total budget")?;
        assert_eq!(
            refuse_recovery(refusal, None).status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(issuer_dispatches, 6);
        Ok(())
    }

    fn check_total_budget(mut limiter: WindowLimiter) {
        for _ in 0..6 {
            assert!(limiter.allow(b"attempt-and-handle", Duration::ZERO).is_ok());
        }
        for seconds in [121, 3600, 43199] {
            assert!(
                limiter
                    .allow(b"attempt-and-handle", Duration::from_secs(seconds))
                    .is_err()
            );
        }
    }
}
