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

const PRODUCER_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

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
pub async fn stream_download(
    pool: &PgPool,
    config: &ExportConfig,
    session: &str,
) -> Result<axum::body::Body, StatusCode> {
    let deadline = tokio::time::Instant::now() + PRODUCER_DEADLINE;
    tokio::time::timeout_at(deadline, prepare_download(pool, config, session, deadline))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn prepare_download(
    pool: &PgPool,
    config: &ExportConfig,
    session: &str,
    deadline: tokio::time::Instant,
) -> Result<axum::body::Body, StatusCode> {
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
    // Dropping a SQL future need not interrupt PostgreSQL. Bound the server-side
    // transaction too, so an in-flight query cannot outlive cancellation forever.
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    sqlx::query("SELECT set_config('transaction_timeout',$1,true)")
        .bind(format!("{}ms", remaining.as_millis().max(1)))
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id: Option<uuid::Uuid> = sqlx::query_scalar("SELECT a.id FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.idle_expires_at>now() AND s.absolute_expires_at>now() AND s.sudo_until>now() AND a.status='active' AND a.email_verified_at IS NOT NULL")
        .bind(Sha256::digest(session.as_bytes()).to_vec()).fetch_optional(&mut *tx).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = id.ok_or(StatusCode::FORBIDDEN)?;
    let account: Value = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'email',email,'email_verified_at',email_verified_at,'status',status,'locale',locale,'public_account_association',public_account_association,'public_status_visible',public_status_visible,'created_at',created_at,'updated_at',updated_at) FROM accounts WHERE id=$1")
        .bind(id).fetch_one(&mut *tx).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(1);
    let errors = sender.clone();
    let (status_tx, status_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let work = tokio::time::timeout_at(deadline, async move {
            let prefix = format!("{{\"schema_version\":1,\"account\":{account}");
            if sender.send(Ok(prefix.into_bytes())).await.is_err() {
                return Ok(());
            }
            for (key, query) in [
                (
                    "consents",
                    "SELECT jsonb_build_object('document_kind',document_kind,'document_version',document_version,'granted',granted,'accepted_at',accepted_at,'actor',actor,'source',source,'ip',host(ip)) FROM account_consents WHERE account_id=$1 ORDER BY document_kind,document_version,accepted_at,id",
                ),
                (
                    "sessions",
                    "SELECT jsonb_build_object('created_at',created_at,'last_seen_at',last_seen_at,'ip',host(ip),'device',user_agent_summary,'expires_at',absolute_expires_at) FROM web_sessions WHERE account_id=$1 ORDER BY created_at,id_hash",
                ),
                (
                    "notification_preferences",
                    "SELECT jsonb_build_object('category',category,'email_enabled',email_enabled,'portal_enabled',portal_enabled) FROM account_notification_prefs WHERE account_id=$1 ORDER BY category",
                ),
                (
                    "notifications",
                    "SELECT jsonb_build_object('id',id,'category',category,'subject',subject,'body',body,'read_at',read_at,'created_at',created_at) FROM notifications WHERE account_id=$1 ORDER BY created_at,id",
                ),
            ] {
                if sender
                    .send(Ok(format!(",\"{key}\":[").into_bytes()))
                    .await
                    .is_err()
                {
                    return Ok(());
                }
                // A server cursor bounds both PostgreSQL and application-side batches.
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DECLARE export_rows NO SCROLL CURSOR FOR {query}"
                )))
                .bind(id)
                .execute(&mut *tx)
                .await?;
                let mut first = true;
                loop {
                    let rows: Vec<Value> = sqlx::query_scalar("FETCH FORWARD 64 FROM export_rows")
                        .fetch_all(&mut *tx)
                        .await?;
                    let done = rows.len() < 64;
                    for row in rows {
                        let chunk = format!("{}{row}", if first { "" } else { "," });
                        first = false;
                        if sender.send(Ok(chunk.into_bytes())).await.is_err() {
                            return Ok(());
                        }
                    }
                    if done {
                        break;
                    }
                }
                sqlx::query("CLOSE export_rows").execute(&mut *tx).await?;
                if sender.send(Ok(b"]".to_vec())).await.is_err() {
                    return Ok(());
                }
            }
            tx.commit().await?;
            let _ = sender.send(Ok(b"}".to_vec())).await;
            Ok::<(), sqlx::Error>(())
        });
        let outcome = tokio::select! {
            _ = errors.closed() => return,
            outcome = work => outcome,
        };
        // Terminal status is independent of data backpressure and owns no transaction.
        let _ = status_tx.send(matches!(outcome, Ok(Ok(()))));
    });
    Ok(axum::body::Body::from_stream(futures_util::stream::unfold(
        (receiver, Some(status_rx)),
        |(mut receiver, mut status)| async move {
            if let Some(chunk) = receiver.recv().await {
                return Some((chunk, (receiver, status)));
            }
            if let Some(status_rx) = status.take()
                && status_rx.await != Ok(true)
            {
                return Some((
                    Err(std::io::Error::other("account export failed")),
                    (receiver, status),
                ));
            }
            None
        },
    )))
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
    let response = match stream_download(
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
            [(header::CONTENT_TYPE, "application/json")],
            value,
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

#[cfg(test)]
pub async fn download(
    pool: &PgPool,
    config: &ExportConfig,
    session: &str,
) -> Result<Value, StatusCode> {
    let body = stream_download(pool, config, session).await?;
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    serde_json::from_slice(&bytes).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
