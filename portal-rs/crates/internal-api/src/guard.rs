//! Route-local Axum middleware; transport provenance never comes from headers.
use crate::canonical::{self, Shape};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::Response,
};
use std::sync::Arc;
use subtle::{Choice, ConstantTimeEq};

/// SPKI SHA-256 or exact printable ASCII SAN from a verified certificate.
#[derive(Debug, Clone)]
pub struct Identity {
    kind: u8,
    bytes: [u8; 128],
    len: u8,
}
impl Identity {
    pub fn spki_sha256(fingerprint: [u8; 32]) -> Self {
        let mut bytes = [0; 128];
        bytes[..32].copy_from_slice(&fingerprint);
        Self {
            kind: 0,
            bytes,
            len: 32,
        }
    }
    pub fn san(san: &str) -> Result<Self, ConfigError> {
        if san.is_empty() || san.len() > 128 || !san.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(ConfigError);
        }
        let mut bytes = [0; 128];
        bytes[..san.len()].copy_from_slice(san.as_bytes());
        Ok(Self {
            kind: 1,
            bytes,
            len: san.len() as u8,
        })
    }
    fn matches(&self, other: &Self) -> Choice {
        self.kind.ct_eq(&other.kind) & self.len.ct_eq(&other.len) & self.bytes.ct_eq(&other.bytes)
    }
}
/// Constant-time: kind, length and padded bytes are always all compared.
impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        bool::from(self.matches(other))
    }
}
impl Eq for Identity {}
/// Only the trusted connection adapter may insert this extension, after verifying
/// the certificate chain, TLS 1.3 and HTTP/1.1. Never populate from public headers.
#[derive(Debug, Clone)]
pub struct VerifiedClient(Identity);
impl VerifiedClient {
    pub fn from_verified_tls13_http11(identity: Identity) -> Self {
        Self(identity)
    }
}
/// Purpose-specific group; Store and Premium must use distinct names even though
/// both URLs share the products-entitlements prefix.
#[derive(Debug, Clone)]
pub struct Binding {
    pub identity: Identity,
    pub group: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigError;
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid or overlapping internal identity configuration")
    }
}
impl std::error::Error for ConfigError {}
/// Duplicate identities fail closed, including duplicates within one purpose.
#[derive(Debug, Clone, Default)]
pub struct IdentityMap(Vec<Binding>);
impl IdentityMap {
    pub fn new(bindings: Vec<Binding>) -> Result<Self, ConfigError> {
        for (index, binding) in bindings.iter().enumerate() {
            if binding.group.is_empty() || binding.group.len() > 128 || !binding.group.is_ascii() {
                return Err(ConfigError);
            }
            for previous in &bindings[..index] {
                if bool::from(binding.identity.matches(&previous.identity)) {
                    return Err(ConfigError);
                }
            }
        }
        Ok(Self(bindings))
    }
    fn has_group(&self, group: &str) -> bool {
        self.0.iter().any(|binding| binding.group == group)
    }
    fn permits(&self, identity: &Identity, group: &str) -> bool {
        // Scan every binding: identity bytes and match position never cause an
        // early exit. Fixed-size padded inputs include kind and original length.
        let mut allowed = Choice::from(0);
        for binding in &self.0 {
            allowed |= identity.matches(&binding.identity)
                & Choice::from(u8::from(binding.group == group));
        }
        bool::from(allowed)
    }
}
#[derive(Debug, Clone, Copy)]
pub enum JsonRule {
    /// Premium requests permit member order and insignificant whitespace.
    Object,
    Canonical(&'static Shape),
}
/// Flags default off; group configuration and positive body bounds are required.
#[derive(Debug, Clone)]
pub struct RoutePolicy {
    pub enabled: bool,
    pub group: String,
    pub max_body_bytes: usize,
    pub max_json_depth: usize,
    pub json: JsonRule,
}
impl Default for RoutePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            group: String::new(),
            max_body_bytes: 0,
            max_json_depth: 4,
            json: JsonRule::Object,
        }
    }
}
#[derive(Debug, Clone)]
struct Guard {
    identities: Arc<IdentityMap>,
    policy: RoutePolicy,
}
/// Mount separately for routes with different purpose, limit, shape or flag.
pub fn protect<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    identities: Arc<IdentityMap>,
    policy: RoutePolicy,
) -> Router<S> {
    router.route_layer(middleware::from_fn_with_state(
        Guard { identities, policy },
        check,
    ))
}
/// Peer failures: empty body, no challenge, diagnostics or application headers.
pub fn failure(status: StatusCode) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    response
}
fn single_header(
    headers: &HeaderMap,
    name: header::HeaderName,
) -> Result<Option<&str>, StatusCode> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    first
        .map(|v| v.to_str().map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()
}
fn transport(headers: &HeaderMap, max: usize) -> Result<(), StatusCode> {
    if single_header(headers, header::CONTENT_TYPE)? != Some("application/json")
        || headers.contains_key(header::TRANSFER_ENCODING)
        || !matches!(
            single_header(headers, header::CONTENT_ENCODING)?,
            None | Some("identity")
        )
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(length) = single_header(headers, header::CONTENT_LENGTH)? {
        let valid = length == "0"
            || (length.len() <= 10
                && !length.starts_with('0')
                && length.bytes().all(|b| b.is_ascii_digit()));
        if !valid || length.parse::<usize>().map_or(true, |n| n > max) {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
    }
    Ok(())
}
async fn check(State(guard): State<Guard>, request: Request, next: Next) -> Response {
    let policy = &guard.policy;
    if !policy.enabled
        || policy.max_body_bytes == 0
        || !(2..=128).contains(&policy.max_json_depth)
        || !guard.identities.has_group(&policy.group)
    {
        return failure(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !request
        .extensions()
        .get::<VerifiedClient>()
        .is_some_and(|client| guard.identities.permits(&client.0, &policy.group))
    {
        return failure(StatusCode::UNAUTHORIZED);
    }
    if let Err(status) = transport(request.headers(), policy.max_body_bytes) {
        return failure(status);
    }
    let (mut parts, body) = request.into_parts();
    // Limit collection itself, including streams without Content-Length.
    let bytes = match to_bytes(body, policy.max_body_bytes).await {
        Ok(bytes) => bytes,
        Err(error) => {
            use std::error::Error;
            let too_large = error
                .source()
                .is_some_and(|source| source.is::<http_body_util::LengthLimitError>());
            return failure(if too_large {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            });
        }
    };
    let decoded = match policy.json {
        JsonRule::Object => canonical::decode(&bytes, policy.max_body_bytes, policy.max_json_depth),
        JsonRule::Canonical(shape) => {
            canonical::decode_canonical(&bytes, policy.max_body_bytes, policy.max_json_depth, shape)
        }
    };
    match decoded {
        Ok(value) => {
            parts.extensions.insert(value);
        }
        Err(_) => return failure(StatusCode::BAD_REQUEST),
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}
