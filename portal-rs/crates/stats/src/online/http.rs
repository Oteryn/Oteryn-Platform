use super::*;
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Ingest {
    db: PgPool,
    config: Config,
    family: Option<Family>,
    // One purpose-bound publisher per configured identity map, per route.
    requests: Arc<Mutex<VecDeque<std::time::Instant>>>,
}
/// Mount on the private listener only. VerifiedClient must come from the TLS adapter.
pub fn router(db: PgPool, identities: Arc<IdentityMap>, config: Config) -> Router {
    let mut app = Router::new();
    for (path, family) in [
        (
            "character-online-session",
            Some(Family::CharacterOnlineSessions),
        ),
        ("world-daily-kills", Some(Family::WorldDailyKills)),
        ("watermark", None),
    ] {
        let enabled = family.map_or(
            config.enabled
                && (config.intervals_enabled || config.world_totals_enabled)
                && wire::authority(&config.source_authority),
            |f| config.permits(f),
        );
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), post(handle))
            .with_state(Ingest {
                db: db.clone(),
                config: config.clone(),
                family,
                requests: Arc::new(Mutex::new(VecDeque::new())),
            });
        app = app.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled,
                group: "game-public".into(),
                max_body_bytes: if family.is_some() { 2048 } else { 512 },
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        ));
    }
    app.layer(middleware::from_fn(headers))
}
async fn headers(request: Request, next: Next) -> Response {
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
async fn handle(
    State(state): State<Ingest>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let allowed = match state.requests.lock() {
        Ok(mut requests) => {
            let now = std::time::Instant::now();
            while requests
                .front()
                .is_some_and(|v| now.duration_since(*v).as_secs() >= 60)
            {
                requests.pop_front();
            }
            if requests.len() >= 1200 {
                false
            } else {
                requests.push_back(now);
                true
            }
        }
        Err(_) => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    if !allowed {
        let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        response
            .headers_mut()
            .insert("retry-after", axum::http::HeaderValue::from_static("60"));
        return response;
    }
    if headers
        .get("content-length")
        .is_some_and(|v| v.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    match ingest(&state.db, &state.config, state.family, &body, now).await {
        Ok(receipt) => {
            use axum::response::IntoResponse;
            ([("content-type", "application/json")], receipt.body()).into_response()
        }
        Err(error) => guard::failure(error.status()),
    }
}
/// Defaults off and is restricted to testing/preproduction. No owner/public
/// duration presentation is mounted without current authoritative policy adapters.
pub fn configured_router(db: PgPool) -> Router {
    let enabled = std::env::var("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Ok("true")
        && matches!(
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
    router(
        db,
        Arc::new(identities),
        Config {
            enabled,
            intervals_enabled: std::env::var("PORTAL_STATS_ONLINE_INTERVALS_ENABLED").as_deref()
                == Ok("true"),
            world_totals_enabled: std::env::var("PORTAL_STATS_WORLD_TOTALS_ENABLED").as_deref()
                == Ok("true"),
            owner_rankings_enabled: false,
            source_authority: std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY")
                .unwrap_or_default(),
        },
    )
}
