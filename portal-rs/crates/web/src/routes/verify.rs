//! Verification and resend endpoints.
use crate::{
    AppState, Language,
    session::{BrowserSession, SessionKey},
};
use askama::Template;
use axum::{
    Extension, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_identity::verify;
use serde::Deserialize;
const FLASH: SessionKey<String> = SessionKey::new("identity.verification_flash");
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/verify-email", get(verification).post(verification))
        .route("/pl/verify-email", get(verification).post(verification))
        .route("/en/verify-email", get(verification).post(verification))
        .route("/verify-email/resend", get(resend_form).merge(post(resend)))
        .route(
            "/pl/verify-email/resend",
            get(resend_form).merge(post(resend)),
        )
        .route(
            "/en/verify-email/resend",
            get(resend_form).merge(post(resend)),
        )
}
#[derive(Clone, Deserialize)]
struct Token {
    token: Option<String>,
}
#[derive(Template)]
#[template(path = "account/verify.html")]
struct VerifyPage {
    lang: &'static str,
    language: Language,
    key: &'static str,
    csrf: String,
    authenticated: bool,
}
impl VerifyPage {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }
}
pub(crate) async fn message(
    language: Language,
    session: &BrowserSession,
    key: &'static str,
    status: StatusCode,
) -> Result<Response, StatusCode> {
    let page = VerifyPage {
        lang: language.code(),
        language,
        key,
        authenticated: session.account_id().await.is_some(),
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };
    let mut response = crate::render(&page);
    *response.status_mut() = status;
    Ok(response)
}
#[derive(Template)]
#[template(
    ext = "html",
    source = r#"{% extends "account/register_base.html" %}
{% block title %}{{ title }}{% endblock %}
{% block content %}<h1>{{ title }}</h1>
<form method="post" action="/{{ lang }}/verify-email?token={{ token }}">
<input type="hidden" name="_csrf" value="{{ csrf }}">
<button type="submit">{{ confirm }}</button></form>{% endblock %}"#
)]
struct Confirmation<'a> {
    lang: &'static str,
    language: Language,
    title: String,
    confirm: &'static str,
    token: &'a str,
    csrf: String,
}

impl Confirmation<'_> {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }
}
async fn verification(
    State(state): State<AppState>,
    Extension(limiter): Extension<crate::ratelimit::RateLimiter>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(query): Extension<Token>,
    peer: Option<Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    (method, headers): (axum::http::Method, axum::http::HeaderMap),
) -> Result<Response, StatusCode> {
    let Some(Extension(axum::extract::ConnectInfo(peer))) = peer else {
        return Ok(crate::ratelimit::Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer, &headers) {
        Ok(source) => source,
        Err(error) => return Ok(error.response(language, false)),
    };
    if let Err(error) = limiter
        .check(
            crate::ratelimit::NamedPolicy::IdentityVerificationSource,
            &[crate::ratelimit::Keys::source(source)],
        )
        .await
    {
        return Ok(error.response(language, false));
    }
    let token = query.token.as_deref().unwrap_or_default();
    if method != axum::http::Method::POST {
        if verify::available(&state.db, token)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Ok(crate::render(&Confirmation {
                lang: language.code(),
                language,
                title: language.text("verify-title"),
                confirm: match language {
                    Language::En => "Verify e-mail address",
                    Language::Pl => "Potwierdź adres e-mail",
                },
                token,
                csrf: session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            }));
        }
        return message(
            language,
            &session,
            "verify-invalid",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    let valid = verify::consume(&state.db, token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(id) = valid {
        // Start a fresh anonymous session after verification, including when
        // registration left this browser anonymous. Preserve other live identities.
        let current = session.account_id().await;
        if current.is_some_and(|account| account != id) {
            return message(language, &session, "verify-success", StatusCode::OK).await;
        }
        // An anonymous browser keeps its pending state (a referral code) under a
        // fresh identifier; the verified account's own session is ended.
        let reissued = current.is_none()
            && crate::session::rotate(&session, crate::session::PrivilegeChange::Reissue)
                .await
                .is_ok();
        if !reissued {
            crate::session::rotate(&session, crate::session::PrivilegeChange::Logout)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        session
            .set(FLASH, &language.text("verify-success"))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        message(language, &session, "verify-success", StatusCode::OK).await
    } else {
        message(
            language,
            &session,
            "verify-invalid",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await
    }
}
#[derive(Template)]
#[template(
    ext = "html",
    source = r#"{% extends "account/register_base.html" %}
{% block title %}{{ self.text("verify-resend") }}{% endblock %}
{% block content %}
<h1>{{ self.text("verify-resend") }}</h1>
<form method="post" action="/{{ lang }}/verify-email/resend">
<input type="hidden" name="_csrf" value="{{ csrf }}">
<label>{{ self.text("register-email") }} <input type="email" name="email" autocomplete="email" maxlength="254" required></label>
<button type="submit">{{ self.text("verify-resend") }}</button>
</form>
{% endblock %}"#
)]
struct ResendPage {
    lang: &'static str,
    language: Language,
    csrf: String,
}
impl ResendPage {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }
}
#[derive(Deserialize)]
struct ResendForm {
    email: String,
}
async fn resend_form(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_none() {
        return Ok(crate::render(&ResendPage {
            lang: language.code(),
            language,
            csrf: session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        }));
    }
    message(language, &session, "verify-resend", StatusCode::OK).await
}
async fn resend(
    State(state): State<AppState>,
    Extension(limiter): Extension<crate::ratelimit::RateLimiter>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: axum::http::HeaderMap,
    form: Result<axum::Form<ResendForm>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let Some(Extension(axum::extract::ConnectInfo(peer))) = peer else {
        return Ok(crate::ratelimit::Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer, &headers) {
        Ok(source) => source,
        Err(error) => return Ok(error.response(language, false)),
    };
    let mut keys = crate::ratelimit::Keys::source(source);
    if let Err(error) = limiter
        .check(
            crate::ratelimit::NamedPolicy::IdentityRegistration,
            std::slice::from_ref(&keys),
        )
        .await
    {
        return Ok(error.response(language, false));
    }
    let actor = if let Some(id) = session.account_id().await {
        keys.account = Some(id.to_string());
        if let Err(error) = limiter
            .check(
                crate::ratelimit::NamedPolicy::IdentitySecurityMutation,
                &[keys],
            )
            .await
        {
            return Ok(error.response(language, false));
        }
        Some(oteryn_identity::mutation_actor::MutationActor::browser(
            id,
            session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ))
    } else {
        let axum::Form(form) = form.map_err(|_| StatusCode::BAD_REQUEST)?;
        sqlx::query_scalar::<_, oteryn_identity::AccountId>(
            "SELECT id FROM accounts WHERE lower(email)=$1 AND status='pending_verification'",
        )
        .bind(oteryn_identity::accounts::normalize_email(&form.email))
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Into::into)
    };
    if let Some(actor) = actor {
        verify::resend(&state.db, actor)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Redirect::to(&format!("/{}/register/check-inbox", language.code())).into_response())
}

/// Remove bearer query material before the HTTP trace layer creates its request span.
/// The parsed value lives only in request memory and is never included in Debug output.
pub async fn protect_token(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if matches!(
        request.uri().path(),
        "/verify-email" | "/pl/verify-email" | "/en/verify-email"
    ) {
        let query = serde_urlencoded::from_str::<Token>(request.uri().query().unwrap_or_default())
            .unwrap_or(Token { token: None });
        request.extensions_mut().insert(query);
        let path = request.uri().path().to_owned();
        if let Ok(uri) = path.parse() {
            *request.uri_mut() = uri;
        }
    }
    next.run(request).await
}

#[cfg(test)]
#[path = "verify_budget_tests.rs"]
mod budget_tests;
