//! Platform side of Oteryn-Game `OTERYN_NATIVE_LOGIN_CHALLENGE_V1` (Game
//! 42496a66, `docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md`).
//!
//! Off unless `PORTAL_NATIVE_LOGIN_CHALLENGE` enables it. Failed sign-ins per
//! identity and token attempts per source are counted in PostgreSQL, so every
//! replica shares them. Past a threshold `/oauth/token` answers 403
//! `oteryn_challenge_required` before the code is consumed; the browser check
//! at `/oauth/challenge` returns a signed, one-use proof to the client's
//! loopback URI, and the retry consumes proof and code and issues the token in
//! one transaction. Signing keys come only from the environment.
use super::*;
use ed25519_dalek::{Signature, Signer, SigningKey};
use std::{net::IpAddr, sync::Arc};

/// JSON policy; absent means off. Malformed JSON turns the feature on in a
/// refusing state, so a broken deployment never issues tokens unchecked.
pub const CONFIG_ENV: &str = "PORTAL_NATIVE_LOGIN_CHALLENGE";
/// `kid=<base64url 32-byte Ed25519 seed>` entries, comma-separated. The first
/// signs; the rest still verify, which is how a key is rotated out.
pub const KEYS_ENV: &str = "PORTAL_NATIVE_LOGIN_CHALLENGE_KEYS";
const AUDIENCE: &str = "urn:oteryn:platform:oauth-token:native-login-challenge";
const PROOF_VERSION: &str = "v1";
const IDENTITY: &str = "identity_failure";
const SOURCE: &str = "source_attempt";
const MAX_PROOF: usize = 2048;
const ANSWER_DIGITS: usize = 6;

/// Thresholds default to the owner decision of 2026-11-25: 5 failed sign-ins
/// per identity in 10 minutes, 20 attempts per source in 1 minute, counters
/// kept 24 hours.
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChallengeConfig {
    pub enabled: bool,
    pub identity_failures: u32,
    pub identity_window_seconds: u32,
    pub source_attempts: u32,
    pub source_window_seconds: u32,
    pub retention_seconds: u32,
    pub challenge_ttl_seconds: u32,
    pub max_answer_attempts: u32,
    pub policy_version: u32,
    #[serde(skip)]
    pub(super) keys: Arc<Vec<(String, SigningKey)>>,
    #[serde(skip)]
    pub(super) broken: bool,
}
impl Default for ChallengeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            identity_failures: 5,
            identity_window_seconds: 600,
            source_attempts: 20,
            source_window_seconds: 60,
            retention_seconds: 86_400,
            challenge_ttl_seconds: 120,
            max_answer_attempts: 5,
            policy_version: 1,
            keys: Arc::default(),
            broken: false,
        }
    }
}
impl std::fmt::Debug for ChallengeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeConfig")
            .field("enabled", &self.enabled)
            .field("usable", &self.usable())
            .finish_non_exhaustive()
    }
}
impl ChallengeConfig {
    pub fn from_env() -> Self {
        Self::from_values(std::env::var(CONFIG_ENV), std::env::var(KEYS_ENV))
    }
    /// [`Self::from_env`] over already read variables. Only an absent policy
    /// is off; one that is not UTF-8 is as broken as malformed JSON.
    pub(crate) fn from_values(
        raw: Result<String, std::env::VarError>,
        keys: Result<String, std::env::VarError>,
    ) -> Self {
        let broken = || Self {
            enabled: true,
            broken: true,
            ..Self::default()
        };
        let mut config = match raw {
            Err(std::env::VarError::NotPresent) => return Self::default(),
            Err(std::env::VarError::NotUnicode(_)) => broken(),
            Ok(raw) => serde_json::from_str::<Self>(&raw).unwrap_or_else(|_| broken()),
        };
        if config.enabled {
            match keys.ok().as_deref().and_then(parse_keys) {
                Some(keys) => config.keys = Arc::new(keys),
                None => config.broken = true,
            }
        }
        config
    }
    /// Replaces the key ring from a `KEYS_ENV`-formatted value; `false` (and a
    /// refusing configuration) when it is malformed.
    pub fn set_keys(&mut self, value: &str) -> bool {
        match parse_keys(value) {
            Some(keys) => {
                self.keys = Arc::new(keys);
                true
            }
            None => {
                self.keys = Arc::default();
                false
            }
        }
    }
    /// Enabled with a sane policy and at least one signing key; `usable`
    /// implies `enabled`, never the reverse. An enabled but unusable policy
    /// still counts (`record_sign_in`, the token route) and refuses.
    pub(crate) fn usable(&self) -> bool {
        self.enabled && !self.broken && !self.keys.is_empty() && self.policy_valid()
    }
    /// The numbers alone are in range, whatever the flag and the key ring.
    fn policy_valid(&self) -> bool {
        self.identity_failures >= 1
            && self.source_attempts >= 1
            && self.retention_valid()
            && (1..=300).contains(&self.challenge_ttl_seconds)
            && (1..=20).contains(&self.max_answer_attempts)
            && (1..=i32::MAX as u32).contains(&self.policy_version)
    }
    /// The windows and the retention, the only numbers the retention job uses,
    /// are in range.
    fn retention_valid(&self) -> bool {
        let window = 1..=86_400;
        window.contains(&self.identity_window_seconds)
            && window.contains(&self.source_window_seconds)
            && self.retention_seconds
                >= self.identity_window_seconds.max(self.source_window_seconds)
            && self.retention_seconds <= 30 * 86_400
    }
}
fn valid_kid(kid: &str) -> bool {
    (1..=32).contains(&kid.len())
        && kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn parse_keys(value: &str) -> Option<Vec<(String, SigningKey)>> {
    let mut keys = Vec::new();
    for entry in value.split(',') {
        let (kid, seed) = entry.trim().split_once('=')?;
        let seed = zeroize::Zeroizing::new(URL_SAFE_NO_PAD.decode(seed).ok()?);
        let seed: &[u8; 32] = seed.as_slice().try_into().ok()?;
        if !valid_kid(kid) || keys.iter().any(|(k, _)| k == kid) || keys.len() >= 8 {
            return None;
        }
        keys.push((kid.to_owned(), SigningKey::from_bytes(seed)));
    }
    (!keys.is_empty()).then_some(keys)
}

/// Identity key: the sign-in email normalized as `identity::accounts`
/// normalizes it (trim, ASCII lowercase). Recorded whether or not an account
/// exists, so the counter reveals nothing about registration.
fn identity_key(email: &str) -> Vec<u8> {
    hash(&format!(
        "native-login-identity\0{}",
        email.trim().to_ascii_lowercase()
    ))
}
fn source_key(source: IpAddr) -> Vec<u8> {
    hash(&format!("native-login-source\0{source}"))
}
/// Counters are kept in buckets of one fixed width, whatever the policy, so
/// every replica writes the buckets the others read, even after a window
/// changes or on a replica whose policy is out of range.
const BUCKET_SECONDS: i64 = 10;
/// How far back a window reaches: every bucket that overlaps it counts, so a
/// window may over-count by at most one bucket but never forgets a hit inside
/// it.
fn reach_seconds(window: u32) -> f64 {
    f64::from(window) + BUCKET_SECONDS as f64
}
async fn bump(db: &mut sqlx::PgConnection, kind: &str, key: &[u8]) -> Result<(), Error> {
    sqlx::query("INSERT INTO native_login_challenge_counters(kind,key_hash,bucket_start,hits) VALUES($1,$2,to_timestamp((floor(extract(epoch FROM clock_timestamp())/$3::bigint)*$3::bigint)::float8),1) ON CONFLICT(kind,key_hash,bucket_start) DO UPDATE SET hits=LEAST(native_login_challenge_counters.hits,2147483646)+1")
        .bind(kind).bind(key).bind(BUCKET_SECONDS).execute(db).await?;
    Ok(())
}
async fn count(
    db: &mut sqlx::PgConnection,
    kind: &str,
    key: &[u8],
    window: u32,
) -> Result<i64, Error> {
    // `statement_timestamp()` is stable, so the cutoff is one bound value and
    // the primary key range serves the scan.
    Ok(sqlx::query_scalar("SELECT COALESCE(SUM(hits),0)::bigint FROM native_login_challenge_counters WHERE kind=$1 AND key_hash=$2 AND bucket_start>statement_timestamp()-make_interval(secs=>$3::float8)")
        .bind(kind).bind(key).bind(reach_seconds(window)).fetch_one(db).await?)
}
/// Rows past retention. A counter is kept while any window could still reach
/// it. A challenge is kept while its authorization code is unexpired, so its
/// binding outlives every chance to redeem that code. Locked rows are skipped:
/// they belong to a request in flight.
const PRUNE_COUNTERS: &str = "DELETE FROM native_login_challenge_counters WHERE (kind,key_hash,bucket_start) IN (SELECT kind,key_hash,bucket_start FROM native_login_challenge_counters WHERE bucket_start<statement_timestamp()-make_interval(secs=>$1::float8) LIMIT $2 FOR UPDATE SKIP LOCKED)";
const PRUNE_CHALLENGES: &str = "DELETE FROM native_login_challenges WHERE id IN (SELECT n.id FROM native_login_challenges n JOIN oauth_authorization_codes c ON c.code_hash=n.code_hash WHERE n.expires_at<statement_timestamp()-make_interval(secs=>$1::float8) AND c.expires_at<statement_timestamp() LIMIT $2 FOR UPDATE OF n SKIP LOCKED)";
async fn sweep(
    db: &mut sqlx::PgConnection,
    config: &ChallengeConfig,
    batch: i64,
) -> Result<u64, Error> {
    let retention = f64::from(config.retention_seconds);
    let reach = reach_seconds(config.identity_window_seconds)
        .max(reach_seconds(config.source_window_seconds))
        .max(retention);
    let counters = sqlx::query(PRUNE_COUNTERS)
        .bind(reach)
        .bind(batch)
        .execute(&mut *db)
        .await?
        .rows_affected();
    let challenges = sqlx::query(PRUNE_CHALLENGES)
        .bind(retention)
        .bind(batch)
        .execute(db)
        .await?
        .rows_affected();
    Ok(counters + challenges)
}
/// Small sweep beside every counter write; [`Retention`] does the rest.
async fn prune(db: &mut sqlx::PgConnection, config: &ChallengeConfig) -> Result<(), Error> {
    if config.usable() {
        sweep(db, config, 200).await?;
    }
    Ok(())
}
/// The worker's retention job. It stays registered with the flag off, so rows
/// written while it was on still expire.
#[derive(Clone, Debug)]
pub struct Retention {
    pub pool: PgPool,
}
impl Retention {
    pub const BATCH: i64 = 1000;
    /// Deletes at most [`Self::BATCH`] rows of each table and returns how
    /// many it removed. The configured windows and retention apply whenever
    /// they are in range, with or without a signing key; only out-of-range
    /// windows or retention prune on the defaults.
    pub async fn prune(&self) -> Result<u64, Error> {
        self.prune_with(ChallengeConfig::from_env()).await
    }
    pub(crate) async fn prune_with(&self, config: ChallengeConfig) -> Result<u64, Error> {
        let config = if config.retention_valid() {
            config
        } else {
            ChallengeConfig::default()
        };
        let mut db = self.pool.acquire().await?;
        sweep(&mut db, &config, Self::BATCH).await
    }
}

/// Called by the web sign-in adapters for every password sign-in, whether or
/// not the email belongs to an account: each counts for the source, a refused
/// one also for the identity. A no-op while the flag is off. An enabled but
/// unusable policy still records, so a replica with a broken key ring keeps
/// the evidence the others act on.
pub async fn record_sign_in(
    pool: &PgPool,
    email: &str,
    source: IpAddr,
    failed: bool,
) -> Result<(), Error> {
    let config = ChallengeConfig::from_env();
    if !config.enabled {
        return Ok(());
    }
    record(pool, &config, email, source, failed).await
}
#[cfg(test)]
pub(crate) async fn record_failure(
    pool: &PgPool,
    config: &ChallengeConfig,
    email: &str,
    source: IpAddr,
) -> Result<(), Error> {
    record(pool, config, email, source, true).await
}
pub(crate) async fn record(
    pool: &PgPool,
    config: &ChallengeConfig,
    email: &str,
    source: IpAddr,
    failed: bool,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    if failed {
        bump(&mut tx, IDENTITY, &identity_key(email)).await?;
    }
    bump(&mut tx, SOURCE, &source_key(source)).await?;
    prune(&mut tx, config).await?;
    tx.commit().await?;
    Ok(())
}
/// Counts one token request for its source, in its own transaction, so the
/// count stands whatever the request's outcome.
pub(crate) async fn charge_source(
    pool: &PgPool,
    config: &ChallengeConfig,
    source: IpAddr,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    bump(&mut tx, SOURCE, &source_key(source)).await?;
    prune(&mut tx, config).await?;
    tx.commit().await?;
    Ok(())
}

/// What the token route answers on the challenge-aware path.
pub(crate) enum Outcome {
    Tokens(Tokens),
    Challenge { id: Uuid, expires_in: i32 },
}
/// Canonical lowercase UUIDv7 only.
fn challenge_id(value: &str) -> Option<Uuid> {
    let id = Uuid::parse_str(value).ok()?;
    (id.get_version_num() == 7
        && id.get_variant() == uuid::Variant::RFC4122
        && id.hyphenated().to_string() == value)
        .then_some(id)
}
/// Contract alphabet: unpadded base64url segments joined by `.`.
fn proof_shape(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROOF
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}
/// The authorization-code grant with the challenge policy. The caller has
/// checked that OAuth and the challenge are enabled and usable.
pub(crate) async fn exchange(
    pool: &PgPool,
    oauth: &Config,
    config: &ChallengeConfig,
    input: &TokenRequest,
    source: IpAddr,
) -> Result<Outcome, Error> {
    // Every attempt counts, before any check can refuse it.
    charge_source(pool, config, source).await?;
    if !oauth.client(&input.client_id) {
        return Err(Error::InvalidClient);
    }
    if !code_shape(oauth, input) {
        return Err(Error::InvalidRequest);
    }
    let retry = match (&input.challenge_id, &input.challenge_token) {
        (None, None) => None,
        (Some(id), Some(token)) => Some((
            challenge_id(id).ok_or(Error::InvalidGrant)?,
            Some(token.as_str())
                .filter(|t| proof_shape(t))
                .ok_or(Error::InvalidGrant)?,
        )),
        _ => return Err(Error::InvalidRequest),
    };
    let mut tx = pool.begin().await?;
    let code = pending_code(&mut tx, oauth, input).await?;
    let code_hash = hash(&input.code);
    let bound = sqlx::query("SELECT id, account_id, client_id, code_challenge, redirect_uri, credential_hash, policy_version, return_uri, return_state_hash, proof_nonce_hash, expires_at>clock_timestamp() AS live, completed_at IS NOT NULL AND consumed_at IS NULL AS redeemable, floor(extract(epoch FROM expires_at))::bigint AS expires_unix FROM native_login_challenges WHERE code_hash=$1 FOR UPDATE")
        .bind(&code_hash).fetch_optional(&mut *tx).await?;
    match (retry, bound) {
        (Some((id, token)), Some(row)) => {
            let challenge = Bound::read(&row, &code_hash)?;
            if challenge.id != id
                || !row.try_get::<bool, _>("live")?
                || !row.try_get::<bool, _>("redeemable")?
                || challenge.policy_version != config.policy_version
                || challenge.account != code.account
                || challenge.client_id != input.client_id
                || challenge.redirect_uri != input.redirect_uri
                || !same(&challenge.credential_hash, &code.stamp)
                || !same(
                    URL_SAFE_NO_PAD
                        .encode(hash(&input.code_verifier))
                        .as_bytes(),
                    challenge.code_challenge.as_bytes(),
                )
            {
                return Err(Error::InvalidGrant);
            }
            let nonce = verify_proof(config, &challenge, token).ok_or(Error::InvalidGrant)?;
            let stored: Option<Vec<u8>> = row.try_get("proof_nonce_hash")?;
            if !stored.is_some_and(|s| same(&s, &hash(&nonce))) {
                return Err(Error::InvalidGrant);
            }
            let consumed = sqlx::query("UPDATE native_login_challenges SET consumed_at=clock_timestamp() WHERE id=$1 AND consumed_at IS NULL AND completed_at IS NOT NULL AND expires_at>clock_timestamp()")
                .bind(id).execute(&mut *tx).await?;
            if consumed.rows_affected() != 1 {
                return Err(Error::InvalidGrant);
            }
            event(&mut tx, Some(code.account), "native_challenge_redeemed").await?;
            redeem_code(tx, input, code).await.map(Outcome::Tokens)
        }
        // A proof without a challenge, or a challenged code without its proof:
        // the client does not loop, so neither gets a second chance.
        (Some(_), None) | (None, Some(_)) => Err(Error::InvalidGrant),
        (None, None) => {
            let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
                .bind(code.account)
                .fetch_one(&mut *tx)
                .await?;
            let failures = count(
                &mut tx,
                IDENTITY,
                &identity_key(&email),
                config.identity_window_seconds,
            )
            .await?;
            let attempts = count(
                &mut tx,
                SOURCE,
                &source_key(source),
                config.source_window_seconds,
            )
            .await?;
            let reason = if failures >= i64::from(config.identity_failures) {
                "identity"
            } else if attempts >= i64::from(config.source_attempts) {
                "source"
            } else {
                return redeem_code(tx, input, code).await.map(Outcome::Tokens);
            };
            let id = Uuid::now_v7();
            // Never later than the code itself; the code row is not touched.
            // A whole second, so the signed `exp` is the exact redemption end.
            // The code the page shows is set here, so viewing it writes nothing.
            let remaining: i32 = sqlx::query_scalar("INSERT INTO native_login_challenges(id,code_hash,account_id,client_id,code_challenge,redirect_uri,credential_hash,policy_version,reason,expires_at,answer) SELECT $1,c.code_hash,c.account_id,c.client_id,c.challenge,c.redirect_uri,c.credential_hash,$2,$3,date_trunc('second',LEAST(clock_timestamp()+make_interval(secs=>$4::float8),c.expires_at)),$6 FROM oauth_authorization_codes c WHERE c.code_hash=$5 RETURNING ceil(extract(epoch FROM expires_at-clock_timestamp()))::integer")
                .bind(id)
                .bind(i32::try_from(config.policy_version).map_err(|_| Error::Unavailable)?)
                .bind(reason)
                .bind(f64::from(config.challenge_ttl_seconds))
                .bind(&code_hash)
                .bind(new_answer()?)
                .fetch_one(&mut *tx)
                .await?;
            if !(1..=300).contains(&remaining) {
                return Err(Error::InvalidGrant);
            }
            event(&mut tx, Some(code.account), "native_challenge_issued").await?;
            tx.commit().await?;
            Ok(Outcome::Challenge {
                id,
                expires_in: remaining,
            })
        }
    }
}

/// The durable binding of one challenge row.
struct Bound {
    id: Uuid,
    code_hash: Vec<u8>,
    account: Uuid,
    client_id: String,
    code_challenge: String,
    redirect_uri: String,
    credential_hash: Vec<u8>,
    policy_version: u32,
    return_uri: Option<String>,
    return_state_hash: Option<Vec<u8>>,
    expires_unix: i64,
}
impl Bound {
    fn read(row: &sqlx::postgres::PgRow, code_hash: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            code_hash: code_hash.to_vec(),
            account: row.try_get("account_id")?,
            client_id: row.try_get("client_id")?,
            code_challenge: row.try_get("code_challenge")?,
            redirect_uri: row.try_get("redirect_uri")?,
            credential_hash: row.try_get("credential_hash")?,
            policy_version: u32::try_from(row.try_get::<i32, _>("policy_version")?)
                .map_err(|_| Error::InvalidGrant)?,
            return_uri: row.try_get("return_uri")?,
            return_state_hash: row.try_get("return_state_hash")?,
            expires_unix: row.try_get("expires_unix")?,
        })
    }
    /// Digest over the whole pending flow and the browser attempt.
    fn digest(&self) -> Result<String, Error> {
        let input = serde_json::to_string(&(
            URL_SAFE_NO_PAD.encode(&self.code_hash),
            self.id,
            self.account,
            &self.client_id,
            &self.code_challenge,
            &self.redirect_uri,
            URL_SAFE_NO_PAD.encode(&self.credential_hash),
            self.policy_version,
            &self.return_uri,
            self.return_state_hash
                .as_deref()
                .map(|h| URL_SAFE_NO_PAD.encode(h)),
        ))
        .map_err(|_| Error::Unavailable)?;
        Ok(URL_SAFE_NO_PAD.encode(hash(&input)))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    aud: String,
    cid: String,
    bind: String,
    nonce: String,
    exp: i64,
    pv: u32,
}
fn sign_proof(config: &ChallengeConfig, bound: &Bound, nonce: &str) -> Result<String, Error> {
    let (kid, key) = config.keys.first().ok_or(Error::Unavailable)?;
    let claims = Claims {
        aud: AUDIENCE.into(),
        cid: bound.id.hyphenated().to_string(),
        bind: bound.digest()?,
        nonce: nonce.into(),
        exp: bound.expires_unix,
        pv: bound.policy_version,
    };
    let payload =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(|_| Error::Unavailable)?);
    let signed = format!("{PROOF_VERSION}.{kid}.{payload}");
    let signature = key.sign(signed.as_bytes());
    let proof = format!("{signed}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()));
    if !proof_shape(&proof) {
        return Err(Error::Unavailable);
    }
    Ok(proof)
}
fn decode(segment: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(segment)
        .ok()
        .filter(|v| URL_SAFE_NO_PAD.encode(v) == segment)
}
/// Returns the proof's nonce when signature, audience and binding all hold.
fn verify_proof(config: &ChallengeConfig, bound: &Bound, proof: &str) -> Option<String> {
    let mut parts = proof.split('.');
    let (Some(version), Some(kid), Some(payload), Some(signature), None) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return None;
    };
    if version != PROOF_VERSION {
        return None;
    }
    let (_, key) = config.keys.iter().find(|(k, _)| k == kid)?;
    let signature = Signature::from_slice(&decode(signature)?).ok()?;
    key.verifying_key()
        .verify_strict(format!("{version}.{kid}.{payload}").as_bytes(), &signature)
        .ok()?;
    let claims: Claims = serde_json::from_slice(&decode(payload)?).ok()?;
    let bind = bound.digest().ok()?;
    (claims.aud == AUDIENCE
        && claims.cid == bound.id.hyphenated().to_string()
        && same(claims.bind.as_bytes(), bind.as_bytes())
        && claims.exp == bound.expires_unix
        && claims.pv == bound.policy_version
        && claims.pv == config.policy_version
        && opaque(&claims.nonce))
    .then_some(claims.nonce)
}

/// The browser half: `/oauth/challenge` query and form fields.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub client_id: String,
    pub challenge_id: String,
    pub redirect_uri: String,
    pub state: String,
}
impl Attempt {
    /// The return URI is a new IPv4 loopback URI registered for the client,
    /// with an explicit, nonzero port the client is listening on.
    pub(crate) fn valid(&self, oauth: &Config) -> Option<Uuid> {
        let loopback =
            loopback_parts(&self.redirect_uri).is_some_and(|(host, _)| host == "127.0.0.1");
        let port = self
            .redirect_uri
            .strip_prefix("http://127.0.0.1:")
            .and_then(|rest| rest.split('/').next())
            .and_then(|port| port.parse::<u16>().ok())
            .is_some_and(|port| port != 0);
        (loopback
            && port
            && self.state.len() >= 16
            && self.state.len() <= 1024
            && self.state.bytes().all(|b| b.is_ascii_graphic())
            && oauth.redirect(&self.client_id, &self.redirect_uri, registered_redirect))
        .then(|| challenge_id(&self.challenge_id))
        .flatten()
    }
}
/// What the page shows: the one-time code to type back.
pub(crate) struct Prompt {
    pub(crate) code: String,
}
/// The outcome of a submitted answer.
pub(crate) enum Completion {
    /// Redirect target carrying the proof and the exact state.
    Redirect(String),
    /// Wrong answer with attempts left: show this new code.
    Retry(Prompt),
}
fn new_answer() -> Result<String, Error> {
    let mut code = String::with_capacity(ANSWER_DIGITS);
    while code.len() < ANSWER_DIGITS {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
        // Rejection sampling keeps every digit uniform.
        for b in bytes.into_iter().filter(|b| *b < 250) {
            if code.len() < ANSWER_DIGITS {
                code.push(char::from(b'0' + b % 10));
            }
        }
    }
    Ok(code)
}
/// The browser session, as at code issuance: unexpired and of the account's
/// current native security generation.
const SESSION: &str = "SELECT 1 FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.account_id=$2 AND (s.authorization_generation IS NULL OR s.authorization_generation=a.native_security_generation) AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp() FOR UPDATE OF s";
/// The bound code, still redeemable under the current credentials.
const CODE: &str = "SELECT c.expires_at>clock_timestamp() AND c.used_at IS NULL AND c.revoked_at IS NULL AND c.credential_hash=$2 FROM oauth_authorization_codes c WHERE c.code_hash=(SELECT code_hash FROM native_login_challenges WHERE id=$1) FOR UPDATE OF c";
const LIVE: &str = "SELECT id, code_hash, account_id, client_id, code_challenge, redirect_uri, credential_hash, policy_version, return_uri, return_state_hash, answer, answer_attempts, floor(extract(epoch FROM expires_at))::bigint AS expires_unix FROM native_login_challenges WHERE id=$1 AND client_id=$2 AND account_id=$3 AND policy_version=$4 AND expires_at>clock_timestamp() AND completed_at IS NULL AND answer_attempts<$5 FOR UPDATE";
/// Locks in the token route's order: account, then (here) the browser
/// session, then the code, then the challenge. Every refusal is
/// `InvalidRequest`, so the page tells nothing apart.
async fn live(
    tx: &mut Transaction<'_, Postgres>,
    config: &ChallengeConfig,
    account: Uuid,
    proof: &BrowserProof,
    id: Uuid,
    attempt: &Attempt,
) -> Result<sqlx::postgres::PgRow, Error> {
    let refuse = |e: Error| match e {
        Error::Unavailable => Error::Unavailable,
        _ => Error::InvalidRequest,
    };
    let current = stamp(tx, account).await.map_err(refuse)?;
    sqlx::query(SESSION)
        .bind(&proof.session_hash)
        .bind(account)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::InvalidRequest)?;
    let code: Option<Option<bool>> = sqlx::query_scalar(CODE)
        .bind(id)
        .bind(&current)
        .fetch_optional(&mut **tx)
        .await?;
    if code != Some(Some(true)) {
        return Err(Error::InvalidRequest);
    }
    let max = i32::try_from(config.max_answer_attempts).map_err(|_| Error::Unavailable)?;
    let version = i32::try_from(config.policy_version).map_err(|_| Error::Unavailable)?;
    sqlx::query(LIVE)
        .bind(id)
        .bind(&attempt.client_id)
        .bind(account)
        .bind(version)
        .bind(max)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::InvalidRequest)
}
/// The first answer posted binds the return URI and state (`Ok(true)`: not
/// bound yet); every later view and answer must repeat them.
fn bind_attempt(row: &sqlx::postgres::PgRow, attempt: &Attempt) -> Result<bool, Error> {
    let uri: Option<String> = row.try_get("return_uri")?;
    let state: Option<Vec<u8>> = row.try_get("return_state_hash")?;
    match (uri, state) {
        (None, None) => Ok(true),
        (Some(uri), Some(state))
            if uri == attempt.redirect_uri && same(&state, &hash(&attempt.state)) =>
        {
            Ok(false)
        }
        _ => Err(Error::InvalidRequest),
    }
}
/// A view writes nothing: it shows the current code, so a reload or second
/// tab keeps it valid. Binding waits for the CSRF-checked answer.
pub(crate) async fn show(
    pool: &PgPool,
    config: &ChallengeConfig,
    account: Uuid,
    proof: &BrowserProof,
    id: Uuid,
    attempt: &Attempt,
) -> Result<Prompt, Error> {
    let mut tx = pool.begin().await?;
    let row = live(&mut tx, config, account, proof, id, attempt).await?;
    bind_attempt(&row, attempt)?;
    let code: Option<String> = row.try_get("answer")?;
    tx.commit().await?;
    code.map(|code| Prompt { code })
        .ok_or(Error::InvalidRequest)
}
pub(crate) async fn complete(
    pool: &PgPool,
    config: &ChallengeConfig,
    account: Uuid,
    proof: &BrowserProof,
    id: Uuid,
    attempt: &Attempt,
    answer: &str,
) -> Result<Completion, Error> {
    let mut tx = pool.begin().await?;
    let row = live(&mut tx, config, account, proof, id, attempt).await?;
    if bind_attempt(&row, attempt)? {
        let bound = sqlx::query("UPDATE native_login_challenges SET return_uri=$2, return_state_hash=$3 WHERE id=$1 AND return_uri IS NULL")
            .bind(id)
            .bind(&attempt.redirect_uri)
            .bind(hash(&attempt.state))
            .execute(&mut *tx)
            .await?;
        if bound.rows_affected() != 1 {
            return Err(Error::InvalidRequest);
        }
        event(&mut tx, Some(account), "native_challenge_bound").await?;
    }
    let expected: Option<String> = row.try_get("answer")?;
    let correct = answer.len() == ANSWER_DIGITS
        && answer.bytes().all(|b| b.is_ascii_digit())
        && expected.is_some_and(|e| same(e.as_bytes(), answer.as_bytes()));
    if !correct {
        let attempts: i32 = row.try_get("answer_attempts")?;
        let left = i64::from(attempts) + 1 < i64::from(config.max_answer_attempts);
        let code = new_answer()?;
        sqlx::query("UPDATE native_login_challenges SET answer_attempts=answer_attempts+1, answer=$2 WHERE id=$1")
            .bind(id)
            .bind(left.then_some(code.as_str()))
            .execute(&mut *tx)
            .await?;
        event(&mut tx, Some(account), "native_challenge_answer_refused").await?;
        tx.commit().await?;
        return if left {
            Ok(Completion::Retry(Prompt { code }))
        } else {
            Err(Error::InvalidRequest)
        };
    }
    let code_hash: Vec<u8> = row.try_get("code_hash")?;
    let mut bound = Bound::read(&row, &code_hash)?;
    // The row was read before this request may have bound it; the attempt is
    // now the binding either way.
    bound.return_uri = Some(attempt.redirect_uri.clone());
    bound.return_state_hash = Some(hash(&attempt.state));
    let nonce = secret()?;
    let signed = sign_proof(config, &bound, &nonce)?;
    // Challenge expiry and the browser session are checked again at the write
    // that completes the challenge: time moves on while the locks are held.
    let completed = sqlx::query("UPDATE native_login_challenges SET completed_at=clock_timestamp(), proof_nonce_hash=$2, answer=NULL WHERE id=$1 AND completed_at IS NULL AND expires_at>clock_timestamp() AND EXISTS (SELECT 1 FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$3 AND s.account_id=$4 AND (s.authorization_generation IS NULL OR s.authorization_generation=a.native_security_generation) AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp())")
        .bind(id)
        .bind(hash(&nonce))
        .bind(&proof.session_hash)
        .bind(account)
        .execute(&mut *tx)
        .await?;
    if completed.rows_affected() != 1 {
        return Err(Error::InvalidRequest);
    }
    event(&mut tx, Some(account), "native_challenge_completed").await?;
    tx.commit().await?;
    let mut target = url::Url::parse(&attempt.redirect_uri).map_err(|_| Error::InvalidRequest)?;
    target
        .query_pairs_mut()
        .append_pair("code", &signed)
        .append_pair("state", &attempt.state);
    Ok(Completion::Redirect(target.into()))
}

impl std::fmt::Debug for Attempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attempt([redacted])")
    }
}
impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Prompt([redacted])")
    }
}
