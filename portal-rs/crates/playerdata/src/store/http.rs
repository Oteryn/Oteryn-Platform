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
use std::{collections::HashMap, sync::Mutex, time::Instant};
use tokio::sync::Semaphore;

/// G-10 purpose (`OTERYN_GAME_JOURNAL`): `journal/records` and `journal/cursor`.
pub const ITEMS_GROUP: &str = "playerdata-journal-items";
/// G-11 purpose (`OTERYN_GAME_JOURNAL_PROGRESS`): `progress/record` and `progress/watermark`.
pub const PROGRESS_GROUP: &str = "playerdata-journal-progress";
pub const RECORDS_PATH: &str = "/internal/v1/playerdata/journal/records";
pub const CURSOR_PATH: &str = "/internal/v1/playerdata/journal/cursor";
pub const PROGRESS_RECORD_PATH: &str = "/internal/v1/playerdata/progress/record";
pub const PROGRESS_WATERMARK_PATH: &str = "/internal/v1/playerdata/progress/watermark";
/// G-11 §3: 32 in-flight exchanges per peer.
const PROGRESS_IN_FLIGHT: usize = 32;
/// G-11 §3: 60 progress records per minute per account.
const PROGRESS_PER_ACCOUNT: u32 = 60;

/// The one configured `source_authority` each purpose is bound to. Empty, or not
/// a contract `key`, leaves that purpose unavailable.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    pub items: String,
    pub progress: String,
}
fn authority_key(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

pub trait Clock: Debug + Send + Sync {
    fn trusted_time(&self) -> Result<TrustedTime>;
}
#[derive(Debug)]
pub struct NoClock;
impl Clock for NoClock {
    fn trusted_time(&self) -> Result<TrustedTime> {
        Err(Error::Unavailable)
    }
}
#[derive(Debug, Clone)]
pub struct HttpState {
    pub store: Store,
    pub clock: Arc<dyn Clock>,
    sources: Sources,
    accounts: Accounts,
}
impl HttpState {
    /// No source authority is configured: every route stays unavailable.
    pub fn new(store: Store, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock,
            sources: Sources::default(),
            accounts: Accounts::default(),
        }
    }
    pub fn with_sources(mut self, sources: Sources) -> Self {
        self.sources = sources;
        self
    }
}
/// Fixed one-minute windows of progress records per account. Expired windows are
/// dropped on every admission, so the map holds at most the accounts seen in the
/// last minute, which the per-peer budget bounds.
#[derive(Debug, Clone, Default)]
struct Accounts(Arc<Mutex<HashMap<String, (Instant, u32)>>>);
impl Accounts {
    fn admit(&self, account: &str) -> Result<bool> {
        let mut windows = self.0.lock().map_err(|_| Error::Unavailable)?;
        windows.retain(|_, window| window.0.elapsed().as_secs() < 60);
        let window = windows
            .entry(account.to_owned())
            .or_insert((Instant::now(), 0));
        if window.1 >= PROGRESS_PER_ACCOUNT {
            return Ok(false);
        }
        window.1 += 1;
        Ok(true)
    }
}
/// Per-peer operational limits of one purpose (each purpose has one identity).
#[derive(Debug, Clone)]
struct Budget {
    window: Arc<Mutex<(Instant, u32)>>,
    per_minute: u32,
    in_flight: Option<Arc<Semaphore>>,
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
    let r = match playerdata::progress(&bytes) {
        Ok(r) => r,
        Err(e) => return response(Err(e.into())),
    };
    match state.accounts.admit(&r.scope.account) {
        Ok(true) => {}
        Ok(false) => return too_many("60"),
        Err(e) => return response(Err(e)),
    }
    let result = async { state.store.ingest(r, state.clock.trusted_time()?).await }.await;
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
fn too_many(retry_after: &'static str) -> Response {
    let mut response = response(Err(Error::Unavailable));
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    response.headers_mut().insert(
        header::RETRY_AFTER,
        header::HeaderValue::from_static(retry_after),
    );
    response
}
/// Set by [`over_budget`] on a request from a peer whose budget is spent.
#[derive(Debug, Clone, Copy)]
struct OverBudget(&'static str);
/// Runs before the guard (G-10/G-11 §3 order: 503, 401, 429, then transport).
/// A request over the peer budget keeps its method, URI and peer extension but
/// reaches the guard with an empty JSON object in place of its body, so the
/// guard still answers 503 and 401 first and no transport or size check runs
/// on the refused body; [`budget`] then answers 429. Only [`budget`] counts.
async fn over_budget(
    State(budget): State<Budget>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let spent = match budget.window.lock() {
        Ok(b) => b.0.elapsed().as_secs() < 60 && b.1 >= budget.per_minute,
        Err(_) => return response(Err(Error::Unavailable)),
    };
    let full = budget
        .in_flight
        .as_ref()
        .is_some_and(|slots| slots.available_permits() == 0);
    if !spent && !full {
        return next.run(request).await;
    }
    let (mut parts, _) = request.into_parts();
    parts
        .extensions
        .insert(OverBudget(if spent { "60" } else { "1" }));
    parts.headers.remove(header::TRANSFER_ENCODING);
    parts.headers.remove(header::CONTENT_ENCODING);
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    parts.headers.insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_static("2"),
    );
    next.run(axum::extract::Request::from_parts(parts, Body::from("{}")))
        .await
}
async fn budget(
    State(budget): State<Budget>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if let Some(OverBudget(retry_after)) = request.extensions().get::<OverBudget>() {
        return too_many(retry_after);
    }
    let allowed = match budget.window.lock() {
        Ok(mut b) => {
            if b.0.elapsed().as_secs() >= 60 {
                *b = (Instant::now(), 0)
            };
            if b.1 >= budget.per_minute {
                false
            } else {
                b.1 += 1;
                true
            }
        }
        Err(_) => return response(Err(Error::Unavailable)),
    };
    if !allowed {
        return too_many("60");
    };
    let _permit = match &budget.in_flight {
        Some(slots) => match slots.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => return too_many("1"),
        },
        None => None,
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
/// Source binding after the peer guard: a body naming another `source_authority`
/// is a peer/source failure (401) before any content check. A body without a
/// string member is left to the closed codec (400).
async fn source(
    State(configured): State<Arc<str>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let named = request
        .extensions()
        .get::<Value>()
        .and_then(|body| body.get("source_authority"))
        .and_then(|value| match value {
            Value::String(name) => Some(name.as_str() != &*configured),
            _ => None,
        });
    if named == Some(true) {
        return response(Err(Error::Denied));
    }
    next.run(request).await
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
/// G-10 and G-11 are separate purposes: each binds its own identity group,
/// `source_authority`, flag and per-peer budget; one `source_authority`
/// configured for both is an overlapping binding and disables both. Order per
/// request: flag and configuration (503), peer identity (401), per-peer budget
/// (429), transport and JSON bounds (400/413), source binding (401), closed
/// codec (400), per-account budget for progress records (429), store.
pub fn router(state: HttpState, identities: Arc<IdentityMap>) -> Router {
    let flags = &state.store.flags;
    let items = Budget {
        window: Arc::new(Mutex::new((Instant::now(), 0))),
        per_minute: 120,
        in_flight: None,
    };
    let progress_budget = Budget {
        window: Arc::new(Mutex::new((Instant::now(), 0))),
        per_minute: 600,
        in_flight: Some(Arc::new(Semaphore::new(PROGRESS_IN_FLIGHT))),
    };
    let items_source = &state.sources.items;
    let progress_source = &state.sources.progress;
    let distinct = items_source != progress_source;
    let items_on = flags.items_ingest && authority_key(items_source) && distinct;
    let progress_on = flags.progress_ingest && authority_key(progress_source) && distinct;
    let mut combined = Router::new();
    for (path, handler, max, enabled, group, budget_state, configured) in [
        (
            RECORDS_PATH,
            post(publish),
            65536,
            items_on,
            ITEMS_GROUP,
            items.clone(),
            items_source,
        ),
        (
            CURSOR_PATH,
            post(cursor),
            1024,
            items_on,
            ITEMS_GROUP,
            items,
            items_source,
        ),
        (
            PROGRESS_RECORD_PATH,
            post(progress),
            262144,
            progress_on,
            PROGRESS_GROUP,
            progress_budget.clone(),
            progress_source,
        ),
        (
            PROGRESS_WATERMARK_PATH,
            post(watermark),
            1024,
            progress_on,
            PROGRESS_GROUP,
            progress_budget,
            progress_source,
        ),
    ] {
        let routes = Router::new()
            .route(path, handler)
            .route_layer(middleware::from_fn_with_state(
                Arc::<str>::from(configured.as_str()),
                source,
            ))
            .route_layer(middleware::from_fn_with_state(budget_state.clone(), budget));
        let routes = guard::protect(
            routes,
            identities.clone(),
            RoutePolicy {
                enabled,
                group: group.to_owned(),
                max_body_bytes: max,
                max_json_depth: 9,
                json: JsonRule::Object,
            },
        )
        .route_layer(middleware::from_fn_with_state(budget_state, over_budget));
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
