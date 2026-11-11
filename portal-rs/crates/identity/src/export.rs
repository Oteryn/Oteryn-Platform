//! Owner-only JSON export. T08 integration seam: existing browser sessions with sudo.
//! Only explicitly selected fields are exported; credentials and session data never are.
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Debug, Clone, Default)]
pub struct ExportConfig {
    pub enabled: bool,
}

#[derive(Clone)]
struct ExportState {
    pool: PgPool,
    config: ExportConfig,
}

/// No account identifier is accepted from the caller. Ownership comes from the session.
pub async fn download(
    pool: &PgPool,
    config: &ExportConfig,
    session: &str,
) -> Result<Value, StatusCode> {
    if !config.enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    if session.len() != 64
        || !session
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id: Option<uuid::Uuid> = sqlx::query_scalar("SELECT a.id FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.idle_expires_at>now() AND s.absolute_expires_at>now() AND s.sudo_until>now() AND a.status='active' AND a.email_verified_at IS NOT NULL")
        .bind(Sha256::digest(session.as_bytes()).to_vec()).fetch_optional(&mut *tx).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = id.ok_or(StatusCode::FORBIDDEN)?;
    let account: Value = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'email',email,'email_verified_at',email_verified_at,'status',status,'locale',locale,'public_account_association',public_account_association,'public_status_visible',public_status_visible,'created_at',created_at,'updated_at',updated_at) FROM accounts WHERE id=$1")
        .bind(id).fetch_one(&mut *tx).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut result = serde_json::json!({"schema_version":1,"account":account});
    // Ordered aggregates make each section stable and preserve empty sections as arrays.
    for (key, query) in [
        (
            "consents",
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('document_kind',document_kind,'document_version',document_version,'granted',granted,'accepted_at',accepted_at,'actor',actor,'source',source,'ip',host(ip)) ORDER BY document_kind,document_version,accepted_at,id),'[]'::jsonb) FROM account_consents WHERE account_id=$1",
        ),
        (
            "sessions",
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('created_at',created_at,'last_seen_at',last_seen_at,'ip',host(ip),'device',user_agent_summary,'expires_at',absolute_expires_at) ORDER BY created_at,id_hash),'[]'::jsonb) FROM web_sessions WHERE account_id=$1",
        ),
        (
            "notification_preferences",
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('category',category,'email_enabled',email_enabled,'portal_enabled',portal_enabled) ORDER BY category),'[]'::jsonb) FROM account_notification_prefs WHERE account_id=$1",
        ),
        (
            "notifications",
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('id',id,'category',category,'subject',subject,'body',body,'read_at',read_at,'created_at',created_at) ORDER BY created_at,id),'[]'::jsonb) FROM notifications WHERE account_id=$1",
        ),
    ] {
        let section: Value = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        result[key] = section;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(result)
}

fn session_cookie(headers: &HeaderMap) -> Option<&str> {
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

async fn handler(State(state): State<ExportState>, headers: HeaderMap) -> Response {
    let response = match download(
        &state.pool,
        &state.config,
        session_cookie(&headers).unwrap_or_default(),
    )
    .await
    {
        Ok(value) => (
            [(
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"account-data.json\"",
            )],
            axum::Json(value),
        )
            .into_response(),
        Err(status) => status.into_response(),
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

pub fn router(pool: PgPool, config: ExportConfig) -> Router {
    Router::new()
        .route("/pl/account/privacy/export", get(handler))
        .route("/en/account/privacy/export", get(handler))
        .with_state(ExportState { pool, config })
}

pub fn configured_router(pool: PgPool) -> Router {
    router(
        pool,
        ExportConfig {
            enabled: std::env::var("OTERYN_ACCOUNT_EXPORT_ENABLED")
                .is_ok_and(|value| value == "true"),
        },
    )
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
