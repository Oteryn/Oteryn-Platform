//! Localized registration forms; session middleware enforces CSRF before extraction.
use crate::{
    AppState, Language,
    session::{self, BrowserSession, PrivilegeChange},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::{
    consents::ConsentConfig,
    register::{self, Registration},
};
use std::{collections::BTreeMap, net::SocketAddr};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", get(show).post(submit))
        .route("/pl/register", get(show).post(submit))
        .route("/en/register", get(show).post(submit))
        .route("/register/check-inbox", get(inbox))
        .route("/pl/register/check-inbox", get(inbox))
        .route("/en/register/check-inbox", get(inbox))
}
#[derive(Template)]
#[template(path = "account/register.html")]
struct RegisterPage {
    lang: &'static str,
    language: Language,
    csrf: String,
    config: ConsentConfig,
    email: String,
    errors: BTreeMap<&'static str, String>,
}
impl RegisterPage {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }
    fn error(&self, field: &str) -> &str {
        self.errors.get(field).map_or("", String::as_str)
    }
}
pub(crate) fn config(value: Option<Extension<ConsentConfig>>) -> Result<ConsentConfig, StatusCode> {
    let config = value.map(|v| v.0).unwrap_or_default();
    if !config.validate() {
        tracing::error!("sign-up blocked: invalid OTERYN_TERMS_VERSION or OTERYN_PRIVACY_VERSION");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(config)
}
async fn show(
    State(state): State<AppState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    value: Option<Extension<ConsentConfig>>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_some() {
        return Ok(Redirect::to("/account").into_response());
    }
    let config = config(value)?;
    config
        .check_publication(&state.db, language.code())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let page = RegisterPage {
        lang: language.code(),
        language,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        config,
        email: String::new(),
        errors: BTreeMap::new(),
    };
    Ok(crate::render(&page))
}
async fn submit(
    State(state): State<AppState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    (limiter, headers): (
        Extension<crate::ratelimit::RateLimiter>,
        axum::http::HeaderMap,
    ),
    value: Option<Extension<ConsentConfig>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: Result<Form<Registration>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    // Registration starts an identity; it must never replace an authenticated
    // identity or reuse that account's authorization generation for a new one.
    if session.account_id().await.is_some() {
        return Ok(Redirect::to("/account").into_response());
    }
    let Extension(limiter) = limiter;
    let Some(Extension(ConnectInfo(peer_addr))) = peer else {
        return Ok(crate::ratelimit::Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer_addr, &headers) {
        Ok(source) => source,
        Err(error) => return Ok(error.response(language, false)),
    };
    if let Err(error) = limiter
        .check(
            crate::ratelimit::NamedPolicy::IdentityRegistration,
            &[crate::ratelimit::Keys::source(source)],
        )
        .await
    {
        return Ok(error.response(language, false));
    }
    let config = config(value)?;
    config
        .check_publication(&state.db, language.code())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let input = form.map(|Form(input)| input).unwrap_or_default();
    let errors = register::validate(&input, &config);
    if !errors.is_empty() {
        let page = RegisterPage {
            lang: language.code(),
            language,
            csrf: session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            config,
            email: input.email,
            errors: errors
                .into_iter()
                .map(|(f, k)| (f, language.text(k)))
                .collect(),
        };
        let mut response = crate::render(&page);
        *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
        return Ok(response);
    }
    register::register(&state.db, input, &config, language.code(), Some(source))
        .await
        .map_err(|error| match error {
            register::Error::LegalUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            register::Error::Backend(_) => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    session::rotate(&session, PrivilegeChange::Reissue)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Redirect::to(&format!("/{}/register/check-inbox", language.code())).into_response())
}
async fn inbox(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    super::verify::message(language, &session, "verify-inbox", StatusCode::OK).await
}

/// Registers legal-document versions and T02's trusted browser origin at process startup.
/// Deployments set OTERYN_PUBLIC_ORIGIN to the canonical origin, and may override
/// OTERYN_TERMS_VERSION and OTERYN_PRIVACY_VERSION when publishing new documents.
/// Without a configured origin, browser requests carrying Origin remain refused.
pub fn configured(app: Router) -> Router {
    let allow_placeholders = matches!(
        std::env::var("OTERYN_ENV").as_deref(),
        Ok("local" | "testing")
    );
    let default_version = if allow_placeholders {
        oteryn_identity::consents::PLACEHOLDER_LEGAL_VERSION
    } else {
        oteryn_identity::consents::DEFAULT_LEGAL_VERSION
    };
    let consents = ConsentConfig::published(
        std::env::var("OTERYN_TERMS_VERSION").unwrap_or_else(|_| default_version.into()),
        std::env::var("OTERYN_PRIVACY_VERSION").unwrap_or_else(|_| default_version.into()),
        allow_placeholders,
    );
    let csrf = crate::csrf::CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..Default::default()
    };
    app.layer(Extension(consents)).layer(Extension(csrf))
}

#[cfg(test)]
#[path = "register_authenticated_tests.rs"]
mod authenticated_tests;
