//! Browser adapter for personal-data exports.
use crate::export::navigation::{self, SudoReturn};
use crate::export::{
    EXPORT_REAUTH_QUERY, ExportConfig, PRODUCER_DEADLINE, confirmation_access, export_account, stream_download_until,
};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::get,
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
    path = "account/export.html",
    config = "../../templates/account/lifecycle/askama.toml"
)]
struct Confirmation {
    language: Language,
    csrf: Option<String>,
}

impl Confirmation {
    fn text(&self, key: &str) -> String {
        self.language.text(key)
    }

    fn sudo_path(&self) -> String {
        SudoReturn::Export.sudo_path(self.language.code())
    }

    fn overview_enabled(&self) -> bool {
        navigation::overview_enabled()
    }
}

#[derive(serde::Deserialize)]
struct ConfirmationQuery {
    #[serde(default)]
    start: bool,
}

async fn confirmation(
    State(state): State<ExportState>,
    Extension(language): Extension<Language>,
    Extension(browser): Extension<BrowserSession>,
    query: Result<Query<ConfirmationQuery>, axum::extract::rejection::QueryRejection>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    let Ok(Query(query)) = query else {
        return private_response(StatusCode::BAD_REQUEST.into_response());
    };
    let session = session_cookie(&headers).unwrap_or_default();
    let result = confirmation_access(&state.pool, &state.config, session).await;
    let response = match result {
        Ok(true) => match browser.csrf_token().await {
            Ok(csrf) => oteryn_web::render(&Confirmation {
                language,
                csrf: Some(csrf),
            }),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
        Ok(false) if query.start && navigation::sudo_enabled() => {
            Redirect::to(&SudoReturn::Export.sudo_path(language.code())).into_response()
        }
        Ok(false)
            if method == Method::GET
                && navigation::sudo_enabled()
                && headers.get("sec-fetch-site").is_some_and(|site| site == "same-origin")
                && headers.get(header::ACCEPT)
                    .and_then(|accept| accept.to_str().ok())
                    .is_some_and(|accept| accept.contains("text/html"))
                && browser.account_id().await.is_some() =>
        {
            Redirect::to(&format!("/{}/account/sudo?{EXPORT_REAUTH_QUERY}", language.code()))
                .into_response()
        }
        Ok(false) if navigation::sudo_enabled() => (
            StatusCode::FORBIDDEN,
            oteryn_web::render(&Confirmation {
                language,
                csrf: None,
            }),
        )
            .into_response(),
        Ok(false) => StatusCode::FORBIDDEN.into_response(),
        Err(status) => status.into_response(),
    };
    private_response(response)
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

pub(super) fn router(pool: PgPool, config: ExportConfig) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    if let Ok(origin) = std::env::var("OTERYN_PUBLIC_ORIGIN") {
        sessions.csrf.trusted_origins.push(origin);
    }
    Router::new()
        .route(
            "/pl/account/privacy/export",
            get(confirmation)
                .post(handler)
                .layer(Extension(Language::Pl)),
        )
        .route(
            "/en/account/privacy/export",
            get(confirmation)
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
        })
}
