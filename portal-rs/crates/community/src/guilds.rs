//! G-4 guild snapshots and privacy-filtered public pages. No gameplay mutations.
use crate::v2_watermark::Part;
use axum::{
    Extension, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical,
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
#[path = "guilds_pages.rs"]
mod pages;
#[path = "guilds_wire.rs"]
mod wire;
pub use pages::{Config, MemberRef, PublicVisibility, Unavailable, VisibilityResult, router};

#[derive(Clone, Debug, Default)]
pub struct IngestConfig {
    pub enabled: bool,
    pub source_authority: String,
}
#[derive(Clone)]
struct Ingest {
    db: PgPool,
    config: IngestConfig,
}
/// Mount on the trusted mTLS listener; identity headers never grant access.
pub fn ingest_router(db: PgPool, identities: Arc<IdentityMap>, config: IngestConfig) -> Router {
    ingest_routes(db, identities, config, Part::All)
}
fn ingest_routes(
    db: PgPool,
    identities: Arc<IdentityMap>,
    config: IngestConfig,
    part: Part,
) -> Router {
    let mut result = Router::new();
    for (path, bound) in [("guild", 393216), ("watermark", 512)] {
        if !part.includes(path) {
            continue;
        }
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), post(handle))
            .with_state(Ingest {
                db: db.clone(),
                config: config.clone(),
            });
        result = result.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: config.enabled && wire::authority(&config.source_authority),
                group: "game-public".into(),
                max_body_bytes: bound,
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        ));
    }
    result.layer(axum::middleware::from_fn(policy))
}
async fn policy(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert("pragma", axum::http::HeaderValue::from_static("no-cache"));
    response
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
async fn handle(
    State(state): State<Ingest>,
    uri: axum::extract::OriginalUri,
    Extension(value): Extension<canonical::Value>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if headers
        .get("content-length")
        .is_some_and(|v| v.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let publication = match wire::parse(&value, &body, uri.path().ends_with("/watermark"), now()) {
        Ok(p) => p,
        Err(()) => return guard::failure(StatusCode::BAD_REQUEST),
    };
    if publication.authority != state.config.source_authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    match apply(&state.db, &publication).await {
        Ok(result) => {
            axum::Json(serde_json::json!({"contract_version":2,"result":result})).into_response()
        }
        Err(status) => guard::failure(status),
    }
}
struct Publication {
    authority: String,
    subject: String,
    epoch: String,
    revision: String,
    observed: i64,
    watermark: bool,
    payload: serde_json::Value,
}
async fn apply(db: &PgPool, p: &Publication) -> Result<&'static str, StatusCode> {
    let unavailable = |_| StatusCode::SERVICE_UNAVAILABLE;
    let mut tx = db.begin().await.map_err(unavailable)?;
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(unavailable)?;
    let number = |s: &str| {
        s.parse::<u64>()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    };
    if let Some(h) = highest
        && number(&p.epoch)? < number(&h)?
    {
        return Ok("superseded");
    }
    let row=sqlx::query("SELECT epoch::text,version::text,watermark,payload FROM public_snapshots WHERE family='guild' AND subject=$1 AND is_watermark=$2").bind(&p.subject).bind(p.watermark).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    if let Some(r) = row {
        let epoch: String = r.try_get("epoch").map_err(unavailable)?;
        let revision: String = r.try_get("version").map_err(unavailable)?;
        let order =
            (number(&p.epoch)?, number(&p.revision)?).cmp(&(number(&epoch)?, number(&revision)?));
        if p.watermark {
            let through: i64 = r.try_get("watermark").map_err(unavailable)?;
            if epoch == p.epoch && p.observed <= through {
                return Ok("superseded");
            }
        } else {
            if order.is_lt() {
                return Ok("superseded");
            }
            if order.is_eq() {
                let payload: serde_json::Value = r.try_get("payload").map_err(unavailable)?;
                return if payload == p.payload {
                    Ok("accepted")
                } else {
                    Err(StatusCode::CONFLICT)
                };
            }
        }
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(&p.epoch)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('guild',$1,$2,$3::numeric,$4::numeric,$5,$6) ON CONFLICT(family,subject,is_watermark) DO UPDATE SET epoch=EXCLUDED.epoch,version=EXCLUDED.version,watermark=EXCLUDED.watermark,payload=EXCLUDED.payload,previous_snapshot=NULL").bind(&p.subject).bind(p.watermark).bind(&p.epoch).bind(&p.revision).bind(p.observed).bind(&p.payload).execute(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok("accepted")
}
/// Environment activation remains testing/preproduction only; both flags default off.
/// The shared V2 watermark is mounted once by [`crate::v2_watermark`].
pub fn configured_router(db: PgPool) -> Router {
    configured(db, Part::Payload)
}
/// Only the guarded watermark route, for [`crate::v2_watermark::router`].
pub fn configured_watermark_router(db: PgPool) -> Router {
    configured(db, Part::Watermark)
}
fn configured(db: PgPool, part: Part) -> Router {
    let environment = matches!(
        std::env::var("APP_ENV").as_deref(),
        Ok("testing" | "preproduction")
    );
    let identities = std::env::var("PORTAL_PUBLIC_PROJECTIONS_SAN")
        .ok()
        .and_then(|s| guard::Identity::san(&s).ok())
        .and_then(|identity| {
            IdentityMap::new(vec![guard::Binding {
                identity,
                group: "game-public".into(),
            }])
            .ok()
        })
        .unwrap_or_default();
    let source_authority = std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default();
    let ingest = ingest_routes(
        db.clone(),
        Arc::new(identities),
        IngestConfig {
            enabled: environment
                && std::env::var("PORTAL_GUILDS_INGEST_ENABLED").as_deref() == Ok("true"),
            source_authority: source_authority.clone(),
        },
        part,
    );
    if part == Part::Watermark {
        return ingest;
    }
    pages::router_with_contract(
        db.clone(),
        Arc::new(crate::privacy::Authority::configured(db)),
        Config {
            source_authority,
            enabled: environment && std::env::var("PORTAL_GUILDS_ENABLED").as_deref() == Ok("true"),
        },
        false, // G-4 rev 1 is CANDIDATE, without a Game producer.
    )
    .merge(ingest)
}
#[cfg(test)]
#[path = "guilds_tests.rs"]
mod tests;
