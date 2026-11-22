//! Browser adapter for personal-data exports.
use crate::export::{ExportConfig, PRODUCER_DEADLINE, export_account, stream_download_until};
use axum::{
    Extension, Router,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_web::{
    Language,
    ratelimit::{Denial, Keys, NamedPolicy, RateLimiter},
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
    let result = tokio::time::timeout_at(deadline, result)
        .await
        .unwrap_or_else(|_| Err(StatusCode::SERVICE_UNAVAILABLE.into_response()));
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
    let mut response = response;
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
    Router::new()
        .route(
            "/pl/account/privacy/export",
            get(handler).layer(Extension(Language::Pl)),
        )
        .route(
            "/en/account/privacy/export",
            get(handler).layer(Extension(Language::En)),
        )
        .with_state(ExportState {
            limiter: RateLimiter::new(pool.clone()),
            pool,
            config,
        })
}
