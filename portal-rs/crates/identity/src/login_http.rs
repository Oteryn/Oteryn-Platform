//! Binary composition adapter: keeps identity independent of the web crate.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_identity::login::{self, Credentials};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, PrivilegeChange, SessionKey, SessionLayer},
};
use sqlx::PgPool;
use std::net::SocketAddr;

pub fn configured_router(pool: PgPool) -> Router {
    let enabled = std::env::var("OTERYN_ACCOUNT_LOGIN").is_ok_and(|v| v == "true");
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    router(pool, enabled, sessions)
}
fn router(pool: PgPool, enabled: bool, sessions: SessionLayer) -> Router {
    if !enabled {
        return Router::new();
    }
    let limiter = RateLimiter::new(pool.clone());
    let routes = Router::new()
        .route("/login", get(show).post(submit))
        .route("/pl/login", get(show).post(submit))
        .route("/en/login", get(show).post(submit))
        .route("/logout", post(logout))
        .route("/pl/logout", post(logout))
        .route("/en/logout", post(logout))
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(Extension(limiter))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024))
        .with_state(pool);
    oteryn_web::ratelimit::finish_routes(routes)
}
#[derive(Template)]
#[template(
    path = "account/login/page.html",
    config = "../../templates/account/login/askama.toml"
)]
struct LoginPage {
    lang: &'static str,
    language: Language,
    csrf: String,
    failed: bool,
}
impl LoginPage {
    fn text(&self, key: &str) -> String {
        if !key.starts_with("login-") && key != "logout-submit" {
            return self.language.text(key);
        }
        let source = if self.language == Language::En {
            include_str!("../../../templates/account/login/en.ftl")
        } else {
            include_str!("../../../templates/account/login/pl.ftl")
        };
        let resource = match FluentResource::try_new(source.to_owned()) {
            Ok(r) => r,
            Err(_) => return String::new(),
        };
        let Ok(locale) = self.lang.parse::<unic_langid::LanguageIdentifier>() else {
            return String::new();
        };
        let mut bundle = FluentBundle::new(vec![locale]);
        if bundle.add_resource(resource).is_err() {
            return String::new();
        }
        let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
            return String::new();
        };
        bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned()
    }
}
async fn page(
    language: Language,
    session: &BrowserSession,
    failed: bool,
) -> Result<Response, StatusCode> {
    let page = LoginPage {
        lang: language.code(),
        language,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        failed,
    };
    Ok(oteryn_web::render(&page))
}
async fn show(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_some() {
        return Ok(redirect(&format!("/{}/", language.code())));
    }
    let failed = session
        .get(LOGIN_FAILED)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .unwrap_or(false);
    if failed {
        session
            .set(LOGIN_FAILED, &false)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    page(language, &session, failed).await
}
async fn submit(
    State(pool): State<PgPool>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Credentials>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Ok(Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer, &headers) {
        Ok(ip) => ip,
        Err(e) => return Ok(e.response(language, false)),
    };
    let mut keys = Keys::source(source);
    if let Err(e) = limiter
        .check(NamedPolicy::IdentityLoginSource, &[keys.clone()])
        .await
    {
        return Ok(e.response(language, false));
    }
    let Ok(Form(input)) = form else {
        return refuse(language, &session).await;
    };
    if !input.valid() {
        return refuse(language, &session).await;
    }
    keys.email = Some(oteryn_identity::accounts::normalize_email(&input.email));
    if let Err(e) = limiter.check(NamedPolicy::IdentityLogin, &[keys]).await {
        return Ok(e.response(language, false));
    }
    if session.account_id().await.is_some() {
        return Ok(redirect(&format!("/{}/", language.code())));
    }
    let account = login::authenticate(&pool, input)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(id) = account {
        session
            .set(LOGIN_FAILED, &false)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        session::rotate(&session, PrivilegeChange::Login(id))
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let target = take_return(&session)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(redirect(
            &target.unwrap_or_else(|| format!("/{}/", language.code())),
        ))
    } else {
        refuse(language, &session).await
    }
}
async fn logout(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_none() {
        return Ok(redirect(&format!("/{}/login", language.code())));
    }
    session::rotate(&session, PrivilegeChange::Logout)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(redirect(&format!("/{}/", language.code())))
}

#[cfg(test)]
#[path = "login_http_tests.rs"]
mod tests;

const LOGIN_FAILED: SessionKey<bool> = SessionKey::new("login_failed");
const RETURN: SessionKey<Option<String>> = SessionKey::new(login::RETURN_KEY);
/// The stored post-login target, read and cleared once a login has succeeded.
/// Only the native OAuth authorize route and the exact coin checkout path are
/// accepted; anything else (absent, malformed, another origin) yields `None`.
pub(crate) async fn take_return(session: &BrowserSession) -> Result<Option<String>, sqlx::Error> {
    let Some(target) = session.get(RETURN).await?.flatten() else {
        return Ok(None);
    };
    session.set(RETURN, &None).await?;
    Ok(Some(target).filter(|t| accepted(t)))
}
fn accepted(target: &str) -> bool {
    login::authorize_return(target) || oteryn_payments::coin::is_checkout_return(target)
}
/// Reads the stored target before a session is replaced, so a challenge on
/// the new session can still return to it.
pub(crate) async fn pending_return(
    session: &BrowserSession,
) -> Result<Option<String>, sqlx::Error> {
    Ok(session.get(RETURN).await?.flatten().filter(|t| accepted(t)))
}
/// Stores a target kept by [`pending_return`] on the current session.
pub(crate) async fn keep_return(
    session: &BrowserSession,
    target: Option<String>,
) -> Result<(), sqlx::Error> {
    match target {
        Some(target) => session.set(RETURN, &Some(target)).await,
        None => Ok(()),
    }
}
// Laravel browser forms use a redirect and a generic flash error on refusal.
fn redirect(path: &str) -> Response {
    let mut response = Redirect::to(path).into_response();
    *response.status_mut() = StatusCode::FOUND;
    response
}
async fn refuse(language: Language, session: &BrowserSession) -> Result<Response, StatusCode> {
    session
        .set(LOGIN_FAILED, &true)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(redirect(&format!("/{}/login", language.code())))
}
