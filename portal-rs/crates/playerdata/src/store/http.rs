//! Private listener composition. Production remains disabled until callers provide
//! accepted private authority, restore, trusted-clock and TLS purpose adapters.
use super::*;
use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use std::{sync::Mutex, time::Instant};

pub trait Clock: Debug + Send + Sync {
    fn trusted_time(&self) -> Result<TrustedTime>;
}
#[derive(Debug)]
struct NoClock;
impl Clock for NoClock {
    fn trusted_time(&self) -> Result<TrustedTime> {
        Err(Error::Unavailable)
    }
}
#[derive(Debug, Clone)]
pub struct HttpState {
    pub store: Store,
    pub clock: Arc<dyn Clock>,
    budget: Arc<Mutex<(Instant, u32)>>,
}
impl HttpState {
    pub fn new(store: Store, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock,
            budget: Arc::new(Mutex::new((Instant::now(), 0))),
        }
    }
}
fn response(result: Result<Vec<u8>>) -> Response {
    let (status, bytes) = match result {
        Ok(bytes) => (StatusCode::OK, bytes),
        Err(e) => (
            match e {
                Error::Invalid => StatusCode::BAD_REQUEST,
                Error::Denied => StatusCode::UNAUTHORIZED,
                Error::Conflict => StatusCode::CONFLICT,
                Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            },
            Vec::new(),
        ),
    };
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    if status == StatusCode::OK {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
    }
    response
}
async fn publish(State(state): State<HttpState>, bytes: Bytes) -> Response {
    let result = async {
        let r = playerdata::items(&bytes)?;
        state.store.ingest(r, state.clock.trusted_time()?).await
    }
    .await;
    response(result)
}
async fn progress(State(state): State<HttpState>, bytes: Bytes) -> Response {
    let result = async {
        let r = playerdata::progress(&bytes)?;
        state.store.ingest(r, state.clock.trusted_time()?).await
    }
    .await;
    response(result)
}
async fn watermark(State(state): State<HttpState>, bytes: Bytes) -> Response {
    let result = async {
        let w = playerdata::watermark(&bytes)?;
        state.store.watermark(w, state.clock.trusted_time()?).await
    }
    .await;
    response(result)
}
async fn cursor(State(state): State<HttpState>, bytes: Bytes) -> Response {
    let result = async {
        let scope = playerdata::cursor(&bytes)?;
        let (cursor, id) = state.store.cursor(&scope).await?;
        let id = match id {
            Some(id) => format!("\"{id}\""),
            None => "null".to_owned(),
        };
        Ok(format!(
            "{{\"contract_version\":1,\"after_source_revision\":\"{}\",\"last_record_id\":{id}}}",
            cursor.stored
        )
        .into_bytes())
    }
    .await;
    response(result)
}
async fn budget(
    State(state): State<HttpState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let allowed = match state.budget.lock() {
        Ok(mut b) => {
            if b.0.elapsed().as_secs() >= 60 {
                *b = (Instant::now(), 0)
            };
            if b.1 >= 120 {
                false
            } else {
                b.1 += 1;
                true
            }
        }
        Err(_) => return response(Err(Error::Unavailable)),
    };
    if !allowed {
        let mut response = response(Err(Error::Unavailable));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
        return response;
    };
    let expected = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    // The shared guard already bounded and restored this body. Validate actual
    // length as well; Content-Length is never a substitute for bounded collection.
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, 1048576).await {
        Ok(b) => b,
        Err(_) => return response(Err(Error::Invalid)),
    };
    if expected.is_some_and(|n| n != bytes.len()) {
        return response(Err(Error::Invalid));
    };
    next.run(axum::extract::Request::from_parts(parts, Body::from(bytes)))
        .await
}
async fn no_store(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
/// Mount only on a trusted internal listener. The caller must use the shared
/// IdentityMap containing all purposes, so overlapping credentials fail closed.
pub fn router(state: HttpState, identities: Arc<IdentityMap>) -> Router {
    let mut combined = Router::new();
    for (path, handler, max, enabled) in [
        (
            "/internal/v1/playerdata/journal/records",
            post(publish),
            65536,
            state.store.flags.items_ingest,
        ),
        (
            "/internal/v1/playerdata/journal/cursor",
            post(cursor),
            1024,
            state.store.flags.items_ingest,
        ),
        (
            "/internal/v1/playerdata/progress/record",
            post(progress),
            262144,
            state.store.flags.progress_ingest,
        ),
        (
            "/internal/v1/playerdata/progress/watermark",
            post(watermark),
            1024,
            state.store.flags.progress_ingest,
        ),
    ] {
        let routes = Router::new()
            .route(path, handler)
            .route_layer(middleware::from_fn_with_state(state.clone(), budget));
        let routes = guard::protect(
            routes,
            identities.clone(),
            RoutePolicy {
                enabled,
                group: "private-journal".to_owned(),
                max_body_bytes: max,
                max_json_depth: 9,
                json: JsonRule::Object,
            },
        );
        combined = combined.merge(routes);
    }
    combined
        .with_state(state)
        .layer(middleware::from_fn(no_store))
}
/// No ambient environment variable can replace missing accepted dependencies.
pub fn configured_router(pool: PgPool) -> Router {
    router(
        HttpState::new(
            Store::new(pool, Arc::new(UnavailableAuthority), Flags::default()),
            Arc::new(NoClock),
        ),
        Arc::new(IdentityMap::default()),
    )
}
