//! Composition adapter, compiled by the binary to keep identity independent of web.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_identity::{
    login::Credentials,
    mfa::{Authenticated, ManagementAction, ManagementResult, Mfa},
    mutation_actor::MutationActor,
};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, PrivilegeChange, SessionKey, SessionLayer},
};
use serde::Deserialize;
use sqlx::{PgPool, types::time::OffsetDateTime};
use std::net::SocketAddr;
pub(crate) const PENDING: SessionKey<String> = SessionKey::new("mfa_pending");
const REFERRAL: SessionKey<String> = SessionKey::new("referral.code");
const FAILED: SessionKey<bool> = SessionKey::new("mfa_failed");
pub(crate) const GRANT: SessionKey<String> =
    SessionKey::new(oteryn_identity::mfa::SESSION_GRANT_KEY);
const TRUSTED: &str = "__Host-oteryn_mfa_device";
#[derive(Clone)]
struct Context {
    mfa: Mfa,
    limiter: RateLimiter,
    native_enabled: bool,
}
/// Public adapter for future staff guards. Ambiguous state always denies.
pub async fn session_satisfied(mfa: &Mfa, session: &BrowserSession) -> bool {
    let Some(id) = session.account_id().await else {
        return false;
    };
    let Ok(Some(grant)) = session.get(GRANT).await else {
        return false;
    };
    mfa.satisfied(id, &grant).await.unwrap_or(false)
}
pub fn configured(app: Router, pool: PgPool) -> Result<Router, sqlx::Error> {
    if !std::env::var("OTERYN_ACCOUNT_MFA").is_ok_and(|v| v == "true") {
        return Ok(app);
    }
    if !std::env::var("OTERYN_ACCOUNT_LOGIN").is_ok_and(|v| v == "true") {
        return Ok(app);
    }
    let mfa = Mfa::configured(pool.clone())?;
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let native_enabled = oteryn_gameauth::devices::owner_panel_enabled()
        && std::env::var("OTERYN_ACCOUNT_CREDENTIALS").as_deref() == Ok("true");
    Ok(compose(app, pool, mfa, sessions, native_enabled))
}
fn compose(
    app: Router,
    pool: PgPool,
    mfa: Mfa,
    sessions: SessionLayer,
    native_enabled: bool,
) -> Router {
    let context = Context {
        mfa,
        limiter: RateLimiter::new(pool),
        native_enabled,
    };
    let mut routes = Router::new();
    for prefix in ["", "/pl", "/en"] {
        routes = routes
            .route(&format!("{prefix}/mfa/login"), get(show_login).post(login))
            .route(
                &format!("{prefix}/mfa/challenge"),
                get(show_challenge).post(challenge),
            )
            .route(&format!("{prefix}/mfa/email"), post(email))
            .route(&format!("{prefix}/mfa"), get(settings))
            .route(&format!("{prefix}/mfa/enroll"), post(enroll))
            .route(&format!("{prefix}/mfa/confirm"), post(confirm))
            .route(&format!("{prefix}/mfa/devices"), post(remove_device))
            .route(&format!("{prefix}/mfa/manage"), post(manage))
            .route(
                &format!("{prefix}/mfa/replace/confirm"),
                post(confirm_replacement),
            );
        if context.native_enabled {
            routes = routes.route(&format!("{prefix}/mfa/native-devices"), post(revoke_native));
        }
    }
    let admin_host = std::env::var("OTERYN_ADMIN_HOST").ok().filter(|host| {
        std::env::var("OTERYN_ADMIN_PANEL").is_ok_and(|flag| flag == "true")
            && host.parse::<axum::http::uri::Authority>().is_ok()
    });
    let routes = routes
        .route_layer(axum::middleware::from_fn_with_state(
            context.mfa.clone(),
            audit_account_refusal,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            (sessions, admin_host),
            mfa_session,
        ))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024))
        .with_state(context);
    let app = app.merge(oteryn_web::ratelimit::finish_routes(routes));
    #[cfg(test)]
    {
        let recorded = crate::record_route_registrations("mfa-fallback", &app);
        assert!(recorded.is_ok(), "MFA route registrations: {recorded:?}");
    }
    // The outer fallback dispatches after rewriting, avoiding duplicate /login route ownership.
    Router::new()
        .fallback_service(app)
        .layer(axum::middleware::from_fn(rewrite_login))
}
async fn mfa_session(
    State((layer, admin_host)): State<(SessionLayer, Option<String>)>,
    mut request: Request,
    next: Next,
) -> Response {
    let mut hosts = request.headers().get_all(header::HOST).iter();
    if let Some(host) = admin_host
        && hosts
            .next()
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case(&host))
        && hosts.next().is_none()
    {
        // The outer router can supply its public CSRF policy via an extension.
        // Keep its other settings while selecting the dedicated host's origin.
        let csrf = request.extensions_mut().get_or_insert(layer.csrf.clone());
        csrf.trusted_origins = vec![format!("https://{}", host.to_ascii_lowercase())];
    }
    session::middleware(State(layer), request, next).await
}
async fn audit_account_refusal(
    State(mfa): State<Mfa>,
    Extension(session): Extension<BrowserSession>,
    request: Request,
    next: Next,
) -> Response {
    let account = session.account_id().await;
    let response = next.run(request).await;
    let status = response.status();
    if let Some(account) = account
        && status != StatusCode::TOO_MANY_REQUESTS
        && (status.is_client_error() || status.is_server_error())
    {
        let _ = mfa.record_http_refusal(account).await;
    }
    response
}

async fn rewrite_login(mut request: Request, next: Next) -> Response {
    let security_request =
        request.uri().path().contains("/mfa") || request.uri().path().ends_with("/login");
    let path = request.uri().path();
    if ["/login", "/pl/login", "/en/login"].contains(&path) {
        let replacement = path.replace("/login", "/mfa/login");
        if let Ok(uri) = replacement.parse() {
            *request.uri_mut() = uri;
        }
    }
    let mut response = next.run(request).await;
    if security_request {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, private"),
        );
        response.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            ),
        );
    }
    response
}
fn redirect(language: Language, path: &str) -> Response {
    let mut response = Redirect::to(&format!("/{}{path}", language.code())).into_response();
    *response.status_mut() = StatusCode::FOUND;
    response
}
/// After a completed login: the stored OAuth authorize or checkout target
/// (consumed once), otherwise the home page.
async fn home_or_return(
    language: Language,
    session: &BrowserSession,
) -> Result<Response, StatusCode> {
    match crate::login_http::take_return(session)
        .await
        .map_err(unavailable)?
    {
        Some(target) => {
            let mut response = Redirect::to(&target).into_response();
            *response.status_mut() = StatusCode::FOUND;
            Ok(response)
        }
        None => Ok(redirect(language, "/")),
    }
}
fn unavailable(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
#[derive(Template)]
#[template(
    path = "account/mfa/page.html",
    config = "../../templates/account/mfa/askama.toml"
)]
struct Page {
    lang: &'static str,
    language: Language,
    csrf: String,
    mode: &'static str,
    failed: bool,
    secret: String,
    codes: Vec<String>,
    devices: Vec<String>,
    remaining_codes: i64,
    native_enabled: bool,
    native_devices: Vec<oteryn_gameauth::devices::RememberedFamily>,
}
impl Page {
    fn text(&self, key: &str) -> String {
        let source = if self.language == Language::En {
            include_str!("../../../templates/account/mfa/en.ftl")
        } else {
            include_str!("../../../templates/account/mfa/pl.ftl")
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
            return self.language.text(key);
        };
        bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned()
    }
}
async fn page(
    language: Language,
    session: &BrowserSession,
    mode: &'static str,
    failed: bool,
    secret: String,
    codes: Vec<String>,
    devices: Vec<String>,
) -> Result<Response, StatusCode> {
    Ok(oteryn_web::render(
        &page_data(language, session, mode, failed, secret, codes, devices).await?,
    ))
}
async fn page_data(
    language: Language,
    session: &BrowserSession,
    mode: &'static str,
    failed: bool,
    secret: String,
    codes: Vec<String>,
    devices: Vec<String>,
) -> Result<Page, StatusCode> {
    let (failed, csrf) = if mode == "codes" {
        (false, String::new())
    } else {
        let flash = session
            .get(FAILED)
            .await
            .map_err(unavailable)?
            .unwrap_or(false);
        if flash {
            session.set(FAILED, &false).await.map_err(unavailable)?;
        }
        (
            failed || flash,
            session.csrf_token().await.map_err(unavailable)?,
        )
    };
    Ok(Page {
        lang: language.code(),
        language,
        csrf,
        mode,
        failed,
        secret,
        codes,
        devices,
        remaining_codes: 0,
        native_enabled: false,
        native_devices: vec![],
    })
}
async fn show_login(
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    page(
        language,
        &session,
        "login",
        false,
        String::new(),
        vec![],
        vec![],
    )
    .await
}
async fn refusal(
    language: Language,
    session: &BrowserSession,
    path: &str,
) -> Result<Response, StatusCode> {
    session.set(FAILED, &true).await.map_err(unavailable)?;
    Ok(redirect(language, path))
}
async fn pending(
    context: &Context,
    session: &BrowserSession,
) -> Result<Option<String>, StatusCode> {
    let token = session
        .get(PENDING)
        .await
        .map_err(unavailable)?
        .unwrap_or_default();
    Ok(context
        .mfa
        .pending_account(&token)
        .await
        .map_err(unavailable)?
        .map(|_| token))
}
async fn show_challenge(
    State(context): State<Context>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_some() {
        return Ok(redirect(language, "/"));
    }
    if pending(&context, &session).await?.is_none() {
        return Ok(redirect(language, "/login"));
    }
    page(
        language,
        &session,
        "challenge",
        false,
        String::new(),
        vec![],
        vec![],
    )
    .await
}
async fn charge(
    limiter: &RateLimiter,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
    language: Language,
    scopes: (Option<String>, Option<String>, Option<String>),
    policies: &[NamedPolicy],
) -> Result<(), Response> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Err(Denial::Unavailable.response(language, false));
    };
    let source = limiter
        .trusted_proxies
        .client_ip(peer, headers)
        .map_err(|e| e.response(language, false))?;
    let (account, pending, email) = scopes;
    let mut keys = Keys::source(source);
    keys.account = account;
    keys.pending_account = pending;
    keys.email = email;
    for policy in policies {
        limiter
            .check(*policy, &[keys.clone()])
            .await
            .map_err(|e| e.response(language, false))?;
    }
    Ok(())
}
fn device_cookie(headers: &HeaderMap) -> Option<String> {
    let mut values = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|v| v.trim().split_once('='))
        .filter(|(k, _)| *k == TRUSTED)
        .map(|(_, v)| v.to_owned());
    let first = values.next()?;
    if values.next().is_some() {
        None
    } else {
        Some(first)
    }
}
async fn validate_rotated_session(
    context: &Context,
    session: &BrowserSession,
    authorization: &oteryn_identity::mfa::SessionAuthorization,
) -> Result<(), StatusCode> {
    let valid = authorization.current(&context.mfa).await;
    if matches!(valid, Ok(true)) {
        return Ok(());
    }
    session::rotate(session, PrivilegeChange::Logout)
        .await
        .map_err(unavailable)?;
    Err(if valid.is_err() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::UNAUTHORIZED
    })
}
async fn establish(
    context: &Context,
    language: Language,
    session: &BrowserSession,
    auth: Authenticated,
) -> Result<Response, StatusCode> {
    let authorization = context
        .mfa
        .authorize_session(&auth)
        .await
        .map_err(unavailable)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    session
        .set(PENDING, &String::new())
        .await
        .map_err(unavailable)?;
    session::rotate(
        session,
        PrivilegeChange::AuthorizedLogin(auth.account, authorization.generation()),
    )
    .await
    .map_err(unavailable)?;
    if let Err(error) = session.set(GRANT, &auth.grant).await {
        session::rotate(session, PrivilegeChange::Logout)
            .await
            .map_err(unavailable)?;
        return Err(unavailable(error));
    }
    validate_rotated_session(context, session, &authorization).await?;
    crate::sessions_http::complete_login(&context.mfa.pool(), session, auth.account).await?;
    let mut response = home_or_return(language, session).await?;
    if let Some(device) = auth.trusted {
        let value = HeaderValue::from_str(&format!(
            "{TRUSTED}={device}; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000"
        ))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    Ok(response)
}
async fn login(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Credentials>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (None, None, None),
        &[NamedPolicy::IdentityLoginSource],
    )
    .await
    {
        return Ok(r);
    }
    if session.account_id().await.is_some() {
        return Ok(redirect(language, "/"));
    }
    let Ok(Form(input)) = form else {
        return refusal(language, &session, "/login").await;
    };
    if !input.valid() {
        return refusal(language, &session, "/login").await;
    }
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (
            None,
            None,
            Some(oteryn_identity::accounts::normalize_email(&input.email)),
        ),
        &[NamedPolicy::IdentityLogin],
    )
    .await
    {
        return Ok(r);
    }
    if let Some(challenge) = context.mfa.begin_login(&input).await.map_err(unavailable)? {
        // The logout rotation drops the session; the challenge session keeps
        // a valid post-login target so `establish` can return to it.
        let target = crate::login_http::pending_return(&session)
            .await
            .map_err(unavailable)?;
        let referral = if session.account_id().await.is_none() {
            session.get(REFERRAL).await.map_err(unavailable)?
        } else {
            None
        };
        session::rotate(&session, PrivilegeChange::Logout)
            .await
            .map_err(unavailable)?;
        if let Some(code) = referral {
            session.set(REFERRAL, &code).await.map_err(unavailable)?;
        }
        crate::login_http::keep_return(&session, target)
            .await
            .map_err(unavailable)?;
        session
            .set(GRANT, &String::new())
            .await
            .map_err(unavailable)?;
        session
            .set(PENDING, &challenge.token)
            .await
            .map_err(unavailable)?;
        if let Some(device) = device_cookie(&headers)
            && let Some(auth) = context
                .mfa
                .trust(&challenge.token, &device)
                .await
                .map_err(unavailable)?
        {
            return establish(&context, language, &session, auth).await;
        }
        return Ok(redirect(language, "/mfa/challenge"));
    }
    if let Some(authorization) = context
        .mfa
        .password_session(input)
        .await
        .map_err(unavailable)?
    {
        let account = authorization.account;
        session
            .set(GRANT, &String::new())
            .await
            .map_err(unavailable)?;
        session
            .set(PENDING, &String::new())
            .await
            .map_err(unavailable)?;
        session::rotate(
            &session,
            PrivilegeChange::AuthorizedLogin(account, authorization.generation()),
        )
        .await
        .map_err(unavailable)?;
        validate_rotated_session(&context, &session, &authorization).await?;
        crate::sessions_http::complete_login(&context.mfa.pool(), &session, account).await?;
        return home_or_return(language, &session).await;
    }
    refusal(language, &session, "/login").await
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Input {
    password: String,
    code: String,
    remember: Option<String>,
    device: String,
}
async fn challenge(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_some() {
        return Ok(redirect(language, "/"));
    }
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (None, None, None),
        &[NamedPolicy::IdentityMfaChallengeSource],
    )
    .await
    {
        return Ok(r);
    }
    let Some(token) = pending(&context, &session).await? else {
        return Ok(redirect(language, "/login"));
    };
    let id = context
        .mfa
        .pending_account(&token)
        .await
        .map_err(unavailable)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (None, Some(id.to_string()), None),
        &[
            NamedPolicy::IdentityMfaChallenge,
            NamedPolicy::IdentityMfaChallengeIdentity,
        ],
    )
    .await
    {
        return Ok(r);
    }
    let input = form.map(|v| v.0).unwrap_or_default();
    if let Some(auth) = context
        .mfa
        .complete(
            &token,
            &input.code,
            input.remember.as_deref() == Some("yes"),
        )
        .await
        .map_err(unavailable)?
    {
        return establish(&context, language, &session, auth).await;
    }
    refusal(language, &session, "/mfa/challenge").await
}
async fn email(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_some() {
        return Ok(redirect(language, "/"));
    }
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (None, None, None),
        &[NamedPolicy::IdentityMfaChallengeSource],
    )
    .await
    {
        return Ok(r);
    }
    let Some(token) = pending(&context, &session).await? else {
        return Ok(redirect(language, "/login"));
    };
    let id = context
        .mfa
        .pending_account(&token)
        .await
        .map_err(unavailable)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (None, Some(id.to_string()), None),
        &[
            NamedPolicy::IdentityMfaChallenge,
            NamedPolicy::IdentityMfaChallengeIdentity,
        ],
    )
    .await
    {
        return Ok(r);
    }
    // SMTP must be configured before creating a fallback code; no browser response contains it.
    let host = std::env::var("OTERYN_SMTP_HOST").map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let from = std::env::var("OTERYN_MAIL_FROM").map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let Some((to, code)) = context.mfa.email_code(&token).await.map_err(unavailable)? else {
        return Ok(redirect(language, "/login"));
    };
    let text = Page {
        lang: language.code(),
        language,
        csrf: String::new(),
        mode: "",
        failed: false,
        secret: String::new(),
        codes: vec![],
        devices: vec![],
        remaining_codes: 0,
        native_enabled: false,
        native_devices: vec![],
    };
    let subject = text.text("mfa-email-subject");
    let body = format!("{}\n\n{code}", text.text("mfa-email-body"));
    send_email(&host, &from, &to, subject, body).await?;
    Ok(redirect(language, "/mfa/challenge"))
}

pub(crate) async fn send_email(
    host: &str,
    from: &str,
    to: &str,
    subject: String,
    body: String,
) -> Result<(), StatusCode> {
    use lettre::AsyncTransport;
    let smtp = oteryn_notify::smtp::transport(
        host,
        std::env::var("OTERYN_SMTP_USERNAME").map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        std::env::var("OTERYN_SMTP_PASSWORD").map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let message = lettre::Message::builder()
        .from(from.parse().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?)
        .to(to.parse().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?)
        .subject(subject)
        .body(body)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    smtp.send(message)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(())
}
async fn settings_mode(mfa: &Mfa, id: uuid::Uuid) -> Result<&'static str, StatusCode> {
    Ok(if mfa.enrolled(id).await.map_err(unavailable)? {
        "enabled"
    } else {
        "settings"
    })
}

async fn settings(
    State(context): State<Context>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let Some(id) = session.account_id().await else {
        return Ok(redirect(language, "/login"));
    };
    let devices = if session_satisfied(&context.mfa, &session).await {
        context.mfa.devices(id).await.map_err(unavailable)?
    } else {
        vec![]
    };
    let mut page = page_data(
        language,
        &session,
        settings_mode(&context.mfa, id).await?,
        false,
        String::new(),
        vec![],
        devices,
    )
    .await?;
    if page.mode == "enabled" {
        page.remaining_codes = context.mfa.remaining_codes(id).await.map_err(unavailable)?;
    }
    if context.native_enabled {
        page.native_enabled = true;
        page.native_devices =
            oteryn_gameauth::devices::remembered_for_owner(&context.mfa.pool(), id)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    Ok(oteryn_web::render(&page))
}

async fn enroll(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    let Some(id) = session.account_id().await else {
        return Ok(redirect(language, "/login"));
    };
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (Some(id.to_string()), None, None),
        &[NamedPolicy::IdentityMfaEnrollment],
    )
    .await
    {
        return Ok(r);
    }
    let input = form.map(|v| v.0).unwrap_or_default();
    if let Some(enrollment) = context
        .mfa
        .enroll(
            oteryn_identity::mutation_actor::MutationActor::browser(
                id,
                session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            ),
            &input.password,
        )
        .await
        .map_err(unavailable)?
    {
        return page(
            language,
            &session,
            "enrollment",
            false,
            enrollment.secret,
            vec![],
            vec![],
        )
        .await;
    }
    refusal(language, &session, "/mfa").await
}
async fn confirm(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    let Some(id) = session.account_id().await else {
        return Ok(redirect(language, "/login"));
    };
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (Some(id.to_string()), None, None),
        &[NamedPolicy::IdentityMfaEnrollment],
    )
    .await
    {
        return Ok(r);
    }
    let input = form.map(|v| v.0).unwrap_or_default();
    if let Some(codes) = context
        .mfa
        .confirm(
            oteryn_identity::mutation_actor::MutationActor::browser(
                id,
                session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            ),
            &input.password,
            &input.code,
        )
        .await
        .map_err(unavailable)?
    {
        session::clear_revoked_session(&session).await;
        return page(
            language,
            &session,
            "codes",
            false,
            String::new(),
            codes,
            vec![],
        )
        .await;
    }
    refusal(language, &session, "/mfa").await
}
async fn remove_device(
    State(context): State<Context>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode> {
    let limiter = supplied
        .map(|v| v.0)
        .unwrap_or_else(|| context.limiter.clone());
    let Some(id) = session.account_id().await else {
        return Ok(redirect(language, "/login"));
    };
    if let Err(r) = charge(
        &limiter,
        peer,
        &headers,
        language,
        (Some(id.to_string()), None, None),
        &[NamedPolicy::IdentitySecurityMutation],
    )
    .await
    {
        return Ok(r);
    }
    if !session_satisfied(&context.mfa, &session).await {
        return Err(StatusCode::FORBIDDEN);
    }
    let input = form.map(|v| v.0).unwrap_or_default();
    let removed = context
        .mfa
        .remove_device(
            oteryn_identity::mutation_actor::MutationActor::browser(
                id,
                session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            ),
            &input.device,
        )
        .await
        .map_err(unavailable)?;
    if !removed {
        return Err(StatusCode::FORBIDDEN);
    }
    session::rotate(&session, PrivilegeChange::Reissue)
        .await
        .map_err(unavailable)?;
    settings(State(context), Extension(language), Extension(session)).await
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Replace,
    Remove,
    RotateCodes,
}

#[derive(Deserialize)]
struct ManagementInput {
    action: Operation,
    password: String,
    code: String,
}

#[derive(Deserialize)]
struct ConfirmationInput {
    password: String,
    code: String,
}

struct SecurityRequest {
    language: Language,
    session: BrowserSession,
    id: uuid::Uuid,
}

impl axum::extract::FromRequestParts<Context> for SecurityRequest {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        context: &Context,
    ) -> Result<Self, Response> {
        let language = *parts
            .extensions
            .get::<Language>()
            .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
        let session = parts
            .extensions
            .get::<BrowserSession>()
            .cloned()
            .ok_or_else(|| StatusCode::FORBIDDEN.into_response())?;
        let id = session
            .account_id()
            .await
            .ok_or_else(|| StatusCode::FORBIDDEN.into_response())?;
        let limiter = parts
            .extensions
            .get::<RateLimiter>()
            .unwrap_or(&context.limiter);
        charge(
            limiter,
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .copied()
                .map(Extension),
            &parts.headers,
            language,
            (Some(id.to_string()), None, None),
            &[NamedPolicy::IdentitySecurityMutation],
        )
        .await?;
        Ok(Self {
            language,
            session,
            id,
        })
    }
}

impl SecurityRequest {
    async fn actor(&self) -> Result<MutationActor, StatusCode> {
        Ok(MutationActor::browser(
            self.id,
            self.session.csrf_token().await.map_err(unavailable)?,
        ))
    }

    async fn proof_refusal(
        &self,
        status: StatusCode,
        mode: &'static str,
    ) -> Result<Response, StatusCode> {
        let page = page_data(
            self.language,
            &self.session,
            mode,
            true,
            String::new(),
            vec![],
            vec![],
        )
        .await?;
        Ok(oteryn_web::render_form_refusal(status, &page))
    }

    async fn codes(&self, codes: Vec<String>) -> Result<Response, StatusCode> {
        session::clear_revoked_session(&self.session).await;
        page(
            self.language,
            &self.session,
            "codes",
            false,
            String::new(),
            codes,
            vec![],
        )
        .await
    }
}

fn validate_proof(password: &str, code: &str) -> Result<(), StatusCode> {
    if password.is_empty()
        || password.len() > 1024
        || code.len() != 6
        || !code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

async fn manage(
    State(context): State<Context>,
    request: SecurityRequest,
    Form(input): Form<ManagementInput>,
) -> Result<Response, StatusCode> {
    if let Err(status) = validate_proof(&input.password, &input.code) {
        return request.proof_refusal(status, "management-refused").await;
    }
    let action = match input.action {
        Operation::Replace => ManagementAction::Replace,
        Operation::Remove => ManagementAction::Remove,
        Operation::RotateCodes => ManagementAction::RotateCodes,
    };
    let Some(result) = context
        .mfa
        .manage(request.actor().await?, &input.password, &input.code, action)
        .await
        .map_err(unavailable)?
    else {
        return request
            .proof_refusal(StatusCode::FORBIDDEN, "management-refused")
            .await;
    };
    match result {
        ManagementResult::Replacement(secret) => {
            page(
                request.language,
                &request.session,
                "replacement",
                false,
                secret,
                vec![],
                vec![],
            )
            .await
        }
        ManagementResult::Removed => {
            session::clear_revoked_session(&request.session).await;
            Ok(redirect(request.language, "/login"))
        }
        ManagementResult::Codes(codes) => request.codes(codes).await,
    }
}

async fn confirm_replacement(
    State(context): State<Context>,
    request: SecurityRequest,
    Form(input): Form<ConfirmationInput>,
) -> Result<Response, StatusCode> {
    if let Err(status) = validate_proof(&input.password, &input.code) {
        return request.proof_refusal(status, "replacement").await;
    }
    let Some(codes) = context
        .mfa
        .confirm_replacement(request.actor().await?, &input.password, &input.code)
        .await
        .map_err(unavailable)?
    else {
        return request
            .proof_refusal(StatusCode::FORBIDDEN, "replacement")
            .await;
    };
    request.codes(codes).await
}

#[derive(Deserialize)]
struct NativeInput {
    family: uuid::Uuid,
}

async fn revoke_native(
    State(context): State<Context>,
    request: SecurityRequest,
    Form(input): Form<NativeInput>,
) -> Result<Response, StatusCode> {
    if !context.native_enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    let hash = request
        .session
        .identity_hash()
        .await
        .ok_or(StatusCode::FORBIDDEN)?;
    let mut tx = context.mfa.pool().begin().await.map_err(unavailable)?;
    let active: Option<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE")
        .bind(request.id).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    if active.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }
    let until: Option<OffsetDateTime> = sqlx::query_scalar("SELECT sudo_until FROM web_sessions WHERE id_hash=$1 AND account_id=$2 AND sudo_until>clock_timestamp() AND sudo_until<=clock_timestamp()+interval '15 minutes' AND least(idle_expires_at,absolute_expires_at)>clock_timestamp()")
        .bind(hash).bind(request.id).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    let until = until.ok_or(StatusCode::FORBIDDEN)?;
    let revoked =
        oteryn_gameauth::devices::revoke_remembered_for_owner(&mut tx, request.id, input.family)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !revoked {
        return Err(StatusCode::FORBIDDEN);
    }
    sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'account.native_device.revoked',$2,'{\"revoked\":false}','{\"revoked\":true}',$3)")
        .bind(request.id).bind(input.family.to_string()).bind(uuid::Uuid::now_v7())
        .execute(&mut *tx).await.map_err(unavailable)?;
    let rotation = session::rotate_in_transaction(&request.session, &mut tx, until)
        .await
        .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    session::apply_rotation(&request.session, rotation).await;
    Ok(redirect(request.language, "/mfa"))
}

#[cfg(test)]
#[path = "mfa_http_tests.rs"]
mod tests;
