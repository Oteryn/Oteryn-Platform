use super::{Error, GROUP, PATH, Restore, storage, wire};
use crate::commands;
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical,
    guard::{self, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tower::ServiceExt;
use uuid::Uuid;
#[derive(Debug)]
struct Rate {
    start: Instant,
    used: u32,
}
#[derive(Clone)]
struct NoticeState {
    restore: Restore,
    exchange: Router,
    qualified: bool,
    rates: Arc<Mutex<HashMap<String, Rate>>>,
}
#[derive(Clone)]
struct ExchangeState {
    restore: Restore,
    probe: Router,
    slots: Arc<Semaphore>,
}

/// Mount this composite on the private verified TLS listener, replacing the bare
/// command router. Recovery enables only exchange, never `Commands::enqueue`.
/// `exchange_available` must reflect deployment/environment qualification; the
/// command guard is additionally probed so absent identities cannot acknowledge
/// work that Game cannot answer. Use a pool with at least two connections.
pub fn router(
    restore: Restore,
    mut command_settings: commands::Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    if restore.settings.available() && restore.settings.exchange_available {
        command_settings.enabled = true;
    }
    let commands = commands::Commands::new(restore.pool.clone(), command_settings.clone());
    let slots = restore.pool.options().get_max_connections() / 2;
    let probe = commands::router(commands.clone(), identities.clone());
    let exchange_state = ExchangeState {
        probe: probe.clone(),
        restore: restore.clone(),
        slots: Arc::new(Semaphore::new(slots as usize)),
    };
    let exchange = commands::router(commands, identities.clone()).layer(
        middleware::from_fn_with_state(exchange_state, serialize_receipt),
    );
    let exchange = guard::protect(
        exchange,
        identities.clone(),
        RoutePolicy {
            enabled: command_settings.enabled && slots > 0,
            group: commands::GROUP.into(),
            max_body_bytes: 1024,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    );
    let state = NoticeState {
        restore: restore.clone(),
        exchange: probe,
        qualified: slots > 0
            && command_settings.source_authority == restore.settings.source_authority,
        rates: Arc::new(Mutex::new(HashMap::new())),
    };
    let notices = Router::new().route(PATH, post(notice));
    let notices = guard::protect(
        notices,
        identities,
        RoutePolicy {
            enabled: restore.settings.available(),
            group: GROUP.into(),
            max_body_bytes: 512,
            max_json_depth: 4,
            json: JsonRule::Canonical(&wire::SHAPE),
        },
    )
    .with_state(state);
    exchange.merge(notices).layer(middleware::from_fn(no_cache))
}
async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
}
fn failure(error: Error) -> Response {
    guard::failure(match error {
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    })
}
async fn notice(
    State(state): State<NoticeState>,
    client: axum::Extension<VerifiedClient>,
    body: Bytes,
) -> Response {
    // The guard has already restricted the key space to configured identities.
    let allowed = if let Ok(mut rates) = state.rates.lock() {
        let rate = rates.entry(format!("{:?}", client.0)).or_insert(Rate {
            start: Instant::now(),
            used: 0,
        });
        if rate.start.elapsed() >= Duration::from_secs(60) {
            rate.start = Instant::now();
            rate.used = 0;
        }
        if rate.used < state.restore.settings.requests_per_minute {
            rate.used += 1;
            true
        } else {
            false
        }
    } else {
        return failure(Error::Unavailable);
    };
    if !allowed {
        let mut r = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        r.headers_mut().insert(
            header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("60"),
        );
        return r;
    }
    let mut restore = state.restore;
    // A peerless probe returns 401 only when command identities are configured;
    // it never enters storage. Also validate the inner command configuration.
    let ready = exchange_ready(state.exchange).await;
    if !ready || !state.qualified {
        Arc::make_mut(&mut restore.settings).exchange_available = false;
    }
    match restore.notice(&body).await {
        Ok(raw) => ([(header::CONTENT_TYPE, "application/json")], raw).into_response(),
        Err(e) => failure(e),
    }
}
async fn serialize_receipt(
    State(state): State<ExchangeState>,
    request: Request,
    next: Next,
) -> Response {
    if request.uri().path() != "/internal/v1/game-auth/character-commands/receipt" {
        return next.run(request).await;
    }
    match receipt(state, request, next).await {
        Ok(r) => r,
        Err(e) => failure(e),
    }
}
async fn exchange_ready(probe: Router) -> bool {
    let request =
        Request::post("/internal/v1/game-auth/character-commands/pending").body(Body::empty());
    match request {
        Ok(request) => match probe.oneshot(request).await {
            Ok(response) => response.status() == StatusCode::UNAUTHORIZED,
            Err(error) => match error {},
        },
        Err(_) => false,
    }
}
async fn receipt(state: ExchangeState, request: Request, next: Next) -> Result<Response, Error> {
    if !exchange_ready(state.probe.clone()).await {
        return Err(Error::Unavailable);
    }
    let (parts, body) = request.into_parts();
    let raw = to_bytes(body, 1024).await.map_err(|_| Error::BadInput)?;
    let _slot = state
        .slots
        .acquire()
        .await
        .map_err(|_| Error::Unavailable)?;
    let mut tx = state.restore.pool.begin().await?;
    storage::lock(&mut tx, false).await?;
    let mut response = next
        .run(Request::from_parts(parts, Body::from(raw.clone())))
        .await;
    if matches!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE | StatusCode::CONFLICT
    ) {
        // T22 validates the complete receipt before its UPDATE. The trigger
        // rejects a generation lost to restore; translate that storage refusal
        // only after T22 has applied its ordinary wire and binding checks.
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT max(new_recovery_generation) FROM character_restore_notices",
        )
        .fetch_one(&mut *tx)
        .await?;
        let v = canonical::decode(&raw, 1024, 4).map_err(|_| Error::BadInput)?;
        let generation = wire::counter(wire::text(&v, "recovery_generation")?)?;
        if latest.is_some_and(|latest| format!("{generation:020}") < latest) {
            let id =
                Uuid::parse_str(wire::text(&v, "operation_id")?).map_err(|_| Error::BadInput)?;
            tx.rollback().await?;
            storage::alarm(
                &state.restore.pool,
                None,
                Some(id),
                "character_authority.command_receipt_lost_to_restore",
            )
            .await?;
            response = failure(Error::Conflict);
            return Ok(response);
        }
    }
    tx.commit().await?;
    Ok(response)
}
