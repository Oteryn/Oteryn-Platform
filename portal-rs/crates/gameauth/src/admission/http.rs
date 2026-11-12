use super::{
    AccountStateLookup, AdmissionAuthority, AdmissionRequest, Error, GrantSigner,
    protocol3::Protocol3, store, wire,
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
}
impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tokens: None,
            requests_per_minute: 120,
            signer: None,
            protocol3: None,
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
async fn handle(State(app): State<App>, request: Request) -> Response {
    let Some(tokens) = app.config.tokens else {
        return response(
            StatusCode::SERVICE_UNAVAILABLE,
            br#"{"error":"service_unavailable"}"#.to_vec(),
        );
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
        return response(
            StatusCode::UNAUTHORIZED,
            br#"{"error":"unauthorized_service"}"#.to_vec(),
        );
    }
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
