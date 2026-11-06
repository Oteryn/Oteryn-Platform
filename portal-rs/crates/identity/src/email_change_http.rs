//! Browser adapter: GET renders only; every mutation uses the session CSRF layer.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::email_change::*;
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionKey, SessionLayer},
};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;
const FLASH: SessionKey<String> = SessionKey::new("email_change_status");
pub fn configured_router(pool: PgPool) -> Router {
    let enabled = std::env::var("OTERYN_ACCOUNT_EMAIL_CHANGE").as_deref() == Ok("true");
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN").unwrap_or_default();
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = vec![origin.clone()];
    let Ok(mfa) = oteryn_identity::mfa::Mfa::configured(pool.clone()) else {
        return Router::new();
    };
    router(
        Service {
            pool,
            enabled,
            origin,
            security: Arc::new(TransactionalSecurity(mfa)),
        },
        sessions,
    )
}
pub fn router(service: Service, sessions: SessionLayer) -> Router {
    if !service.ready() {
        return Router::new();
    }
    let mut routes = Router::new();
    for prefix in ["", "/pl", "/en"] {
        routes = routes
            .route(
                &format!("{prefix}/account/security/email"),
                get(show).post(request),
            )
            .route(
                &format!("{prefix}/email-change/confirm/{{token}}"),
                get(show_confirm).post(confirm),
            )
            .route(
                &format!("{prefix}/email-change/revert/{{token}}"),
                get(show_revert).post(revert),
            );
    }
    let limiter = RateLimiter::new(service.pool.clone());
    oteryn_web::ratelimit::finish_routes(
        routes
            .route_layer(axum::middleware::from_fn_with_state(
                sessions,
                session::middleware,
            ))
            .layer(Extension(limiter))
            .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024))
            .with_state(service),
    )
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"></head><body class=\"op-theme\"><main class=\"op-card op-stack\"><h1>{{ title }}</h1><p role=\"status\">{{ status }}</p><form class=\"op-stack\" method=\"post\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\">{% if request %}<p><label>{{ email }} <input class=\"op-input\" type=\"email\" name=\"email\" autocomplete=\"email\" maxlength=\"254\" required></label></p><p><label>{{ repeat }} <input class=\"op-input\" type=\"email\" name=\"email_confirmation\" maxlength=\"254\" required></label></p><p><label>{{ password }} <input class=\"op-input\" type=\"password\" name=\"current_password\" autocomplete=\"current-password\" maxlength=\"1024\" required></label></p><p><label>{{ proof }} <input class=\"op-input\" type=\"password\" name=\"proof\" maxlength=\"1024\" required></label></p><p><label><input type=\"checkbox\" name=\"recovery\" value=\"true\">{{ recovery }}</label></p><p><label><input type=\"checkbox\" name=\"backup\" value=\"true\">{{ backup }}</label></p>{% else %}<p>{{ explanation }}</p>{% endif %}<button class=\"op-button\" type=\"submit\">{{ submit }}</button></form></main></body></html>",
    ext = "html"
)]
struct Page {
    lang: String,
    title: String,
    status: String,
    csrf: String,
    request: bool,
    email: String,
    repeat: String,
    password: String,
    proof: String,
    recovery: String,
    backup: String,
    explanation: String,
    submit: String,
}
async fn page(
    language: Language,
    session: BrowserSession,
    kind: &str,
) -> Result<Response, StatusCode> {
    let text = |key: &str| text(language.code(), key).map_err(|_| StatusCode::SERVICE_UNAVAILABLE);
    let status = session
        .get(FLASH)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let status = match status {
        Some(key) if !key.is_empty() => text(&key)?,
        _ => String::new(),
    };
    session
        .set(FLASH, &String::new())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let p = Page {
        lang: language.code().into(),
        title: text("title")?,
        status,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        request: kind == "request",
        email: text("email")?,
        repeat: text("repeat")?,
        password: text("password")?,
        proof: text("proof")?,
        recovery: text("recovery")?,
        backup: text("backup")?,
        explanation: if kind == "request" {
            String::new()
        } else {
            text(kind)?
        },
        submit: text(if kind == "revert" { "revert" } else { "submit" })?,
    };
    let mut response = oteryn_web::render(&p);
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "referrer-policy",
        axum::http::HeaderValue::from_static("same-origin"),
    );
    Ok(response)
}
async fn show(
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    if s.account_id().await.is_none() {
        return Ok(redirect(&format!("/{}/login", l.code())));
    }
    page(l, s, "request").await
}
async fn show_confirm(
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(l, s, "confirm").await
}
async fn show_revert(
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(l, s, "revert").await
}
async fn limit(
    l: Language,
    limiter: &RateLimiter,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
    account: Option<Uuid>,
    token: Option<&str>,
) -> Result<(), Response> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Err(Denial::Unavailable.response(l, false));
    };
    let source = limiter
        .trusted_proxies
        .client_ip(peer, headers)
        .map_err(|e| e.response(l, false))?;
    let mut keys = Keys::source(source);
    keys.account = account.map(|id| id.to_string());
    keys.token = token.map(|v| format!("{:x}", sha2::Sha256::digest(v.as_bytes())));
    limiter
        .check(
            if token.is_some() {
                NamedPolicy::IdentityEmailToken
            } else {
                NamedPolicy::IdentityEmailChange
            },
            &[keys],
        )
        .await
        .map_err(|e| e.response(l, false))
}
fn redirect(path: &str) -> Response {
    let mut r = Redirect::to(path).into_response();
    *r.status_mut() = StatusCode::FOUND;
    r
}
async fn finish(
    l: Language,
    s: BrowserSession,
    outcome: Outcome,
    path: &str,
) -> Result<Response, StatusCode> {
    let key = match outcome {
        Outcome::Requested => "requested",
        Outcome::Confirmed => "confirmed",
        Outcome::Changed => "changed",
        Outcome::Cancelled => "cancelled",
        Outcome::Reverted => "reverted",
        Outcome::Refused => "failed",
    };
    // Changes revoke authenticated sessions. Do not write into a deleted session.
    if matches!(outcome, Outcome::Changed | Outcome::Reverted) {
        return Ok(redirect(&format!("/{}/login", l.code())));
    }
    s.set(FLASH, &key.to_owned())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(redirect(path))
}
async fn request(
    State(service): State<Service>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Request>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let Some(account) = s.account_id().await else {
        return Ok(redirect(&format!("/{}/login", l.code())));
    };
    if let Err(r) = limit(l, &limiter, peer, &headers, Some(account), None).await {
        return Ok(r);
    }
    let outcome = match form {
        Ok(Form(input)) => service
            .request(account, input)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        Err(_) => Outcome::Refused,
    };
    finish(
        l,
        s,
        outcome,
        &format!("/{}/account/security/email", l.code()),
    )
    .await
}
async fn token_action(
    service: Service,
    l: Language,
    s: BrowserSession,
    token: String,
    revert: bool,
) -> Result<Response, StatusCode> {
    let outcome = if revert {
        service.revert(&token).await
    } else {
        service.confirm(&token).await
    }
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    finish(
        l,
        s,
        outcome,
        &format!(
            "/{}/email-change/{}/{token}",
            l.code(),
            if revert { "revert" } else { "confirm" }
        ),
    )
    .await
}
macro_rules! token_handler {
    ($name:ident,$revert:expr) => {
        async fn $name(
            State(service): State<Service>,
            Extension(l): Extension<Language>,
            Extension(s): Extension<BrowserSession>,
            Extension(limiter): Extension<RateLimiter>,
            peer: Option<Extension<ConnectInfo<SocketAddr>>>,
            headers: HeaderMap,
            Path(token): Path<String>,
        ) -> Result<Response, StatusCode> {
            if let Err(r) = limit(l, &limiter, peer, &headers, None, Some(&token)).await {
                return Ok(r);
            }
            if !valid_token(&token) {
                return finish(
                    l,
                    s,
                    Outcome::Refused,
                    &format!(
                        "/{}/email-change/{}/invalid",
                        l.code(),
                        if $revert { "revert" } else { "confirm" }
                    ),
                )
                .await;
            }
            token_action(service, l, s, token, $revert).await
        }
    };
}
use sha2::Digest;
token_handler!(confirm, false);
token_handler!(revert, true);
#[cfg(test)]
#[path = "email_change_http_tests.rs"]
mod tests;
