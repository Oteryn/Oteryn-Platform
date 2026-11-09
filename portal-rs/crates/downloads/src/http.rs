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
    document(
        result,
        if filename
            .split('.')
            .next()
            .is_some_and(|v| v.parse::<u64>().is_ok())
        {
            IMMUTABLE
        } else {
            MUTABLE
        },
    )
}

async fn policy(State(downloads): State<Downloads>, Path(channel): Path<String>) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, None).await,
        Err(error) => Err(error),
    };
    document(result, MUTABLE)
}

async fn target(
    State(downloads): State<Downloads>,
    Path((channel, target)): Path<(String, String)>,
) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, Some(&target)).await,
        Err(error) => Err(error),
    };
    document(result, IMMUTABLE)
}

const IMMUTABLE: &str = "public, max-age=31536000, immutable, no-transform";
const MUTABLE: &str = "no-store, no-transform";

fn document(result: Result<Vec<u8>, Error>, cache: &'static str) -> Response {
    match result {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, cache),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_unavailable_or_refused_and_never_cached() {
        for (error, status) in [
            (Error::Disabled, StatusCode::NOT_FOUND),
            (Error::NotFound, StatusCode::NOT_FOUND),
            (Error::Invalid("channel"), StatusCode::BAD_REQUEST),
            (Error::Verification, StatusCode::SERVICE_UNAVAILABLE),
            (
                Error::Storage(sqlx::Error::PoolClosed),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let response = document(Err(error), IMMUTABLE);
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
    }
}
