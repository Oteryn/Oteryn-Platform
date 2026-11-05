//! Browser composition adapter; identity does not depend on the web crate.
use axum::{
    Extension, Json, Router,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use oteryn_identity::passkeys::{Completion, Error, Passkeys};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, PrivilegeChange, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use uuid::Uuid;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    if !std::env::var("OTERYN_ACCOUNT_PASSKEYS").is_ok_and(|v| v == "true") {
        return Ok(Router::new());
    }
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")?;
    let rp = std::env::var("OTERYN_WEBAUTHN_RP_ID")?;
    let service =
        Passkeys::new(pool.clone(), &rp, &origin).map_err(|_| "invalid WebAuthn configuration")?;
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = vec![origin];
    Ok(router(service, sessions))
}
fn router(service: Passkeys, sessions: SessionLayer) -> Router {
    let limiter = RateLimiter::new(service.pool.clone());
    let routes = Router::new()
        .route("/{lang}/account/passkeys", get(page))
        .route(
            "/{lang}/account/passkeys/register/start",
            post(register_start),
        )
        .route(
            "/{lang}/account/passkeys/register/finish",
            post(register_finish),
        )
        .route(
            "/{lang}/account/passkeys/authenticate/start",
            post(authenticate_start),
        )
        .route(
            "/{lang}/account/passkeys/authenticate/finish",
            post(authenticate_finish),
        )
        .route("/{lang}/account/passkeys/{id}/remove", post(remove))
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(Extension(limiter))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(32 * 1024))
        .with_state(service.clone());
    oteryn_web::ratelimit::finish_routes(routes)
        .layer(axum::middleware::from_fn_with_state(service, headers))
}
fn language(lang: &str) -> Result<Language, StatusCode> {
    match lang {
        "pl" => Ok(Language::Pl),
        "en" => Ok(Language::En),
        _ => Err(StatusCode::NOT_FOUND),
    }
}
#[derive(askama::Template)]
#[template(
    path = "account/passkeys/page.html",
    config = "../../templates/account/passkeys/askama.toml"
)]
struct Page {
    lang: &'static str,
    csrf: String,
    account: bool,
    ids: Vec<Uuid>,
}
impl Page {
    fn text(&self, key: &str) -> String {
        if !key.starts_with("passkeys-") {
            return if self.lang == "en" {
                Language::En
            } else {
                Language::Pl
            }
            .text(key);
        }
        use fluent_bundle::{FluentBundle, FluentResource};
        let source = if self.lang == "en" {
            include_str!("../../../templates/account/passkeys/en.ftl")
        } else {
            include_str!("../../../templates/account/passkeys/pl.ftl")
        };
        let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
            return String::new();
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
    State(service): State<Passkeys>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let lang = language(&lang)?.code();
    let account = session.account_id().await;
    let ids = if let Some(id) = account {
        service.list(id).await.map_err(|_| StatusCode::FORBIDDEN)?
    } else {
        Vec::new()
    };
    let page = Page {
        lang,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        account: account.is_some(),
        ids,
    };
    let mut response = oteryn_web::render(&page);
    response.headers_mut().insert(
        "cache-control",
        "no-store"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
/// Both absent peer information and untrusted proxy state deny before any ceremony work.
async fn charge(
    service: &Passkeys,
    limiter: &RateLimiter,
    session: &BrowserSession,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
    lang: Language,
) -> Result<(), Response> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Err(Denial::Unavailable.response(lang, true));
    };
    let ip = limiter
        .trusted_proxies
        .client_ip(peer, headers)
        .map_err(|e| e.response(lang, true))?;
    let mut keys = Keys::source(ip);
    limiter
        .check(NamedPolicy::IdentityLoginSource, &[keys.clone()])
        .await
        .map_err(|e| e.response(lang, true))?;
    if let Some(account) = session.account_id().await {
        keys.account = Some(account.to_string());
        limiter
            .check(NamedPolicy::IdentitySecurityMutation, &[keys])
            .await
            .map_err(|e| e.response(lang, true))?;
    }
    service
        .audit(session.account_id().await, "passkey.attempt")
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())?;
    Ok(())
}
async fn refused(
    service: &Passkeys,
    session: &BrowserSession,
    result: Result<serde_json::Value, Error>,
) -> Response {
    match result {
        Ok(value) => {
            let mut response = Json(value).into_response();
            response.headers_mut().insert(
                "cache-control",
                axum::http::HeaderValue::from_static("no-store"),
            );
            response
        }
        Err(error) => {
            if service
                .audit(session.account_id().await, "passkey.refused")
                .await
                .is_err()
            {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            match error {
                Error::Refused => StatusCode::FORBIDDEN.into_response(),
                Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            }
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Password {
    password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn confirm(
    service: &Passkeys,
    session: &BrowserSession,
    input: Password,
) -> Result<Uuid, Error> {
    let account = session.account_id().await.ok_or(Error::Refused)?;
    service.confirm_password(account, input.password).await?;
    Ok(account)
}
// Each endpoint charges the shared limiter before credential verification.
macro_rules! endpoint {
    ($name:ident, $body:ident, $payload:ty) => {
        async fn $name(
            State(service): State<Passkeys>,
            Path(path): Path<std::collections::HashMap<String, String>>,
            Extension(session): Extension<BrowserSession>,
            Extension(limiter): Extension<RateLimiter>,
            peer: Option<Extension<ConnectInfo<SocketAddr>>>,
            headers: HeaderMap,
            payload: Result<Json<$payload>, axum::extract::rejection::JsonRejection>,
        ) -> Response {
            let Ok(lang) = language(path.get("lang").map_or("", String::as_str)) else {
                return StatusCode::NOT_FOUND.into_response();
            };
            if let Err(response) = charge(&service, &limiter, &session, peer, &headers, lang).await
            {
                return response;
            }
            let result = async {
                let Json(input) = payload.map_err(|_| Error::Refused)?;
                let binding = session.csrf_token().await?;
                $body(&service, &session, &binding, input, &path).await
            }
            .await;
            refused(&service, &session, result).await
        }
    };
}
async fn begin_register(
    service: &Passkeys,
    session: &BrowserSession,
    binding: &str,
    input: Password,
    _: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    let account = confirm(service, session, input).await?;
    serde_json::to_value(service.begin_registration(binding, account).await?)
        .map_err(|_| Error::Refused)
}
async fn end_register(
    service: &Passkeys,
    session: &BrowserSession,
    binding: &str,
    input: Completion,
    _: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    let account = session.account_id().await.ok_or(Error::Refused)?;
    service.finish_registration(binding, account, input).await?;
    session::rotate(session, PrivilegeChange::Logout).await?;
    Ok(serde_json::json!({"ok":true}))
}
async fn begin_authenticate(
    service: &Passkeys,
    session: &BrowserSession,
    binding: &str,
    _: Empty,
    _: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    if session.account_id().await.is_some() {
        return Err(Error::Refused);
    }
    serde_json::to_value(service.begin_authentication(binding).await?).map_err(|_| Error::Refused)
}
async fn end_authenticate(
    service: &Passkeys,
    session: &BrowserSession,
    binding: &str,
    input: Completion,
    _: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    if session.account_id().await.is_some() {
        return Err(Error::Refused);
    }
    let account = service.finish_authentication(binding, input).await?;
    session::rotate(session, PrivilegeChange::Login(account)).await?;
    Ok(serde_json::json!({"ok":true}))
}
async fn delete_key(
    service: &Passkeys,
    session: &BrowserSession,
    _: &str,
    input: Password,
    path: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    let id = path
        .get("id")
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or(Error::Refused)?;
    let account = confirm(service, session, input).await?;
    service.remove(account, id).await?;
    session::rotate(session, PrivilegeChange::Logout).await?;
    Ok(serde_json::json!({"ok":true}))
}
endpoint!(register_start, begin_register, Password);
endpoint!(register_finish, end_register, Completion);
endpoint!(authenticate_start, begin_authenticate, Empty);
endpoint!(authenticate_finish, end_authenticate, Completion);
endpoint!(remove, delete_key, Password);

async fn headers(
    State(service): State<Passkeys>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    if matches!(response.status().as_u16(), 400 | 403 | 413 | 422 | 429)
        && service.audit(None, "passkey.http_refused").await.is_err()
    {
        response = StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        ("cache-control", "no-store"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}

#[cfg(test)]
#[path = "passkeys_http_tests.rs"]
mod tests;
