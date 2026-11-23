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

struct Api {
    pool: PgPool,
    qualified: Option<Watches>,
}

pub fn router(watches: Watches) -> Router {
    if !watches.enabled {
        return Router::new();
    }
    routes(Api {
        pool: watches.pool.clone(),
        qualified: Some(watches),
    })
}

fn routes(api: Api) -> Router {
    Router::new()
        .route("/api/v1/stats/watches", get(list).post(add).delete(remove))
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::from_fn(private))
        .with_state(Arc::new(api))
}

pub(super) async fn private(request: axum::extract::Request, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    headers.insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
    }
    response
}

pub(super) fn unavailable<E>(_: E) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

pub fn configured_router(pool: PgPool) -> Router {
    configured_with(pool, web::Readiness::configured())
}

pub(super) fn configured_with(pool: PgPool, config: web::Readiness) -> Router {
    let browser = web::router(pool.clone(), config);
    if config.watches {
        browser.merge(routes(Api {
            pool,
            qualified: None,
        }))
    } else {
        browser
    }
}

fn bearer(headers: &HeaderMap) -> std::result::Result<&str, StatusCode> {
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
    Ok(token)
}

async fn owner(api: &Api, token: &str) -> std::result::Result<Uuid, StatusCode> {
    let actor = match &api.qualified {
        Some(watches) => watches.policy.owner(token).await.map_err(unavailable)?,
        None => access::owner(&api.pool, token).await?,
    }
    .filter(|id| !id.is_nil())
    .ok_or(StatusCode::UNAUTHORIZED)?;
    budget(&api.pool, actor).await?;
    Ok(actor)
}

pub(super) async fn budget<'e>(
    db: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    owner: Uuid,
) -> std::result::Result<(), StatusCode> {
    let requests: i32 = sqlx::query_scalar("INSERT INTO stats_watch_budgets(account_id,requests) VALUES($1,1) ON CONFLICT(account_id) DO UPDATE SET requests=CASE WHEN stats_watch_budgets.window_start <= now()-interval '1 minute' THEN 1 ELSE LEAST(stats_watch_budgets.requests+1,61) END, window_start=CASE WHEN stats_watch_budgets.window_start <= now()-interval '1 minute' THEN now() ELSE stats_watch_budgets.window_start END,updated_at=now() RETURNING requests")
        .bind(owner).fetch_one(db).await.map_err(unavailable)?;
    if requests > 60 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(())
}

async fn list(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
) -> std::result::Result<Response, StatusCode> {
    let actor = owner(&api, bearer(&headers)?).await?;
    let watches = api
        .qualified
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(watches.list(actor).await.map_err(unavailable)?).into_response())
}

async fn mutate(
    api: Arc<Api>,
    headers: HeaderMap,
    body: &[u8],
    remove: bool,
) -> std::result::Result<StatusCode, StatusCode> {
    let token = bearer(&headers)?;
    let actor = owner(&api, token).await?;
    let subscription: Subscription =
        serde_json::from_slice(body).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if subscription.target_id.is_nil() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    match (&api.qualified, remove) {
        (Some(watches), true) => watches
            .unsubscribe(actor, subscription)
            .await
            .map_err(unavailable)?,
        (Some(watches), false) => watches
            .subscribe(actor, subscription)
            .await
            .map_err(unavailable)?,
        (None, true) => access::remove(&api.pool, actor, token, subscription).await?,
        (None, false) => return Err(StatusCode::SERVICE_UNAVAILABLE),
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn add(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> std::result::Result<StatusCode, StatusCode> {
    mutate(api, headers, &body, false).await
}
async fn remove(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> std::result::Result<StatusCode, StatusCode> {
    mutate(api, headers, &body, true).await
}
