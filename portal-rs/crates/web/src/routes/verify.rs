//! Verification and authenticated resend endpoints.
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
        .route("/verify-email", get(consume))
        .route("/pl/verify-email", get(consume))
        .route("/en/verify-email", get(consume))
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
async fn consume(
    State(state): State<AppState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(query): Extension<Token>,
) -> Result<Response, StatusCode> {
    let valid = verify::consume(&state.db, query.token.as_deref().unwrap_or_default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if valid {
        // Activating the account revokes its sessions through the credential-change
        // trigger. Discard the request's stale session before writing flash state.
        crate::session::rotate(&session, crate::session::PrivilegeChange::Logout)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
async fn resend_form(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    message(language, &session, "verify-resend", StatusCode::OK).await
}
async fn resend(
    State(state): State<AppState>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    verify::resend(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
