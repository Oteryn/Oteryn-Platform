//! Serve the existing shared-shell brand mark for the browser's automatic icon request.
use axum::{Router, http::header, routing::get};

pub fn router() -> Router {
    Router::new().route("/favicon.ico", get(icon))
}

async fn icon() -> ([(header::HeaderName, &'static str); 4], String) {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; sandbox",
            ),
        ],
        include_str!("../../../templates/components/compass.html").replacen(
            "<svg ",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" ",
            1,
        ),
    )
}
