//! Browser composition adapter; session middleware owns CSRF validation.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::reset::{self, DeliveryKey, Input};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionKey, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::SocketAddr;

pub fn configured_router(pool: PgPool) -> Router {
    if !std::env::var("OTERYN_ACCOUNT_PASSWORD_RESET").is_ok_and(|v| v == "true") {
        return Router::new();
    }
    let Some(key) = std::env::var("OTERYN_RESET_DELIVERY_KEY")
        .ok()
        .and_then(|s| DeliveryKey::parse(&s))
    else {
        return Router::new();
    };
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    router(pool, key, sessions)
}
fn router(pool: PgPool, key: DeliveryKey, sessions: SessionLayer) -> Router {
    let limiter = RateLimiter::new(pool.clone());
    let mut routes = Router::new();
    // Bare GET recovery routes already belong to the legacy redirect router.
    routes = routes
        .route("/forgot-password", axum::routing::post(submit_forgot))
        .route("/reset-password", get(show_reset).post(submit_reset));
    for prefix in ["/pl", "/en"] {
        routes = routes
            .route(
                &format!("{prefix}/forgot-password"),
                get(forgot).post(submit_forgot),
            )
            .route(
                &format!("{prefix}/reset-password"),
                get(show_reset).post(submit_reset),
            )
            .route(
                &format!("{prefix}/reset-password/{{token}}"),
                get(show_link),
            );
    }
    oteryn_web::ratelimit::finish_routes(
        routes
            .route_layer(axum::middleware::from_fn_with_state(
                sessions,
                session::middleware,
            ))
            .layer(Extension(RecoveryState {
                pool: pool.clone(),
                key,
            }))
            .layer(Extension(limiter))
            .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024))
            .layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                audit_refusals,
            ))
            .with_state(pool),
    )
}
#[derive(Template)]
#[template(
    path = "account/reset/page.html",
    config = "../../templates/account/reset/askama.toml"
)]
struct Page {
    lang: &'static str,
    language: Language,
    csrf: String,
    token: String,
    email: String,
    reset: bool,
    notice: bool,
    failed: bool,
}
impl Page {
    fn text(&self, key: &str) -> String {
        text(self.language, key)
    }
}
pub fn text(language: Language, key: &str) -> String {
    if !key.starts_with("reset-") {
        return language.text(key);
    }
    let source = if language == Language::En {
        include_str!("../../../templates/account/reset/en.ftl")
    } else {
        include_str!("../../../templates/account/reset/pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let Ok(locale) = language.code().parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
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
const NOTICE: SessionKey<bool> = SessionKey::new("reset_notice");
const FAILED: SessionKey<bool> = SessionKey::new("reset_failed");
async fn page(
    language: Language,
    session: BrowserSession,
    reset: bool,
    token: String,
    email: String,
) -> Result<Response, StatusCode> {
    let notice = session
        .get(NOTICE)
        .await
        .map_err(unavailable)?
        .unwrap_or(false);
    let failed = session
        .get(FAILED)
        .await
        .map_err(unavailable)?
        .unwrap_or(false);
    session.set(NOTICE, &false).await.map_err(unavailable)?;
    session.set(FAILED, &false).await.map_err(unavailable)?;
    let page = Page {
        lang: language.code(),
        language,
        csrf: session.csrf_token().await.map_err(unavailable)?,
        token,
        email,
        reset,
        notice,
        failed,
    };
    let mut response = oteryn_web::render(&page);
    response.headers_mut().insert(
        "cache-control",
        "no-store"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    response.headers_mut().insert(
        "referrer-policy",
        "no-referrer"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
fn unavailable(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
async fn forgot(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(language, session, false, String::new(), String::new()).await
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Link {
    email: String,
    token: String,
}
async fn show_reset(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Query(link): Query<Link>,
) -> Result<Response, StatusCode> {
    page(language, session, true, link.token, link.email).await
}
async fn show_link(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Path(token): Path<String>,
    Query(link): Query<Link>,
) -> Result<Response, StatusCode> {
    page(language, session, true, token, link.email).await
}
fn redirect(language: Language, path: &str) -> Response {
    let mut response = Redirect::to(&format!("/{}/{path}", language.code())).into_response();
    *response.status_mut() = StatusCode::FOUND;
    response
}
async fn charge(
    limiter: &RateLimiter,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
    email: Option<&str>,
    reset: bool,
) -> Result<(), Denial> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Err(Denial::Unavailable);
    };
    let ip = limiter.trusted_proxies.client_ip(peer, headers)?;
    let mut keys = Keys::source(ip);
    limiter
        .check(NamedPolicy::IdentityPasswordRecoverySource, &[keys.clone()])
        .await?;
    if let Some(email) = email.filter(|email| reset::valid_email(email)) {
        keys.email = Some(oteryn_identity::accounts::normalize_email(email));
        limiter
            .check(
                if reset {
                    NamedPolicy::IdentityPasswordReset
                } else {
                    NamedPolicy::IdentityPasswordRecovery
                },
                &[keys],
            )
            .await?;
    }
    Ok(())
}
#[derive(Clone)]
struct RecoveryState {
    pool: PgPool,
    key: DeliveryKey,
}
async fn submit_forgot(
    Extension(state): Extension<RecoveryState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let RecoveryState { pool, key } = state;
    let input = form.ok().map(|Form(i)| i);
    if let Err(e) = charge(
        &limiter,
        peer,
        &headers,
        input.as_ref().map(|i| i.email.as_str()),
        false,
    )
    .await
    {
        return denied(&pool, e, language).await;
    }
    if let Some(id) = session.account_id().await {
        let mut tx = pool.begin().await.map_err(unavailable)?;
        reset::audit(
            &mut tx,
            Some(id),
            "identity.password_reset_authenticated_refused",
        )
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        return Ok(redirect(language, ""));
    }
    reset::request(&pool, input.as_ref().map_or("", |i| i.email.as_str()), &key)
        .await
        .map_err(unavailable)?;
    session.set(NOTICE, &true).await.map_err(unavailable)?;
    Ok(redirect(language, "forgot-password"))
}
async fn submit_reset(
    State(pool): State<PgPool>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let input = form.ok().map(|Form(i)| i).unwrap_or_default();
    if let Err(e) = charge(&limiter, peer, &headers, Some(&input.email), true).await {
        return denied(&pool, e, language).await;
    }
    if let Some(id) = session.account_id().await {
        let mut tx = pool.begin().await.map_err(unavailable)?;
        reset::audit(
            &mut tx,
            Some(id),
            "identity.password_reset_authenticated_refused",
        )
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        return Ok(redirect(language, ""));
    }
    let retry = if input.token.len() == 64 && input.token.bytes().all(|b| b.is_ascii_hexdigit()) {
        format!("reset-password/{}", input.token)
    } else {
        "reset-password".to_owned()
    };
    if reset::complete(&pool, input).await.map_err(unavailable)? {
        session::rotate(&session, session::PrivilegeChange::Logout)
            .await
            .map_err(unavailable)?;
        return Ok(redirect(language, "login"));
    }
    session.set(FAILED, &true).await.map_err(unavailable)?;
    Ok(redirect(language, &retry))
}
async fn denied(pool: &PgPool, denial: Denial, language: Language) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    reset::audit(&mut tx, None, "identity.password_reset_rate_denied")
        .await
        .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(denial.response(language, false))
}
#[cfg(test)]
#[path = "reset_http_tests.rs"]
pub(crate) mod tests;

async fn audit_refusals(
    State(pool): State<PgPool>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let response = next.run(request).await;
    if matches!(
        response.status(),
        StatusCode::FORBIDDEN
            | StatusCode::BAD_REQUEST
            | StatusCode::PAYLOAD_TOO_LARGE
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
    ) {
        let result = async {
            let mut tx = pool.begin().await?;
            reset::audit(&mut tx, None, "identity.password_reset_browser_refused").await?;
            tx.commit().await
        }
        .await;
        if result.is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    }
    response
}
