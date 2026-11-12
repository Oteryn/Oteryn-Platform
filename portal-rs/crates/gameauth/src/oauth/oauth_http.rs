use super::*;
use askama::Template;
use axum::{
    Extension, Form, Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use std::collections::BTreeMap;

/// Supplied exclusively by the web session adapter after CSRF verification.
#[derive(Clone)]
pub struct Browser {
    pub account: Option<Uuid>,
    pub csrf: String,
    pub language: String,
}
/// Supplied exclusively by the web session adapter, next to [`Browser`], when the
/// browser session holds a live web MFA grant for `account`. Never built from request input.
#[derive(Clone, Copy)]
pub struct MfaSession {
    pub account: Uuid,
}
#[derive(Clone)]
struct HttpState {
    pool: PgPool,
    config: Config,
}
pub fn browser_routes(pool: PgPool, config: Config) -> Router {
    Router::new()
        .route("/oauth/authorize", get(show).post(consent))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .with_state(HttpState { pool, config })
}
pub fn token_routes(pool: PgPool, config: Config) -> Router {
    Router::new()
        .route("/oauth/token", post(token))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .with_state(HttpState { pool, config })
}
fn response(error: Error) -> Response {
    let (status, code, description) = match error {
        Error::Disabled => (
            StatusCode::NOT_FOUND,
            "invalid_request",
            "The requested resource is unavailable.",
        ),
        Error::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request is missing a required parameter or is malformed.",
        ),
        Error::InvalidGrant => (
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "The provided authorization grant is invalid.",
        ),
        Error::InvalidClient => (
            StatusCode::UNAUTHORIZED,
            "invalid_client",
            "Client authentication failed.",
        ),
        Error::UnsupportedGrant => (
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "The authorization grant type is not supported by the authorization server.",
        ),
        Error::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "server_error",
            "The authorization server is unavailable.",
        ),
    };
    no_store(
        (
            status,
            Json(serde_json::json!({"error":code,"error_description":description})),
        )
            .into_response(),
    )
}
fn no_store(mut r: Response) -> Response {
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    r.headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    r
}
async fn refused(state: &HttpState, account: Option<Uuid>, error: Error) -> Response {
    if audit(&state.pool, account, "oauth_refused").await.is_err() {
        return response(Error::Unavailable);
    }
    response(error)
}
#[derive(Template)]
#[template(
    path = "oauth/consent.html",
    config = "../../templates/oauth/askama.toml"
)]
struct ConsentPage {
    lang: String,
    title: String,
    description: String,
    approve: String,
    deny: String,
    csrf: String,
    skip: String,
    request: Authorization,
}
fn texts(language: &str) -> Result<[String; 5], Error> {
    let source = if language == "en" {
        include_str!("../../../../templates/oauth/en.ftl")
    } else {
        include_str!("../../../../templates/oauth/pl.ftl")
    };
    let locale = language
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|_| Error::Unavailable)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    let resource = FluentResource::try_new(source.to_owned()).map_err(|_| Error::Unavailable)?;
    bundle
        .add_resource(resource)
        .map_err(|_| Error::Unavailable)?;
    let mut output = Vec::new();
    for key in [
        "oauth-title",
        "oauth-description",
        "oauth-approve",
        "oauth-deny",
        "oauth-skip",
    ] {
        let pattern = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or(Error::Unavailable)?;
        let mut errors = Vec::new();
        output.push(
            bundle
                .format_pattern(pattern, None, &mut errors)
                .into_owned(),
        );
        if !errors.is_empty() {
            return Err(Error::Unavailable);
        }
    }
    output.try_into().map_err(|_| Error::Unavailable)
}
/// The existing sign-in page; the language is one of the two fixed site prefixes.
fn sign_in(language: &str) -> Response {
    let path = if language == "en" {
        "/en/login"
    } else {
        "/pl/login"
    };
    no_store(Redirect::to(path).into_response())
}
/// True only for a server-supplied proof bound to the same account.
fn mfa_proven(mfa: Option<Extension<MfaSession>>, account: Uuid) -> bool {
    mfa.is_some_and(|Extension(m)| m.account == account)
}
async fn show(
    State(state): State<HttpState>,
    Extension(browser): Extension<Browser>,
    mfa: Option<Extension<MfaSession>>,
    query: Result<Query<Authorization>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if !state.config.enabled {
        return response(Error::Disabled);
    }
    let Ok(Query(request)) = query else {
        return refused(&state, browser.account, Error::InvalidRequest).await;
    };
    if !request.valid(&state.config) {
        return refused(&state, browser.account, Error::InvalidRequest).await;
    }
    let Some(account) = browser.account else {
        return sign_in(&browser.language);
    };
    let mfa = mfa_proven(mfa, account);
    let language = browser.language.clone();
    let result = async {
        let mut tx = state.pool.begin().await?;
        if step_up(&mut tx, account, mfa).await?.is_none() {
            tx.commit().await?;
            return Ok(None);
        }
        event(&mut tx, Some(account), "consent_presented").await?;
        tx.commit().await?;
        let [title, description, approve, deny, skip] = texts(&browser.language)?;
        let page = ConsentPage {
            lang: browser.language,
            title,
            description,
            approve,
            deny,
            csrf: browser.csrf,
            skip,
            request,
        };
        page.render().map(Some).map_err(|_| Error::Unavailable)
    }
    .await;
    match result {
        Ok(Some(html)) => no_store(Html(html).into_response()),
        Ok(None) => sign_in(&language),
        Err(e) => refused(&state, Some(account), e).await,
    }
}
async fn consent(
    State(state): State<HttpState>,
    Extension(browser): Extension<Browser>,
    mfa: Option<Extension<MfaSession>>,
    form: Result<Form<Vec<(String, String)>>, axum::extract::rejection::FormRejection>,
) -> Response {
    if !state.config.enabled {
        return response(Error::Disabled);
    }
    let Some(account) = browser.account else {
        return refused(&state, None, Error::InvalidGrant).await;
    };
    let Ok(Form(pairs)) = form else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    let mut fields = BTreeMap::new();
    for (key, value) in pairs {
        if fields.insert(key, value).is_some() {
            return refused(&state, Some(account), Error::InvalidRequest).await;
        }
    }
    let csrf = fields.remove("_csrf").unwrap_or_default();
    if !same(csrf.as_bytes(), browser.csrf.as_bytes()) || csrf.len() != 64 {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    }
    let decision = fields.remove("decision").unwrap_or_default();
    let Ok(request) = serde_json::from_value::<Authorization>(serde_json::json!(fields)) else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    if !request.valid(&state.config) || !matches!(decision.as_str(), "approve" | "deny") {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    }
    let mut redirect = match url::Url::parse(&request.redirect_uri) {
        Ok(u) => u,
        Err(_) => return refused(&state, Some(account), Error::InvalidRequest).await,
    };
    if decision == "deny" {
        if audit(&state.pool, Some(account), "consent_denied")
            .await
            .is_err()
        {
            return response(Error::Unavailable);
        }
        redirect
            .query_pairs_mut()
            .append_pair("error", "access_denied")
            .append_pair("state", &request.state);
    } else {
        let mfa = mfa_proven(mfa, account);
        match approve(&state.pool, &state.config, account, mfa, &request).await {
            Ok(None) => return sign_in(&browser.language),
            Ok(Some(code)) => {
                redirect
                    .query_pairs_mut()
                    .append_pair("code", &code)
                    .append_pair("state", &request.state);
            }
            Err(e) => return refused(&state, Some(account), e).await,
        }
    }
    let mut r = Redirect::to(redirect.as_str()).into_response();
    *r.status_mut() = StatusCode::FOUND;
    no_store(r)
}
async fn token(
    State(state): State<HttpState>,
    form: Result<Form<TokenRequest>, axum::extract::rejection::FormRejection>,
) -> Response {
    if !state.config.enabled {
        return response(Error::Disabled);
    }
    let Ok(Form(input)) = form else {
        return refused(&state, None, Error::InvalidRequest).await;
    };
    match exchange(&state.pool, &state.config, &input).await {
        Ok(tokens) => no_store(Json(tokens).into_response()),
        Err(e) => refused(&state, None, e).await,
    }
}

/// OAuth owns its protocol error envelope; the site HTML error renderer must not replace it.
pub async fn secure_response(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = no_store(next.run(request).await);
    // A no-referrer document gives native form POSTs an opaque Origin. Keep the
    // consent query out of Referer while preserving the browser's real origin
    // for the independent CSRF origin check. Protocol JSON stays no-referrer.
    let referrer = if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"))
    {
        "strict-origin"
    } else {
        "no-referrer"
    };
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", referrer),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    response
}
