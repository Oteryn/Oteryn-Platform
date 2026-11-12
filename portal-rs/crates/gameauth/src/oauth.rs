//! Root OAuth endpoints and opaque, account-bound PKCE grants.
#[cfg(test)]
mod oauth_fixture_tests;
mod oauth_http;
#[cfg(test)]
mod oauth_tests;
pub use oauth_http::{Browser, MfaSession, browser_routes, token_routes};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub clients: Vec<Client>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    pub client_id: String,
    pub redirect_uris: Vec<String>,
}
impl Config {
    /// Reads `PORTAL_OAUTH_CONFIG` as JSON with `enabled` and `clients` fields.
    /// Each client has `client_id` and `redirect_uris`; redirects include the exact port.
    /// Missing, malformed or ambiguous deployment configuration denies OAuth traffic.
    pub fn from_env() -> Self {
        std::env::var("PORTAL_OAUTH_CONFIG")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn client(&self, id: &str) -> bool {
        self.enabled
            && !id.is_empty()
            && self.clients.iter().filter(|c| c.client_id == id).count() == 1
    }
    fn redirect(&self, id: &str, redirect: &str) -> bool {
        self.client(id)
            && safe_redirect(redirect)
            && self
                .clients
                .iter()
                .any(|c| c.client_id == id && c.redirect_uris.iter().any(|r| r == redirect))
    }
}
fn safe_redirect(s: &str) -> bool {
    let Ok(u) = url::Url::parse(s) else {
        return false;
    };
    s.len() <= 2048
        && u.username().is_empty()
        && u.password().is_none()
        && u.fragment().is_none()
        && u.query().is_none()
        && u.host_str().is_some()
        && (u.scheme() == "https"
            || (u.scheme() == "http" && matches!(u.host_str(), Some("127.0.0.1" | "[::1]"))))
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authorization {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}
impl Authorization {
    fn valid(&self, config: &Config) -> bool {
        self.response_type == "code"
            && self.scope == "game:ticket"
            && self.code_challenge_method == "S256"
            && self.state.len() >= 16
            && self.state.len() <= 1024
            && self.state.bytes().all(|b| b.is_ascii_graphic())
            && URL_SAFE_NO_PAD
                .decode(&self.code_challenge)
                .is_ok_and(|v| v.len() == 32 && URL_SAFE_NO_PAD.encode(v) == self.code_challenge)
            && config.redirect(&self.client_id, &self.redirect_uri)
    }
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code: String,
    pub code_verifier: String,
    pub refresh_token: String,
    pub scope: String,
}
#[derive(Serialize)]
pub struct Tokens {
    pub token_type: &'static str,
    pub expires_in: u32,
    pub access_token: String,
    pub refresh_token: String,
    pub scope: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidRequest,
    InvalidGrant,
    InvalidClient,
    UnsupportedGrant,
    Unavailable,
    Disabled,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
fn hash(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}
fn same(a: &[u8], b: &[u8]) -> bool {
    bool::from(a.ct_eq(b))
}
fn secret() -> Result<String, Error> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn opaque(s: &str) -> bool {
    s.len() == 43
        && URL_SAFE_NO_PAD
            .decode(s)
            .is_ok_and(|b| b.len() == 32 && URL_SAFE_NO_PAD.encode(b) == s)
}
fn verifier(s: &str) -> bool {
    (43..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
/// The credential stamp binds codes and token families to the password, email and
/// verification state. `login_mfa_required` is deliberately not hashed: MFA is
/// enforced where a browser approves a grant (`approve`), and turning it on rotates
/// `game_auth_generation`, which fences native enrollment of older families.
async fn credentials(tx: &mut sqlx::PgConnection, account: Uuid) -> Result<(Vec<u8>, bool), Error> {
    let row = sqlx::query("SELECT password_hash, email, email_verified_at::text AS verified, login_mfa_required, status FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account).fetch_optional(&mut *tx).await?.ok_or(Error::InvalidGrant)?;
    if row.get::<String, _>("status") != "active"
        || row.get::<Option<String>, _>("verified").is_none()
    {
        return Err(Error::InvalidGrant);
    }
    let input = serde_json::to_string(&(
        row.get::<String, _>("password_hash"),
        row.get::<String, _>("email"),
        row.get::<Option<String>, _>("verified"),
    ))
    .map_err(|_| Error::Unavailable)?;
    Ok((hash(&input), row.get::<bool, _>("login_mfa_required")))
}
async fn stamp(tx: &mut sqlx::PgConnection, account: Uuid) -> Result<Vec<u8>, Error> {
    Ok(credentials(tx, account).await?.0)
}
async fn event(
    tx: &mut Transaction<'_, Postgres>,
    account: Option<Uuid>,
    name: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO oauth_security_events(account_id,event) VALUES($1,$2)")
        .bind(account)
        .bind(name)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn audit(pool: &PgPool, account: Option<Uuid>, name: &str) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    event(&mut tx, account, name).await?;
    tx.commit().await?;
    Ok(())
}
/// Approval without a browser MFA proof: MFA-protected accounts are refused.
pub async fn authorize(
    pool: &PgPool,
    config: &Config,
    account: Uuid,
    request: &Authorization,
) -> Result<String, Error> {
    approve(pool, config, account, false, request)
        .await?
        .ok_or(Error::InvalidGrant)
}
/// Issues a code for a browser approval. `mfa` is true only when the web session
/// adapter proved a live MFA grant for `account`; an MFA-protected account without
/// it gets `None` (step up) and no code row.
pub(crate) async fn approve(
    pool: &PgPool,
    config: &Config,
    account: Uuid,
    mfa: bool,
    request: &Authorization,
) -> Result<Option<String>, Error> {
    if !request.valid(config) {
        return Err(Error::InvalidRequest);
    }
    let mut tx = pool.begin().await?;
    let Some(stamp) = step_up(&mut tx, account, mfa).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let code = secret()?;
    sqlx::query("INSERT INTO oauth_authorization_codes(code_hash,account_id,client_id,redirect_uri,challenge,credential_hash) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(hash(&code)).bind(account).bind(&request.client_id).bind(&request.redirect_uri).bind(&request.code_challenge).bind(stamp).execute(&mut *tx).await?;
    event(&mut tx, Some(account), "authorization_approved").await?;
    tx.commit().await?;
    Ok(Some(code))
}
/// Locks the account and returns its credential stamp; `None` (with an audit
/// event) when MFA is required but the browser session has not proven it.
async fn step_up(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
    mfa: bool,
) -> Result<Option<Vec<u8>>, Error> {
    let (stamp, required) = credentials(tx, account).await?;
    if required && !mfa {
        event(tx, Some(account), "mfa_step_up_required").await?;
        return Ok(None);
    }
    Ok(Some(stamp))
}
async fn issue(tx: &mut Transaction<'_, Postgres>, family: Uuid) -> Result<Tokens, Error> {
    let access = secret()?;
    let refresh = secret()?;
    sqlx::query(
        "INSERT INTO oauth_tokens(id,family_id,access_hash,refresh_hash) VALUES($1,$2,$3,$4)",
    )
    .bind(Uuid::now_v7())
    .bind(family)
    .bind(hash(&access))
    .bind(hash(&refresh))
    .execute(&mut **tx)
    .await?;
    Ok(Tokens {
        token_type: "bearer",
        expires_in: 1800,
        access_token: access,
        refresh_token: refresh,
        scope: "game:ticket",
    })
}
pub async fn exchange(
    pool: &PgPool,
    config: &Config,
    input: &TokenRequest,
) -> Result<Tokens, Error> {
    if !config.enabled {
        return Err(Error::Disabled);
    }
    if !config.client(&input.client_id) {
        return Err(Error::InvalidClient);
    }
    match input.grant_type.as_str() {
        "authorization_code" => exchange_code(pool, config, input).await,
        "refresh_token" => refresh(pool, input).await,
        _ => Err(Error::UnsupportedGrant),
    }
}
async fn exchange_code(
    pool: &PgPool,
    config: &Config,
    input: &TokenRequest,
) -> Result<Tokens, Error> {
    if !opaque(&input.code)
        || !verifier(&input.code_verifier)
        || !input.refresh_token.is_empty()
        || !input.scope.is_empty()
        || !config.redirect(&input.client_id, &input.redirect_uri)
    {
        return Err(Error::InvalidRequest);
    }
    let mut tx = pool.begin().await?;
    let account: Uuid =
        sqlx::query_scalar("SELECT account_id FROM oauth_authorization_codes WHERE code_hash=$1")
            .bind(hash(&input.code))
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::InvalidGrant)?;
    let current = stamp(&mut tx, account).await?;
    let row = sqlx::query("SELECT *, expires_at>clock_timestamp() AND used_at IS NULL AND revoked_at IS NULL AS valid FROM oauth_authorization_codes WHERE code_hash=$1 FOR UPDATE")
        .bind(hash(&input.code)).fetch_one(&mut *tx).await?;
    if !row.get::<bool, _>("valid")
        || row.get::<String, _>("client_id") != input.client_id
        || row.get::<String, _>("redirect_uri") != input.redirect_uri
        || !same(&current, &row.get::<Vec<u8>, _>("credential_hash"))
        || !same(
            URL_SAFE_NO_PAD
                .encode(hash(&input.code_verifier))
                .as_bytes(),
            row.get::<String, _>("challenge").as_bytes(),
        )
    {
        return Err(Error::InvalidGrant);
    }
    sqlx::query(
        "UPDATE oauth_authorization_codes SET used_at=now(),updated_at=now() WHERE code_hash=$1",
    )
    .bind(hash(&input.code))
    .execute(&mut *tx)
    .await?;
    let family = Uuid::now_v7();
    let (game_generation, native_generation): (i64, i64) = sqlx::query_as(
        "SELECT game_auth_generation,native_security_generation FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO oauth_token_families(id,account_id,client_id,credential_hash,game_generation,native_generation) VALUES($1,$2,$3,$4,$5,$6)").bind(family).bind(account).bind(&input.client_id).bind(current).bind(game_generation).bind(native_generation).execute(&mut *tx).await?;
    let tokens = issue(&mut tx, family).await?;
    event(&mut tx, Some(account), "code_exchanged").await?;
    tx.commit().await?;
    Ok(tokens)
}
async fn refresh(pool: &PgPool, input: &TokenRequest) -> Result<Tokens, Error> {
    if !opaque(&input.refresh_token)
        || !input.code.is_empty()
        || !input.code_verifier.is_empty()
        || !input.redirect_uri.is_empty()
        || (!input.scope.is_empty() && input.scope != "game:ticket")
    {
        return Err(Error::InvalidRequest);
    }
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT f.account_id, f.id FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.refresh_hash=$1").bind(hash(&input.refresh_token)).fetch_optional(&mut *tx).await?.ok_or(Error::InvalidGrant)?;
    let account: Uuid = row.get("account_id");
    let family: Uuid = row.get("id");
    let current = stamp(&mut tx, account).await?;
    let row = sqlx::query("SELECT *, expires_at>clock_timestamp() AND revoked_at IS NULL AS valid FROM oauth_token_families WHERE id=$1 FOR UPDATE").bind(family).fetch_one(&mut *tx).await?;
    if row.get::<String, _>("client_id") != input.client_id {
        return Err(Error::InvalidGrant);
    }
    let used: bool =
        sqlx::query_scalar("SELECT used_at IS NOT NULL FROM oauth_tokens WHERE refresh_hash=$1")
            .bind(hash(&input.refresh_token))
            .fetch_one(&mut *tx)
            .await?;
    if used || !same(&current, &row.get::<Vec<u8>, _>("credential_hash")) {
        sqlx::query(
            "UPDATE oauth_token_families SET revoked_at=now(),updated_at=now() WHERE id=$1",
        )
        .bind(family)
        .execute(&mut *tx)
        .await?;
        event(
            &mut tx,
            Some(account),
            "refresh_reuse_or_credentials_changed",
        )
        .await?;
        tx.commit().await?;
        return Err(Error::InvalidGrant);
    }
    if !row.get::<bool, _>("valid") {
        return Err(Error::InvalidGrant);
    }
    sqlx::query("UPDATE oauth_tokens SET used_at=now(),updated_at=now() WHERE refresh_hash=$1")
        .bind(hash(&input.refresh_token))
        .execute(&mut *tx)
        .await?;
    let tokens = issue(&mut tx, family).await?;
    event(&mut tx, Some(account), "refresh_rotated").await?;
    tx.commit().await?;
    Ok(tokens)
}
/// All credential-changing callers must invoke this in their account transaction.
/// Lock the account before changing credentials or invoking this function.
pub async fn revoke_account(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
) -> Result<(), Error> {
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE oauth_authorization_codes SET revoked_at=now(),updated_at=now() WHERE account_id=$1 AND revoked_at IS NULL").bind(account).execute(&mut **tx).await?;
    sqlx::query("UPDATE oauth_token_families SET revoked_at=now(),updated_at=now() WHERE account_id=$1 AND revoked_at IS NULL").bind(account).execute(&mut **tx).await?;
    event(tx, Some(account), "account_oauth_revoked").await
}
/// Public ticket consumers must verify the current account and family on every use.
pub async fn verify_access(pool: &PgPool, config: &Config, token: &str) -> Result<Uuid, Error> {
    if !config.enabled {
        return Err(Error::Disabled);
    }
    if !opaque(token) {
        return Err(Error::InvalidGrant);
    }
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT f.account_id, f.client_id, f.credential_hash FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.access_hash=$1 AND t.access_expires_at>clock_timestamp() AND f.expires_at>clock_timestamp() AND f.revoked_at IS NULL").bind(hash(token)).fetch_optional(&mut *tx).await?.ok_or(Error::InvalidGrant)?;
    let account = row.get("account_id");
    let current = stamp(&mut tx, account).await?;
    // Re-read after acquiring the account lock so concurrent revocation cannot
    // leave a stale family snapshot authorized.
    let row = sqlx::query("SELECT f.client_id, f.credential_hash FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.access_hash=$1 AND t.access_expires_at>clock_timestamp() AND f.expires_at>clock_timestamp() AND f.revoked_at IS NULL").bind(hash(token)).fetch_optional(&mut *tx).await?.ok_or(Error::InvalidGrant)?;
    if !config.client(&row.get::<String, _>("client_id"))
        || !same(&current, &row.get::<Vec<u8>, _>("credential_hash"))
    {
        return Err(Error::InvalidGrant);
    }
    tx.commit().await?;
    Ok(account)
}
pub use oauth_http::secure_response;

macro_rules! redacted_debug {
    ($($kind:ty),+ $(,)?) => { $(impl std::fmt::Debug for $kind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($kind), "([redacted])")) }
    })+ };
}
redacted_debug!(Authorization, TokenRequest, Tokens, Browser, MfaSession);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OAuth request failed: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Native enrollment uses the same opaque-token grammar, credential stamp and client
/// allowlist as OAuth, while retaining all locks in its caller's transaction.
pub(crate) async fn native_enrollment(
    db: &mut sqlx::PgConnection,
    config: &Config,
    token: &str,
) -> Result<crate::devices::Enrollment, Error> {
    crate::devices::native::check_config(config).map_err(|_| Error::Unavailable)?;
    if !opaque(token) {
        return Err(Error::InvalidGrant);
    }
    let (account, client): (Uuid, String) = sqlx::query_as("SELECT f.account_id,f.client_id FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.access_hash=$1")
        .bind(hash(token)).fetch_optional(&mut *db).await?.ok_or(Error::InvalidGrant)?;
    let current = stamp(db, account).await?;
    let client_id = Uuid::parse_str(&client).map_err(|_| Error::InvalidClient)?;
    crate::devices::native::lock_client(db, config, client_id)
        .await
        .map_err(|e| {
            if e == crate::devices::Failure::Denied {
                Error::InvalidClient
            } else {
                Error::Unavailable
            }
        })?;
    let row = sqlx::query("SELECT t.id, t.access_hash, f.account_id, f.client_id, f.credential_hash, f.game_generation, f.native_generation FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.access_hash=$1 AND t.access_expires_at>clock_timestamp() AND t.used_at IS NULL AND f.expires_at>clock_timestamp() AND f.revoked_at IS NULL FOR UPDATE OF f,t")
        .bind(hash(token)).fetch_optional(&mut *db).await?.ok_or(Error::InvalidGrant)?;
    let client: String = row.try_get("client_id")?;
    if row.try_get::<Uuid, _>("account_id")? != account
        || client != client_id.to_string()
        || !same(&hash(token), &row.try_get::<Vec<u8>, _>("access_hash")?)
        || !config.client(&client)
        || !same(&current, &row.try_get::<Vec<u8>, _>("credential_hash")?)
    {
        return Err(Error::InvalidGrant);
    }
    let client_id = Uuid::parse_str(&client).map_err(|_| Error::InvalidClient)?;
    if client_id.to_string() != client {
        return Err(Error::InvalidClient);
    }
    let (game_generation, native_generation): (i64, i64) = sqlx::query_as(
        "SELECT game_auth_generation,native_security_generation FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(db)
    .await?;
    if row.try_get::<Option<i64>, _>("game_generation")? != Some(game_generation)
        || row.try_get::<Option<i64>, _>("native_generation")? != Some(native_generation)
    {
        return Err(Error::InvalidGrant);
    }
    Ok(crate::devices::Enrollment {
        authorization: crate::devices::Authorization {
            account_id: account,
            client_id,
            game_generation,
            native_generation,
        },
        token_id: row.try_get::<Uuid, _>("id")?.to_string(),
    })
}
pub(crate) fn native_client(config: &Config, client: Uuid) -> bool {
    config.client(&client.to_string())
}
