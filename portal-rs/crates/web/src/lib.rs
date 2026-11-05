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
    let mut oauth_sessions = session_layer.clone();
    oauth_sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let oauth_routes = oteryn_gameauth::oauth::browser_routes(
        state.db.clone(),
        oteryn_gameauth::oauth::Config::from_env(),
    )
    .route_layer(axum::middleware::from_fn(
        |mut request: axum::extract::Request, next: axum::middleware::Next| async move {
            use axum::response::IntoResponse;
            let Some(session) = request
                .extensions()
                .get::<session::BrowserSession>()
                .cloned()
            else {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            };
            let language = request
                .headers()
                .get(axum::http::header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .filter(|v| v.trim().split(['-', ';']).next() == Some("en"))
                .map_or("pl", |_| "en")
                .to_owned();
            let pending = session::SessionKey::<Option<String>>::new("oauth_return");
            if request.method() == axum::http::Method::GET
                && request.uri().to_string().len() <= 8192
            {
                let target = if session.account_id().await.is_none() {
                    Some(request.uri().to_string())
                } else {
                    None
                };
                if session.set(pending, &target).await.is_err() {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
            }
            let csrf = match session.csrf_token().await {
                Ok(v) => v,
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            };
            request
                .extensions_mut()
                .insert(oteryn_gameauth::oauth::Browser {
                    account: session.account_id().await,
                    csrf,
                    language,
                });
            next.run(request).await
        },
    ))
    .route_layer(axum::middleware::from_fn_with_state(
        oauth_sessions,
        session::middleware,
    ))
    .merge(oteryn_gameauth::oauth::token_routes(
        state.db.clone(),
        oteryn_gameauth::oauth::Config::from_env(),
    ))
    .route_layer(axum::middleware::from_fn_with_state(
        state.db.clone(),
        |State(pool): State<PgPool>,
         request: axum::extract::Request,
         next: axum::middleware::Next| async move {
            if matches!(request.uri().path(), "/oauth/authorize" | "/oauth/token") {
                if !oteryn_gameauth::oauth::Config::from_env().enabled {
                    return StatusCode::NOT_FOUND.into_response();
                }
                let limiter = ratelimit::RateLimiter::new(pool.clone());
                let Some(peer) = request
                    .extensions()
                    .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                else {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                };
                let source = match limiter.trusted_proxies.client_ip(peer.0, request.headers()) {
                    Ok(v) => v,
                    Err(e) => return e.response(Language::Pl, true),
                };
                if let Err(e) = limiter
                    .check(
                        ratelimit::NamedPolicy::IdentityLoginSource,
                        &[ratelimit::Keys::source(source)],
                    )
                    .await
                {
                    if oteryn_gameauth::oauth::audit(&pool, None, "oauth_rate_refused")
                        .await
                        .is_err()
                    {
                        return StatusCode::SERVICE_UNAVAILABLE.into_response();
                    }
                    return e.response(Language::Pl, true);
                }
            }
            let response = next.run(request).await;
            if (response.status().is_client_error() || response.status().is_server_error())
                && oteryn_gameauth::oauth::audit(&pool, None, "oauth_http_refused")
                    .await
                    .is_err()
            {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            response
        },
    ))
    .route_layer(axum::middleware::from_fn(
        oteryn_gameauth::oauth::secure_response,
    ));
    let router = Router::new()
        .route("/", get(home))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest_service("/static", ServeDir::new(static_dir))
        .merge(surface::module_router())
        .merge(surface::module_routes())
        .merge(oteryn_identity::abuse::http::routes(state.db.clone()))
        .merge(register::routes())
        .merge(verify::routes())
        .route(
            "/account",
            get(
                |State(state): State<AppState>,
                 axum::Extension(session): axum::Extension<session::BrowserSession>,
                 axum::Extension(lang): axum::Extension<Language>| async move {
                    oteryn_characters::overview::show(
                        &state.db,
                        session.account_id().await,
                        lang.code(),
                        std::env::var("OTERYN_ACCOUNT_OVERVIEW_ENABLED").as_deref() == Ok("true"),
                        oteryn_characters::overview::configured_projection(),
                    )
                    .await
                },
            ),
        )
        .route(
            "/account/badges",
            get(
                |State(state): State<AppState>,
                 axum::Extension(session): axum::Extension<session::BrowserSession>,
                 axum::Extension(lang): axum::Extension<Language>| async move {
                    oteryn_characters::badges::http::show(
                        &oteryn_characters::badges::http::configured(state.db),
                        session.account_id().await,
                        lang.code(),
                        session.csrf_token().await,
                    )
                    .await
                },
            )
            .post(
                |State(state): State<AppState>,
                 axum::Extension(session): axum::Extension<session::BrowserSession>,
                 axum::Extension(lang): axum::Extension<Language>,
                 axum::Form(fields): axum::Form<Vec<(String, String)>>| async move {
                    oteryn_characters::badges::http::save(
                        &oteryn_characters::badges::http::configured(state.db),
                        session.account_id().await,
                        lang.code(),
                        fields,
                    )
                    .await
                },
            ),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.db.clone(),
            oteryn_identity::abuse::http::protect,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            session_layer,
            session::middleware,
        ))
        .method_not_allowed_fallback(|| async { StatusCode::METHOD_NOT_ALLOWED })
        .fallback(|| async { StatusCode::NOT_FOUND })
        .merge(oteryn_gameauth::runtime::routes(state.db.clone()))
        .merge(oteryn_gameauth::scopes::routes(state.db.clone()))
        .merge(oteryn_gameauth::devices::routes(state.db.clone()))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http());
    let router = router.layer(axum::middleware::from_fn(verify::protect_token));
    security_headers(router)
        // OAuth keeps its protocol envelopes outside the site error renderer.
        .merge(oauth_routes)
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
