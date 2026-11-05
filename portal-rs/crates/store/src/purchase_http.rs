//! Only mount this router on the internal TLS listener. Headers never establish
//! peer identity; the listener must supply the shared VerifiedClient extension.
use super::*;
use ::http::{StatusCode, header};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::guard::{
    self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy, VerifiedClient,
};
use oteryn_wallet::{AccountStateReader, AuditHook, TransferBlockCheck};
use std::sync::Arc;
use tower::ServiceExt;

pub const GROUP: &str = "products-entitlements-store";
pub const BALANCE_PATH: &str = "/internal/v1/products-entitlements/store/balance/read";
pub const HISTORY_PATH: &str = "/internal/v1/products-entitlements/store/history/read";
pub const PURCHASE_PATH: &str = "/internal/v1/products-entitlements/store/purchases";

struct HttpState<A, B, H, C> {
    purchases: Arc<Purchases<A, B, H, C>>,
    authenticate: Router,
}
impl<A, B, H, C> Clone for HttpState<A, B, H, C> {
    fn clone(&self) -> Self {
        Self {
            purchases: self.purchases.clone(),
            authenticate: self.authenticate.clone(),
        }
    }
}
/// Supply every other configured internal-purpose binding, excluding Store.
/// This factory adds the single configured Store identity and fails closed on
/// overlaps, including another identity registered under the Store group.
pub fn router<A, B, H, C>(purchases: Purchases<A, B, H, C>, mut others: Vec<Binding>) -> Router
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
    C: Accounts + 'static,
{
    let settings = purchases.settings();
    let identity = Identity::san(&settings.peer_identity);
    let valid = settings.valid() && identity.is_ok() && others.iter().all(|b| b.group != GROUP);
    if let Ok(identity) = identity {
        others.push(Binding {
            identity,
            group: GROUP.into(),
        });
    }
    let map = IdentityMap::new(others);
    let enabled = valid && map.is_ok();
    let identities = Arc::new(map.unwrap_or_default());
    // A body-independent authentication probe uses the shared public guard API.
    // It copies only trusted TLS provenance. This lets the peer budget precede
    // transport/canonical validation without consuming the actual request body.
    let authenticate = guard::protect(
        Router::new().route("/", post(|| async { StatusCode::NO_CONTENT })),
        identities.clone(),
        RoutePolicy {
            enabled,
            group: GROUP.into(),
            max_body_bytes: 2,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    );
    let state = HttpState {
        purchases: Arc::new(purchases),
        authenticate,
    };
    let mut routes = Router::new();
    for (path, max) in [
        (BALANCE_PATH, 512),
        (HISTORY_PATH, 512),
        (PURCHASE_PATH, 1024),
    ] {
        let route = guard::protect(
            Router::new().route(path, post(handle::<A, B, H, C>)),
            identities.clone(),
            RoutePolicy {
                enabled,
                group: GROUP.into(),
                max_body_bytes: max,
                max_json_depth: 4,
                json: JsonRule::Object,
            },
        );
        routes = routes.merge(route);
    }
    routes
        .with_state(state.clone())
        .route_layer(middleware::from_fn_with_state(state, peer::<A, B, H, C>))
        .layer(middleware::from_fn(no_store))
}
async fn peer<A, B, H, C>(
    State(state): State<HttpState<A, B, H, C>>,
    mut request: Request,
    next: Next,
) -> Response
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
    C: Accounts + 'static,
{
    let mut probe = Request::new(Body::from("{}"));
    *probe.method_mut() = ::http::Method::POST;
    probe.headers_mut().insert(
        header::CONTENT_TYPE,
        ::http::HeaderValue::from_static("application/json"),
    );
    if let Some(client) = request.extensions().get::<VerifiedClient>() {
        probe.extensions_mut().insert(client.clone());
    }
    let auth = match state.authenticate.oneshot(probe).await {
        Ok(response) => response,
        Err(never) => match never {},
    };
    if auth.status() != StatusCode::NO_CONTENT {
        return auth;
    }
    if let Err(error) = state.purchases.peer_limit().await {
        return failure(error);
    }
    if let Err(error) = normalize_transport(&mut request) {
        return guard::failure(error);
    }
    let mut response = next.run(request).await;
    // Only an oversized Content-Length is 413. A collected body over its bound
    // is a malformed message, including a stream without Content-Length.
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    response
}
fn normalize_transport(request: &mut Request) -> Result<(), StatusCode> {
    let max = if request.uri().path() == PURCHASE_PATH {
        1024
    } else {
        512
    };
    let headers = request.headers_mut();
    if headers.get_all(header::CONTENT_LENGTH).iter().count() > 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(length) = headers.get(header::CONTENT_LENGTH) {
        let length = length.to_str().map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        if !(length == "0"
            || (length.len() <= 10
                && !length.starts_with('0')
                && length.bytes().all(|b| b.is_ascii_digit())))
            || length.parse::<usize>().map_or(true, |n| n > max)
        {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
    }
    // Match the reference's media-type parameters and case-insensitive identity
    // encoding. Ambiguous duplicate headers remain errors in the shared guard.
    for name in [header::CONTENT_TYPE, header::CONTENT_ENCODING] {
        if headers.get_all(&name).iter().count() == 1
            && let Some(value) = headers.get(&name).and_then(|v| v.to_str().ok())
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
            if let Ok(value) = ::http::HeaderValue::from_str(&normalized) {
                headers.insert(name, value);
            }
        }
    }
    Ok(())
}
async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() != StatusCode::OK {
        *response.body_mut() = Body::empty();
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        ::http::HeaderValue::from_static("no-store"),
    );
    response
}
fn failure(error: Error) -> Response {
    let status = match error {
        Error::Invalid => StatusCode::BAD_REQUEST,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::UnknownAccount => StatusCode::NOT_FOUND,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Limited(_) => StatusCode::TOO_MANY_REQUESTS,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    let mut response = guard::failure(status);
    if let Error::Limited(seconds) = error
        && let Ok(value) = ::http::HeaderValue::from_str(&seconds.max(1).to_string())
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}
async fn handle<A, B, H, C>(
    State(state): State<HttpState<A, B, H, C>>,
    request: Request,
) -> Response
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
    C: Accounts + 'static,
{
    let path = request.uri().path().to_owned();
    let max = if path == PURCHASE_PATH { 1024 } else { 512 };
    let raw = match to_bytes(request.into_body(), max).await {
        Ok(raw) => raw,
        Err(_) => return failure(Error::Invalid),
    };
    let result = if path == PURCHASE_PATH {
        state.purchases.purchase(&raw).await
    } else {
        state.purchases.read(&raw, path == HISTORY_PATH).await
    };
    match result {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                ::http::HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(error) => failure(error),
    }
}
