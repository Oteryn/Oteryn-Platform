//! Account UI composition over notification and termination services.
use askama::Template;
use axum::{
    Extension, Form, Router,
    body::Bytes,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::{
    notification_preferences::{self, Choices, MailLanguage},
    sessions::NotificationAccounts as AccountState,
    termination::{self, ActorProof, BoundaryFuture},
};
use oteryn_web::{
    Language,
    ratelimit::{Keys, NamedPolicy, RateLimiter},
    session::{BrowserSession, SessionLayer},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;
#[derive(Clone)]
struct Services {
    limiter: RateLimiter,
    pool: PgPool,
    notifications: Option<Arc<oteryn_notify::Notifications>>,
    termination: termination::Terminations,
}
#[derive(Debug)]
pub(crate) struct TerminationBoundary;
impl termination::Boundary for TerminationBoundary {
    fn authorize<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        proof: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp() AND sudo_until>clock_timestamp() AND sudo_until<=clock_timestamp()+interval '15 minutes')").bind(Sha256::digest(proof.0.as_bytes()).to_vec()).bind(account).fetch_one(&mut **tx).await?;
            if valid {
                Ok(())
            } else {
                Err(termination::Error::Unauthorized)
            }
        })
    }
    fn requested<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: &'a termination::Termination,
    ) -> BoundaryFuture<'a> {
        // Only Game can initialize new closure work.
        Box::pin(async { Err(termination::Error::UnknownState) })
    }
    fn cancelled<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        termination: &'a termination::Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            let issued: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM plat3_closure_work WHERE termination_id=$1 AND close_operation_id IS NOT NULL)")
                .bind(termination.id).fetch_one(&mut **tx).await?;
            if issued {
                return Err(termination::Error::UnknownState);
            }
            Ok(())
        })
    }
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            if !termination::game_closure::runtime::financial_obligations_clear(tx, account).await?
            {
                return Err(termination::Error::UnknownState);
            }
            // Game's CloseAccount detaches characters after intake. Its finalization
            // proof requires a committed receipt and an empty character projection.
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM account_email_changes WHERE account_id=$1
                    AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL
                    AND expires_at>clock_timestamp())",
            )
            .bind(account)
            .fetch_one(&mut **tx)
            .await?;
            if blocked {
                Err(termination::Error::UnknownState)
            } else {
                Ok(())
            }
        })
    }
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        finalizing: bool,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            // Finalization needs the separate Game lifecycle contract and remains unavailable.
            if finalizing {
                return Err(termination::Error::UnknownState);
            }
            oteryn_identity::recovery_key::revoke_authorizations(tx, account).await?;
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                .bind(account)
                .execute(&mut **tx)
                .await?;
            Ok(())
        })
    }
}
pub fn configured_router(pool: PgPool, termination: termination::Terminations) -> Router {
    let key = std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_KEY").unwrap_or_default();
    let notifications = oteryn_notify::Unsubscribe::new(
        key.into_bytes(),
        std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_URL").unwrap_or_default(),
    )
    .ok()
    .map(|unsubscribe| {
        Arc::new(oteryn_notify::Notifications {
            pool: pool.clone(),
            accounts: Arc::new(AccountState(pool.clone())),
            unsubscribe,
        })
    });
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let mut routes = Router::new()
        .route("/{lang}/account/notifications", get(preferences).post(save))
        .route("/{lang}/account/privacy/delete", get(deletion))
        .route(
            "/{lang}/account/privacy/delete/cancel",
            axum::routing::post(cancel),
        );
    if termination.accepts_requests() {
        routes = routes.route(
            "/{lang}/account/privacy/delete",
            axum::routing::post(delete),
        );
    }
    if notifications.is_some() {
        routes = routes
            .route("/{lang}/unsubscribe", get(unsubscribe_page))
            .route(
                "/{lang}/unsubscribe/confirm",
                axum::routing::post(confirm_unsubscribe),
            );
    }
    let mut routes = routes.route_layer(axum::middleware::from_fn_with_state(
        sessions,
        oteryn_web::session::middleware,
    ));
    if notifications.is_some() {
        routes = routes.merge(Router::new().route(
            "/{lang}/unsubscribe",
            axum::routing::post(one_click_unsubscribe),
        ));
    }
    let routes = routes
        .with_state(Services {
            limiter: RateLimiter::new(pool.clone()),
            pool,
            notifications,
            termination,
        })
        .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024));
    oteryn_web::ratelimit::finish_routes(routes)
        .layer(axum::middleware::from_fn(unsubscribe_referrer))
}

async fn unsubscribe_referrer(request: Request, next: axum::middleware::Next) -> Response {
    let unsubscribe = matches!(
        request.uri().path(),
        "/en/unsubscribe"
            | "/pl/unsubscribe"
            | "/en/unsubscribe/confirm"
            | "/pl/unsubscribe/confirm"
    );
    let mut response = next.run(request).await;
    if unsubscribe {
        response.headers_mut().insert(
            header::REFERRER_POLICY,
            axum::http::HeaderValue::from_static("origin"),
        );
    }
    response
}
fn text(l: Language, key: &str) -> String {
    let source = if l.code() == "en" {
        include_str!("account_lifecycle_en.ftl")
    } else {
        include_str!("account_lifecycle_pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.into()) else {
        return String::new();
    };
    let Ok(locale) = l.code().parse() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(
    path = "account/lifecycle.html",
    config = "../../templates/account/lifecycle/askama.toml"
)]
struct Page {
    lang: String,
    title: String,
    csrf: String,
    action: String,
    delete: bool,
    pending: bool,
    restoration_pending: bool,
    unsubscribe: bool,
    token: String,
    mail_locale: String,
    choices: Choices,
}
impl Page {
    fn shell_text(&self, key: &str) -> String {
        oteryn_ui::text(&self.lang, key)
    }

    fn overview_enabled(&self) -> bool {
        oteryn_identity::navigation::overview_enabled()
    }

    fn sudo_path(&self) -> Option<String> {
        use oteryn_identity::navigation::{SudoReturn, sudo_enabled};
        sudo_enabled().then(|| SudoReturn::Closure.sudo_path(&self.lang))
    }

    fn text(&self, key: &str) -> String {
        text(
            if self.lang == "en" {
                Language::En
            } else {
                Language::Pl
            },
            key,
        )
    }
}
async fn page(
    l: Language,
    s: &BrowserSession,
    action: &str,
    delete: bool,
    pending: bool,
    unsubscribe: bool,
    token: String,
) -> Result<Page, StatusCode> {
    Ok(Page {
        lang: l.code().into(),
        title: text(
            l,
            if delete {
                "delete"
            } else if unsubscribe {
                "unsubscribe"
            } else {
                "preferences"
            },
        ),
        csrf: s
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        action: action.into(),
        delete,
        pending,
        restoration_pending: false,
        unsubscribe,
        token,
        mail_locale: l.code().into(),
        choices: Choices::default(),
    })
}
fn render(p: Page) -> Response {
    let mut r = oteryn_web::render(&p);
    r.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    r
}
async fn preferences(
    State(st): State<Services>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let id = s.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let mut p = page(l, &s, "", false, false, false, String::new()).await?;
    let preferences = notification_preferences::current(&st.pool, id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::FORBIDDEN)?;
    p.mail_locale = preferences.language;
    p.choices = preferences.choices;
    Ok(render(p))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PreferenceInput {
    _csrf: String,
    news_email: Option<String>,
    news_portal: Option<String>,
    support_email: Option<String>,
    support_portal: Option<String>,
    events_email: Option<String>,
    forum_digest_email: Option<String>,
    mail_locale: Option<MailLanguage>,
}
fn boolean(v: Option<String>) -> Result<bool, StatusCode> {
    match v.as_deref() {
        None => Ok(false),
        Some("true") => Ok(true),
        _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
}
async fn rate(
    limiter: &RateLimiter,
    headers: &HeaderMap,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    account: Option<Uuid>,
    l: Language,
    policy: NamedPolicy,
    token: Option<&str>,
) -> Result<(), Response> {
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Err(oteryn_web::ratelimit::Denial::Unavailable.response(l, false));
    };
    let source = limiter
        .trusted_proxies
        .client_ip(peer, headers)
        .map_err(|e| e.response(l, false))?;
    let mut keys = Keys::source(source);
    keys.account = account.map(|a| a.to_string());
    keys.token = token.map(|t| format!("{:x}", Sha256::digest(t.as_bytes())));
    limiter
        .check(policy, &[keys])
        .await
        .map_err(|e| e.response(l, false))
}
async fn save(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Form(input): Form<PreferenceInput>,
) -> Result<Response, StatusCode> {
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    let id = s.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    if let Err(r) = rate(
        &limiter,
        &headers,
        peer,
        Some(id),
        l,
        NamedPolicy::IdentitySecurityMutation,
        None,
    )
    .await
    {
        return Ok(r);
    }
    if input.mail_locale.is_none()
        && (input.events_email.is_some() || input.forum_digest_email.is_some())
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let choices = Choices {
        news: (boolean(input.news_email)?, boolean(input.news_portal)?),
        support: (
            boolean(input.support_email)?,
            boolean(input.support_portal)?,
        ),
        events: boolean(input.events_email)?,
        forum_digest: boolean(input.forum_digest_email)?,
    };
    let actor = oteryn_identity::mutation_actor::MutationActor::browser(
        id,
        s.csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    );
    if !notification_preferences::save(&st.pool, &actor, input.mail_locale, choices)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Redirect::to(&format!("/{}/account/notifications", l.code())).into_response())
}
#[derive(serde::Deserialize, Default)]
struct Token {
    #[serde(default)]
    token: String,
}
async fn unsubscribe_page(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<Token>,
) -> Result<Response, StatusCode> {
    if q.token.is_empty() {
        return Ok(oteryn_web::render(&Message {
            lang: l.code(),
            title: text(l, "unsubscribe-link-required"),
        }));
    }
    unsubscribe_target(&st, &q.token)?;
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    if let Err(response) = rate(
        &limiter,
        &headers,
        peer,
        None,
        l,
        NamedPolicy::IdentityEmailToken,
        Some(&q.token),
    )
    .await
    {
        return Ok(response);
    }
    let action = format!("/{}/unsubscribe/confirm", l.code());
    Ok(render(
        page(l, &s, &action, false, false, true, q.token).await?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UnsubscribeConfirmation {
    _csrf: String,
    token: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OneClickForm {
    #[serde(rename = "List-Unsubscribe")]
    action: String,
}

async fn confirm_unsubscribe(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Form(input): Form<UnsubscribeConfirmation>,
) -> Result<Response, StatusCode> {
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    unsubscribe(&st, &limiter, l, peer, &headers, &input.token).await
}

async fn one_click_unsubscribe(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<Token>,
    body: Bytes,
) -> Result<Response, StatusCode> {
    if headers.contains_key(header::ORIGIN) || headers.contains_key("sec-fetch-site") {
        return Err(StatusCode::FORBIDDEN);
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let mut media = content_type.split(';');
    let kind = media.next().unwrap_or_default().trim();
    let confirmed = if kind.eq_ignore_ascii_case("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes::<OneClickForm>(&body)
            .is_ok_and(|input| input.action == "One-Click")
    } else if kind.eq_ignore_ascii_case("multipart/form-data") {
        let boundary = media.find_map(|parameter| mime_parameter(parameter, "boundary"));
        boundary.is_some_and(|boundary| multipart_one_click(boundary, &body))
    } else {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };
    if !confirmed {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    unsubscribe(&st, &limiter, l, peer, &headers, &q.token).await
}

fn mime_parameter<'a>(parameter: &'a str, key: &str) -> Option<&'a str> {
    let (name, value) = parameter.split_once('=')?;
    let value = value.trim();
    name.trim().eq_ignore_ascii_case(key).then(|| {
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value)
    })
}

fn multipart_one_click(boundary: &str, body: &[u8]) -> bool {
    // RFC 8058 defines a single ASCII form field. Match its complete MIME part.
    if boundary.is_empty() || boundary.len() > 70 || boundary.contains(['\r', '\n']) {
        return false;
    }
    let Ok(body) = std::str::from_utf8(body) else {
        return false;
    };
    let opening = format!("--{boundary}\r\n");
    let closing = format!("\r\n--{boundary}--");
    let Some((headers, content)) = body
        .strip_prefix(&opening)
        .and_then(|body| body.split_once("\r\n\r\n"))
    else {
        return false;
    };
    let mut dispositions = headers.split("\r\n").filter_map(|header| {
        let (name, value) = header.split_once(':')?;
        name.eq_ignore_ascii_case("content-disposition")
            .then_some(value)
    });
    let Some(disposition) = dispositions.next() else {
        return false;
    };
    let mut parts = disposition.split(';');
    dispositions.next().is_none()
        && parts
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("form-data"))
        && parts.find_map(|part| mime_parameter(part, "name")) == Some("List-Unsubscribe")
        && content
            .strip_prefix("One-Click")
            .and_then(|content| content.strip_prefix(&closing))
            .is_some_and(|tail| matches!(tail, "" | "\r\n"))
}

fn unsubscribe_target<'a>(
    st: &'a Services,
    token: &str,
) -> Result<(&'a oteryn_notify::Notifications, Uuid), StatusCode> {
    if token.is_empty() || token.len() > 256 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let notifications = st
        .notifications
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let (account, _) = notifications
        .unsubscribe
        .verify(
            token,
            sqlx::types::time::OffsetDateTime::now_utc().unix_timestamp(),
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    Ok((notifications, account))
}

async fn unsubscribe(
    st: &Services,
    limiter: &RateLimiter,
    l: Language,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: &HeaderMap,
    token: &str,
) -> Result<Response, StatusCode> {
    let (notifications, account) = unsubscribe_target(st, token)?;
    if let Err(r) = rate(
        limiter,
        headers,
        peer,
        None,
        l,
        NamedPolicy::IdentityEmailToken,
        Some(token),
    )
    .await
    {
        return Ok(r);
    }
    let mut tx = st
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    oteryn_db::mutation_budget(&mut tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(account)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    notifications
        .unsubscribe(&mut tx, token)
        .await
        .map_err(|error| {
            if error.is::<sqlx::Error>() {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            }
        })?;
    tx.commit()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut r = oteryn_web::render(&Message {
        lang: l.code(),
        title: text(l, "unsubscribed"),
    });
    r.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok(r)
}
async fn deletion(
    State(st): State<Services>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let id = s.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let pending = st.termination.has_pending(id).await.map_err(failure)?;
    if !pending && !st.termination.accepts_requests() {
        return Err(StatusCode::NOT_FOUND);
    }
    let action = if pending {
        format!("/{}/account/privacy/delete/cancel", l.code())
    } else {
        String::new()
    };
    let mut page = page(l, &s, &action, true, pending, false, String::new()).await?;
    page.restoration_pending=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM plat3_closure_work w WHERE w.account_id=$1 AND w.generation=(SELECT max(latest.generation) FROM plat3_closure_work latest WHERE latest.account_id=w.account_id) AND w.resolution='CANCEL' AND w.close_operation_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM plat3_closure_commands c JOIN plat3_closure_receipts r USING(operation_id) JOIN plat3_closure_snapshots s ON s.account_id=w.account_id WHERE c.termination_id=w.termination_id AND c.command='ResolveAccountClosure' AND NOT s.invalid AND convert_from(r.receipt,'UTF8')::jsonb->'result_data'->>'closure_state'='CANCELLED' AND convert_from(s.snapshot,'UTF8')::jsonb->'closures'->0->>'operation_id'=c.operation_id::text AND convert_from(s.snapshot,'UTF8')::jsonb->'closures'->0->>'receipt_digest'=encode(r.receipt_sha256,'hex')))").bind(id).fetch_one(&st.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(render(page))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteInput {
    _csrf: String,
    current_password: String,
    #[serde(default)]
    confirmation: String,
}
fn proof(headers: &HeaderMap) -> ActorProof {
    ActorProof(
        headers
            .get_all("cookie")
            .iter()
            .filter_map(|field| field.to_str().ok())
            .flat_map(|field| field.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find_map(|(name, value)| (name == oteryn_web::session::COOKIE_NAME).then_some(value))
            .unwrap_or_default()
            .into(),
    )
}
fn failure(e: termination::Error) -> StatusCode {
    match e {
        termination::Error::InvalidInput => StatusCode::UNPROCESSABLE_ENTITY,
        termination::Error::Unauthorized => StatusCode::FORBIDDEN,
        termination::Error::NoPending => StatusCode::NOT_FOUND,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
async fn delete(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Form(input): Form<DeleteInput>,
) -> Result<Response, StatusCode> {
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    let id = s.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    if let Err(r) = rate(
        &limiter,
        &headers,
        peer,
        Some(id),
        l,
        NamedPolicy::IdentityTermination,
        None,
    )
    .await
    {
        return Ok(r);
    }
    st.termination
        .request(
            id,
            &proof(&headers),
            &input.current_password,
            &input.confirmation,
        )
        .await
        .map_err(failure)?;
    Ok(Redirect::to(&format!("/{}/login", l.code())).into_response())
}
async fn cancel(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Form(input): Form<DeleteInput>,
) -> Result<Response, StatusCode> {
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    let id = s.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    if !st.termination.has_pending(id).await.map_err(failure)? {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Err(r) = rate(
        &limiter,
        &headers,
        peer,
        Some(id),
        l,
        NamedPolicy::IdentitySecurityMutation,
        None,
    )
    .await
    {
        return Ok(r);
    }
    st.termination
        .cancel(id, &proof(&headers), &input.current_password)
        .await
        .map_err(failure)?;
    let destination = if st.termination.accepts_requests() {
        format!("/{}/account/privacy/delete", l.code())
    } else if oteryn_identity::navigation::overview_enabled() {
        "/account".to_owned()
    } else if std::env::var("OTERYN_ACCOUNT_PRIVACY_ENABLED").as_deref() == Ok("true") {
        format!("/{}/account/privacy", l.code())
    } else {
        format!("/{}/", l.code())
    };
    Ok(Redirect::to(&destination).into_response())
}

#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><main><h1>{{ title }}</h1></main></body></html>",
    ext = "html"
)]
struct Message<'a> {
    lang: &'a str,
    title: String,
}

#[cfg(test)]
pub(crate) async fn test_termination_actor(
    pool: &PgPool,
    account: Uuid,
    password: &str,
) -> Result<ActorProof, Box<dyn std::error::Error>> {
    let hash = oteryn_identity::password::hash(password)?;
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(account)
        .bind(hash)
        .execute(pool)
        .await?;
    let token = oteryn_identity::mfa::random_token()?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until)
                 VALUES($1,$2,$3,'closure financial fixture',now()+interval '1 hour',now()+interval '1 day',now()+interval '5 minutes')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind(oteryn_identity::mfa::random_token()?).execute(pool).await?;
    Ok(ActorProof(token))
}

#[cfg(test)]
pub(crate) async fn request_test_termination(
    pool: &PgPool,
    account: Uuid,
) -> Result<termination::Result<termination::Termination>, Box<dyn std::error::Error>> {
    let password = "closure financial fixture password";
    let actor = test_termination_actor(pool, account, password).await?;
    let service = test_game_terminations(pool)?;
    Ok(service
        .request(account, &actor, password, "TERMINATE")
        .await)
}

#[cfg(test)]
fn test_game_terminations(
    pool: &PgPool,
) -> Result<termination::Terminations, Box<dyn std::error::Error>> {
    // Accepting a request must initialize Game closure work in the same transaction.
    let boundary = termination::game_closure::runtime::Config::load(|key| match key {
        "PORTAL_ACCOUNT_CLOSURE_ENABLED" => Some("true".into()),
        "PORTAL_ACCOUNT_CLOSURE_RECOVERY_GENERATION" => Some("3".into()),
        _ => None,
    })?
    .boundary(Arc::new(TerminationBoundary), true)?
    .ok_or("closure boundary disabled")?;
    Ok(termination::Terminations::new(
        pool.clone(),
        termination::Config {
            enabled: true,
            ..Default::default()
        },
        boundary,
    )?)
}

#[cfg(test)]
mod deletion_tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn postgres_delete_with_character_creates_pending_game_closure()
    -> Result<(), Box<dyn std::error::Error>> {
        let Some(_guard) = oteryn_db::test_support::global_lock_guard().await? else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let (pool, admin, schema) = crate::privacy_web::test_support::database(&url).await?;
        let result = post_delete_with_character(&pool).await;
        crate::privacy_web::test_support::close(pool, admin, schema).await?;
        result
    }

    async fn post_delete_with_character(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
        let account: Uuid = sqlx::query_scalar(
            "INSERT INTO accounts(email,password_hash,status,locale)
             VALUES('closure@example.test','!fixture','active','en') RETURNING id",
        )
        .fetch_one(pool)
        .await?;
        let character = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
             VALUES($1,$2,$3,'Closure Fixture','AVAILABLE')",
        )
        .bind(account)
        .bind(character)
        .bind(Uuid::now_v7())
        .execute(pool)
        .await?;
        let password = "closure fixture password";
        let actor = test_termination_actor(pool, account, password).await?;
        let csrf: String =
            sqlx::query_scalar("SELECT csrf_token FROM web_sessions WHERE id_hash=$1")
                .bind(Sha256::digest(actor.0.as_bytes()).to_vec())
                .fetch_one(pool)
                .await?;
        let form = serde_urlencoded::to_string([
            ("_csrf", csrf.as_str()),
            ("current_password", password),
            ("confirmation", "TERMINATE"),
        ])?;
        let peer: SocketAddr = "127.0.0.1:12345".parse()?;
        let response = configured_router(pool.clone(), test_game_terminations(pool)?)
            .layer(Extension(Language::En))
            .layer(Extension(ConnectInfo(peer)))
            .oneshot(
                Request::post("/en/account/privacy/delete")
                    .header(
                        "cookie",
                        format!("{}={}", oteryn_web::session::COOKIE_NAME, actor.0),
                    )
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok()),
            Some("/en/login")
        );
        let work_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM account_terminations t JOIN plat3_closure_work w
                 ON w.termination_id=t.id AND w.account_id=t.account_id
             WHERE t.account_id=$1 AND t.cancelled_at IS NULL AND t.finalized_at IS NULL
                 AND w.close_operation_id IS NULL AND w.resolution IS NULL AND NOT w.invalid",
        )
        .bind(account)
        .fetch_one(pool)
        .await?;
        assert_eq!(work_count, 1);
        let projected_character: Uuid =
            sqlx::query_scalar("SELECT character_id FROM account_characters WHERE account_id=$1")
                .bind(account)
                .fetch_one(pool)
                .await?;
        assert_eq!(projected_character, character);
        Ok(())
    }
}
#[cfg(test)]
mod proof_tests {
    use super::*;

    #[test]
    fn r9_termination_proof_reads_second_cookie_field()
    -> Result<(), axum::http::header::InvalidHeaderValue> {
        let mut headers = HeaderMap::new();
        headers.append("cookie", "lang=en".parse()?);
        headers.append("cookie", "__Host-oteryn_session=session-id".parse()?);
        assert_eq!(proof(&headers).0, "session-id");
        Ok(())
    }
}
