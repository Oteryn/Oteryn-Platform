//! Private route factory. Only mount on the internal verified-TLS listener.
use super::{Error, Ledger};
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use http::{StatusCode, header};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const PATH: &str = "/internal/v1/products-entitlements/loyalty-snapshots/read";
pub const GROUP: &str = "products-entitlements-loyalty-snapshot";

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub producer_revision: String,
    pub requests_per_minute: u32,
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            enabled: get("PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED").as_deref()
                == Some("true"),
            producer_revision: get("PLATFORM_BUILD_REVISION").unwrap_or_default(),
            requests_per_minute: get("PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_REQUESTS_PER_MINUTE")
                .map_or(1200, |v| v.parse().unwrap_or(0)),
        }
    }
    /// Build this dedicated binding alongside every other internal peer binding;
    /// IdentityMap rejects any identity reused for another route group.
    pub fn identity_binding(
        mut get: impl FnMut(&str) -> Option<String>,
    ) -> Result<guard::Binding, guard::ConfigError> {
        let identity = get("PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY")
            .ok_or(guard::ConfigError)?;
        Ok(guard::Binding {
            identity: guard::Identity::san(&identity)?,
            group: GROUP.into(),
        })
    }
    fn valid(&self) -> bool {
        self.enabled
            && hex(&self.producer_revision, 40)
            && (1..=6000).contains(&self.requests_per_minute)
    }
}
pub(super) fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn decode(value: &Value) -> Result<(Uuid, String), Error> {
    let Value::Object(members) = value else {
        return Err(Error::Invalid);
    };
    if members.len() != 3 {
        return Err(Error::Invalid);
    }
    let text = |key| match value.get(key) {
        Some(Value::String(s)) => Ok(s.as_str()),
        _ => Err(Error::Invalid),
    };
    if text("schema")? != "oteryn.loyalty_snapshot_request.v1" {
        return Err(Error::Invalid);
    }
    let account = text("account_id")?;
    let id = Uuid::parse_str(account).map_err(|_| Error::Invalid)?;
    if id.to_string() != account
        || id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::Invalid);
    }
    let nonce = text("nonce")?;
    if !hex(nonce, 32) {
        return Err(Error::Invalid);
    }
    Ok((id, nonce.to_owned()))
}
#[derive(Debug)]
struct StateData {
    ledger: Ledger,
    settings: Settings,
    rate: Mutex<(Instant, u32)>,
}
/// IdentityMap must contain the dedicated GROUP binding and every other peer
/// binding so overlapping configuration fails closed before route construction.
/// The default peer budget is 1200/minute; one process owns this route group.
pub fn router(ledger: Ledger, settings: Settings, identities: Arc<IdentityMap>) -> Router {
    let enabled = settings.valid();
    let state = Arc::new(StateData {
        ledger,
        settings,
        rate: Mutex::new((Instant::now(), 0)),
    });
    let routes = Router::new().route(PATH, post(read)).with_state(state);
    guard::protect(
        routes,
        identities,
        RoutePolicy {
            enabled,
            group: GROUP.into(),
            max_body_bytes: 256,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    )
    .layer(middleware::from_fn(contract_response))
}
pub(super) async fn contract_response(mut request: Request, next: Next) -> Response {
    // Laravel accepts media-type parameters and case-insensitive identity encoding.
    // Keep duplicate headers intact so the shared guard rejects ambiguity.
    for name in [header::CONTENT_TYPE, header::CONTENT_ENCODING] {
        if request.headers().get_all(&name).iter().count() == 1
            && let Some(value) = request.headers().get(&name).and_then(|v| v.to_str().ok())
        {
            let normalized = if name == header::CONTENT_TYPE {
                value
                    .split(';')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase()
            } else {
                value.trim().to_ascii_lowercase()
            };
            if let Ok(value) = http::HeaderValue::from_str(&normalized) {
                request.headers_mut().insert(name, value);
            }
        }
    }
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-store, private"),
    );
    response
}
async fn read(State(state): State<Arc<StateData>>, request: Request) -> Response {
    let allowed = match state.rate.lock() {
        Ok(mut rate) => {
            if rate.0.elapsed() >= Duration::from_secs(60) {
                *rate = (Instant::now(), 0);
            }
            if rate.1 >= state.settings.requests_per_minute {
                false
            } else {
                rate.1 += 1;
                true
            }
        }
        Err(_) => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    if !allowed {
        let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, http::HeaderValue::from_static("60"));
        return response;
    }
    let parsed = request
        .extensions()
        .get::<Value>()
        .ok_or(Error::Invalid)
        .and_then(decode);
    let (account, nonce) = match parsed {
        Ok(value) => value,
        Err(_) => return guard::failure(StatusCode::BAD_REQUEST),
    };
    match state
        .ledger
        .snapshot(account, &nonce, &state.settings.producer_revision)
        .await
    {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                http::HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(Error::UnknownAccount) => guard::failure(StatusCode::NOT_FOUND),
        Err(_) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// Request helper for callers that have raw bytes rather than the guard extension.
pub fn decode_bytes(bytes: &[u8]) -> Result<(Uuid, String), Error> {
    decode(&canonical::decode(bytes, 256, 4).map_err(|_| Error::Invalid)?)
}

/// Mount both exports on the verified internal listener. Each route keeps its own
/// flag, dedicated peer binding and rate budget. Snapshot wire behavior is unchanged.
pub fn router_with_stream(
    ledger: Ledger,
    settings: Settings,
    stream_settings: super::stream::http::Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    let stream = super::stream::Stream::new(
        ledger.pool.clone(),
        ledger.accounts.clone(),
        stream_settings.enabled,
    );
    router(ledger, settings, identities.clone()).merge(super::stream::http::router(
        stream,
        stream_settings,
        identities,
    ))
}
