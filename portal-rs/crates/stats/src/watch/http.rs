//! Bearer-only private API. Cookies never authorize these mutations.
use super::*;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};

pub fn router(watches: Watches) -> Router {
    if !watches.enabled {
        return Router::new();
    }
    Router::new()
        .route("/api/v1/stats/watches", get(list).post(add).delete(remove))
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::from_fn(
            |request: axum::extract::Request, next: middleware::Next| async move {
                let mut response = next.run(request).await;
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("private, no-store"),
                );
                response
                    .headers_mut()
                    .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
                if response.status() == StatusCode::TOO_MANY_REQUESTS {
                    response
                        .headers_mut()
                        .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
                }
                response
            },
        ))
        .with_state(Arc::new(watches))
}
#[derive(Debug)]
struct Deny;
impl Policy for Deny {
    fn owner<'a>(&'a self, _: &'a str) -> PolicyFuture<'a, Option<Uuid>> {
        Box::pin(async { Ok(None) })
    }
    fn allowed<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
        _: Subscription,
        _: Option<Event>,
    ) -> PolicyFuture<'a, bool> {
        Box::pin(async { Ok(false) })
    }
    fn locale(&self, _: Uuid) -> PolicyFuture<'_, Locale> {
        Box::pin(async { Err(invalid()) })
    }
}
/// Enabling the flag alone grants no authority; an accepted T66 adapter is required.
pub fn configured_router(pool: PgPool) -> Router {
    router(Watches {
        pool,
        enabled: std::env::var("PORTAL_STATS_WATCHES_ENABLED").as_deref() == Ok("true"),
        policy: Arc::new(Deny),
    })
}
async fn owner(w: &Watches, headers: &HeaderMap) -> std::result::Result<Uuid, StatusCode> {
    if !w.enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .filter(|s| !s.is_empty() && s.len() <= 8192 && s.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let owner = w
        .policy
        .owner(token)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .filter(|id| !id.is_nil())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let requests: i32 = sqlx::query_scalar("INSERT INTO stats_watch_budgets(account_id,requests) VALUES($1,1) ON CONFLICT(account_id) DO UPDATE SET requests=CASE WHEN stats_watch_budgets.window_start <= now()-interval '1 minute' THEN 1 ELSE LEAST(stats_watch_budgets.requests+1,61) END, window_start=CASE WHEN stats_watch_budgets.window_start <= now()-interval '1 minute' THEN now() ELSE stats_watch_budgets.window_start END,updated_at=now() RETURNING requests")
        .bind(owner).fetch_one(&w.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    if requests > 60 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(owner)
}
async fn list(State(w): State<Arc<Watches>>, headers: HeaderMap) -> Response {
    let owner = match owner(&w, &headers).await {
        Ok(id) => id,
        Err(code) => return code.into_response(),
    };
    match w.list(owner).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn mutate(w: Arc<Watches>, headers: HeaderMap, body: &[u8], remove: bool) -> Response {
    let owner = match owner(&w, &headers).await {
        Ok(id) => id,
        Err(code) => return code.into_response(),
    };
    let s: Subscription = match serde_json::from_slice(body) {
        Ok(s) => s,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    if s.target_id.is_nil() {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }
    let result = if remove {
        w.unsubscribe(owner, s).await
    } else {
        w.subscribe(owner, s).await
    };
    match result {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
    .into_response()
}
async fn add(
    State(w): State<Arc<Watches>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    mutate(w, headers, &body, false).await
}
async fn remove(
    State(w): State<Arc<Watches>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    mutate(w, headers, &body, true).await
}
