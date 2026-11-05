//! Proposed G-7 routes remain unavailable until version-2 CHAR_CMD result data
//! and capability advertisement are accepted. No alternate receipt route exists.
use axum::{
    Router,
    body::Body,
    http::{StatusCode, header},
    response::Response,
    routing::post,
};
pub fn router() -> Router {
    let mut router = Router::new();
    for path in [
        "commands/pending",
        "commands/read",
        "commands/reconcile",
        "transferability",
        "status",
        "watermark",
    ] {
        router = router.route(&format!("/internal/v1/bazaar/{path}"), post(unavailable));
    }
    router
}
async fn unavailable() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn proposed_routes_deny_before_body_or_peer_processing() {
        for path in [
            "commands/pending",
            "commands/read",
            "commands/reconcile",
            "transferability",
            "status",
            "watermark",
        ] {
            let request = axum::http::Request::post(format!("/internal/v1/bazaar/{path}"))
                .body(Body::from("bad input"))
                .unwrap();
            let response = router().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert!(
                axum::body::to_bytes(response.into_body(), 1)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
