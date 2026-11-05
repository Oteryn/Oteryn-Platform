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
use std::sync::Arc;
use tower::ServiceExt;

pub const GROUP: &str = "products-entitlements-store";
pub const PENDING_PATH: &str = "/internal/v1/products-entitlements/store/operations/pending";
pub const READ_PATH: &str = "/internal/v1/products-entitlements/store/operations/read";
pub const RECEIPT_PATH: &str = "/internal/v1/products-entitlements/store/operations/receipt";

pub const RECONCILE_PATH: &str = "/internal/v1/products-entitlements/store/operations/reconcile";

struct HttpState {
    purchases: Arc<Delivery>,
    authenticate: Router,
}
impl Clone for HttpState {
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
pub fn router(purchases: Delivery, mut others: Vec<Binding>) -> Router {
    let settings = purchases.settings();
    let identity = Identity::san(&settings.peer_identity);
    let valid = purchases.enabled() && identity.is_ok() && others.iter().all(|b| b.group != GROUP);
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
        (PENDING_PATH, 512),
        (READ_PATH, 512),
        (RECEIPT_PATH, 1024),
        (RECONCILE_PATH, 512),
    ] {
        let route = guard::protect(
            Router::new().route(path, post(handle)),
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
        .route_layer(middleware::from_fn_with_state(state, peer))
        .layer(middleware::from_fn(no_store))
}
async fn peer(State(state): State<HttpState>, mut request: Request, next: Next) -> Response {
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
    let max = if request.uri().path() == RECEIPT_PATH {
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
async fn handle(State(state): State<HttpState>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    let max = if path == RECEIPT_PATH { 1024 } else { 512 };
    let raw = match to_bytes(request.into_body(), max).await {
        Ok(raw) => raw,
        Err(_) => return failure(Error::Invalid),
    };
    let result = match path.as_str() {
        RECEIPT_PATH => state.purchases.receipt(&raw).await,
        PENDING_PATH => state.purchases.pending(&raw).await,
        READ_PATH => state.purchases.read(&raw).await,
        _ => state.purchases.reconcile(&raw).await,
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
