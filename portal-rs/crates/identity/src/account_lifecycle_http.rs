//! Account UI composition over notification and termination services.
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::{
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
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_email_changes WHERE account_id=$1 AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL AND expires_at>clock_timestamp()) OR EXISTS(SELECT 1 FROM account_characters WHERE account_id=$1)").bind(account).fetch_one(&mut **tx).await?;
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
            .route("/{lang}/account/notifications", get(preferences).post(save))
            .route("/{lang}/unsubscribe", get(unsubscribe_page));
    }
    let mut routes = routes.route_layer(axum::middleware::from_fn_with_state(
        sessions,
        oteryn_web::session::middleware,
    ));
    if notifications.is_some() {
        routes = routes
            .merge(Router::new().route("/{lang}/unsubscribe", axum::routing::post(unsubscribe)));
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
    news_email: bool,
    news_portal: bool,
    support_email: bool,
    support_portal: bool,
}
impl Page {
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
        news_email: false,
        news_portal: true,
        support_email: false,
        support_portal: true,
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
    (p.news_email, p.news_portal) = st
        .notifications
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .preference(id, oteryn_notify::Category::News)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    (p.support_email, p.support_portal) = st
        .notifications
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .preference(id, oteryn_notify::Category::Support)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
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
    let mut tx = st
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let actor = oteryn_identity::mutation_actor::MutationActor::browser(
        id,
        s.csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    );
    if !actor
        .live(&mut tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
    for (category, email, portal) in [
        (
            oteryn_notify::Category::News,
            input.news_email,
            input.news_portal,
        ),
        (
            oteryn_notify::Category::Support,
            input.support_email,
            input.support_portal,
        ),
    ] {
        st.notifications
            .as_ref()
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
            .preferences(&mut tx, id, category, boolean(email)?, boolean(portal)?)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Redirect::to(&format!("/{}/account/notifications", l.code())).into_response())
}
#[derive(serde::Deserialize, Default)]
struct Token {
    #[serde(default)]
    token: String,
}
async fn unsubscribe_page(
    Extension(l): Extension<Language>,
    Extension(s): Extension<BrowserSession>,
    axum::extract::Query(q): axum::extract::Query<Token>,
) -> Result<Response, StatusCode> {
    if q.token.len() > 512 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(render(page(l, &s, "", false, false, true, q.token).await?))
}
async fn unsubscribe(
    State(st): State<Services>,
    supplied: Option<Extension<RateLimiter>>,
    Extension(l): Extension<Language>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<Token>,
) -> Result<Response, StatusCode> {
    let limiter = supplied.map(|v| v.0).unwrap_or_else(|| st.limiter.clone());
    if let Err(r) = rate(
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
        return Ok(r);
    }
    let mut tx = st
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    st.notifications
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .unsubscribe(&mut tx, &q.token)
        .await
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
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
    } else {
        format!("/{}/account/dashboard", l.code())
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
