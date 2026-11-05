//! Routers for T72's trusted listener/session assembly; never mount on a public ingest port.
use super::*;
use axum::{
    Router,
    body::to_bytes,
    extract::{Extension, Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Installed by T72 after Identity authentication and resolution of accepted current
/// source/epoch/recovery. No client-supplied stream fields are trusted by these routes.
#[derive(Debug, Clone)]
pub struct ReadContext {
    pub actor: Actor,
    pub streams: Vec<Stream>,
}
fn now() -> Result<u64, Failure> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .filter(|time| *time <= MAX_TIME)
        .ok_or(Failure::Unavailable)
}
fn response(result: Result<Vec<u8>, Failure>, json: bool) -> Response {
    match result {
        Ok(bytes) => (
            [
                (header::CACHE_CONTROL, "private, no-store"),
                (
                    header::CONTENT_TYPE,
                    if json {
                        "application/json"
                    } else {
                        "text/html; charset=utf-8"
                    },
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(failure) => (
            StatusCode::from_u16(failure.status()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
            [(header::CACHE_CONTROL, "private, no-store")],
        )
            .into_response(),
    }
}
/// Requires a purpose-bound authenticated Authority per listener, including T72's
/// trusted TLS metadata, partition binding, rate limits and accepted gate config.
/// Default flags and DenyAll answer 503. This router does not enable any flag.
pub fn internal_router<A: Authority + 'static>(repository: Deaths<A>) -> Router {
    Router::new()
        .route("/internal/v1/playerdata/progress/record", post(ingest::<A>))
        .route(
            "/internal/v1/playerdata/progress/watermark",
            post(watermark::<A>),
        )
        .with_state(Arc::new(repository))
}
async fn body(request: Request, cap: usize) -> Result<Vec<u8>, Failure> {
    let headers = request.headers();
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        != Some("application/json")
        || headers.contains_key(header::TRANSFER_ENCODING)
        || headers
            .get(header::CONTENT_ENCODING)
            .is_some_and(|h| h != "identity")
    {
        return Err(Failure::BadInput);
    }
    if let Some(length) = headers.get(header::CONTENT_LENGTH) {
        let length = length.to_str().map_err(|_| Failure::BadInput)?;
        if length.len() > 10 {
            return Err(Failure::BadInput);
        }
        let length = decimal(length, true, 9_999_999_999)?;
        if length > cap as u64 {
            return Err(Failure::TooLarge);
        }
    }
    to_bytes(request.into_body(), cap)
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|_| Failure::TooLarge)
}
async fn ingest<A: Authority>(
    State(repository): State<Arc<Deaths<A>>>,
    request: Request,
) -> Response {
    if !repository.ingest_enabled() {
        return response(Err(Failure::Unavailable), true);
    }
    let result = async {
        let bytes = body(request, 262144).await?;
        repository.ingest(&bytes, now()?).await
    }
    .await;
    response(result, true)
}
async fn watermark<A: Authority>(
    State(repository): State<Arc<Deaths<A>>>,
    request: Request,
) -> Response {
    if !repository.ingest_enabled() {
        return response(Err(Failure::Unavailable), true);
    }
    let result = async {
        let bytes = body(request, 1024).await?;
        repository.watermark(&bytes, now()?).await
    }
    .await;
    response(result, true)
}
/// Mount only after the trusted session and ReadContext middleware. Missing context
/// and staff identities deny, with no redirect or private data in an error body.
pub fn owner_router<A: Authority + 'static>(repository: Deaths<A>) -> Router {
    Router::new()
        .route(
            "/{locale}/account/journal/{character}/deaths",
            get(page::<A>),
        )
        .with_state(Arc::new(repository))
}
async fn page<A: Authority>(
    State(repository): State<Arc<Deaths<A>>>,
    Path((locale, character)): Path<(String, String)>,
    context: Option<Extension<ReadContext>>,
) -> Response {
    let result = async {
        let language = match locale.as_str() { "pl" => Language::Polish, "en" => Language::English, _ => return Err(Failure::BadInput) };
        let character = id(&character)?;
        let context = context.ok_or(Failure::Denied)?.0;
        let Actor::Owner(owner) = context.actor else { return Err(Failure::Denied); };
        let now = now()?;
        let mut combined = Projection { stale:false, records:vec![] };
        for family in [Family::DeathRecap,Family::Pvp,Family::Skull] {
            let mut matching = context.streams.iter().filter(|s| s.family==family);
            let stream=matching.next().ok_or(Failure::Unavailable)?;
            if matching.next().is_some() || id(&stream.account_id)? != owner { return Err(Failure::Denied); }
            let projection=repository.read(context.actor,stream,character,now).await?;
            combined.stale |= projection.stale;
            combined.records.extend(projection.records);
        }
        let fragment = render(&combined,language)?;
        Ok(format!("<!doctype html><html lang=\"{locale}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><main>{fragment}</main></html>",page_title(language)?).into_bytes())
    }.await;
    response(result, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    #[tokio::test]
    async fn endpoints_default_off_and_deny_missing_session() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@localhost/unused")
            .unwrap();
        let router = internal_router(Deaths::new(pool.clone(), Flags::default(), DenyAll));
        for path in ["record", "watermark"] {
            let response = router
                .clone()
                .oneshot(
                    Request::post(format!("/internal/v1/playerdata/progress/{path}"))
                        .body(Body::from("invalid"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "private, no-store"
            );
            assert!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        let router = owner_router(Deaths::new(pool, Flags::default(), DenyAll));
        let response = router
            .oneshot(
                Request::get("/en/account/journal/01934f10-7c04-7001-805b-3b1122334402/deaths")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
