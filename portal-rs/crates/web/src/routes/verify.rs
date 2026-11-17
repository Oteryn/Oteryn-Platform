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
    Extension(limiter): Extension<crate::ratelimit::RateLimiter>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(query): Extension<Token>,
    peer: Option<Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: axum::http::HeaderMap,
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
    let valid = verify::consume(&state.db, query.token.as_deref().unwrap_or_default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(id) = valid {
        // Verification revokes the verified account's sessions. Preserve an
        // unrelated browser identity, but discard any already-revoked record.
        let stale = match session.identity_hash().await {
            Some(hash) => !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1)",
            )
            .bind(hash)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            None => false,
        };
        if session.account_id().await == Some(id) || stale {
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
async fn resend_form(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    message(language, &session, "verify-resend", StatusCode::OK).await
}
async fn resend(
    State(state): State<AppState>,
    Extension(limiter): Extension<crate::ratelimit::RateLimiter>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let Some(Extension(axum::extract::ConnectInfo(peer))) = peer else {
        return Ok(crate::ratelimit::Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer, &headers) {
        Ok(source) => source,
        Err(error) => return Ok(error.response(language, false)),
    };
    let mut keys = crate::ratelimit::Keys::source(source);
    keys.account = Some(id.to_string());
    for policy in [
        crate::ratelimit::NamedPolicy::IdentityRegistration,
        crate::ratelimit::NamedPolicy::IdentitySecurityMutation,
    ] {
        if let Err(error) = limiter.check(policy, &[keys.clone()]).await {
            return Ok(error.response(language, false));
        }
    }
    verify::resend(
        &state.db,
        oteryn_identity::mutation_actor::MutationActor::browser(
            id,
            session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
    )
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

#[cfg(test)]
#[path = "verify_budget_tests.rs"]
mod budget_tests;
