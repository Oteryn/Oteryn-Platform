use super::*;
use axum::{
    Router,
    body::Bytes,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
#[derive(Debug)]
struct Rate {
    window: Instant,
    used: u32,
}
#[derive(Debug, Clone)]
struct HttpState {
    commands: Commands,
    rate: Arc<Mutex<Rate>>,
}
/// Mount only on the private TLS listener. Dedicated identity, body bounds and
/// one shared request budget apply across all three routes.
pub fn router(commands: Commands, identities: Arc<IdentityMap>) -> Router {
    let state = HttpState {
        commands: commands.clone(),
        rate: Arc::new(Mutex::new(Rate {
            window: Instant::now(),
            used: 0,
        })),
    };
    let mut router = Router::new();
    for (path, handler, limit) in [
        ("pending", post(pending), 512),
        ("read", post(read), 512),
        ("receipt", post(receipt), 1024),
    ] {
        let route = Router::new()
            .route(
                &format!("/internal/v1/game-auth/character-commands/{path}"),
                handler,
            )
            .route_layer(middleware::from_fn_with_state(state.clone(), rate_limit));
        router = router.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: commands.settings.available(),
                group: GROUP.into(),
                max_body_bytes: limit,
                max_json_depth: 4,
                json: JsonRule::Object,
            },
        ));
    }
    router
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
async fn rate_limit(State(state): State<HttpState>, request: Request, next: Next) -> Response {
    let allowed = if let Ok(mut rate) = state.rate.lock() {
        if rate.window.elapsed() >= Duration::from_secs(60) {
            rate.window = Instant::now();
            rate.used = 0;
        }
        if rate.used < state.commands.settings.requests_per_minute {
            rate.used += 1;
            true
        } else {
            false
        }
    } else {
        false
    };
    if !allowed {
        let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        response.headers_mut().insert(
            header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("60"),
        );
        return response;
    }
    next.run(request).await
}
fn answer(result: Result<String, Error>) -> Response {
    match result {
        Ok(raw) => ([(header::CONTENT_TYPE, "application/json")], raw).into_response(),
        Err(error) => guard::failure(match error {
            Error::BadInput => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Denied => StatusCode::FORBIDDEN,
            Error::Conflict => StatusCode::CONFLICT,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Unavailable | Error::BacklogFull => StatusCode::SERVICE_UNAVAILABLE,
        }),
    }
}
async fn pending(State(state): State<HttpState>, body: Bytes) -> Response {
    answer(state.commands.pending(&body).await)
}
async fn read(State(state): State<HttpState>, body: Bytes) -> Response {
    answer(state.commands.read(&body).await)
}
async fn receipt(State(state): State<HttpState>, body: Bytes) -> Response {
    answer(
        state
            .commands
            .publish(&body)
            .await
            .map(|()| "{\"contract_version\":1,\"result\":\"accepted\"}".into()),
    )
}
