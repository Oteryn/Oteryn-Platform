use super::{Error, GROUP, PATH, Reader, wire};
use axum::{
    Extension, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::Value,
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// Mount with the exact GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY
/// in the purpose-specific identity map and the trusted TLS adapter. Missing or
/// overlapping identity configuration must fail closed through IdentityMap.
pub fn router(reader: Reader, identities: Arc<IdentityMap>) -> Router {
    let policy = RoutePolicy {
        enabled: reader.settings.enabled,
        group: GROUP.into(),
        max_body_bytes: 256,
        max_json_depth: 2,
        json: JsonRule::Object,
    };
    guard::protect(Router::new().route(PATH, post(read)), identities, policy)
        .with_state(reader)
        .layer(middleware::from_fn(transport))
}
async fn read(State(reader): State<Reader>, Extension(value): Extension<Value>) -> Response {
    let result = async {
        let operation = wire::request(&value)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?;
        let now = i64::try_from(now.as_secs()).map_err(|_| Error::Unavailable)?;
        reader.current(operation, now).await
    }
    .await;
    match result {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(error) => guard::failure(match error {
            Error::BadInput => StatusCode::BAD_REQUEST,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }),
    }
}
async fn transport(mut request: Request, next: Next) -> Response {
    // The reconciliation contract decodes JSON independently of media type.
    // Supply the shared guard's normalized type for absent or JSON charset headers.
    let types = request
        .headers()
        .get_all(header::CONTENT_TYPE)
        .iter()
        .count();
    if types == 0
        || (types == 1
            && request
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value
                        .split(';')
                        .next()
                        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
                }))
    {
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        response = guard::failure(StatusCode::BAD_REQUEST);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}
