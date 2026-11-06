//! Private listener composition only. Disabled/configuration-unknown routes stay 503.
use super::{Authority, Error, IngestResult, Outbox, ReceiptResult, Settings};
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{OriginalUri, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use sqlx::PgPool;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const BAZAAR_GROUP: &str = "OTERYN_GAME_BAZAAR_COMMANDS";
/// This is the existing Character receipt identity, separately provisioned from Bazaar.
pub const RECEIPT_GROUP: &str = "OTERYN_GAME_CHARACTER_COMMANDS";
#[derive(Clone, Debug)]
struct Http {
    authority: Authority,
    pool: PgPool,
    rate: Arc<Mutex<(Instant, u32)>>,
}
/// Merge the six Bazaar routes onto the private verified TLS listener, never a
/// browser listener. The Character facility can use `receipt_extension` below.
pub fn router(authority: Authority, pool: PgPool, identities: Arc<IdentityMap>) -> Router {
    let state = Http {
        authority,
        pool,
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let mut router = Router::new();
    for (path, route, max) in [
        ("commands/pending", post(exchange), 512),
        ("commands/read", post(exchange), 512),
        ("commands/reconcile", post(exchange), 512),
        ("transferability", post(ingest), 262144),
        ("status", post(ingest), 2048),
        ("watermark", post(ingest), 512),
    ] {
        let route = Router::new()
            .route(&format!("/internal/v1/bazaar/{path}"), route)
            .route_layer(middleware::from_fn_with_state(state.clone(), rate));
        router = router.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: state.authority.config.available(),
                group: BAZAAR_GROUP.into(),
                max_body_bytes: max,
                max_json_depth: 12,
                json: JsonRule::Object,
            },
        ));
    }
    router
        .with_state(state)
        .layer(middleware::from_fn(no_cache))
}
/// Extension handler at the existing receipt path. The composing Character
/// facility must dispatch v1 to its original handler; do not mount both routers
/// at the same path. No alternative receipt URL exists.
fn receipt_extension(authority: Authority, pool: PgPool, identities: Arc<IdentityMap>) -> Router {
    let state = Http {
        authority,
        pool,
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let route = Router::new()
        .route(
            "/internal/v1/game-auth/character-commands/receipt",
            post(receipt),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), rate));
    guard::protect(
        route,
        identities,
        RoutePolicy {
            enabled: state.authority.config.available(),
            group: RECEIPT_GROUP.into(),
            max_body_bytes: 270336,
            max_json_depth: 12,
            json: JsonRule::Object,
        },
    )
    .with_state(state)
    .layer(middleware::from_fn(no_cache))
}
async fn rate(State(state): State<Http>, request: Request, next: Next) -> Response {
    let allowed = state.rate.lock().is_ok_and(|mut r| {
        if r.0.elapsed() >= Duration::from_secs(60) {
            *r = (Instant::now(), 0);
        }
        if r.1 >= 120 {
            false
        } else {
            r.1 += 1;
            true
        }
    });
    if !allowed {
        return (StatusCode::TOO_MANY_REQUESTS, [(header::RETRY_AFTER, "10")]).into_response();
    }
    next.run(request).await
}
async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
}
fn result(value: Result<String, Error>) -> Response {
    match value {
        Ok(raw) => ([(header::CONTENT_TYPE, "application/json")], raw).into_response(),
        Err(e) => guard::failure(match e {
            Error::Unavailable | Error::Backlog | Error::Database(_) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Error::Denied => StatusCode::UNAUTHORIZED,
            Error::Conflict => StatusCode::CONFLICT,
            Error::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        }),
    }
}
async fn exchange(State(s): State<Http>, OriginalUri(uri): OriginalUri, raw: Bytes) -> Response {
    let value = async {
        route_operation(uri.path(), &raw)?;
        let mut tx = s.pool.begin().await?;
        let result = s
            .authority
            .exchange(&mut tx, &s.authority.config.projection_authority, &raw)
            .await?;
        tx.commit().await?;
        Ok(result)
    }
    .await;
    result(value)
}
async fn receipt(State(s): State<Http>, raw: Bytes) -> Response {
    let value = async {
        let mut tx = s.pool.begin().await?;
        let now = clock(&mut tx).await?;
        let outcome = s
            .authority
            .receive(&mut tx, &s.authority.config.receipt_authority, &raw, now)
            .await?;
        tx.commit().await?;
        if outcome == ReceiptResult::Quarantined {
            return Err(Error::Conflict);
        }
        Ok("{\"contract_version\":1,\"result\":\"accepted\"}".into())
    }
    .await;
    result(value)
}
async fn ingest(State(s): State<Http>, OriginalUri(uri): OriginalUri, raw: Bytes) -> Response {
    let value = async {
        route_operation(uri.path(), &raw)?;
        s.authority.enabled()?;
        let mut tx = s.pool.begin().await?;
        s.authority.lock(&mut tx).await?;
        let now = clock(&mut tx).await?;
        let outbox = Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: false,
            },
        };
        let outcome = outbox
            .ingest(&mut tx, &s.authority.config.projection_authority, &raw, now)
            .await?;
        tx.commit().await?;
        if outcome == IngestResult::Quarantined {
            return Err(Error::Conflict);
        }
        Ok(format!(
            "{{\"contract_version\":2,\"result\":\"{}\"}}",
            if outcome == IngestResult::Superseded {
                "superseded"
            } else {
                "accepted"
            }
        ))
    }
    .await;
    result(value)
}
async fn clock(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<u64, Error> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    u64::try_from(now).map_err(|_| Error::Invalid)
}

/// Preserve the original Character v1 facility byte-for-byte while selecting the
/// new version before applying its larger bound. Both paths retain their own
/// purpose guard; version selection cannot select another peer's trust.
pub fn extend_receipt_facility(
    legacy: Router,
    authority: Authority,
    pool: PgPool,
    identities: Arc<IdentityMap>,
) -> Router {
    let extension = receipt_extension(authority, pool, identities);
    let state = ReceiptFacility {
        legacy: legacy.clone(),
        extension,
    };
    Router::new()
        .route(
            "/internal/v1/game-auth/character-commands/receipt",
            post(select_receipt),
        )
        .with_state(state)
        .fallback_service(legacy)
}
#[derive(Clone, Debug)]
struct ReceiptFacility {
    legacy: Router,
    extension: Router,
}
async fn select_receipt(State(s): State<ReceiptFacility>, request: Request) -> Response {
    use tower::ServiceExt;
    let (parts, body) = request.into_parts();
    let raw = match to_bytes(body, 270336).await {
        Ok(raw) => raw,
        Err(_) => return guard::failure(StatusCode::PAYLOAD_TOO_LARGE),
    };
    let v2 = super::authority_wire::decode(&raw, 270336)
        .ok()
        .is_some_and(|v| {
            v.get("contract_version") == Some(&oteryn_internal_api::canonical::Value::Unsigned(2))
        });
    let router = if v2 { s.extension } else { s.legacy };
    match router
        .oneshot(Request::from_parts(parts, Body::from(raw)))
        .await
    {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

fn route_operation(path: &str, raw: &[u8]) -> Result<(), Error> {
    let expected = match path {
        "/internal/v1/bazaar/commands/pending" => "ListPendingBazaarCommandsV1",
        "/internal/v1/bazaar/commands/read" => "ReadBazaarCommandIntentV1",
        "/internal/v1/bazaar/commands/reconcile" => "ListBazaarCommandStatesV1",
        "/internal/v1/bazaar/transferability" => "PublishBazaarTransferabilityV1",
        "/internal/v1/bazaar/status" => "PublishBazaarListingStatusV1",
        "/internal/v1/bazaar/watermark" => "PublishBazaarWatermarkV1",
        _ => return Err(Error::Invalid),
    };
    let v = super::authority_wire::decode(raw, 262144)?;
    if super::game_wire::text(&v, "operation")? != expected {
        return Err(Error::Invalid);
    }
    Ok(())
}
