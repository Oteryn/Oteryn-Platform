//! Private B1/B2 route factory. Mount only on the internal verified-TLS listener.
use super::{Authority, Changes, Error, Product};
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
    canonical::Value,
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const AUTHORITY_PATH: &str = "/internal/v1/products-entitlements/profile-b/authority/read";
pub const CHANGES_PATH: &str = "/internal/v1/products-entitlements/profile-b/changes/read";
pub const AUTHORITY_GROUP: &str = "products-entitlements-profile-b-authority";
pub const CHANGES_GROUP: &str = "products-entitlements-profile-b-changes";
pub const AUTHORITY_IDENTITY: &str =
    "PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_MTLS_CLIENT_IDENTITY";
pub const CHANGES_IDENTITY: &str = "PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_MTLS_CLIENT_IDENTITY";
const READ_TIMEOUT: Duration = Duration::from_secs(3);

/// §8.5 configuration. Every value that is missing or malformed leaves its group
/// unavailable (`503`); nothing here can widen the §9.1 lease.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub authority_enabled: bool,
    pub changes_enabled: bool,
    pub producer_revision: String,
    pub products: Option<Vec<Product>>,
    pub authority_budget: Option<u32>,
    pub changes_budget: Option<u32>,
}
fn budget(raw: Option<String>) -> Option<u32> {
    match raw {
        None => Some(1200),
        Some(raw) => super::positive(&raw, 6000).and_then(|n| u32::try_from(n).ok()),
    }
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            authority_enabled: get("PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED").as_deref()
                == Some("true"),
            changes_enabled: get("PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED").as_deref()
                == Some("true"),
            producer_revision: get("PLATFORM_BUILD_REVISION").unwrap_or_default(),
            products: super::products(get("PRODUCTS_ENTITLEMENTS_PROFILE_B_PRODUCTS").as_deref()),
            authority_budget: budget(get(
                "PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_REQUESTS_PER_MINUTE",
            )),
            changes_budget: budget(get(
                "PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_REQUESTS_PER_MINUTE",
            )),
        }
    }
    pub fn authority_valid(&self) -> bool {
        self.authority_enabled
            && crate::premium::http::hex(&self.producer_revision, 40)
            && self.products.is_some()
            && self.authority_budget.is_some()
    }
    pub fn changes_valid(&self) -> bool {
        self.changes_enabled && self.changes_budget.is_some()
    }
}

/// §8.1 request; any unknown, missing or wrongly typed member is `Invalid`.
pub fn decode_authority(value: &Value) -> Result<(Uuid, String), Error> {
    let Value::Object(members) = value else {
        return Err(Error::Invalid);
    };
    let text = |key| match value.get(key) {
        Some(Value::String(s)) => Ok(s.as_str()),
        _ => Err(Error::Invalid),
    };
    if members.len() != 3 || text("schema")? != "oteryn.entitlement_authority_request.v1" {
        return Err(Error::Invalid);
    }
    let account = text("account_id")?;
    let id = Uuid::parse_str(account).map_err(|_| Error::Invalid)?;
    if id.to_string() != account || !super::uuid7(id) {
        return Err(Error::Invalid);
    }
    let nonce = text("nonce")?;
    if !crate::premium::http::hex(nonce, 32) {
        return Err(Error::Invalid);
    }
    Ok((id, nonce.to_owned()))
}

/// §8.4 request with the §1.5 cursor form; returns (`after`, `limit`).
pub fn decode_changes(value: &Value) -> Result<(Option<i64>, i64), Error> {
    let Value::Object(members) = value else {
        return Err(Error::Invalid);
    };
    if members.len() != 3
        || value.get("schema")
            != Some(&Value::String(
                "oteryn.entitlement_change_request.v1".into(),
            ))
    {
        return Err(Error::Invalid);
    }
    let after = match value.get("after") {
        Some(Value::Null) => None,
        Some(Value::String(cursor)) if (4..=22).contains(&cursor.len()) => Some(
            cursor
                .strip_prefix("v1:")
                .and_then(|n| super::positive(n, i64::MAX))
                .ok_or(Error::Invalid)?,
        ),
        _ => return Err(Error::Invalid),
    };
    let limit = match value.get("limit") {
        Some(Value::Signed(n)) => *n,
        Some(Value::Unsigned(n)) => i64::try_from(*n).map_err(|_| Error::Invalid)?,
        _ => return Err(Error::Invalid),
    };
    if !(1..=500).contains(&limit) {
        return Err(Error::Invalid);
    }
    Ok((after, limit))
}

/// Per-process peer budget per minute.
#[derive(Debug)]
struct Budget {
    limit: u32,
    window: Mutex<(Instant, u32)>,
}
impl Budget {
    fn new(limit: Option<u32>) -> Self {
        Self {
            limit: limit.unwrap_or(0),
            window: Mutex::new((Instant::now(), 0)),
        }
    }
    /// `None` when the lock is poisoned.
    fn take(&self) -> Option<bool> {
        let mut window = self.window.lock().ok()?;
        if window.0.elapsed() >= Duration::from_secs(60) {
            *window = (Instant::now(), 0);
        }
        if window.1 >= self.limit {
            return Some(false);
        }
        window.1 += 1;
        Some(true)
    }
    fn refuse(&self) -> Option<Response> {
        match self.take() {
            Some(true) => None,
            Some(false) => {
                let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, http::HeaderValue::from_static("60"));
                Some(response)
            }
            None => Some(guard::failure(StatusCode::SERVICE_UNAVAILABLE)),
        }
    }
}

#[derive(Debug)]
struct AuthorityState {
    authority: Authority,
    settings: Settings,
}
#[derive(Debug)]
struct ChangesState {
    changes: Changes,
}

/// IdentityMap must hold both dedicated group bindings and every other internal
/// peer binding, so a reused identity fails closed before route construction.
pub fn router(
    authority: Authority,
    changes: Changes,
    settings: Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    let authority_enabled = settings.authority_valid();
    let changes_enabled = settings.changes_valid();
    let authority_state = Arc::new(AuthorityState {
        authority,
        settings: settings.clone(),
    });
    let changes_state = Arc::new(ChangesState { changes });
    let policy = |enabled, group: &str| RoutePolicy {
        enabled,
        group: group.into(),
        max_body_bytes: 256,
        max_json_depth: 4,
        json: JsonRule::Object,
    };
    let authority_policy = policy(authority_enabled, AUTHORITY_GROUP);
    let changes_policy = policy(changes_enabled, CHANGES_GROUP);
    let b1 = guard::protect_peer(
        guard::validate_body(
            Router::new()
                .route(AUTHORITY_PATH, post(read_authority))
                .with_state(authority_state),
            authority_policy.clone(),
        )
        .route_layer(middleware::from_fn_with_state(
            Arc::new(Budget::new(settings.authority_budget)),
            refuse_mw,
        )),
        identities.clone(),
        authority_policy,
    );
    let b2 = guard::protect_peer(
        guard::validate_body(
            Router::new()
                .route(CHANGES_PATH, post(read_changes))
                .with_state(changes_state),
            changes_policy.clone(),
        )
        .route_layer(middleware::from_fn_with_state(
            Arc::new(Budget::new(settings.changes_budget)),
            refuse_mw,
        )),
        identities,
        changes_policy,
    );
    b1.merge(b2).layer(middleware::from_fn(contract_response))
}

async fn refuse_mw(State(budget): State<Arc<Budget>>, request: Request, next: Next) -> Response {
    if let Some(refused) = budget.refuse() {
        return refused;
    }
    next.run(request).await
}

async fn contract_response(request: Request, next: Next) -> Response {
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

fn json(bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/json"),
    );
    response
}

async fn read_authority(State(state): State<Arc<AuthorityState>>, request: Request) -> Response {
    if request.uri().query().is_some() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let Ok((account, nonce)) = request
        .extensions()
        .get::<Value>()
        .ok_or(Error::Invalid)
        .and_then(decode_authority)
    else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    let Some(products) = state.settings.products.as_deref() else {
        return guard::failure(StatusCode::SERVICE_UNAVAILABLE);
    };
    let read = state
        .authority
        .read(account, &nonce, &state.settings.producer_revision, products);
    match tokio::time::timeout(READ_TIMEOUT, read).await {
        Ok(Ok(bytes)) => json(bytes),
        Ok(Err(Error::UnknownAccount)) => guard::failure(StatusCode::NOT_FOUND),
        _ => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn read_changes(State(state): State<Arc<ChangesState>>, request: Request) -> Response {
    if request.uri().query().is_some() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let Ok((after, limit)) = request
        .extensions()
        .get::<Value>()
        .ok_or(Error::Invalid)
        .and_then(decode_changes)
    else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    match tokio::time::timeout(READ_TIMEOUT, state.changes.read(after, limit)).await {
        Ok(Ok(bytes)) => json(bytes),
        Ok(Err(Error::Invalid)) => guard::failure(StatusCode::BAD_REQUEST),
        _ => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
