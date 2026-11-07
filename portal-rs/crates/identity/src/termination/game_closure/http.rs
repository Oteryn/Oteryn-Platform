//! CC v2 pull/read/receipt and private closure projection transport. Mount only
//! behind verified TLS 1.3/HTTP 1.1. Signatures are detached HTTP metadata; the
//! contract's original JSON bytes are never enveloped or reformatted.
use super::*;
use axum::{
    Router,
    body::Bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
pub const GROUP: &str = "OTERYN_GAME_CHARACTER_COMMANDS";
#[derive(Clone)]
struct StateData {
    pool: PgPool,
    boundary: Arc<GameBoundary>,
    key_id: String,
    rate: Arc<Mutex<(Instant, u32)>>,
}
pub fn router(
    pool: PgPool,
    boundary: Arc<GameBoundary>,
    key_id: String,
    identities: Arc<IdentityMap>,
) -> Router {
    let state = StateData {
        pool,
        boundary,
        key_id,
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let mut routes = Router::new();
    for (path, handler, max, depth) in [
        (
            "/internal/v1/game-auth/account-closure/lifecycle/read",
            post(lifecycle),
            512,
            3,
        ),
        (
            "/internal/v1/game-auth/character-commands/pending",
            post(pending),
            512,
            3,
        ),
        (
            "/internal/v1/game-auth/character-commands/read",
            post(read),
            512,
            3,
        ),
        (
            "/internal/v1/game-auth/character-commands/receipt",
            post(receipt),
            2048,
            4,
        ),
        (
            "/internal/v1/game-auth/account-closure",
            post(projection),
            2048,
            4,
        ),
        (
            "/internal/v1/game-auth/account-closure/watermark",
            post(watermark),
            512,
            3,
        ),
    ] {
        let route = Router::new()
            .route(path, handler)
            .route_layer(middleware::from_fn_with_state(state.clone(), rate));
        routes = routes.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: state.boundary.settings.ready().is_ok(),
                group: GROUP.into(),
                max_body_bytes: max,
                max_json_depth: depth,
                json: JsonRule::Object,
            },
        ));
    }
    routes
        .with_state(state)
        .layer(middleware::from_fn(no_cache))
}
async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
}
async fn rate(State(state): State<StateData>, request: Request, next: Next) -> Response {
    let allowed = state.rate.lock().is_ok_and(|mut rate| {
        if rate.0.elapsed() >= Duration::from_secs(60) {
            *rate = (Instant::now(), 0);
        }
        if rate.1 >= 600 {
            false
        } else {
            rate.1 += 1;
            true
        }
    });
    if !allowed {
        return guard::failure(StatusCode::TOO_MANY_REQUESTS);
    }
    next.run(request).await
}
fn signature(headers: &HeaderMap, state: &StateData) -> Result<Vec<u8>> {
    let one = |name: &str| -> Result<&str> {
        let mut all = headers.get_all(name).iter();
        let value = all.next().ok_or(Error::Unauthorized)?;
        if all.next().is_some() {
            return Err(Error::Unauthorized);
        }
        value.to_str().map_err(|_| Error::Unauthorized)
    };
    if one("x-oteryn-key-id")? != state.key_id {
        return Err(Error::Unauthorized);
    }
    let signature = runtime::decode_hex(one("x-oteryn-signature")?)?;
    if signature.len() != 64 {
        return Err(Error::Unauthorized);
    }
    Ok(signature)
}
fn response(result: Result<Vec<u8>>) -> Response {
    match result {
        Ok(body) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(error) => guard::failure(match error {
            Error::InvalidInput => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::NoPending => StatusCode::NOT_FOUND,
            Error::Conflict => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }),
    }
}
fn accepted() -> Result<Vec<u8>> {
    Ok(b"{\"contract_version\":2,\"result\":\"accepted\"}".to_vec())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    contract_version: u8,
    operation: String,
    source_authority: String,
    after_source_revision: String,
    after_operation_id: String,
    max_entries: u8,
}
#[derive(Serialize)]
struct Entry {
    command: String,
    operation_id: String,
    source_revision: String,
}
#[derive(Serialize)]
struct PendingResponse {
    contract_version: u8,
    pending: Vec<Entry>,
}
async fn pending(State(state): State<StateData>, body: Bytes) -> Response {
    response(
        async {
            let request: Pending = wire::canonical(&body, 512)?;
            if request.contract_version != 2
                || request.operation != "ListPendingCharacterCommandsV1"
                || request.source_authority != state.boundary.authority
                || !(1..=32).contains(&request.max_entries)
            {
                return Err(Error::InvalidInput);
            }
            let revision = wire::number(&request.after_source_revision, false)?;
            if !((revision == 0 && request.after_operation_id == Uuid::nil().to_string())
                || wire::id(&request.after_operation_id).is_ok())
            {
                return Err(Error::InvalidInput);
            }
            let raws = if let Ok(revision) = i64::try_from(revision) {
                state
                    .boundary
                    .pending_intents(&state.pool, revision, i64::from(request.max_entries))
                    .await?
            } else {
                Vec::new()
            };
            let mut entries = Vec::with_capacity(raws.len());
            for raw in raws {
                let intent = Intent::parse(&raw)?;
                entries.push(Entry {
                    command: intent.command,
                    operation_id: intent.operation_id,
                    source_revision: intent.source_revision,
                });
            }
            serde_json::to_vec(&PendingResponse {
                contract_version: 2,
                pending: entries,
            })
            .map_err(|_| Error::InvalidInput)
        }
        .await,
    )
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    contract_version: u8,
    operation: String,
    source_authority: String,
    issuer_authority: String,
    command: String,
    operation_id: String,
}
async fn read(State(state): State<StateData>, body: Bytes) -> Response {
    response(
        async {
            let request: Read = wire::canonical(&body, 512)?;
            if request.contract_version != 2
                || request.operation != "ReadCharacterCommandIntentV1"
                || request.source_authority != state.boundary.authority
                || request.issuer_authority != "OTERYN_PLATFORM_CHARACTER_AUTHORITY"
            {
                return Err(Error::InvalidInput);
            }
            let raw = state
                .boundary
                .read_intent(&state.pool, wire::id(&request.operation_id)?)
                .await?
                .ok_or(Error::NoPending)?;
            if Intent::parse(&raw)?.command != request.command {
                return Err(Error::Conflict);
            }
            Ok(raw)
        }
        .await,
    )
}
async fn receipt(State(state): State<StateData>, headers: HeaderMap, body: Bytes) -> Response {
    response(
        async {
            state
                .boundary
                .ingest_receipt(&state.pool, &body, &signature(&headers, &state)?)
                .await?;
            accepted()
        }
        .await,
    )
}
async fn projection(State(state): State<StateData>, headers: HeaderMap, body: Bytes) -> Response {
    response(
        async {
            state
                .boundary
                .ingest_projection(&state.pool, &body, &signature(&headers, &state)?)
                .await?;
            accepted()
        }
        .await,
    )
}
async fn watermark(State(state): State<StateData>, headers: HeaderMap, body: Bytes) -> Response {
    response(
        async {
            state
                .boundary
                .ingest_watermark(&state.pool, &body, &signature(&headers, &state)?)
                .await?;
            accepted()
        }
        .await,
    )
}

async fn lifecycle(State(state): State<StateData>, body: Bytes) -> Response {
    response(state.boundary.read_lifecycle(&state.pool, &body).await)
}
