//! Server-side browser sessions. Raw identifiers are never persisted or logged.

use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    types::{Uuid, time::OffsetDateTime},
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::Mutex;

pub const COOKIE_NAME: &str = "__Host-oteryn_session";

fn referral_activity_enabled() -> bool {
    std::env::var("PORTAL_REFERRALS_ENABLED").as_deref() == Ok("true")
}

/// Use the same deployment flag for session activity and referral callbacks
/// reached through shared database connections.
pub fn configured_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = oteryn_db::lazy_pool(database_url)?;
    let options = (*pool.connect_options()).clone().options([(
        "oteryn.referrals_enabled",
        if referral_activity_enabled() {
            "true"
        } else {
            "false"
        },
    )]);
    Ok(pool.options().clone().connect_lazy_with(options))
}

/// Durations in seconds; absolute lifetime is never extended by activity or rotation.
#[derive(Clone, Copy, Debug)]
pub struct SessionConfig {
    pub idle_seconds: i32,
    pub absolute_seconds: i32,
}
impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_seconds: 1800,
            absolute_seconds: 86400,
        }
    }
}
impl SessionConfig {
    pub fn validate(self) -> Result<Self, sqlx::Error> {
        if self.idle_seconds <= 0 || self.absolute_seconds <= 0 {
            return Err(sqlx::Error::Protocol(
                "session lifetimes must be positive".into(),
            ));
        }
        Ok(self)
    }
}

pub(crate) fn random_token() -> Result<String, sqlx::Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| sqlx::Error::Protocol("secure randomness unavailable".into()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn hash(id: &str) -> Vec<u8> {
    Sha256::digest(id.as_bytes()).to_vec()
}
fn valid_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Typed keys keep flash messages and pending state separate from authentication metadata.
#[derive(Clone, Copy, Debug)]
pub struct SessionKey<T> {
    name: &'static str,
    marker: std::marker::PhantomData<T>,
}
impl<T> SessionKey<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: std::marker::PhantomData,
        }
    }
}

struct Record {
    session_id: Uuid,
    id: String,
    csrf: String,
    account_id: Option<Uuid>,
}
struct Inner {
    record: Option<Record>,
    cookie: Option<String>,
}
/// Request extension shared with the response middleware. Debug omits all bearer material.
#[derive(Clone)]
pub struct BrowserSession {
    pool: PgPool,
    config: SessionConfig,
    ip: Option<IpAddr>,
    agent: String,
    inner: Arc<Mutex<Inner>>,
}
impl std::fmt::Debug for BrowserSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserSession").finish_non_exhaustive()
    }
}
impl BrowserSession {
    fn new(pool: PgPool, config: SessionConfig, ip: Option<IpAddr>, agent: &str) -> Self {
        Self {
            pool,
            config,
            ip,
            agent: agent
                .chars()
                .filter(|c| !c.is_control())
                .take(256)
                .collect(),
            inner: Arc::new(Mutex::new(Inner {
                record: None,
                cookie: None,
            })),
        }
    }
    async fn ensure(&self, inner: &mut Inner) -> Result<(), sqlx::Error> {
        if inner.record.is_some() {
            return Ok(());
        }
        let id = random_token()?;
        let csrf = random_token()?;
        let session_id = sqlx::query_scalar("INSERT INTO web_sessions (id_hash, csrf_token, ip, user_agent_summary, idle_expires_at, absolute_expires_at) VALUES ($1,$2,$3::text::inet,$4,now()+make_interval(secs=>$5),now()+make_interval(secs=>$6)) RETURNING session_id")
            .bind(hash(&id)).bind(&csrf).bind(self.ip.map(|ip| ip.to_string())).bind(&self.agent).bind(f64::from(self.config.idle_seconds)).bind(f64::from(self.config.absolute_seconds)).fetch_one(&self.pool).await?;
        inner.cookie = Some(cookie(&id));
        inner.record = Some(Record {
            session_id,
            id,
            csrf,
            account_id: None,
        });
        Ok(())
    }
    /// Creates an anonymous session only when a form or pending state needs one.
    pub async fn csrf_token(&self) -> Result<String, sqlx::Error> {
        let mut inner = self.inner.lock().await;
        self.ensure(&mut inner).await?;
        inner
            .record
            .as_ref()
            .map(|r| r.csrf.clone())
            .ok_or(sqlx::Error::RowNotFound)
    }
    pub(crate) async fn existing_csrf_token(&self) -> Option<String> {
        self.inner
            .lock()
            .await
            .record
            .as_ref()
            .map(|r| r.csrf.clone())
    }
    pub async fn account_id(&self) -> Option<Uuid> {
        self.inner
            .lock()
            .await
            .record
            .as_ref()
            .and_then(|r| r.account_id)
    }
    pub async fn get<T: DeserializeOwned>(
        &self,
        key: SessionKey<T>,
    ) -> Result<Option<T>, sqlx::Error> {
        let inner = self.inner.lock().await;
        let Some(record) = &inner.record else {
            return Ok(None);
        };
        let value: Option<Option<String>> = sqlx::query_scalar("SELECT (data -> $2)::text FROM web_sessions WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
            .bind(hash(&record.id)).bind(key.name).fetch_optional(&self.pool).await?;
        value
            .flatten()
            .map(|v| serde_json::from_str(&v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()
    }
    /// Read live session state without acquiring a second pool connection.
    pub async fn get_in<T: DeserializeOwned>(
        &self,
        connection: &mut sqlx::PgConnection,
        key: SessionKey<T>,
    ) -> Result<Option<T>, sqlx::Error> {
        let inner = self.inner.lock().await;
        let Some(record) = &inner.record else {
            return Ok(None);
        };
        let value: Option<Option<String>> = sqlx::query_scalar("SELECT (data -> $2)::text FROM web_sessions WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
            .bind(hash(&record.id)).bind(key.name).fetch_optional(connection).await?;
        value
            .flatten()
            .map(|v| serde_json::from_str(&v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()
    }
    pub async fn set<T: Serialize>(
        &self,
        key: SessionKey<T>,
        value: &T,
    ) -> Result<(), sqlx::Error> {
        if key.name.is_empty() || key.name.len() > 128 {
            return Err(sqlx::Error::Protocol("invalid session key".into()));
        }
        let value = serde_json::to_string(value).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
        if value.len() > 16384 {
            return Err(sqlx::Error::Protocol("session value too large".into()));
        }
        let mut inner = self.inner.lock().await;
        self.ensure(&mut inner).await?;
        let record = inner.record.as_ref().ok_or(sqlx::Error::RowNotFound)?;
        let result = sqlx::query("UPDATE web_sessions SET data=jsonb_set(data,ARRAY[$2],$3::text::jsonb), updated_at=now() WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
            .bind(hash(&record.id)).bind(key.name).bind(value).execute(&self.pool).await?;
        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }
}

/// Every authentication boundary uses this single function. Logout deletes the row;
/// login and sudo replace the identifier and CSRF token atomically without extending lifetime.
#[derive(Debug, Clone, Copy)]
pub enum PrivilegeChange {
    Login(Uuid),
    AuthorizedLogin(Uuid, i64),
    Logout,
    Sudo(OffsetDateTime),
    /// Replaces the identifier and CSRF token without changing the account.
    Reissue,
}
/// Prepared durable rotation. Apply to request memory only after its transaction commits.
pub struct CommittedRotation {
    session_id: Uuid,
    id: String,
    csrf: String,
    account: Option<Uuid>,
}
pub async fn rotate_in_transaction(
    session: &BrowserSession,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    until: OffsetDateTime,
) -> Result<CommittedRotation, sqlx::Error> {
    let inner = session.inner.lock().await;
    let record = inner.record.as_ref().ok_or(sqlx::Error::RowNotFound)?;
    let account = record.account_id.ok_or(sqlx::Error::RowNotFound)?;
    let id = random_token()?;
    let csrf = random_token()?;
    let changed = sqlx::query("UPDATE web_sessions SET id_hash=$2,csrf_token=$3,sudo_until=$4,updated_at=clock_timestamp() WHERE id_hash=$1 AND account_id=$5 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
        .bind(hash(&record.id)).bind(hash(&id)).bind(&csrf).bind(until).bind(account).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(CommittedRotation {
        session_id: record.session_id,
        id,
        csrf,
        account: Some(account),
    })
}
pub async fn apply_rotation(session: &BrowserSession, rotation: CommittedRotation) {
    let mut inner = session.inner.lock().await;
    inner.cookie = Some(cookie(&rotation.id));
    inner.record = Some(Record {
        session_id: rotation.session_id,
        id: rotation.id,
        csrf: rotation.csrf,
        account_id: rotation.account,
    });
}
impl BrowserSession {
    /// Hash of the current server-issued identity, for transactional security recording.
    pub async fn identity_hash(&self) -> Option<Vec<u8>> {
        self.inner
            .lock()
            .await
            .record
            .as_ref()
            .map(|record| hash(&record.id))
    }
}
pub async fn rotate(session: &BrowserSession, change: PrivilegeChange) -> Result<(), sqlx::Error> {
    let mut inner = session.inner.lock().await;
    if matches!(change, PrivilegeChange::Logout) {
        let previous = inner.record.as_ref().map(|record| record.session_id);
        // Withhold any newly rotated identity even when durable deletion is unavailable.
        inner.record = None;
        inner.cookie = Some(format!(
            "{COOKIE_NAME}=; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
        ));
        if let Some(previous) = previous {
            sqlx::query("DELETE FROM web_sessions WHERE session_id=$1")
                .bind(previous)
                .execute(&session.pool)
                .await?;
        }
        return Ok(());
    }
    session.ensure(&mut inner).await?;
    let record = inner.record.as_ref().ok_or(sqlx::Error::RowNotFound)?;
    let account = match change {
        PrivilegeChange::Login(id) | PrivilegeChange::AuthorizedLogin(id, _) => Some(id),
        _ => record.account_id,
    };
    let sudo = match change {
        PrivilegeChange::Sudo(until) if account.is_some() => Some(until),
        PrivilegeChange::Sudo(_) => {
            return Err(sqlx::Error::Protocol("sudo requires authentication".into()));
        }
        _ => None,
    };
    let session_id = record.session_id;
    let previous_id = record.id.clone();
    let previous_account = record.account_id;
    // Release request memory before waiting for the account: a caller-owned
    // authentication transaction can already hold that account before rotation.
    drop(inner);
    let mut tx = session.pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    let accounts: Vec<Uuid> = previous_account.into_iter().chain(account).collect();
    sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(&accounts)
        .execute(&mut *tx)
        .await?;
    let mut inner = session.inner.lock().await;
    if !inner
        .record
        .as_ref()
        .is_some_and(|current| current.id == previous_id && current.account_id == previous_account)
    {
        return Err(sqlx::Error::RowNotFound);
    }
    let id = random_token()?;
    let csrf = random_token()?;
    let result = if let PrivilegeChange::AuthorizedLogin(_, generation) = change {
        sqlx::query("UPDATE web_sessions SET id_hash=$2,csrf_token=$3,account_id=$4,sudo_until=$5,authorization_generation=$6,updated_at=clock_timestamp() WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
            .bind(hash(&previous_id)).bind(hash(&id)).bind(&csrf).bind(account).bind(sudo).bind(generation).execute(&mut *tx).await?
    } else {
        sqlx::query("UPDATE web_sessions SET id_hash=$2, csrf_token=$3, account_id=$4, sudo_until=$5, updated_at=now() WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()")
        .bind(hash(&previous_id)).bind(hash(&id)).bind(&csrf).bind(account).bind(sudo).execute(&mut *tx).await?
    };
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    tx.commit().await?;
    inner.cookie = Some(cookie(&id));
    inner.record = Some(Record {
        session_id,
        id,
        csrf,
        account_id: account,
    });
    Ok(())
}
fn cookie(id: &str) -> String {
    format!("{COOKIE_NAME}={id}; Secure; HttpOnly; SameSite=Lax; Path=/")
}

#[derive(Clone, Debug)]
pub struct SessionLayer {
    pub pool: PgPool,
    pub config: SessionConfig,
    pub csrf: crate::csrf::CsrfConfig,
}
impl SessionLayer {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            config: SessionConfig::default(),
            csrf: crate::csrf::CsrfConfig::default(),
        }
    }
}
/// Attach after route matching. Deployment overrides can be supplied as outer
/// `Extension<SessionConfig>` and `Extension<CsrfConfig>` layers on the public router.
pub async fn middleware(
    State(layer): State<SessionLayer>,
    mut request: Request,
    next: Next,
) -> Response {
    let mut csrf = request
        .extensions()
        .get::<crate::csrf::CsrfConfig>()
        .cloned()
        .unwrap_or_else(|| layer.csrf.clone());
    // The login router knows both deployment origins. Bind logout to the
    // configured origin matching this host, including the dedicated staff host.
    if matches!(
        request.uri().path(),
        "/logout" | "/en/logout" | "/pl/logout"
    ) {
        let mut hosts = request.headers().get_all(header::HOST).iter();
        if let Some(host) = hosts.next().and_then(|value| value.to_str().ok())
            && hosts.next().is_none()
        {
            let https_origin = format!("https://{host}");
            if let Some(origin) = layer
                .csrf
                .trusted_origins
                .iter()
                .find(|origin| origin.eq_ignore_ascii_case(&https_origin))
            {
                csrf.trusted_origins = vec![origin.clone()];
            }
        }
    }
    let layer = SessionLayer {
        config: request
            .extensions()
            .get::<SessionConfig>()
            .copied()
            .unwrap_or(layer.config),
        csrf,
        pool: layer.pool,
    };
    match run(&layer, &mut request).await {
        Ok(session) => {
            request.extensions_mut().insert(session.clone());
            if let Err(status) = crate::csrf::verify(&layer.csrf, &session, &mut request).await {
                return status.into_response();
            }
            let mut response = next.run(request).await;
            if let Some(cookie) = session.inner.lock().await.cookie.take()
                && let Ok(value) = HeaderValue::from_str(&cookie)
            {
                response.headers_mut().append(header::SET_COOKIE, value);
            }
            response
        }
        Err(sqlx::Error::Protocol(message)) if message == "invalid session cookie" => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn run(layer: &SessionLayer, request: &mut Request) -> Result<BrowserSession, sqlx::Error> {
    layer.config.validate()?;
    let ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|v| crate::ratelimit::client_source(v.0, request.headers()))
        .transpose()
        .map_err(|_| sqlx::Error::Protocol("invalid client source".into()))?;
    let agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let session = BrowserSession::new(layer.pool.clone(), layer.config, ip, agent);
    let mut identifiers = request
        .headers()
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|v| v.trim().split_once('='))
        .filter(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value);
    if let Some(id) = identifiers.next() {
        if !valid_id(id) || identifiers.next().is_some() {
            return Err(sqlx::Error::Protocol("invalid session cookie".into()));
        }
        let row: Option<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
            "UPDATE web_sessions SET last_seen_at=now(), updated_at=now(),
                idle_expires_at=LEAST(absolute_expires_at,now()+make_interval(secs=>$2))
             WHERE id_hash=$1 AND idle_expires_at>clock_timestamp()
                AND absolute_expires_at>clock_timestamp()
             RETURNING session_id,csrf_token,account_id",
        )
        .bind(hash(id))
        .bind(f64::from(layer.config.idle_seconds))
        .fetch_optional(&layer.pool)
        .await?;
        if let Some((session_id, csrf, account_id)) = row {
            session.inner.lock().await.record = Some(Record {
                session_id,
                id: id.into(),
                csrf,
                account_id,
            });
        }
    }
    Ok(session)
}

/// Check for a live signed-in session without touching its idle deadline.
/// Request handlers must still apply their normal CSRF and authorization checks.
pub async fn has_signed_in_session(layer: &SessionLayer, headers: &axum::http::HeaderMap) -> bool {
    signed_in_csrf(layer, headers, layer.config).await.is_some()
}

/// Read presentation state without refreshing expiry, creating a session or setting cookies.
pub(crate) async fn chrome_account(
    layer: &SessionLayer,
    headers: &axum::http::HeaderMap,
    extensions: &axum::http::Extensions,
) -> oteryn_ui::Account {
    let config = extensions
        .get::<SessionConfig>()
        .copied()
        .unwrap_or(layer.config);
    match signed_in_csrf(layer, headers, config).await {
        Some(csrf) => oteryn_ui::Account::SignedIn { csrf },
        None => oteryn_ui::Account::Anonymous,
    }
}

async fn signed_in_csrf(
    layer: &SessionLayer,
    headers: &axum::http::HeaderMap,
    config: SessionConfig,
) -> Option<String> {
    if config.validate().is_err() {
        return None;
    }
    let mut identifiers = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|v| v.trim().split_once('='))
        .filter(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value);
    let id = identifiers.next().filter(|id| valid_id(id))?;
    if identifiers.next().is_some() {
        return None;
    }
    sqlx::query_scalar(
        "SELECT csrf_token FROM web_sessions WHERE id_hash=$1 AND account_id IS NOT NULL
         AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()",
    )
    .bind(hash(id))
    .fetch_optional(&layer.pool)
    .await
    .ok()
    .flatten()
}

/// Bounded cleanup batches can be called repeatedly by the worker.
#[derive(Debug, Clone)]
pub struct SessionCleanup {
    pub pool: PgPool,
}
impl SessionCleanup {
    pub async fn clean(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM web_sessions WHERE id_hash IN (SELECT id_hash FROM web_sessions WHERE idle_expires_at<=now() OR absolute_expires_at<=now() LIMIT 1000 FOR UPDATE SKIP LOCKED)").execute(&self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

impl oteryn_db::outbox::Cleanup for SessionCleanup {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            SessionCleanup::clean(self)
                .await
                .map_err(|error| error.to_string())
        })
    }
}

impl std::fmt::Debug for CommittedRotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CommittedRotation([redacted])")
    }
}
