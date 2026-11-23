use super::{Actor, Flags, Source, Unavailable, render};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::accounts::Accounts;
use oteryn_ui::Chrome;
use oteryn_web::session::{SessionLayer, SignedInSession, read_only_middleware};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct HttpState {
    flags: Flags,
    source: Arc<dyn Source>,
}

/// The deployed view uses the browser's existing session. Game data requires a
/// separately qualified source and stays unavailable until one is composed.
pub fn configured_router(pool: PgPool) -> Router {
    routes(Flags::default(), Arc::new(Unavailable))
        .layer(middleware::from_fn_with_state(pool.clone(), owner_session))
        .layer(middleware::from_fn_with_state(
            SessionLayer::new(pool),
            read_only_middleware,
        ))
        .layer(middleware::from_fn(private_policy))
}

/// The host must provide any Actor from a trusted session adapter.
pub fn router(flags: Flags, source: Arc<dyn Source>) -> Router {
    routes(flags, source).layer(middleware::from_fn(private_policy))
}

fn routes(flags: Flags, source: Arc<dyn Source>) -> Router {
    Router::new()
        .route("/{language}/account/dashboard", get(page))
        .with_state(HttpState { flags, source })
}

async fn owner_session(State(pool): State<PgPool>, mut request: Request, next: Next) -> Response {
    if let Some(session) = request.extensions().get::<SignedInSession>() {
        let owner = session.account_id;
        let account = match (Accounts { pool }).find_by_id(owner).await {
            Ok(Some(account))
                if account.status == "active" && account.email_verified_at.is_some() =>
            {
                account
            }
            Ok(_) => return StatusCode::FORBIDDEN.into_response(),
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        request.extensions_mut().insert(Actor {
            account: account.id,
            active: true,
            staff_access: false,
        });
    }
    next.run(request).await
}

#[derive(Template)]
#[template(
    config = "../ui/askama.toml",
    path = "playerdata/dashboard_unavailable.html"
)]
struct UnavailablePage {
    chrome: Chrome,
}

async fn page(
    State(state): State<HttpState>,
    Path(language): Path<String>,
    actor: Option<Extension<Actor>>,
    chrome: Option<Extension<Chrome>>,
) -> Response {
    if !matches!(language.as_str(), "pl" | "en") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(Extension(actor)) = actor else {
        return if chrome.is_some() {
            Redirect::to(&format!("/{language}/login")).into_response()
        } else {
            StatusCode::UNAUTHORIZED.into_response()
        };
    };
    if actor.authorize(actor.account).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if state.flags.dashboard {
        let now = oteryn_db::outbox::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        if let Some(html) = i64::try_from(now).ok().and_then(|now| {
            render(
                state.flags,
                actor,
                actor.account,
                state.source.as_ref(),
                now,
                &language,
            )
            .ok()
        }) {
            return Html(format!("<!doctype html><html lang=\"{language}\"><head><meta charset=\"utf-8\"><title>Oteryn</title><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/surface.css\"></head><body><main>{html}</main></body></html>")).into_response();
        }
    }
    let chrome = chrome.map_or_else(
        || Chrome {
            lang: language,
            ..Chrome::default()
        },
        |Extension(chrome)| chrome,
    );
    match (UnavailablePage { chrome }).render() {
        Ok(html) => (StatusCode::SERVICE_UNAVAILABLE, Html(html)).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn private_policy(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        header::HeaderValue::from_static(oteryn_web::chrome::PORTAL_CONTENT_SECURITY_POLICY),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response
}
