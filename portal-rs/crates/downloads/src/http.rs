use std::collections::BTreeMap;

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;

use crate::{Channel, Downloads, Error, FeatureFlags};

/// Static-like GET surface. Upload/publication is a Rust API for trusted staff composition only.
/// Trust roots are unnecessary here: only previously verified database publications can be read.
pub fn router(pool: PgPool, flags: FeatureFlags) -> Router {
    Router::new()
        .route("/updates/{channel}/metadata/{filename}", get(metadata))
        .route("/updates/{channel}/policy-v1.json", get(policy))
        .route("/updates/{channel}/targets/{*target}", get(target))
        .with_state(Downloads::new(pool, flags, BTreeMap::new()))
}

async fn metadata(
    State(downloads): State<Downloads>,
    Path((channel, filename)): Path<(String, String)>,
) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.metadata(channel, &filename).await,
        Err(error) => Err(error),
    };
    document(result)
}

async fn policy(State(downloads): State<Downloads>, Path(channel): Path<String>) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, None).await,
        Err(error) => Err(error),
    };
    document(result)
}

async fn target(
    State(downloads): State<Downloads>,
    Path((channel, target)): Path<(String, String)>,
) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, Some(&target)).await,
        Err(error) => Err(error),
    };
    document(result)
}

fn document(result: Result<Vec<u8>, Error>) -> Response {
    match result {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-cache, no-transform"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => {
            let status = match error {
                Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
                Error::Invalid(_) => StatusCode::BAD_REQUEST,
                Error::Denied => StatusCode::FORBIDDEN,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
        }
    }
}
