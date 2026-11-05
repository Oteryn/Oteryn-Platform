//! Private SSR composition. No game-wide revocation capability is available.
use super::{Error, Flags, Logins, UnavailableAuthority};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::{
    Language,
    session::{self, BrowserSession, SessionLayer},
};
use sqlx::PgPool;
use std::sync::Arc;

pub fn configured_router(pool: PgPool) -> Router {
    let enabled = |name| std::env::var(name).is_ok_and(|v| v == "true");
    let service = Logins {
        pool: pool.clone(),
        flags: Flags {
            progress_ingest_enabled: false,
            progress_read_enabled: enabled("OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED"),
            game_login_history_enabled: enabled("OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED"),
        },
        // Replaced only by T72's accepted ownership/baseline adapter. Environment
        // flags alone cannot turn journal telemetry into authority.
        authority: Arc::new(UnavailableAuthority),
    };
    let mut layer = SessionLayer::new(pool);
    layer.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    router(service, layer)
}
pub fn router(service: Logins, layer: SessionLayer) -> Router {
    let mut routes = Router::new();
    for prefix in ["", "/pl", "/en"] {
        routes = routes.route(&format!("{prefix}/account/game-logins"), get(show));
    }
    let routes = routes
        .route_layer(axum::middleware::from_fn_with_state(
            layer,
            session::middleware,
        ))
        .with_state(service);
    oteryn_web::ratelimit::finish_routes(routes).layer(axum::middleware::from_fn(private_policy))
}
async fn private_policy(request: Request, next: Next) -> Response {
    private(next.run(request).await)
}
fn private(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><main><h1>{{ title }}</h1><p>{{ status }}</p><ol>{% for entry in entries %}<li><p>{{ entry }}</p></li>{% endfor %}</ol><details><summary>{{ not_me }}</summary><p>{{ security }}</p><p>{{ password }}</p></details></main></body></html>",
    ext = "html"
)]
struct Page {
    lang: &'static str,
    title: String,
    status: String,
    entries: Vec<String>,
    not_me: String,
    security: String,
    password: String,
}
fn text(language: Language, key: &str) -> Result<String, StatusCode> {
    let source = if language == Language::En {
        include_str!("../logins_en.ftl")
    } else {
        include_str!("../logins_pl.ftl")
    };
    let resource =
        FluentResource::try_new(source.into()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let locale = language
        .code()
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(bundle
        .format_pattern(pattern, None, &mut Vec::new())
        .into_owned())
}
fn date(ms: i64) -> Result<String, StatusCode> {
    sqlx::types::time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(|value| value.to_string())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
async fn show(
    State(service): State<Logins>,
    Extension(language): Extension<Language>,
    session: Option<Extension<BrowserSession>>,
) -> Response {
    let Some(Extension(session)) = session else {
        return private(StatusCode::FORBIDDEN);
    };
    let Some(actor) = session.account_id().await else {
        return private(StatusCode::FORBIDDEN);
    };
    let result = async {
        let entries = service.history(actor).await.map_err(|e| match e {
            Error::Denied => StatusCode::FORBIDDEN,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
        let mut lines = Vec::new();
        let mut stale = false;
        for entry in entries {
            stale |= entry.stale;
            let continuity = if entry.left_at.is_some() {
                "logins-closed"
            } else {
                "logins-unknown"
            };
            let entered = date(entry.entered_at)?;
            let left = entry
                .left_at
                .map(date)
                .transpose()?
                .unwrap_or(text(language, continuity)?);
            let character = entry
                .character
                .map_or(text(language, "logins-account")?, |id| id.to_string());
            lines.push(format!(
                "{}: {}. {}: {}. {}: {}",
                text(language, "logins-character")?,
                character,
                text(language, "logins-entered")?,
                entered,
                text(language, "logins-exit")?,
                left
            ));
        }
        let page = Page {
            lang: language.code(),
            title: text(language, "logins-title")?,
            status: text(
                language,
                if stale {
                    "logins-stale"
                } else if lines.is_empty() {
                    "logins-empty"
                } else {
                    "logins-retention"
                },
            )?,
            entries: lines,
            not_me: text(language, "logins-not-me")?,
            security: text(language, "logins-security")?,
            password: text(language, "logins-password")?,
        };
        page.render()
            .map(Html)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    }
    .await;
    match result {
        Ok(html) => private(html),
        Err(status) => private(status),
    }
}
