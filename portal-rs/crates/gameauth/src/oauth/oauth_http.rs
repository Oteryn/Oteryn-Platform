use super::challenge::{self, Attempt, ChallengeConfig, Completion, Outcome};
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
use std::{collections::BTreeMap, net::SocketAddr};

/// Supplied exclusively by the web session adapter after CSRF verification.
#[derive(Clone)]
pub struct Browser {
    pub account: Option<Uuid>,
    pub csrf: String,
    pub language: String,
}
/// Supplied exclusively by the web session adapter, next to [`Browser`], when the
/// browser session held a live web MFA grant for `account`. Used for rendering only;
/// code issuance revalidates [`BrowserProof`]. Never built from request input.
#[derive(Clone, Copy)]
pub struct MfaSession {
    pub account: Uuid,
}
/// Server-side session identity, revalidated under the account lock at issuance.
#[derive(Clone)]
pub struct BrowserProof {
    pub session_hash: Vec<u8>,
    pub mfa_grant_hash: Option<Vec<u8>>,
}
#[derive(Clone)]
struct HttpState {
    pool: PgPool,
    config: Config,
    challenge: ChallengeConfig,
}
pub fn browser_routes(pool: PgPool, config: Config) -> Router {
    browser_routes_with(pool, config, ChallengeConfig::from_env())
}
/// [`browser_routes`] with an explicit native login challenge policy.
pub fn browser_routes_with(pool: PgPool, config: Config, challenge: ChallengeConfig) -> Router {
    let mut router = Router::new().route("/oauth/authorize", get(show).post(consent));
    // Flag off: the path stays unrouted, exactly as before the challenge existed.
    if challenge.enabled {
        router = router.route(
            "/oauth/challenge",
            get(show_challenge).post(complete_challenge),
        );
    }
    router
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .with_state(HttpState {
            pool,
            config,
            challenge,
        })
}
pub fn token_routes(pool: PgPool, config: Config) -> Router {
    token_routes_with(pool, config, ChallengeConfig::from_env())
}
/// [`token_routes`] with an explicit native login challenge policy.
pub fn token_routes_with(pool: PgPool, config: Config, challenge: ChallengeConfig) -> Router {
    Router::new()
        .route("/oauth/token", post(token))
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
        .with_state(HttpState {
            pool,
            config,
            challenge,
        })
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
fn texts(language: &str, scope: &str) -> Result<[String; 5], Error> {
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
        match scope {
            TICKET_SCOPE => "oauth-description",
            ACCOUNT_SERVICES_SCOPE => "oauth-account-services",
            _ => return Err(Error::InvalidRequest),
        },
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
        let [title, description, approve, deny, skip] = texts(&browser.language, &request.scope)?;
        let origin = url::Url::parse(&request.redirect_uri)
            .map_err(|_| Error::InvalidRequest)?
            .origin()
            .ascii_serialization();
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
        let mut response = Html(page.render().map_err(|_| Error::Unavailable)?).into_response();
        response.extensions_mut().insert(ConsentOrigin(origin));
        Ok(Some(response))
    }
    .await;
    match result {
        Ok(Some(response)) => no_store(response),
        Ok(None) => sign_in(&language),
        Err(e) => refused(&state, Some(account), e).await,
    }
}
async fn consent(
    State(state): State<HttpState>,
    Extension(browser): Extension<Browser>,
    proof: Option<Extension<BrowserProof>>,
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
        let Some(Extension(proof)) = proof else {
            return refused(&state, Some(account), Error::InvalidGrant).await;
        };
        match approve_browser(&state.pool, &state.config, account, &proof, &request).await {
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
    peer: Option<Extension<axum::extract::ConnectInfo<SocketAddr>>>,
    headers: axum::http::HeaderMap,
    form: Result<Form<TokenRequest>, axum::extract::rejection::FormRejection>,
) -> Response {
    if !state.config.enabled {
        return response(Error::Disabled);
    }
    let source = || {
        peer.and_then(|Extension(axum::extract::ConnectInfo(peer))| {
            oteryn_internal_api::proxy::TrustedProxies::configured()
                .ok()?
                .client_ip(peer, &headers)
                .ok()
        })
    };
    let Ok(Form(input)) = form else {
        // A malformed request still counts for its source.
        if let (true, Some(source)) = (state.challenge.usable(), source())
            && challenge::charge_source(&state.pool, &state.challenge, source)
                .await
                .is_err()
        {
            return refused(&state, None, Error::Unavailable).await;
        }
        return refused(&state, None, Error::InvalidRequest).await;
    };
    if !state.challenge.enabled || input.grant_type != "authorization_code" {
        // Flag off: the retry fields are refused exactly as unknown fields were.
        if !state.challenge.enabled && input.has_challenge_fields() {
            return refused(&state, None, Error::InvalidRequest).await;
        }
        return match exchange(&state.pool, &state.config, &input).await {
            Ok(tokens) => no_store(Json(tokens).into_response()),
            Err(e) => refused(&state, None, e).await,
        };
    }
    let (Some(source), true) = (source(), state.challenge.usable()) else {
        return refused(&state, None, Error::Unavailable).await;
    };
    match challenge::exchange(&state.pool, &state.config, &state.challenge, &input, source).await {
        Ok(Outcome::Tokens(tokens)) => no_store(Json(tokens).into_response()),
        Ok(Outcome::Challenge { id, expires_in }) => challenge_required(id, expires_in),
        Err(e) => refused(&state, None, e).await,
    }
}
/// OTERYN_NATIVE_LOGIN_CHALLENGE_V1: exactly these members, in this order.
#[derive(serde::Serialize)]
struct ChallengeRequired {
    error: &'static str,
    challenge_id: String,
    expires_in: i32,
}
fn challenge_required(id: Uuid, expires_in: i32) -> Response {
    no_store(
        (
            StatusCode::FORBIDDEN,
            Json(ChallengeRequired {
                error: "oteryn_challenge_required",
                challenge_id: id.hyphenated().to_string(),
                expires_in,
            }),
        )
            .into_response(),
    )
}

#[derive(Template)]
#[template(
    path = "oauth/challenge.html",
    config = "../../templates/oauth/askama.toml"
)]
struct ChallengePage {
    lang: String,
    title: String,
    intro: String,
    code_label: String,
    code: String,
    alternative_label: String,
    alternative: String,
    answer_label: String,
    submit: String,
    skip: String,
    error: Option<String>,
    csrf: String,
    attempt: Attempt,
}
fn messages<const N: usize>(language: &str, keys: [&str; N]) -> Result<[String; N], Error> {
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
    for key in keys {
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
/// The digits spelled out: the text alternative for the displayed code.
fn spelled(language: &str, code: &str) -> String {
    const EN: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    const PL: [&str; 10] = [
        "zero",
        "jeden",
        "dwa",
        "trzy",
        "cztery",
        "pięć",
        "sześć",
        "siedem",
        "osiem",
        "dziewięć",
    ];
    let words = if language == "en" { EN } else { PL };
    code.bytes()
        .filter_map(|b| words.get(usize::from(b.wrapping_sub(b'0'))).copied())
        .collect::<Vec<_>>()
        .join(", ")
}
fn challenge_page(
    browser: &Browser,
    attempt: Attempt,
    code: &str,
    failed: bool,
) -> Result<Response, Error> {
    let [
        title,
        intro,
        code_label,
        alternative_label,
        answer_label,
        submit,
        skip,
        error,
    ] = messages(
        &browser.language,
        [
            "challenge-title",
            "challenge-intro",
            "challenge-code-label",
            "challenge-alternative-label",
            "challenge-answer-label",
            "challenge-submit",
            "oauth-skip",
            "challenge-error",
        ],
    )?;
    let origin = url::Url::parse(&attempt.redirect_uri)
        .map_err(|_| Error::InvalidRequest)?
        .origin()
        .ascii_serialization();
    let page = ChallengePage {
        lang: browser.language.clone(),
        title,
        intro,
        code_label,
        code: code.chars().map(String::from).collect::<Vec<_>>().join(" "),
        alternative_label,
        alternative: spelled(&browser.language, code),
        answer_label,
        submit,
        skip,
        error: failed.then_some(error),
        csrf: browser.csrf.clone(),
        attempt,
    };
    let mut response = Html(page.render().map_err(|_| Error::Unavailable)?).into_response();
    // The form's redirect target is the loopback return URI.
    response.extensions_mut().insert(ConsentOrigin(origin));
    Ok(no_store(response))
}
fn challenge_off(state: &HttpState) -> bool {
    !state.config.enabled || !state.challenge.enabled
}
async fn show_challenge(
    State(state): State<HttpState>,
    Extension(browser): Extension<Browser>,
    proof: Option<Extension<BrowserProof>>,
    query: Result<Query<Attempt>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if challenge_off(&state) {
        return response(Error::Disabled);
    }
    if !state.challenge.usable() {
        return refused(&state, browser.account, Error::Unavailable).await;
    }
    let Ok(Query(attempt)) = query else {
        return refused(&state, browser.account, Error::InvalidRequest).await;
    };
    let Some(id) = attempt.valid(&state.config) else {
        return refused(&state, browser.account, Error::InvalidRequest).await;
    };
    let Some(account) = browser.account else {
        return sign_in(&browser.language);
    };
    let Some(Extension(proof)) = proof else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    match challenge::show(&state.pool, &state.challenge, account, &proof, id, &attempt).await {
        Ok(prompt) => match challenge_page(&browser, attempt, &prompt.code, false) {
            Ok(page) => page,
            Err(e) => refused(&state, Some(account), e).await,
        },
        Err(e) => refused(&state, Some(account), e).await,
    }
}
async fn complete_challenge(
    State(state): State<HttpState>,
    Extension(browser): Extension<Browser>,
    proof: Option<Extension<BrowserProof>>,
    form: Result<Form<Vec<(String, String)>>, axum::extract::rejection::FormRejection>,
) -> Response {
    if challenge_off(&state) {
        return response(Error::Disabled);
    }
    if !state.challenge.usable() {
        return refused(&state, browser.account, Error::Unavailable).await;
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
    let answer = fields.remove("answer").unwrap_or_default();
    let Ok(attempt) = serde_json::from_value::<Attempt>(serde_json::json!(fields)) else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    let Some(id) = attempt.valid(&state.config) else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    let Some(Extension(proof)) = proof else {
        return refused(&state, Some(account), Error::InvalidRequest).await;
    };
    let answer: String = answer.chars().filter(|c| !c.is_whitespace()).collect();
    match challenge::complete(
        &state.pool,
        &state.challenge,
        account,
        &proof,
        id,
        &attempt,
        &answer,
    )
    .await
    {
        Ok(Completion::Redirect(target)) => {
            let mut r = Redirect::to(&target).into_response();
            *r.status_mut() = StatusCode::FOUND;
            no_store(r)
        }
        Ok(Completion::Retry(prompt)) => {
            match challenge_page(&browser, attempt, &prompt.code, true) {
                Ok(page) => page,
                Err(e) => refused(&state, Some(account), e).await,
            }
        }
        Err(e) => refused(&state, Some(account), e).await,
    }
}

#[derive(Clone)]
struct ConsentOrigin(String);

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
    let mut form_action = String::from("'self'");
    if let Some(ConsentOrigin(origin)) = response.extensions_mut().remove::<ConsentOrigin>() {
        form_action.push(' ');
        form_action.push_str(&origin);
    }
    let policy = format!(
        "default-src 'none'; style-src 'self'; form-action {form_action}; base-uri 'none'; frame-ancestors 'none'"
    );
    let Ok(policy) = header::HeaderValue::from_str(&policy) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    response
        .headers_mut()
        .insert(header::CONTENT_SECURITY_POLICY, policy);
    for (name, value) in [
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
