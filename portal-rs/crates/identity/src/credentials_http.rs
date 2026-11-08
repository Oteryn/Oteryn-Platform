//! Binary composition adapter; identity does not depend on the web router.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_identity::{
    accounts::Accounts,
    credentials::{self, Credentials, Outcome, PasswordChange},
};
use oteryn_web::{
    Language,
    ratelimit::{Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, PrivilegeChange, SessionKey, SessionLayer},
};
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, sync::Arc};

#[derive(Debug)]
struct AccountState(Accounts);
impl oteryn_notify::AccountState for AccountState {
    fn active_verified(
        &self,
        id: uuid::Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async move {
            Ok(self
                .0
                .find_by_id(id)
                .await?
                .filter(|a| a.status == "active" && a.email_verified_at.is_some())
                .map(|a| oteryn_notify::Account {
                    email: a.email,
                    locale: if a.locale == "pl" {
                        oteryn_notify::Locale::Pl
                    } else {
                        oteryn_notify::Locale::En
                    },
                }))
        })
    }
}
pub fn configured_router(pool: sqlx::PgPool) -> Router {
    if !std::env::var("OTERYN_ACCOUNT_CREDENTIALS").is_ok_and(|v| v == "true") {
        return Router::new();
    }
    // Security messages never use unsubscribe links. Keep the optional service
    // unconfigured if its signing configuration cannot be constructed.
    let Ok(unsubscribe) =
        oteryn_notify::Unsubscribe::new(vec![0; 32], "https://localhost/unsubscribe".into())
    else {
        return Router::new();
    };
    let service = Credentials {
        pool: pool.clone(),
        enabled: true,
        revoker: Arc::new(credentials::TransactionalRevoker),
        notifications: Arc::new(oteryn_notify::Notifications {
            pool: pool.clone(),
            accounts: Arc::new(AccountState(Accounts { pool: pool.clone() })),
            unsubscribe,
        }),
    };
    router(service)
}
/// Supply the real T16/T21 revoker at composition once those modules are integrated.
pub fn router(service: Credentials) -> Router {
    if !service.enabled {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(service.pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let limiter = RateLimiter::new(service.pool.clone());
    let mut routes = Router::new();
    for prefix in ["", "/pl", "/en"] {
        routes = routes
            .route(
                &format!("{prefix}/password/change"),
                get(show_password).post(change).put(change),
            )
            .route(&format!("{prefix}/account/sudo"), get(show_sudo).post(sudo))
            .route(&format!("{prefix}/account/security"), get(security_landing));
    }
    let routes = routes
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(Extension(limiter))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(8192))
        .with_state(service);
    oteryn_web::ratelimit::finish_routes(routes)
}
async fn security_landing(Extension(lang): Extension<Language>) -> Redirect {
    Redirect::to(&format!("/{}/password/change", lang.code()))
}

#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"></head><body class=\"op-theme\"><main class=\"op-card op-stack\" id=\"main\"><h1>{{ title }}</h1>{% if failed %}<p role=\"alert\">{{ error }}</p>{% endif %}<form class=\"op-stack\" method=\"post\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><p><label for=\"current\">{{ current }}</label><br><input class=\"op-input\" id=\"current\" type=\"password\" name=\"current_password\" autocomplete=\"current-password\" maxlength=\"1024\" required></p>{% if change %}<p><label for=\"new\">{{ new }}</label><br><input class=\"op-input\" id=\"new\" type=\"password\" name=\"password\" autocomplete=\"new-password\" minlength=\"10\" maxlength=\"256\" required></p><p><label for=\"confirm\">{{ confirm }}</label><br><input class=\"op-input\" id=\"confirm\" type=\"password\" name=\"password_confirmation\" autocomplete=\"new-password\" required></p>{% endif %}<button class=\"op-button op-button--primary\" type=\"submit\">{{ submit }}</button></form></main></body></html>",
    ext = "html"
)]
struct Page {
    lang: &'static str,
    title: String,
    current: String,
    new: String,
    confirm: String,
    submit: String,
    csrf: String,
    change: bool,
    failed: bool,
    error: String,
}
fn text(lang: Language, key: &str) -> String {
    let source = if lang == Language::En {
        include_str!("credentials_en.ftl")
    } else {
        include_str!("credentials_pl.ftl")
    };
    let Ok(resource) = FluentResource::try_new(source.into()) else {
        return String::new();
    };
    let Ok(locale) = lang.code().parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut Vec::new()).into_owned())
        .unwrap_or_default()
}
async fn page(
    lang: Language,
    session: BrowserSession,
    change: bool,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_none() {
        return Ok(redirect("/login"));
    }
    let page = Page {
        lang: lang.code(),
        title: text(
            lang,
            if change {
                "password-title"
            } else {
                "sudo-title"
            },
        ),
        current: text(lang, "current"),
        new: text(lang, "new"),
        confirm: text(lang, "confirm"),
        submit: text(lang, "submit"),
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        change,
        failed: session
            .get(FAILED)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .unwrap_or(false),
        error: text(lang, "refused"),
    };
    session
        .set(FAILED, &false)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(axum::response::Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response())
}
async fn show_password(
    Extension(lang): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(lang, session, true).await
}
async fn show_sudo(
    Extension(lang): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(lang, session, false).await
}
fn session_hash(headers: &HeaderMap) -> Option<Vec<u8>> {
    let mut values = headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|h| h.trim().split_once('='))
        .filter(|(k, _)| *k == session::COOKIE_NAME)
        .map(|(_, v)| v);
    let value = values.next()?;
    if values.next().is_some() || value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some(Sha256::digest(value.as_bytes()).to_vec())
}
#[derive(serde::Deserialize)]
struct Input {
    current_password: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    password_confirmation: String,
}
async fn check(
    limiter: &RateLimiter,
    session: &BrowserSession,
    peer: SocketAddr,
    policy: NamedPolicy,
) -> Result<(), StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::UNAUTHORIZED)?;
    let mut keys = Keys::source(peer.ip());
    keys.account = Some(id.to_string());
    limiter.check(policy, &[keys]).await.map_err(|e| e.status())
}
async fn change(
    Extension(lang): Extension<Language>,
    State(service): State<Credentials>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    check(
        &limiter,
        &session,
        peer,
        NamedPolicy::IdentityPasswordChange,
    )
    .await?;
    let hash = session_hash(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    match service
        .change(
            &hash,
            PasswordChange {
                current_password: input.current_password,
                password: input.password,
                password_confirmation: input.password_confirmation,
            },
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        Outcome::Changed => {
            session::rotate(&session, PrivilegeChange::Logout)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(redirect("/login"))
        }
        Outcome::Refused | Outcome::Invalid => refuse(lang, &session, "password/change").await,
    }
}
async fn sudo(
    Extension(lang): Extension<Language>,
    State(service): State<Credentials>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    check(
        &limiter,
        &session,
        peer,
        NamedPolicy::IdentitySecurityMutation,
    )
    .await?;
    let hash = session_hash(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let mut tx = service
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let Some((id, password)) = credentials::authenticated(&mut tx, &hash)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if !credentials::verify(input.current_password, password)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return refuse(lang, &session, "account/sudo").await;
    }
    let until = sqlx::types::time::OffsetDateTime::now_utc() + std::time::Duration::from_secs(900);
    sqlx::query(
        "INSERT INTO credential_security_events(account_id,kind) VALUES($1,'sudo_confirmed')",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    session::rotate(&session, PrivilegeChange::Sudo(until))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if tx.commit().await.is_err() {
        session::rotate(&session, PrivilegeChange::Logout)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(redirect(&format!("/{}/account/security", lang.code())))
}

#[cfg(test)]
#[path = "credentials_http_tests.rs"]
mod tests;

const FAILED: SessionKey<bool> = SessionKey::new("credentials_failed");
fn redirect(path: &str) -> Response {
    let mut response = Redirect::to(path).into_response();
    *response.status_mut() = StatusCode::FOUND;
    response
}
async fn refuse(
    lang: Language,
    session: &BrowserSession,
    path: &str,
) -> Result<Response, StatusCode> {
    session
        .set(FAILED, &true)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(redirect(&format!("/{}/{path}", lang.code())))
}
