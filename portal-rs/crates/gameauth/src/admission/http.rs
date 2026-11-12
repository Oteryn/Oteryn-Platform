use super::{
    AccountStateLookup, AdmissionAuthority, AdmissionRequest, Error, GrantSigner,
    protocol3::{Handles, Protocol3},
    recovery::{self, RecoveryError, RecoveryIssuer, RecoveryRequest},
    store, wire,
};
use axum::{
    Router,
    body::to_bytes,
    extract::{Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{path::Path, sync::Arc};
use subtle::{Choice, ConstantTimeEq};

#[derive(Clone)]
pub struct ServiceTokens {
    current: [u8; 32],
    previous: Option<[u8; 32]>,
}
impl std::fmt::Debug for ServiceTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServiceTokens{redacted}")
    }
}
impl ServiceTokens {
    /// Files contain a hexadecimal SHA-256 digest, optionally with a final newline.
    /// No plaintext credential configuration is accepted.
    pub fn from_files(current: &Path, previous: Option<&Path>) -> Result<Self, Error> {
        fn read(path: &Path) -> Result<[u8; 32], Error> {
            let data = std::fs::read(path).map_err(|_| Error::Unavailable)?;
            let raw = data.strip_suffix(b"\n").unwrap_or(&data);
            if raw.len() != 64 || !raw.iter().all(u8::is_ascii_hexdigit) {
                return Err(Error::Unavailable);
            }
            let mut hash = [0; 32];
            for (index, pair) in raw.chunks_exact(2).enumerate() {
                let text = std::str::from_utf8(pair).map_err(|_| Error::Unavailable)?;
                hash[index] = u8::from_str_radix(text, 16).map_err(|_| Error::Unavailable)?;
            }
            Ok(hash)
        }
        Ok(Self {
            current: read(current)?,
            previous: previous.map(read).transpose()?,
        })
    }
    fn authenticates(&self, hash: &[u8; 32]) -> bool {
        let current = self.current.ct_eq(hash);
        let previous = self.previous.unwrap_or([0; 32]).ct_eq(hash)
            & Choice::from(u8::from(self.previous.is_some()));
        bool::from(current | previous)
    }
}
#[derive(Debug, Clone)]
pub struct AdmissionConfig {
    pub enabled: bool,
    pub tokens: Option<ServiceTokens>,
    /// Authenticated service requests per minute, bounded to 1..=600.
    pub requests_per_minute: u32,
    /// The grant signer the authority signs with, for replay re-signing (§6.2).
    /// When set, attempt records keep the signing input, never the token.
    pub signer: Option<Arc<dyn GrantSigner>>,
    /// Protocol 3 (§19); `None` keeps every protocol 3 request unsupported.
    pub protocol3: Option<Protocol3>,
    /// §19.4 recovery issuance; `None` keeps `native-recoveries` unavailable.
    pub recovery: Option<RecoveryIssuer>,
}
impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tokens: None,
            requests_per_minute: 120,
            signer: None,
            protocol3: None,
            recovery: None,
        }
    }
}
#[derive(Debug, Clone)]
struct App {
    db: PgPool,
    config: AdmissionConfig,
    accounts: Arc<dyn AccountStateLookup>,
    authority: Arc<dyn AdmissionAuthority>,
}
/// Mount only on private ingress. The native-admission contract explicitly uses
/// service authentication instead of the internal-api mTLS route-group guard.
/// Its shared bounded JSON codec rejects duplicate members and excessive depth.
pub fn router(
    db: PgPool,
    config: AdmissionConfig,
    accounts: Arc<dyn AccountStateLookup>,
    authority: Arc<dyn AdmissionAuthority>,
) -> Router {
    Router::new()
        .route("/internal/v1/game-auth/native-admissions", post(handle))
        .route("/internal/v1/game-auth/native-recoveries", post(recover))
        .route(
            "/internal/v1/game-auth/native-recoveries/revoke",
            post(revoke),
        )
        .with_state(App {
            db,
            config,
            accounts,
            authority,
        })
}
fn response(status: StatusCode, body: Vec<u8>) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store, private"),
            (header::PRAGMA, "no-cache"),
        ],
        body,
    )
        .into_response()
}
fn refusal(error: Error, attempt: Option<&str>) -> Response {
    refusal_for(error, attempt, 2, &[])
}
fn refusal_for(
    error: Error,
    attempt: Option<&str>,
    protocol: u8,
    channels: &[wire::Channel],
) -> Response {
    let mut result = response(error.status(), error.body_for(attempt, protocol, channels));
    if error == Error::RateLimited {
        result.headers_mut().insert(
            header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("60"),
        );
    }
    result
}
/// Gateway service authentication shared by every route: the credential's
/// SHA-256, or the refusal to send.
fn authenticate(app: &App, request: &Request) -> Result<[u8; 32], Box<Response>> {
    let Some(tokens) = &app.config.tokens else {
        return Err(Box::new(response(
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"error":"service_unavailable"}"#.to_vec(),
        )));
    };
    let mut headers = request.headers().get_all(header::AUTHORIZATION).iter();
    let credential = headers
        .next()
        .and_then(|h| h.to_str().ok())
        .and_then(|h| {
            h.get(..7)
                .filter(|prefix| prefix.eq_ignore_ascii_case("Bearer "))
                .map(|_| &h[7..])
        })
        .filter(|v| !v.is_empty());
    let hash: [u8; 32] = Sha256::digest(credential.unwrap_or("").as_bytes()).into();
    if headers.next().is_some() || credential.is_none() || !tokens.authenticates(&hash) {
        return Err(Box::new(response(
            StatusCode::UNAUTHORIZED,
            br#"{"error":"unauthorized_service"}"#.to_vec(),
        )));
    }
    Ok(hash)
}
async fn handle(State(app): State<App>, request: Request) -> Response {
    let hash = match authenticate(&app, &request) {
        Ok(hash) => hash,
        Err(refused) => return *refused,
    };
    let raw = match to_bytes(request.into_body(), wire::MAX_BODY).await {
        Ok(raw) => raw,
        Err(_) => return refusal(Error::Malformed, None),
    };
    let attempt = wire::attempt(&raw);
    // A protocol 3 request answers in protocol 3 only while its switch is on.
    let protocol = if app.config.protocol3.is_some() && wire::version(&raw) == Some(3) {
        3
    } else {
        2
    };
    if !app.config.enabled || !(1..=600).contains(&app.config.requests_per_minute) {
        return refusal_for(Error::Unavailable, attempt.as_deref(), protocol, &[]);
    }
    if let Err(error) = store::budget(&app.db, &hash, app.config.requests_per_minute).await {
        return refusal_for(error, attempt.as_deref(), protocol, &[]);
    }
    let decoded = match AdmissionRequest::decode_with(&raw, app.config.protocol3.is_some()) {
        Ok(decoded) => decoded,
        Err(error) => return refusal_for(error, attempt.as_deref(), protocol, &[]),
    };
    match store::admit(
        &app.db,
        &decoded,
        app.accounts.as_ref(),
        app.authority.as_ref(),
        app.config.signer.as_deref(),
        app.config.protocol3.as_ref(),
    )
    .await
    {
        Ok(body) => response(StatusCode::OK, body),
        Err(refused) => refusal_for(
            refused.error,
            attempt.as_deref(),
            decoded.protocol,
            &refused.channels,
        ),
    }
}
fn recovery_refusal(
    error: RecoveryError,
    attempt: Option<&str>,
    protocol: u8,
    retry_after: Option<u64>,
) -> Response {
    let mut result = response(error.status(), error.body(attempt, protocol));
    let seconds = match error {
        RecoveryError::RateLimited => Some(60),
        RecoveryError::AttemptPending => Some(retry_after.unwrap_or(1).max(1)),
        _ => None,
    };
    if let Some(value) =
        seconds.and_then(|s| axum::http::HeaderValue::from_str(&s.to_string()).ok())
    {
        result.headers_mut().insert(header::RETRY_AFTER, value);
    }
    result
}
/// The recovery routes' shared front: service authentication, the bounded
/// body, the protocol 3 switch (protocol 2 bodies while it is off, like login)
/// and the shared service budget. `Ok` carries the body and protocol 3 settings.
async fn recovery_front(
    app: &App,
    request: Request,
    attempt_of: fn(&[u8]) -> Option<String>,
) -> Result<(axum::body::Bytes, &Protocol3, Option<String>), Box<Response>> {
    let hash = authenticate(app, &request)?;
    let raw = to_bytes(request.into_body(), wire::MAX_BODY)
        .await
        .map_err(|_| Box::new(recovery_refusal(RecoveryError::Malformed, None, 3, None)))?;
    let attempt = attempt_of(&raw);
    let Some(settings) = app.config.protocol3.as_ref() else {
        return Err(Box::new(recovery_refusal(
            RecoveryError::UnsupportedVersion,
            attempt.as_deref(),
            2,
            None,
        )));
    };
    if !app.config.enabled || !(1..=600).contains(&app.config.requests_per_minute) {
        return Err(Box::new(recovery_refusal(
            RecoveryError::Unavailable,
            attempt.as_deref(),
            3,
            None,
        )));
    }
    if let Err(error) = store::budget(&app.db, &hash, app.config.requests_per_minute).await {
        return Err(Box::new(recovery_refusal(
            error.into(),
            attempt.as_deref(),
            3,
            None,
        )));
    }
    Ok((raw, settings, attempt))
}
/// §19.4. Recovery is served only while handles are on. With §19.15 device
/// binding on the request must carry a `device_proof`; with it off (testing
/// only) the member must be absent. With binding on and the device-session
/// switch off, every well-formed request is refused as for an unknown handle.
async fn recover(State(app): State<App>, request: Request) -> Response {
    let (raw, settings, attempt) = match recovery_front(&app, request, wire::attempt).await {
        Ok(front) => front,
        Err(refused) => return *refused,
    };
    let (issuer, binding, devices) = match (&app.config.recovery, settings.handles) {
        (
            Some(issuer),
            Handles::On {
                require_device_binding,
                devices_enabled,
                ..
            },
        ) => (issuer, require_device_binding, devices_enabled),
        _ => return recovery_refusal(RecoveryError::Unavailable, attempt.as_deref(), 3, None),
    };
    let decoded = match RecoveryRequest::decode(&raw, binding) {
        Ok(decoded) => decoded,
        Err(error) => return recovery_refusal(error, attempt.as_deref(), 3, None),
    };
    if binding && !devices {
        return recovery_refusal(RecoveryError::HandleRejected, attempt.as_deref(), 3, None);
    }
    match recovery::recover(
        &app.db,
        &decoded,
        app.accounts.as_ref(),
        issuer,
        &settings.runtime,
    )
    .await
    {
        Ok(body) => response(StatusCode::OK, body),
        Err(refused) => recovery_refusal(refused.error, attempt.as_deref(), 3, refused.retry_after),
    }
}
/// §19.7, available whenever protocol 3 is on: a well-formed request always
/// answers `revoked`, whatever the handle's state.
async fn revoke(State(app): State<App>, request: Request) -> Response {
    let (raw, _, _) = match recovery_front(&app, request, |_| None).await {
        Ok(front) => front,
        Err(refused) => return *refused,
    };
    let hash = match recovery::decode_revoke(&raw) {
        Ok(hash) => hash,
        Err(error) => return recovery_refusal(error, None, 3, None),
    };
    match recovery::revoke(&app.db, &hash).await {
        Ok(()) => response(StatusCode::OK, recovery::REVOKED.to_vec()),
        Err(error) => recovery_refusal(error, None, 3, None),
    }
}
/// Separate loopback listener; never merges service-only routes into the public
/// web router. The caller supplies the account-state lookup and the admission
/// authority (the binary composes the suspension-aware ones) and the grant
/// signer that authority signs with. The feature defaults off even when hashes
/// are configured.
pub async fn start_private_listener(
    db: PgPool,
    accounts: Arc<dyn AccountStateLookup>,
    authority: Arc<dyn AdmissionAuthority>,
    signer: Option<Arc<dyn GrantSigner>>,
    protocol3: Option<Protocol3>,
    recovery: Option<RecoveryIssuer>,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let enabled = std::env::var("PORTAL_NATIVE_ADMISSIONS_ENABLED").is_ok_and(|v| v == "true");
    let current = std::env::var_os("PORTAL_GATEWAY_TOKEN_SHA256_FILE");
    let previous = std::env::var_os("PORTAL_GATEWAY_PREVIOUS_TOKEN_SHA256_FILE");
    let tokens = current.and_then(|path| {
        ServiceTokens::from_files(Path::new(&path), previous.as_deref().map(Path::new)).ok()
    });
    let app = router(
        db,
        AdmissionConfig {
            enabled,
            tokens,
            requests_per_minute: std::env::var("PORTAL_NATIVE_ADMISSION_REQUESTS_PER_MINUTE")
                .map_or(120, |v| v.parse().unwrap_or(0)),
            signer,
            protocol3,
            recovery,
        },
        accounts,
        authority,
    )
    .route("/health", get(|| async { "ok" }));
    let address =
        std::env::var("PORTAL_NATIVE_ADMISSION_BIND").unwrap_or_else(|_| "127.0.0.1:8091".into());
    let address: std::net::SocketAddr = address.parse()?;
    if !address.ip().is_loopback() {
        return Err("native admission listener requires loopback private ingress".into());
    }
    let listener = tokio::net::TcpListener::bind(address).await?;
    Ok(tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    }))
}
