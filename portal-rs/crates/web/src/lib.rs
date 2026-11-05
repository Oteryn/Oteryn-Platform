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
