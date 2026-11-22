//! Browser adapter for personal-data exports.
use crate::export::{
    ExportConfig, PRODUCER_DEADLINE, export_account, needs_password_confirmation,
    stream_download_until,
};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::{IntoResponse, Redirect, Response},
};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionLayer},
};
use sqlx::PgPool;
use std::net::SocketAddr;

#[derive(Clone)]
struct ExportState {
    pool: PgPool,
    config: ExportConfig,
    password_confirmation: bool,
    limiter: RateLimiter,
}

pub(super) fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for part in header.to_str().ok()?.split(';') {
            if let Some(("__Host-oteryn_session", value)) = part.trim().split_once('=') {
                if found.is_some() {
                    return None;
                }
                found = Some(value);
            }
        }
    }
    found
}

#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/app.css"></head><body class="op-theme"><main><h1>{{ title }}</h1><form method="post"><input type="hidden" name="_csrf" value="{{ csrf }}"><button type="submit">{{ title }}</button></form></main></body></html>"#,
    ext = "html"
)]
struct Confirmation<'a> {
    lang: &'a str,
    title: &'a str,
    csrf: String,
}

async fn confirmation(
    State(state): State<ExportState>,
    Extension(language): Extension<Language>,
    Extension(browser): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let session = session_cookie(&headers).unwrap_or_default();
    let result = async {
        match export_account(&state.pool, &state.config, session).await {
            Ok(_) => {}
            Err(StatusCode::FORBIDDEN)
                if state.password_confirmation
                    && needs_password_confirmation(&state.pool, &state.config, session).await? =>
            {
                return Ok(Redirect::to(&format!(
                    "/{}/account/sudo?return_to=export",
                    language.code(),
                ))
                .into_response());
            }
            Err(status) => return Err(status),
        }
        let csrf = browser
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok::<_, StatusCode>(oteryn_web::render(&Confirmation {
            lang: language.code(),
            title: match language {
                Language::Pl => "Pobierz dane konta",
                Language::En => "Download account data",
            },
            csrf,
        }))
    }
    .await;
    private_response(result.into_response())
}

async fn handler(
    State(state): State<ExportState>,
    Extension(language): Extension<Language>,
    supplied: Option<Extension<RateLimiter>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
) -> Response {
    let session = session_cookie(&headers).unwrap_or_default();
    let deadline = tokio::time::Instant::now() + PRODUCER_DEADLINE;
    let result = async {
        // Charge before opening the streaming transaction. Holding a pool
        // connection while the limiter acquires another can starve both queries.
        let account = export_account(&state.pool, &state.config, session)
            .await
            .map_err(IntoResponse::into_response)?;
        let limiter = supplied.as_ref().map_or(&state.limiter, |value| &value.0);
        let charge = async {
            let Some(Extension(ConnectInfo(peer))) = peer else {
                return Err(Denial::Unavailable);
            };
            let mut keys = Keys::source(limiter.trusted_proxies.client_ip(peer, &headers)?);
            keys.account = Some(account.to_string());
            limiter
                .check(NamedPolicy::IdentityDataExport, &[keys])
                .await
        };
        charge
            .await
            .map_err(|error| error.response(language, false))?;
        // Revalidate the session inside the same snapshot used for the download.
        stream_download_until(&state.pool, &state.config, session, deadline)
            .await
            .map_err(IntoResponse::into_response)
    };
    let result = match tokio::time::timeout_at(deadline, result).await {
        Ok(result) => result,
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE.into_response()),
    };
    let response = match result {
        Ok(value) => (
            [(
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"account-data.json\"",
            )],
            [(header::CONTENT_TYPE, "application/json")],
            value,
        )
            .into_response(),
        Err(response) => response,
    };
    private_response(response)
}

fn private_response(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(oteryn_web::chrome::PORTAL_CONTENT_SECURITY_POLICY),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response
}

pub(super) fn router(pool: PgPool, config: ExportConfig, password_confirmation: bool) -> Router {
    if !config.enabled {
        return Router::new();
    }
    let mut sessions = SessionLayer::new(pool.clone());
    if let Ok(origin) = std::env::var("OTERYN_PUBLIC_ORIGIN") {
        sessions.csrf.trusted_origins.push(origin);
    }
    Router::new()
        .route(
            "/pl/account/privacy/export",
            oteryn_web::html::get(confirmation)
                .post(handler)
                .layer(Extension(Language::Pl)),
        )
        .route(
            "/en/account/privacy/export",
            oteryn_web::html::get(confirmation)
                .post(handler)
                .layer(Extension(Language::En)),
        )
        .route_layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .with_state(ExportState {
            limiter: RateLimiter::new(pool.clone()),
            pool,
            config,
            password_confirmation,
        })
}
