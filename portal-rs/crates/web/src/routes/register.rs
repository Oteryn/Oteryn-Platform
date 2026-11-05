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
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(config)
}
async fn show(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    value: Option<Extension<ConsentConfig>>,
) -> Result<Response, StatusCode> {
    let page = RegisterPage {
        lang: language.code(),
        language,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        config: config(value)?,
        email: String::new(),
        errors: BTreeMap::new(),
    };
    Ok(crate::render(&page))
}
async fn submit(
    State(state): State<AppState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    value: Option<Extension<ConsentConfig>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: Result<Form<Registration>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let config = config(value)?;
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
    let account = register::register(
        &state.db,
        input,
        &config,
        language.code(),
        peer.map(|Extension(ConnectInfo(peer))| peer.ip()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(id) = account {
        session::rotate(&session, PrivilegeChange::Login(id))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
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
    let defaults = ConsentConfig::default();
    let consents = ConsentConfig {
        terms_version: std::env::var("OTERYN_TERMS_VERSION").unwrap_or(defaults.terms_version),
        privacy_version: std::env::var("OTERYN_PRIVACY_VERSION")
            .unwrap_or(defaults.privacy_version),
    };
    let csrf = crate::csrf::CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..Default::default()
    };
    app.layer(Extension(consents)).layer(Extension(csrf))
}
