//! HTTP router, layout and cross-cutting web concerns for the Oteryn portal.

use askama::Template;
use axum::{
    Router,
    extract::State,
    http::{HeaderName, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    limit::RequestBodyLimitLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

/// Largest request body accepted by the public surface.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Shared state for every handler. Modules add their own state through extension traits.
#[derive(Debug, Clone)]
pub struct AppState {
    /// PostgreSQL pool.
    pub db: PgPool,
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomePage<'a> {
    lang: &'a str,
    tagline: &'a str,
}

/// Renders a template, answering 500 without leaking details when rendering fails.
pub fn render(template: &impl Template) -> Response {
    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(error) => {
            tracing::error!(%error, "template rendering failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn home() -> Response {
    render(&HomePage {
        lang: "pl",
        tagline: "Portal w budowie.",
    })
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> StatusCode {
    match oteryn_db::ping(&state.db).await {
        Ok(()) => StatusCode::OK,
        Err(error) => {
            tracing::warn!(%error, "database not ready");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

fn security_headers(router: Router) -> Router {
    let router = surface::finish(router);
    let router = router.layer(axum::middleware::from_fn(ratelimit::api_errors));
    let headers: [(HeaderName, &'static str); 5] = [
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains",
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
        (
            HeaderName::from_static("permissions-policy"),
            "camera=(), microphone=(), geolocation=()",
        ),
    ];
    headers.into_iter().fold(router, |router, (name, value)| {
        router.layer(SetResponseHeaderLayer::if_not_present(
            name,
            HeaderValue::from_static(value),
        ))
    })
}

/// Builds the public router. Module routers are merged here as their tasks land.
pub fn router(state: AppState, static_dir: &str) -> Router {
    let session_layer = session::SessionLayer::new(state.db.clone());
    let router = Router::new()
        .route("/", get(home))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest_service("/static", ServeDir::new(static_dir))
        .merge(surface::module_router())
        .merge(surface::module_routes())
        .merge(register::routes())
        .merge(verify::routes())
        .route_layer(axum::middleware::from_fn_with_state(
            session_layer,
            session::middleware,
        ))
        .method_not_allowed_fallback(|| async { StatusCode::METHOD_NOT_ALLOWED })
        .merge(oteryn_gameauth::runtime::routes(state.db.clone()))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http());
    let router = router.layer(axum::middleware::from_fn(verify::protect_token));
    security_headers(router)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    fn app() -> Result<Router, sqlx::Error> {
        let db = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
        Ok(router(AppState { db }, "static"))
    }

    #[tokio::test]
    async fn health_answers_ok_with_security_headers() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()?
            .oneshot(Request::get("/healthz").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
        assert!(
            response
                .headers()
                .contains_key(header::CONTENT_SECURITY_POLICY)
        );
        Ok(())
    }

    #[tokio::test]
    async fn home_renders_the_layout() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()?
            .oneshot(Request::get("/").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers()[header::LOCATION].to_str()?;
        let response = app()?
            .oneshot(Request::get(location).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), MAX_BODY_BYTES).await?;
        let html = String::from_utf8(body.to_vec())?;
        assert!(html.contains("<html lang=\"pl\">"));
        assert!(html.contains("Portal w budowie."));
        Ok(())
    }

    #[tokio::test]
    async fn unknown_route_is_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let response = app()?
            .oneshot(Request::get("/missing").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }
}

mod surface;

pub use surface::{CspNonce, Language, WebOptions, module_router, router_with_options};

/// Dedicated internal listener surface; the TLS adapter supplies verified peers.
pub fn internal_character_router(
    projection: oteryn_characters::lcfa::Projection,
    identities: std::sync::Arc<oteryn_internal_api::guard::IdentityMap>,
) -> Router {
    oteryn_characters::lcfa::router(projection, identities)
}

pub mod csrf;
pub mod session;

pub mod ratelimit;
pub mod redirects;
#[path = "routes/register.rs"]
pub mod register;
pub mod seo;
#[path = "routes/verify.rs"]
pub mod verify;
