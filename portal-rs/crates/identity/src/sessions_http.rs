//! Binary composition adapter for session pages and successful login observation.
use askama::Template;
use axum::{
    Extension, Router,
    extract::{ConnectInfo, Path, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_identity::sessions::{Actor, Outcome, Revocation, Sessions};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionKey, SessionLayer},
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::net::SocketAddr;

const FLASH: SessionKey<bool> = SessionKey::new("sessions_inactive");
pub fn configured(app: Router, pool: PgPool) -> Router {
    if !std::env::var("OTERYN_ACCOUNT_SESSIONS").is_ok_and(|v| v == "true") {
        return app;
    }
    let mut layer = SessionLayer::new(pool.clone());
    layer.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    app.merge(router(pool.clone(), layer))
        .layer(axum::middleware::from_fn_with_state(pool, observe_login))
}
fn router(pool: PgPool, layer: SessionLayer) -> Router {
    let mut routes = Router::new();
    for prefix in ["", "/pl", "/en"] {
        routes = routes
            .route(
                &format!("{prefix}/account/sessions"),
                get(show).delete(revoke_others),
            )
            .route(
                &format!("{prefix}/account/sessions/others"),
                post(revoke_others),
            )
            .route(&format!("{prefix}/account/sessions/all"), post(revoke_all))
            .route(
                &format!("{prefix}/account/sessions/{{id}}"),
                post(revoke_one).delete(revoke_one),
            );
    }
    let routes = routes
        .route_layer(axum::middleware::from_fn_with_state(
            layer,
            session::middleware,
        ))
        .layer(Extension(RateLimiter::new(pool.clone())))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(16 * 1024))
        .with_state(pool);
    oteryn_web::ratelimit::finish_routes(routes)
}
fn redirect(path: &str) -> Response {
    let mut response = Redirect::to(path).into_response();
    *response.status_mut() = StatusCode::FOUND;
    response
}
fn cookie_hash(headers: &HeaderMap) -> Option<Vec<u8>> {
    let mut values = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|v| v.trim().split_once('='))
        .filter(|(name, _)| *name == session::COOKIE_NAME)
        .map(|(_, value)| value);
    let value = values.next()?;
    if values.next().is_some() || !valid_cookie(value) {
        return None;
    }
    Some(Sha256::digest(value.as_bytes()).to_vec())
}
fn valid_cookie(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
async fn actor(session: &BrowserSession, headers: &HeaderMap) -> Result<Actor, StatusCode> {
    Ok(Actor {
        account: session.account_id().await.ok_or(StatusCode::FORBIDDEN)?,
        hash: cookie_hash(headers).ok_or(StatusCode::FORBIDDEN)?,
    })
}
#[derive(Template)]
#[template(
    source = r##"<!doctype html><html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ self.text("sessions-title") }} · Oteryn</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><a class="op-skip" href="#main">{{ self.text("skip") }}</a><header class="op-card"><a href="/{{ lang }}/">Oteryn</a></header><main class="op-card op-stack" id="main" tabindex="-1"><h1>{{ self.text("sessions-title") }}</h1><p>{{ self.text("sessions-note") }}</p>{% if let Some(message) = message %}<p role="status">{{ self.text(message) }}</p>{% endif %}
<ul class="op-stack">{% for entry in entries %}<li class="op-card op-stack"><h2>{{ entry.device }}</h2>{% if entry.current %}<p>{{ self.text("sessions-current") }}</p>{% endif %}<dl><dt>{{ self.text("sessions-ip") }}</dt><dd>{{ entry.ip }}</dd><dt>{{ self.text("sessions-seen") }}</dt><dd>{{ entry.time }}</dd></dl><form method="post" action="/{{ lang }}/account/sessions/{{ entry.id }}"><input type="hidden" name="_csrf" value="{{ csrf }}"><button class="op-button" type="submit">{{ self.text("sessions-revoke") }}</button></form></li>{% else %}<li>{{ self.text("sessions-empty") }}</li>{% endfor %}</ul>
<form method="post" action="/{{ lang }}/account/sessions/others"><input type="hidden" name="_csrf" value="{{ csrf }}"><button class="op-button" type="submit">{{ self.text("sessions-others") }}</button></form><form method="post" action="/{{ lang }}/account/sessions/all"><input type="hidden" name="_csrf" value="{{ csrf }}"><button class="op-button" type="submit">{{ self.text("sessions-all") }}</button></form>
<h2>{{ self.text("sessions-history") }}</h2><ul class="op-stack">{% for event in history %}<li class="op-card"><h3>{{ self.text(event.kind) }}</h3><p>{{ event.device }}</p><dl><dt>{{ self.text("sessions-ip") }}</dt><dd>{{ event.ip }}</dd><dt>{{ self.text("sessions-seen") }}</dt><dd>{{ event.time }}</dd></dl></li>{% else %}<li>{{ self.text("sessions-empty") }}</li>{% endfor %}</ul></main></body></html>"##,
    ext = "html"
)]
struct Page {
    lang: &'static str,
    language: Language,
    csrf: String,
    message: Option<&'static str>,
    entries: Vec<Entry>,
    history: Vec<Event>,
}
struct Entry {
    id: uuid::Uuid,
    device: String,
    ip: String,
    time: String,
    current: bool,
}
struct Event {
    kind: &'static str,
    device: String,
    ip: String,
    time: String,
}
impl Page {
    fn text(&self, key: &str) -> String {
        if !key.starts_with("sessions-") {
            return self.language.text(key);
        }
        let source = if self.language == Language::En {
            include_str!("sessions_en.ftl")
        } else {
            include_str!("sessions_pl.ftl")
        };
        let Ok(resource) = FluentResource::try_new(source.into()) else {
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
            return String::new();
        };
        bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned()
    }
}
async fn show(
    State(pool): State<PgPool>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if session.account_id().await.is_none() {
        return Ok(redirect(&format!("/{}/login", language.code())));
    }
    let actor = actor(&session, &headers).await?;
    let (entries, history) = Sessions { pool }
        .list(&actor)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let message = session
        .get(FLASH)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map(|inactive| {
            if inactive {
                "sessions-inactive"
            } else {
                "sessions-done"
            }
        });
    let mut page = Page {
        lang: language.code(),
        language,
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        message,
        entries: Vec::new(),
        history: Vec::new(),
    };
    let unknown = page.text("sessions-unknown");
    let device = |value: String| {
        if value.trim().is_empty() {
            unknown.clone()
        } else {
            value
        }
    };
    page.entries = entries
        .into_iter()
        .map(|e| Entry {
            id: e.session_id,
            device: device(e.user_agent_summary),
            ip: e.ip.unwrap_or_default(),
            time: e.last_seen_at.to_string(),
            current: e.current,
        })
        .collect();
    page.history = history
        .into_iter()
        .map(|e| Event {
            kind: match e.kind.as_str() {
                "login_succeeded" => "sessions-login",
                "session_revoked" => "sessions-one",
                "other_sessions_revoked" => "sessions-other-event",
                _ => "sessions-all-event",
            },
            device: device(e.device),
            ip: e.ip.unwrap_or_default(),
            time: e.created_at.to_string(),
        })
        .collect();
    Ok(oteryn_web::render(&page))
}
async fn mutate(
    pool: PgPool,
    language: Language,
    session: BrowserSession,
    limiter: RateLimiter,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    selection: Revocation,
) -> Result<Response, StatusCode> {
    let actor = actor(&session, &headers).await?;
    let Some(Extension(ConnectInfo(peer))) = peer else {
        return Ok(Denial::Unavailable.response(language, false));
    };
    let source = match limiter.trusted_proxies.client_ip(peer, &headers) {
        Ok(ip) => ip,
        Err(e) => return Ok(e.response(language, false)),
    };
    let mut keys = Keys::source(source);
    keys.account = Some(actor.account.to_string());
    if let Err(e) = limiter
        .check(NamedPolicy::IdentitySessionRevoke, &[keys])
        .await
    {
        return Ok(e.response(language, false));
    }
    let outcome = Sessions { pool }
        .revoke(&actor, selection)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    if outcome == Outcome::SignedOut {
        session::rotate(&session, session::PrivilegeChange::Logout)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        return Ok(redirect(&format!("/{}/login", language.code())));
    }
    session
        .set(FLASH, &(outcome == Outcome::Inactive))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(redirect(&format!("/{}/account/sessions", language.code())))
}
macro_rules! bulk_handler {
    ($name:ident,$selection:expr) => {
        async fn $name(
            State(pool): State<PgPool>,
            Extension(language): Extension<Language>,
            Extension(session): Extension<BrowserSession>,
            Extension(limiter): Extension<RateLimiter>,
            peer: Option<Extension<ConnectInfo<SocketAddr>>>,
            headers: HeaderMap,
        ) -> Result<Response, StatusCode> {
            mutate(pool, language, session, limiter, peer, headers, $selection).await
        }
    };
}
bulk_handler!(revoke_others, Revocation::Others);
bulk_handler!(revoke_all, Revocation::All);
async fn revoke_one(
    State(pool): State<PgPool>,
    Extension(language): Extension<Language>,
    Extension(session): Extension<BrowserSession>,
    Extension(limiter): Extension<RateLimiter>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, StatusCode> {
    let id = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    mutate(
        pool,
        language,
        session,
        limiter,
        peer,
        headers,
        Revocation::One(id),
    )
    .await
}
/// T04 remains unchanged: this composition middleware observes only login redirects
/// carrying a newly rotated authenticated cookie. Refusals never create login events.
async fn observe_login(State(pool): State<PgPool>, request: Request, next: Next) -> Response {
    let login = request.method() == Method::POST
        && ["/login", "/pl/login", "/en/login"].contains(&request.uri().path());
    let response = next.run(request).await;
    if !login || response.status() != StatusCode::FOUND {
        return response;
    }
    let hash = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|v| v.split(';').next())
        .filter_map(|v| v.split_once('='))
        .find(|(name, value)| *name == session::COOKIE_NAME && valid_cookie(value))
        .map(|(_, value)| Sha256::digest(value.as_bytes()).to_vec());
    let Some(hash) = hash else {
        return response;
    };
    let account: Result<Option<uuid::Uuid>, _> = sqlx::query_scalar(
        "SELECT account_id FROM web_sessions WHERE id_hash=$1 AND account_id IS NOT NULL",
    )
    .bind(&hash)
    .fetch_optional(&pool)
    .await;
    let result = match account {
        Ok(Some(account)) => {
            Sessions { pool: pool.clone() }
                .record_login(&Actor {
                    account,
                    hash: hash.clone(),
                })
                .await
        }
        Ok(None) => return response,
        Err(error) => Err(error.into()),
    };
    if result.is_err() {
        // Never release a successful cookie when its security record could not commit.
        let _ = sqlx::query("DELETE FROM web_sessions WHERE id_hash=$1")
            .bind(hash)
            .execute(&pool)
            .await;
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    response
}

pub fn delivery(
    pool: PgPool,
) -> Result<Option<oteryn_notify::Delivery>, Box<dyn std::error::Error>> {
    if !std::env::var("OTERYN_ACCOUNT_SESSIONS").is_ok_and(|v| v == "true") {
        return Ok(None);
    }
    let notifications = oteryn_identity::sessions::security_notifications(pool)
        .map_err(|_| "security notifications unavailable")?;
    let sender = oteryn_notify::SmtpSender::new(
        &std::env::var("OTERYN_SMTP_HOST")?,
        std::env::var("OTERYN_SMTP_USERNAME")?,
        std::env::var("OTERYN_SMTP_PASSWORD")?,
        std::env::var("OTERYN_SMTP_FROM")?,
    )
    .map_err(|_| "SMTP configuration invalid")?;
    Ok(Some(oteryn_notify::Delivery {
        notifications,
        sender: std::sync::Arc::new(sender),
    }))
}

#[cfg(test)]
#[path = "sessions_http_tests.rs"]
mod tests;
