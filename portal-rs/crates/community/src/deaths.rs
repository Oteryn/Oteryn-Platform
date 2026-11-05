//! Default-off V2 deaths ingestion and privacy-gated public presentation.
use axum::{
    Extension, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use sqlx::PgPool;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
mod storage;
mod web;
mod wire;
pub use storage::Cleanup;
pub use web::{Config, FailClosed, Privacy, PrivacyFuture, Unavailable, router};
pub use wire::V2Family;

#[derive(Clone, Debug)]
pub struct IngestConfig {
    pub requests_per_minute: u16,
    pub enabled: bool,
    pub source_authority: String,
}
impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            source_authority: String::new(),
            requests_per_minute: 6000,
        }
    }
}
#[derive(Clone)]
struct Ingest {
    db: PgPool,
    config: IngestConfig,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
/// Only verified TLS metadata supplied by the trusted adapter establishes identity.
pub fn ingest_router(db: PgPool, identities: Arc<IdentityMap>, config: IngestConfig) -> Router {
    let mut routes = Router::new();
    for (path, handler, limit) in [
        ("character-deaths", post(snapshot), 32768),
        ("watermark", post(watermark), 512),
    ] {
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), handler)
            .with_state(Ingest {
                db: db.clone(),
                config: config.clone(),
            });
        routes = routes.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: config.enabled
                    && wire::authority(&config.source_authority)
                    && (1..=6000).contains(&config.requests_per_minute),
                group: "game-public".into(),
                max_body_bytes: limit,
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        ));
    }
    routes.layer(axum::middleware::from_fn(headers))
}
async fn headers(mut request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    for name in ["content-type", "content-encoding"] {
        if request.headers().get_all(name).iter().count() == 1 {
            let normalized = request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    if name == "content-type" {
                        v.split(';')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_ascii_lowercase()
                    } else {
                        v.trim().to_ascii_lowercase()
                    }
                });
            if let Some(value) = normalized.and_then(|v| axum::http::HeaderValue::from_str(&v).ok())
            {
                request
                    .headers_mut()
                    .insert(axum::http::HeaderName::from_static(name), value);
            }
        }
    }
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
fn decode<T: serde::de::DeserializeOwned + serde::Serialize>(
    v: &Value,
    body: &[u8],
) -> Result<T, ()> {
    if canonical::encode(v).ok().as_deref() != Some(body) {
        return Err(());
    }
    let decoded: T = serde_json::from_slice(body).map_err(|_| ())?;
    // Struct declaration order is the exact wire member order, including inner records.
    let bytes = serde_json::to_vec(&decoded).map_err(|_| ())?;
    let ordered = canonical::decode(&bytes, 32768, 6).map_err(|_| ())?;
    if canonical::encode(&ordered).ok().as_deref() != Some(body) {
        return Err(());
    }
    Ok(decoded)
}
async fn snapshot(
    State(s): State<Ingest>,
    Extension(v): Extension<Value>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if headers
        .get("content-length")
        .is_some_and(|h| h.to_str().ok().and_then(|v| v.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let Ok(value) = decode::<wire::Snapshot>(&v, &body) else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    if value.validate(now()).is_err() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    if value.source_authority != s.config.source_authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    if let Err(response) = rate(&s, "character-deaths").await {
        return response;
    }
    result(storage::snapshot(&s.db, &value).await)
}
async fn watermark(
    State(s): State<Ingest>,
    Extension(v): Extension<Value>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if headers
        .get("content-length")
        .is_some_and(|h| h.to_str().ok().and_then(|v| v.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let Ok(value) = decode::<wire::Watermark>(&v, &body) else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    if value.validate(now()).is_err() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    if value.source_authority != s.config.source_authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    if let Err(response) = rate(&s, "watermark").await {
        return response;
    }
    result(storage::watermark(&s.db, &value).await)
}
async fn rate(s: &Ingest, route: &str) -> Result<(), Response> {
    let count: Result<i32, sqlx::Error> = sqlx::query_scalar("INSERT INTO public_deaths_v2_rate_limits (route,window_start,requests) VALUES ($1,(extract(epoch FROM statement_timestamp())::bigint/60)*60,1) ON CONFLICT (route) DO UPDATE SET window_start=EXCLUDED.window_start, requests=CASE WHEN public_deaths_v2_rate_limits.window_start=EXCLUDED.window_start THEN least(public_deaths_v2_rate_limits.requests+1,6001) ELSE 1 END RETURNING requests").bind(route).fetch_one(&s.db).await;
    match count {
        Ok(n) if n <= i32::from(s.config.requests_per_minute) => Ok(()),
        Ok(_) => {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            response
                .headers_mut()
                .insert("retry-after", axum::http::HeaderValue::from_static("60"));
            Err(response)
        }
        Err(_) => Err(guard::failure(StatusCode::SERVICE_UNAVAILABLE)),
    }
}
fn result(result: Result<&'static str, storage::Error>) -> Response {
    match result {
        Ok(result) => {
            axum::Json(serde_json::json!({"contract_version":2,"result":result})).into_response()
        }
        Err(storage::Error::Conflict) => guard::failure(StatusCode::CONFLICT),
        Err(storage::Error::Database) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// Binary wiring deliberately denies presentation until a privacy authority is provided.
pub fn configured_router(db: PgPool) -> Router {
    let env_allowed = matches!(
        std::env::var("APP_ENV").as_deref(),
        Ok("testing" | "preproduction")
    );
    let base =
        env_allowed && std::env::var("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Ok("true");
    let authority = std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default();
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
    ingest_router(
        db.clone(),
        Arc::new(identities),
        IngestConfig {
            requests_per_minute: std::env::var("PORTAL_DEATHS_INGEST_REQUESTS_PER_MINUTE")
                .map_or(6000, |s| s.parse().unwrap_or(0)),
            enabled: base
                && std::env::var("PORTAL_DEATHS_V2_INGEST_ENABLED").as_deref() == Ok("true"),
            source_authority: authority.clone(),
        },
    )
    .merge(router(
        db,
        Arc::new(FailClosed),
        Config {
            enabled: base && std::env::var("PORTAL_DEATHS_ENABLED").as_deref() == Ok("true"),
            source_authority: authority,
        },
    ))
}
#[cfg(test)]
mod tests;
