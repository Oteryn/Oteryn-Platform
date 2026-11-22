//! Server-side browser sessions. Raw identifiers are never persisted or logged.

use axum::{
    Router,
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
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

/// Monotonic request deadline shared by chrome and bounded session refresh.
#[derive(Clone, Copy, Debug)]
pub struct BrowserMutationDeadline(tokio::time::Instant);

/// Start a browser POST's budget before shared chrome looks up the session.
pub fn with_post_deadline(app: Router, matches_path: fn(&str) -> bool) -> Router {
    app.layer(middleware::from_fn(
        move |mut request: Request, next: Next| async move {
            if request.method() == Method::POST
                && request
                    .extensions()
                    .get::<MatchedPath>()
                    .is_some_and(|path| matches_path(path.as_str()))
            {
                request
                    .extensions_mut()
                    .insert(BrowserMutationDeadline::start());
            }
            next.run(request).await
        },
    ))
}

impl BrowserMutationDeadline {
    pub fn start() -> Self {
        Self(tokio::time::Instant::now() + std::time::Duration::from_secs(3))
    }

    async fn read<T>(
        self,
        read: impl std::future::Future<Output = Result<T, sqlx::Error>>,
    ) -> Result<T, sqlx::Error> {
        self.within(read).await.ok_or(sqlx::Error::PoolTimedOut)?
    }

    async fn within<T>(self, work: impl std::future::Future<Output = T>) -> Option<T> {
        if tokio::time::Instant::now() >= self.0 {
            return None;
        }
        tokio::time::timeout_at(self.0, work).await.ok()
    }

    async fn calibrate(
        self,
        tx: &mut sqlx::PgConnection,
    ) -> Result<SessionMutationDeadline, sqlx::Error> {
        let database_now: OffsetDateTime = self
            .read(sqlx::query_scalar("SELECT clock_timestamp()").fetch_one(tx))
            .await?;
        // Sample the remaining time after the reply. Round-trip latency can only
        // shorten the resulting deadline on PostgreSQL's clock.
        let remaining = self
            .0
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or(sqlx::Error::PoolTimedOut)?;
        Ok(SessionMutationDeadline {
            database: database_now + remaining,
            request: self,
        })
    }
}

/// Same request deadline calibrated once to the database clock at session refresh.
#[derive(Clone, Debug)]
pub struct SessionMutationDeadline {
    database: OffsetDateTime,
    request: BrowserMutationDeadline,
}

impl SessionMutationDeadline {
    /// Bound request body reads without dropping a future that can commit a mutation.
    pub async fn read_request<T>(
        &self,
        read: impl std::future::Future<Output = T>,
    ) -> Result<T, StatusCode> {
        self.request
            .within(read)
            .await
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)
    }

    /// Bound admission work that cannot write, including its pool wait.
    pub async fn read<T>(
        &self,
        read: impl std::future::Future<Output = Result<T, sqlx::Error>>,
    ) -> Result<T, sqlx::Error> {
        self.request.read(read).await
    }

    async fn configure(&self, tx: &mut sqlx::PgConnection) -> Result<(), sqlx::Error> {
        self.read(async {
            sqlx::query("SELECT set_config('oteryn.mutation_deadline', extract(epoch FROM $1::timestamptz)::text, true)")
                .bind(self.database)
                .execute(&mut *tx)
                .await?;
            oteryn_db::mutation_budget(tx).await
        }).await
    }

    /// Acquire a transaction before the request deadline and carry the database budget into it.
    pub async fn begin(
        &self,
        pool: &PgPool,
    ) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, sqlx::Error> {
        let mut tx = self.read(pool.begin()).await?;
        self.configure(&mut tx).await?;
        Ok(tx)
    }
}

#[derive(Clone, Debug)]
pub struct SessionLayer {
    pub pool: PgPool,
    pub config: SessionConfig,
    pub csrf: crate::csrf::CsrfConfig,
    bounded_refresh: bool,
}
impl SessionLayer {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            config: SessionConfig::default(),
            csrf: crate::csrf::CsrfConfig::default(),
            bounded_refresh: false,
        }
    }

    /// Start the mutation budget before refreshing the session for an unsafe method.
    pub fn with_bounded_refresh(mut self) -> Self {
        self.bounded_refresh = true;
        self
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
        bounded_refresh: layer.bounded_refresh,
    };
    match run(&layer, &mut request).await {
        Ok(session) => {
            request.extensions_mut().insert(session.clone());
            let deadline = request
                .extensions()
                .get::<SessionMutationDeadline>()
                .cloned();
            let verification = crate::csrf::verify(&layer.csrf, &session, &mut request);
            let verified = match deadline {
                Some(deadline) => deadline
                    .read_request(verification)
                    .await
                    .and_then(std::convert::identity),
                None => verification.await,
            };
            if let Err(status) = verified {
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
        Err(error) if layer.bounded_refresh && refresh_timed_out(&error) => {
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn refresh_timed_out(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::PoolTimedOut)
        || error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| matches!(code.as_ref(), "55P03" | "57014" | "25P04"))
}

async fn refresh_session<'e>(
    db: impl sqlx::PgExecutor<'e>,
    id_hash: Vec<u8>,
    idle_seconds: i32,
) -> Result<Option<(Uuid, String, Option<Uuid>)>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE web_sessions SET last_seen_at=now(), updated_at=now(),
            idle_expires_at=LEAST(absolute_expires_at,now()+make_interval(secs=>$2))
         WHERE id_hash=$1 AND idle_expires_at>clock_timestamp()
            AND absolute_expires_at>clock_timestamp()
         RETURNING session_id,csrf_token,account_id",
    )
    .bind(id_hash)
    .bind(f64::from(idle_seconds))
    .fetch_optional(db)
    .await
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
    if let Some(id) = cookie_id(request.headers())? {
        let id_hash = hash(id);
        let (row, deadline) = if layer.bounded_refresh && !request.method().is_safe() {
            let request_deadline = request
                .extensions()
                .get::<BrowserMutationDeadline>()
                .copied()
                .unwrap_or_else(BrowserMutationDeadline::start);
            let mut tx = request_deadline.read(layer.pool.begin()).await?;
            let deadline = request_deadline.calibrate(&mut tx).await?;
            deadline.configure(&mut tx).await?;
            // Evaluate expiry in the refresh statement after the row lock is acquired.
            sqlx::query("SELECT id_hash FROM web_sessions WHERE id_hash=$1 FOR UPDATE")
                .bind(&id_hash)
                .execute(&mut *tx)
                .await?;
            oteryn_db::mutation_budget(&mut tx).await?;
            let row = refresh_session(&mut *tx, id_hash, layer.config.idle_seconds).await?;
            oteryn_db::mutation_budget(&mut tx).await?;
            if !oteryn_db::transaction::arm_commit(&mut tx, None).await? {
                return Err(sqlx::Error::PoolTimedOut);
            }
            tx.commit().await?;
            (row, Some(deadline))
        } else {
            (
                refresh_session(&layer.pool, id_hash, layer.config.idle_seconds).await?,
                None,
            )
        };
        if let Some((session_id, csrf, account_id)) = row {
            session.inner.lock().await.record = Some(Record {
                session_id,
                id: id.into(),
                csrf,
                account_id,
            });
        }
        if let Some(deadline) = deadline {
            request.extensions_mut().insert(deadline);
        }
    }
    Ok(session)
}

fn cookie_id(headers: &HeaderMap) -> Result<Option<&str>, sqlx::Error> {
    let mut identifiers = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|v| v.trim().split_once('='))
        .filter(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value);
    let id = identifiers.next();
    if id.is_some_and(|id| !valid_id(id)) || identifiers.next().is_some() {
        return Err(sqlx::Error::Protocol("invalid session cookie".into()));
    }
    Ok(id)
}

/// Existing authenticated session data for a read-only page.
#[derive(Clone)]
pub struct SignedInSession {
    pub account_id: Uuid,
    pub csrf: String,
}
impl std::fmt::Debug for SignedInSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedInSession").finish_non_exhaustive()
    }
}

async fn read_signed_in(
    layer: &SessionLayer,
    request: &mut Request,
) -> Result<Option<SignedInSession>, sqlx::Error> {
    request
        .extensions()
        .get::<SessionConfig>()
        .copied()
        .unwrap_or(layer.config)
        .validate()?;
    let Some(id) = cookie_id(request.headers())? else {
        return Ok(None);
    };
    let lookup = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT account_id,csrf_token FROM web_sessions WHERE id_hash=$1
         AND account_id IS NOT NULL AND idle_expires_at>clock_timestamp()
         AND absolute_expires_at>clock_timestamp()",
    )
    .bind(hash(id))
    .fetch_optional(&layer.pool);
    let row = match request.extensions().get::<BrowserMutationDeadline>() {
        Some(deadline) => deadline.read(lookup).await,
        None => lookup.await,
    }?;
    Ok(row.map(|(account_id, csrf)| SignedInSession { account_id, csrf }))
}

/// Attach existing session data to a read-only route without refreshing or creating a session.
pub async fn read_only_middleware(
    State(layer): State<SessionLayer>,
    mut request: Request,
    next: Next,
) -> Response {
    match read_signed_in(&layer, &mut request).await {
        Ok(session) => {
            if let Some(session) = session {
                request.extensions_mut().insert(session);
            }
            next.run(request).await
        }
        Err(sqlx::Error::Protocol(message)) if message == "invalid session cookie" => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Read presentation state without refreshing expiry, creating a session or setting cookies.
pub(crate) async fn chrome_account(
    layer: &SessionLayer,
    request: &mut Request,
) -> oteryn_ui::Account {
    match read_signed_in(layer, request).await {
        Ok(Some(session)) => oteryn_ui::Account::SignedIn { csrf: session.csrf },
        _ => oteryn_ui::Account::Anonymous,
    }
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
