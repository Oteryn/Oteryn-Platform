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
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct StateData {
    trusted_proxies: Vec<IpAddr>,
    issuer: IssuerClient,
    enabled: bool,
    protocol3: bool,
    version: String,
    start: Instant,
    sources: Mutex<WindowLimiter>,
    attempts: Mutex<AttemptLimits>,
    recovery: RecoveryLimits,
}
/// §19.9: `/v1/recover` per source and per (`attempt_ref`, handle hash),
/// `/v1/recover/revoke` per source; separate from the login budgets.
#[derive(Debug)]
struct RecoveryLimits {
    sources: Mutex<WindowLimiter>,
    attempts: Mutex<AttemptLimits>,
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
    router_with_proxies(issuer, options, Vec::new())
}
/// Only these exact socket peers may supply the single Caddy-overwritten address.
pub fn router_with_proxies(
    issuer: IssuerClient,
    options: Options,
    trusted_proxies: Vec<IpAddr>,
) -> Router {
    let state = Arc::new(StateData {
        trusted_proxies,
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

/// Rate-limit identity: the socket peer unless it is an explicitly trusted proxy
/// (the gateway list or the shared `TRUSTED_PROXIES`). A trusted proxy (Caddy
/// overwrites the header) may name exactly one client address; a chain keeps the
/// peer budget, and malformed forwarding is refused rather than guessed.
fn source_of(req: &Request, trusted: &[IpAddr]) -> Result<String, ()> {
    let Some(peer) = req.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return Ok(String::new());
    };
    let peer = peer.0.ip();
    let shared = oteryn_internal_api::proxy::TrustedProxies::configured().map_err(|_| ())?;
    if !trusted.contains(&peer) && !shared.0.contains(&peer) {
        return Ok(peer.to_string());
    }
    let mut chain = Vec::new();
    for value in req.headers().get_all("x-forwarded-for") {
        let value = value.to_str().map_err(|_| ())?;
        if value.len() > 4096 {
            return Err(());
        }
        for ip in value.split(',') {
            if chain.len() >= 32 {
                return Err(());
            }
            chain.push(ip.trim().parse::<IpAddr>().map_err(|_| ())?);
        }
    }
    match chain.as_slice() {
        [client] => Ok(client.to_string()),
        _ => Ok(peer.to_string()),
    }
}
async fn login(State(s): State<Arc<StateData>>, req: Request) -> Response {
    let Ok(source) = source_of(&req, &s.trusted_proxies) else {
        let mut response = StatusCode::SERVICE_UNAVAILABLE.into_response();
        no_store(&mut response);
        return response;
    };
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
    let Ok(source) = source_of(&req, &s.trusted_proxies) else {
        return Err(StatusCode::SERVICE_UNAVAILABLE.into_response());
    };
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
    recovery_at(s, source, body, s.start.elapsed()).await
}
async fn recovery_at(s: &StateData, source: &str, body: &[u8], now: Duration) -> Response {
    let echo = wire::attempt_echo(body);
    let result = match prepare_recovery(s, source, body, now) {
        Ok(request) => {
            let result = s.issuer.recover(&request).await;
            let key = attempt_key(
                &request.attempt_ref,
                &Sha256::digest(request.recovery_handle.as_bytes()),
            );
            if let Ok(mut limits) = s.recovery.attempts.lock() {
                limits.finish(
                    &key,
                    now,
                    result.is_ok()
                        || result.as_ref().err().is_some_and(|r| {
                            matches!(
                                r.code,
                                RecoverCode::AttemptPending
                                    | RecoverCode::CharacterConflict
                                    | RecoverCode::WorldStale
                                    | RecoverCode::RouteUnavailable
                                    | RecoverCode::GrantExpired
                            )
                        }),
                    result
                        .as_ref()
                        .err()
                        .is_some_and(|r| r.code == RecoverCode::AuthenticationRequired),
                );
            }
            result
        }
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
    let key = attempt_key(
        &request.attempt_ref,
        &Sha256::digest(request.recovery_handle.as_bytes()),
    );
    reserve(&s.recovery.attempts, &key, now).map_err(recover_limit)?;
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
    native_at(s, source, body, s.start.elapsed()).await
}
async fn native_at(s: &StateData, source: &str, body: &[u8], now: Duration) -> Response {
    let echo = wire::attempt_echo(body);
    let request = match prepare(s, source, body, now) {
        Ok(request) => request,
        Err(r) => return refuse(r, echo.as_deref()),
    };
    let result = s.issuer.admit(&request).await;
    finish(
        &s.attempts,
        &attempt_key(&request.attempt_ref, request.game_login_ticket.as_bytes()),
        now,
        result.as_ref().err().map(|r| r.code),
    );
    match result {
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
    native3_at(s, source, body, s.start.elapsed()).await
}
async fn native3_at(s: &StateData, source: &str, body: &[u8], now: Duration) -> Response {
    let echo = wire::attempt_echo(body);
    let result = match prepare_with(s, source, body, now, v3::parse_request) {
        Ok(request) => {
            let result = s.issuer.admit3(&request).await;
            finish(
                &s.attempts,
                &attempt_key(&request.attempt_ref, request.game_login_ticket.as_bytes()),
                now,
                result.as_ref().err().map(|r| r.refusal.code),
            );
            result.map(|ok| (request, ok))
        }
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
    reserve(
        &s.attempts,
        &attempt_key(&request.attempt_ref, request.game_login_ticket.as_bytes()),
        now,
    )?;
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
#[derive(Debug)]
struct PendingAttempt {
    start: Duration,
    count: usize,
    in_flight: usize,
}
#[derive(Debug)]
struct AttemptLimits {
    confirmed: WindowLimiter,
    unverified: std::collections::HashMap<Vec<u8>, PendingAttempt>,
    reservation_window: Duration,
}
impl AttemptLimits {
    fn allow(&mut self, key: &[u8], now: Duration) -> Result<(), u64> {
        if self.confirmed.contains(key, now) {
            return self.confirmed.allow(key, now);
        }
        if let Some(pending) = self.unverified.get_mut(key)
            && now.saturating_sub(pending.start) < self.reservation_window
        {
            if pending.count >= 6 {
                return Err(self
                    .reservation_window
                    .saturating_sub(now.saturating_sub(pending.start))
                    .as_secs()
                    .max(1));
            }
            pending.count += 1;
            pending.in_flight += 1;
            return Ok(());
        }
        // Source admission happens first. In-flight reservations retain a lifetime
        // budget even if a caller is cancelled before finish. Include both tables in
        // the memory bound so promotion always has a reserved slot.
        if self.confirmed.len() + self.unverified.len() >= 100_000 {
            self.confirmed.prune(now);
            self.unverified
                .retain(|_, pending| now.saturating_sub(pending.start) < self.reservation_window);
            if self.confirmed.len() + self.unverified.len() >= 100_000 {
                return Err(60);
            }
        }
        self.unverified.insert(
            key.to_vec(),
            PendingAttempt {
                start: now,
                count: 1,
                in_flight: 1,
            },
        );
        Ok(())
    }
    fn finish(&mut self, key: &[u8], now: Duration, authenticated: bool, rejected: bool) {
        if authenticated || !rejected {
            if let Some(pending) = self.unverified.remove(key)
                && !self.confirmed.contains(key, now)
            {
                // Count all concurrent reservations conservatively. A rejected
                // exchange cannot erase another exchange's pending reservation.
                self.confirmed.insert(key, pending.start, pending.count);
            }
        } else if let Some(pending) = self.unverified.get_mut(key) {
            pending.in_flight = pending.in_flight.saturating_sub(1);
            if pending.in_flight == 0 {
                self.unverified.remove(key);
            }
        }
    }
}
fn attempt_limiter() -> AttemptLimits {
    AttemptLimits {
        confirmed: WindowLimiter::new(6, Duration::from_secs(12 * 60 * 60), 100_000),
        unverified: std::collections::HashMap::new(),
        reservation_window: Duration::from_secs(12 * 60 * 60),
    }
}
fn attempt_key(attempt: &str, credential: &[u8]) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update(attempt.as_bytes());
    digest.update([0]);
    digest.update(credential);
    digest.finalize().to_vec()
}
fn reserve(l: &Mutex<AttemptLimits>, key: &[u8], now: Duration) -> Result<(), Refusal> {
    l.lock()
        .map_err(|_| Refusal::from(Code::Unavailable))?
        .allow(key, now)
        .map_err(|retry_after| Refusal {
            code: Code::RateLimited,
            retry_after,
        })
}
fn finish(l: &Mutex<AttemptLimits>, key: &[u8], now: Duration, code: Option<Code>) {
    if let Ok(mut limits) = l.lock() {
        limits.finish(
            key,
            now,
            code.is_none()
                || matches!(
                    code,
                    Some(
                        Code::CharacterConflict
                            | Code::RouteUnavailable
                            | Code::GrantExpired
                            | Code::ChannelUnavailable
                    )
                ),
            code == Some(Code::AuthenticationRequired),
        );
    }
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
            trusted_proxies: Vec::new(),
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
        let request = recover::parse_request(body).map_err(|_| "invalid test request")?;
        let key = attempt_key(
            &request.attempt_ref,
            &Sha256::digest(request.recovery_handle.as_bytes()),
        );
        state
            .recovery
            .attempts
            .lock()
            .map_err(|_| "poisoned")?
            .finish(&key, Duration::ZERO, true, false);
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

    #[test]
    fn reconciliation_totals_refuse_before_login_and_recovery_dispatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = StateData {
            trusted_proxies: Vec::new(),
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
        let login = include_bytes!("../tests/fixtures/game_client_native_login/request.json");
        let recovery = include_bytes!(
            "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/recover-request.json"
        );
        let mut dispatches = [0, 0];
        for _ in 0..6 {
            let request =
                prepare(&state, "client", login, Duration::ZERO).map_err(|_| "login admission")?;
            dispatches[0] += 1;
            finish(
                &state.attempts,
                &attempt_key(&request.attempt_ref, request.game_login_ticket.as_bytes()),
                Duration::ZERO,
                Some(Code::ReconciliationRequired),
            );
            let request = prepare_recovery(&state, "client", recovery, Duration::ZERO)
                .map_err(|_| "recovery admission")?;
            dispatches[1] += 1;
            state
                .recovery
                .attempts
                .lock()
                .map_err(|_| "poisoned")?
                .finish(
                    &attempt_key(
                        &request.attempt_ref,
                        &Sha256::digest(request.recovery_handle.as_bytes()),
                    ),
                    Duration::ZERO,
                    false,
                    false,
                );
        }
        let login = prepare(&state, "client", login, Duration::from_secs(61))
            .err()
            .ok_or("seventh dispatched")?;
        assert_eq!(login.code, Code::RateLimited);
        let recovery = prepare_recovery(&state, "client", recovery, Duration::from_secs(61))
            .err()
            .ok_or("seventh dispatched")?;
        assert_eq!(recovery.code, RecoverCode::RateLimited);
        assert_eq!(dispatches, [6, 6]);
        Ok(())
    }

    #[test]
    fn rejected_exchange_cannot_erase_concurrent_confirmation() {
        let mut limits = attempt_limiter();
        assert!(limits.allow(b"same-credential", Duration::ZERO).is_ok());
        assert!(limits.allow(b"same-credential", Duration::ZERO).is_ok());
        limits.finish(b"same-credential", Duration::ZERO, false, true);
        limits.finish(b"same-credential", Duration::ZERO, true, false);
        for _ in 0..4 {
            assert!(
                limits
                    .allow(b"same-credential", Duration::from_secs(121))
                    .is_ok()
            );
        }
        assert!(
            limits
                .allow(b"same-credential", Duration::from_secs(121))
                .is_err()
        );
    }

    #[test]
    fn uncertain_capacity_flood_never_evicts_a_live_budget() {
        let mut limits = attempt_limiter();
        for _ in 0..6 {
            assert!(limits.allow(b"confirmed", Duration::ZERO).is_ok());
        }
        limits.finish(b"confirmed", Duration::ZERO, true, false);
        for i in 0u64..99_999 {
            let now = Duration::from_secs(i / 150 * 60);
            let key = i.to_be_bytes();
            assert!(limits.allow(&key, now).is_ok());
            limits.finish(&key, now, false, false);
        }
        assert_eq!(limits.confirmed.len(), 100_000);
        assert!(
            limits
                .allow(b"confirmed", Duration::from_secs(40020))
                .is_err()
        );
        assert!(limits.allow(b"fresh", Duration::from_secs(40020)).is_err());
        assert!(limits.allow(b"fresh", Duration::from_secs(43200)).is_ok());
    }

    #[test]
    fn uncertain_credentials_reserve_lifetime_capacity() {
        let mut limits = attempt_limiter();
        for _ in 0..6 {
            assert!(limits.allow(b"uncertain", Duration::ZERO).is_ok());
            limits.finish(b"uncertain", Duration::ZERO, false, false);
        }
        for seconds in [61, 121, 3600, 43199] {
            assert!(
                limits
                    .allow(b"uncertain", Duration::from_secs(seconds))
                    .is_err()
            );
        }
        assert!(
            limits
                .allow(b"uncertain", Duration::from_secs(43200))
                .is_ok()
        );
    }

    fn check_total_budget(mut limiter: AttemptLimits) {
        for _ in 0..6 {
            assert!(limiter.allow(b"attempt-and-handle", Duration::ZERO).is_ok());
        }
        limiter.finish(b"attempt-and-handle", Duration::ZERO, true, false);
        for seconds in [121, 3600, 43199] {
            assert!(
                limiter
                    .allow(b"attempt-and-handle", Duration::from_secs(seconds))
                    .is_err()
            );
        }
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[tokio::test]
    async fn rejected_credentials_cannot_exhaust_lifetime_capacity()
    -> Result<(), Box<dyn std::error::Error>> {
        let calls = Arc::new(AtomicUsize::new(0));
        let confirmed = Arc::new(AtomicBool::new(false));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let count = calls.clone();
        let authenticated = confirmed.clone();
        let app = Router::new().fallback(post(move |req: Request| {
            let count = count.clone();
            let authenticated = authenticated.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                let recovery = req.uri().path() == recover::RECOVER_PATH;
                let b = to_bytes(req.into_body(), 4096).await.map_err(|_| StatusCode::BAD_REQUEST)?;
                let v: serde_json::Value = serde_json::from_slice(&b).map_err(|_| StatusCode::BAD_REQUEST)?;
                let code = if authenticated.load(Ordering::SeqCst) {
                    if recovery { "NATIVE_RECOVERY_CHARACTER_CONFLICT" } else { "NATIVE_LOGIN_CHARACTER_CONFLICT" }
                } else { "NATIVE_LOGIN_AUTHENTICATION_REQUIRED" };
                Ok::<_, StatusCode>(json(if authenticated.load(Ordering::SeqCst) {409} else {401}, &serde_json::json!({
                    "protocol_version": v["protocol_version"],
                    "attempt_ref": v["attempt_ref"],
                    "error": { "code": code, "public_class": if authenticated.load(Ordering::SeqCst) {"SESSION_UNAVAILABLE"} else {"AUTHENTICATION_REQUIRED"} }
                })))
            }
        }));
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let login2: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/game_client_native_login/request.json"
        ))?;
        let login3: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-request.json"
        ))?;
        let recovery: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/recover-request.json"
        ))?;
        // Each surface gets the entire 100,000-key attack within five sources'
        // 30/minute budgets, using an injected clock over less than 12 hours.
        for (surface, template) in [login2, login3, recovery].into_iter().enumerate() {
            let state = StateData {
                trusted_proxies: Vec::new(),
                issuer: IssuerClient::new(
                    &url,
                    "synthetic".into(),
                    Duration::from_secs(2),
                    Duration::from_secs(2),
                )?,
                enabled: true,
                protocol3: true,
                version: "test".into(),
                start: Instant::now(),
                sources: Mutex::new(WindowLimiter::new(30, Duration::from_secs(60), 100_000)),
                attempts: Mutex::new(attempt_limiter()),
                recovery: RecoveryLimits::default(),
            };
            for i in 0..100_000 {
                if i % 50_000 == 0 {
                    eprintln!("gateway capacity regression surface={surface} rejected={i}");
                }
                let now = Duration::from_secs((i / 150) * 60);
                let mut request = template.clone();
                request["attempt_ref"] =
                    serde_json::json!(format!("0192b3c4-5d6e-7f80-9a1b-{i:012x}"));
                if surface < 2 {
                    request["game_login_ticket"] =
                        serde_json::json!(format!("invented-ticket-{i}"));
                } else {
                    request["recovery_handle"] = serde_json::json!(format!("otr1.{i:042x}A"));
                }
                let body = serde_json::to_vec(&request)?;
                let source = format!("192.0.2.{}", i % 5);
                let response = match surface {
                    0 => native_at(&state, &source, &body, now).await,
                    1 => native3_at(&state, &source, &body, now).await,
                    _ => recovery_at(&state, &source, &body, now).await,
                };
                assert_eq!(
                    response.status(),
                    StatusCode::UNAUTHORIZED,
                    "surface {surface}, request {i}"
                );
            }
            confirmed.store(true, Ordering::SeqCst);
            let body = serde_json::to_vec(&template)?;
            for n in 0..7 {
                let now = Duration::from_secs(40020 + n * 121);
                let response = match surface {
                    0 => native_at(&state, "198.51.100.1", &body, now).await,
                    1 => native3_at(&state, "198.51.100.1", &body, now).await,
                    _ => recovery_at(&state, "198.51.100.1", &body, now).await,
                };
                assert_eq!(
                    response.status(),
                    if n < 6 {
                        StatusCode::CONFLICT
                    } else {
                        StatusCode::TOO_MANY_REQUESTS
                    }
                );
            }
            confirmed.store(false, Ordering::SeqCst);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 300_018);
        server.abort();
        Ok(())
    }
}

#[cfg(test)]
mod proxy_tests {
    use super::*;
    #[test]
    fn trusted_proxy_separates_clients_and_untrusted_peer_cannot_spoof()
    -> Result<(), Box<dyn std::error::Error>> {
        let proxy: IpAddr = "192.0.2.1".parse()?;
        let mut limiter = WindowLimiter::new(30, Duration::from_secs(60), 100);
        let request = |peer: IpAddr, client: &str| -> Result<Request, axum::http::Error> {
            axum::http::Request::builder()
                .header("x-forwarded-for", client)
                .extension(ConnectInfo(SocketAddr::new(peer, 1234)))
                .body(Body::empty())
        };
        let a = source_of(&request(proxy, "198.51.100.1")?, &[proxy]).map_err(|_| "source")?;
        let b = source_of(&request(proxy, "198.51.100.2")?, &[proxy]).map_err(|_| "source")?;
        for _ in 0..30 {
            assert!(limiter.allow(a.as_bytes(), Duration::ZERO).is_ok());
        }
        assert!(limiter.allow(a.as_bytes(), Duration::ZERO).is_err());
        assert!(limiter.allow(b.as_bytes(), Duration::ZERO).is_ok());
        let peer: IpAddr = "203.0.113.9".parse()?;
        for forwarded in [
            "198.51.100.1",
            "198.51.100.2",
            "false",
            "198.51.100.1, 198.51.100.2",
        ] {
            assert_eq!(
                source_of(&request(peer, forwarded)?, &[proxy]).map_err(|_| "source")?,
                peer.to_string()
            );
        }
        assert_eq!(
            source_of(&request(proxy, "198.51.100.1, 198.51.100.2")?, &[proxy])
                .map_err(|_| "source")?,
            proxy.to_string()
        );
        Ok(())
    }
}

#[cfg(test)]
mod proxy_budget_tests {
    use super::*;
    #[tokio::test]
    async fn trusted_proxy_keeps_native_callers_independent()
    -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("L9_GATEWAY_CHILD").is_err() {
            let output = std::process::Command::new(std::env::current_exe()?)
                .args([
                    "--exact",
                    "proxy_budget_tests::trusted_proxy_keeps_native_callers_independent",
                    "--nocapture",
                ])
                .env("L9_GATEWAY_CHILD", "1")
                .env("TRUSTED_PROXIES", "10.0.0.1")
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }
        let request =
            |peer: &str, forwarded: &str| -> Result<Request, Box<dyn std::error::Error>> {
                Ok(Request::builder()
                    .extension(ConnectInfo(peer.parse::<SocketAddr>()?))
                    .header("x-forwarded-for", forwarded)
                    .body(Body::empty())?)
            };
        let a = source_of(&request("10.0.0.1:40000", "192.0.2.1")?, &[]).map_err(|_| "source A")?;
        let b = source_of(&request("10.0.0.1:40000", "192.0.2.2")?, &[]).map_err(|_| "source B")?;
        use tower::ServiceExt;
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let issuer_app = Router::new().route(
            issuer::ISSUER_PATH,
            post(move || {
                let observed = observed.clone();
                async move {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    StatusCode::SERVICE_UNAVAILABLE
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move { axum::serve(listener, issuer_app).await });
        let app = router_with(
            IssuerClient::new(
                &format!("http://{address}"),
                "fixture".into(),
                Duration::from_secs(1),
                Duration::from_secs(1),
            )?,
            Options {
                enabled: true,
                protocol3: true,
                version: "test".into(),
            },
        );
        for _ in 0..30 {
            let mut req = request("10.0.0.1:40000", &a)?;
            *req.method_mut() = axum::http::Method::POST;
            *req.uri_mut() = "/v1/login".parse()?;
            *req.body_mut() = Body::from(r#"{"protocol_version":3}"#);
            assert_eq!(
                app.clone().oneshot(req).await?.status(),
                StatusCode::BAD_REQUEST
            );
        }
        for (source, expected) in [
            (&a, StatusCode::TOO_MANY_REQUESTS),
            (&b, StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let mut req = request("10.0.0.1:40000", source)?;
            *req.method_mut() = axum::http::Method::POST;
            *req.uri_mut() = "/v1/login".parse()?;
            *req.body_mut() = Body::from(include_str!(
                "../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-request.json"
            ));
            assert_eq!(app.clone().oneshot(req).await?.status(), expected);
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        task.abort();
        assert_eq!(
            source_of(&request("198.51.100.1:40000", "192.0.2.2")?, &[]).map_err(|_| "source")?,
            "198.51.100.1"
        );
        assert!(source_of(&request("10.0.0.1:40000", "spoofed")?, &[]).is_err());
        Ok(())
    }
}
