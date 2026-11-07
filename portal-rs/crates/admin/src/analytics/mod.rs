//! Purpose-isolated T10b aggregate consumer. Export has no route.
pub mod purchased;
pub mod store;
pub mod web;
pub mod wire;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use std::sync::Arc;
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub ingest: bool,
    pub present: bool,
    pub authority: String,
    pub privacy: String,
    pub requests_per_minute: u32,
    pub max_staging_bytes: u64,
}
impl Config {
    pub fn from_env() -> Result<Self, guard::ConfigError> {
        let rate = std::env::var("OTERYN_ADMIN_ANALYTICS_RATE")
            .unwrap_or_else(|_| "1200".into())
            .parse()
            .map_err(|_| guard::ConfigError)?;
        if !(1..=6000).contains(&rate) {
            return Err(guard::ConfigError);
        }
        let config = Self {
            ingest: std::env::var("OTERYN_ADMIN_ANALYTICS_INGEST").is_ok_and(|v| v == "true"),
            present: std::env::var("OTERYN_ADMIN_ANALYTICS_PRESENT").is_ok_and(|v| v == "true"),
            authority: std::env::var("OTERYN_ADMIN_ANALYTICS_AUTHORITY").unwrap_or_default(),
            privacy: std::env::var("OTERYN_ADMIN_ANALYTICS_PRIVACY").unwrap_or_default(),
            requests_per_minute: rate,
            max_staging_bytes: std::env::var("OTERYN_ADMIN_ANALYTICS_STAGING_BYTES")
                .unwrap_or_else(|_| "268435456".into())
                .parse()
                .map_err(|_| guard::ConfigError)?,
        };
        if config.max_staging_bytes < 8388608 {
            return Err(guard::ConfigError);
        }
        if (config.ingest || config.present)
            && (config.authority.is_empty() || config.privacy.is_empty())
        {
            return Err(guard::ConfigError);
        }
        Ok(config)
    }
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |v| v.as_secs())
}
#[derive(Clone)]
struct Ingest {
    store: store::Store,
    exchanges: Arc<tokio::sync::Semaphore>,
}
pub fn router(pool: sqlx::PgPool, config: Config, identities: Arc<IdentityMap>) -> Router {
    let state = Ingest {
        exchanges: Arc::new(tokio::sync::Semaphore::new(4)),
        store: store::Store {
            pool,
            authority: config.authority,
            privacy: config.privacy,
            rate: config.requests_per_minute,
            max_staging_bytes: config.max_staging_bytes,
        },
    };
    let mut router = Router::new();
    for (operation, ceiling) in [("page", 131072), ("commit", 8192), ("watermark", 1024)] {
        let path = format!("/internal/v1/admin-analytics/{operation}");
        let op = operation.to_owned();
        let route = Router::new().route(
            &path,
            post(move |s: State<Ingest>, h: HeaderMap, b: Bytes| {
                let op = op.clone();
                async move { ingest(s, h, op, b).await }
            }),
        );
        router = router.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: config.ingest,
                group: "game-admin-analytics".into(),
                max_body_bytes: ceiling,
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        ));
    }
    router
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.exchanges,
            exchange_limit,
        ))
        .layer(middleware::from_fn(status_and_cache))
}
async fn exchange_limit(
    State(exchanges): State<Arc<tokio::sync::Semaphore>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Ok(_permit) = exchanges.try_acquire() else {
        return guard::failure(StatusCode::SERVICE_UNAVAILABLE);
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), next.run(request))
        .await
        .unwrap_or_else(|_| guard::failure(StatusCode::SERVICE_UNAVAILABLE))
}
async fn status_and_cache(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        response = guard::failure(StatusCode::BAD_REQUEST)
    }
    response.headers_mut().insert(
        header::CONNECTION,
        axum::http::HeaderValue::from_static("close"),
    );
    response.headers_mut().insert(
        header::PRAGMA,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
}
async fn ingest(
    State(s): State<Ingest>,
    headers: HeaderMap,
    operation: String,
    raw: Bytes,
) -> Response {
    if headers
        .get(header::CONTENT_LENGTH)
        .is_some_and(|v| v.to_str().ok().and_then(|v| v.parse::<usize>().ok()) != Some(raw.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let now = now();
    let message = match wire::parse(&operation, &raw, now) {
        Ok(m) => m,
        Err(_) => return guard::failure(StatusCode::BAD_REQUEST),
    };
    let authority = match &message {
        wire::Message::Page(p) => &p.source_authority,
        wire::Message::Commit(c) => &c.source_authority,
        wire::Message::Watermark(w) => &w.source_authority,
    };
    if authority != &s.store.authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    match s.store.ingest(message, &raw, now).await {
        Ok("accepted") => (
            [(header::CONTENT_TYPE, "application/json")],
            "{\"contract_version\":1,\"result\":\"accepted\"}",
        )
            .into_response(),
        Ok(_) => (
            [(header::CONTENT_TYPE, "application/json")],
            "{\"contract_version\":1,\"result\":\"superseded\"}",
        )
            .into_response(),
        Err(store::Error::Invalid) => guard::failure(StatusCode::BAD_REQUEST),
        Err(store::Error::Conflict) => guard::failure(StatusCode::CONFLICT),
        Err(store::Error::Rate) => {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            response.headers_mut().insert(
                header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("60"),
            );
            response
        }
        Err(_) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
/// The TLS adapter uses the same global IdentityMap as every other internal purpose.
/// Configuration here supplies only a SAN mapping, never a bearer credential.
pub fn configured_router(pool: sqlx::PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let maintenance = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let _ = store::purge(&maintenance, now()).await;
        }
    });
    let bindings = match std::env::var("OTERYN_ADMIN_ANALYTICS_PEER_SAN") {
        Ok(san) if san == "game-admin-analytics" => vec![guard::Binding {
            identity: guard::Identity::san(&san)?,
            group: "game-admin-analytics".into(),
        }],
        Ok(_) => return Err(Box::new(guard::ConfigError)),
        Err(_) => Vec::new(),
    };
    let internal = router(
        pool.clone(),
        config.clone(),
        Arc::new(IdentityMap::new(bindings)?),
    );
    Ok(internal.merge(web::configured_router(pool, config)?))
}

#[cfg(test)]
mod tests;
