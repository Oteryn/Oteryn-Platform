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
        let Ok(raw) = std::env::var(CONFIG_ENV) else {
            return Self::default();
        };
        let mut config = serde_json::from_str::<Self>(&raw).unwrap_or_else(|_| Self {
            enabled: true,
            broken: true,
            ..Self::default()
        });
        if config.enabled {
            match std::env::var(KEYS_ENV).ok().as_deref().and_then(parse_keys) {
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
    /// Enabled with a sane policy and at least one signing key.
    pub(crate) fn usable(&self) -> bool {
        let window = 1..=86_400;
        !self.broken
            && !self.keys.is_empty()
            && self.identity_failures >= 1
            && self.source_attempts >= 1
            && window.contains(&self.identity_window_seconds)
            && window.contains(&self.source_window_seconds)
            && self.retention_seconds
                >= self.identity_window_seconds.max(self.source_window_seconds)
            && self.retention_seconds <= 30 * 86_400
            && (1..=300).contains(&self.challenge_ttl_seconds)
            && (1..=20).contains(&self.max_answer_attempts)
            && (1..=i32::MAX as u32).contains(&self.policy_version)
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
/// Windows are summed from ten sub-buckets, so a window slides in tenths.
fn bucket_seconds(window: u32) -> i64 {
    i64::from((window / 10).max(1))
}
async fn bump(
    db: &mut sqlx::PgConnection,
    kind: &str,
    key: &[u8],
    window: u32,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO native_login_challenge_counters(kind,key_hash,bucket_start,hits) VALUES($1,$2,to_timestamp((floor(extract(epoch FROM clock_timestamp())/$3::bigint)*$3::bigint)::float8),1) ON CONFLICT(kind,key_hash,bucket_start) DO UPDATE SET hits=LEAST(native_login_challenge_counters.hits,2147483646)+1")
        .bind(kind).bind(key).bind(bucket_seconds(window)).execute(db).await?;
    Ok(())
}
async fn count(
    db: &mut sqlx::PgConnection,
    kind: &str,
    key: &[u8],
    window: u32,
) -> Result<i64, Error> {
    Ok(sqlx::query_scalar("SELECT COALESCE(SUM(hits),0)::bigint FROM native_login_challenge_counters WHERE kind=$1 AND key_hash=$2 AND bucket_start>clock_timestamp()-make_interval(secs=>$3::float8)")
        .bind(kind).bind(key).bind(f64::from(window)).fetch_one(db).await?)
}
/// Bounded retention sweep, run beside every counter write.
async fn prune(db: &mut sqlx::PgConnection, config: &ChallengeConfig) -> Result<(), Error> {
    let retention = f64::from(config.retention_seconds);
    sqlx::query("DELETE FROM native_login_challenge_counters WHERE ctid IN (SELECT ctid FROM native_login_challenge_counters WHERE bucket_start<clock_timestamp()-make_interval(secs=>$1::float8) LIMIT 200)")
        .bind(retention).execute(&mut *db).await?;
    sqlx::query("DELETE FROM native_login_challenges WHERE id IN (SELECT id FROM native_login_challenges WHERE expires_at<clock_timestamp()-make_interval(secs=>$1::float8) LIMIT 200)")
        .bind(retention).execute(db).await?;
    Ok(())
}

/// Called by the web sign-in adapter for every rejected password sign-in,
/// whether or not the email belongs to an account. A no-op unless the feature
/// is on and usable; with it off nothing is read or written.
pub async fn record_sign_in_failure(
    pool: &PgPool,
    email: &str,
    source: IpAddr,
) -> Result<(), Error> {
    let config = ChallengeConfig::from_env();
    if !config.enabled || !config.usable() {
        // A refusing configuration already closes the token route.
        return Ok(());
    }
    record_failure(pool, &config, email, source).await
}
pub(crate) async fn record_failure(
    pool: &PgPool,
    config: &ChallengeConfig,
    email: &str,
    source: IpAddr,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    bump(
        &mut tx,
        IDENTITY,
        &identity_key(email),
        config.identity_window_seconds,
    )
    .await?;
    bump(
        &mut tx,
        SOURCE,
        &source_key(source),
        config.source_window_seconds,
    )
    .await?;
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
    // Each attempt counts in its own transaction, so a refusal still counts.
    {
        let mut tx = pool.begin().await?;
        bump(
            &mut tx,
            SOURCE,
            &source_key(source),
            config.source_window_seconds,
        )
        .await?;
        prune(&mut tx, config).await?;
        tx.commit().await?;
    }
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
            let remaining: i32 = sqlx::query_scalar("INSERT INTO native_login_challenges(id,code_hash,account_id,client_id,code_challenge,redirect_uri,credential_hash,policy_version,reason,expires_at) SELECT $1,c.code_hash,c.account_id,c.client_id,c.challenge,c.redirect_uri,c.credential_hash,$2,$3,LEAST(clock_timestamp()+make_interval(secs=>$4::float8),c.expires_at) FROM oauth_authorization_codes c WHERE c.code_hash=$5 RETURNING ceil(extract(epoch FROM expires_at-clock_timestamp()))::integer")
                .bind(id)
                .bind(i32::try_from(config.policy_version).map_err(|_| Error::Unavailable)?)
                .bind(reason)
                .bind(f64::from(config.challenge_ttl_seconds))
                .bind(&code_hash)
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
    client_id: String,
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
        client_id: bound.client_id.clone(),
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
        && claims.client_id == bound.client_id
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
    /// The return URI is a new IPv4 loopback URI registered for the client.
    pub(crate) fn valid(&self, oauth: &Config) -> Option<Uuid> {
        let loopback =
            loopback_parts(&self.redirect_uri).is_some_and(|(host, _)| host == "127.0.0.1");
        (loopback
            && self.state.len() >= 16
            && self.state.len() <= 1024
            && self.state.bytes().all(|b| b.is_ascii_graphic())
            && oauth.redirect(&self.client_id, &self.redirect_uri, registered_redirect))
        .then(|| challenge_id(&self.challenge_id))
        .flatten()
    }
}
/// What the page shows: a fresh one-time code to type back.
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
fn answer_hash(id: Uuid, answer: &str) -> Vec<u8> {
    hash(&format!("native-login-answer\0{id}\0{answer}"))
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
const LIVE: &str = "SELECT id, code_hash, account_id, client_id, code_challenge, redirect_uri, credential_hash, policy_version, return_uri, return_state_hash, answer_hash, answer_attempts, floor(extract(epoch FROM expires_at))::bigint AS expires_unix FROM native_login_challenges WHERE id=$1 AND client_id=$2 AND account_id=$3 AND expires_at>clock_timestamp() AND completed_at IS NULL AND answer_attempts<$4 FOR UPDATE";
async fn live(
    tx: &mut Transaction<'_, Postgres>,
    config: &ChallengeConfig,
    account: Uuid,
    id: Uuid,
    attempt: &Attempt,
) -> Result<Option<sqlx::postgres::PgRow>, Error> {
    let max = i32::try_from(config.max_answer_attempts).map_err(|_| Error::Unavailable)?;
    Ok(sqlx::query(LIVE)
        .bind(id)
        .bind(&attempt.client_id)
        .bind(account)
        .bind(max)
        .fetch_optional(&mut **tx)
        .await?)
}
/// The first browser attempt binds the return URI and state; later views
/// must repeat them. Unknown, foreign, expired and mismatched challenges are
/// all `InvalidRequest`, so the page tells nothing apart.
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
pub(crate) async fn show(
    pool: &PgPool,
    config: &ChallengeConfig,
    account: Uuid,
    id: Uuid,
    attempt: &Attempt,
) -> Result<Prompt, Error> {
    let mut tx = pool.begin().await?;
    let row = live(&mut tx, config, account, id, attempt)
        .await?
        .ok_or(Error::InvalidRequest)?;
    let first = bind_attempt(&row, attempt)?;
    let code = new_answer()?;
    sqlx::query("UPDATE native_login_challenges SET answer_hash=$2, return_uri=COALESCE(return_uri,$3), return_state_hash=COALESCE(return_state_hash,$4) WHERE id=$1")
        .bind(id)
        .bind(answer_hash(id, &code))
        .bind(&attempt.redirect_uri)
        .bind(hash(&attempt.state))
        .execute(&mut *tx)
        .await?;
    if first {
        event(&mut tx, Some(account), "native_challenge_presented").await?;
    }
    tx.commit().await?;
    Ok(Prompt { code })
}
pub(crate) async fn complete(
    pool: &PgPool,
    config: &ChallengeConfig,
    account: Uuid,
    id: Uuid,
    attempt: &Attempt,
    answer: &str,
) -> Result<Completion, Error> {
    let mut tx = pool.begin().await?;
    let row = live(&mut tx, config, account, id, attempt)
        .await?
        .ok_or(Error::InvalidRequest)?;
    if bind_attempt(&row, attempt)? {
        // A POST never binds: the page must have been shown for this attempt.
        return Err(Error::InvalidRequest);
    }
    let expected: Option<Vec<u8>> = row.try_get("answer_hash")?;
    let correct = answer.len() == ANSWER_DIGITS
        && answer.bytes().all(|b| b.is_ascii_digit())
        && expected.is_some_and(|e| same(&e, &answer_hash(id, answer)));
    if !correct {
        let attempts: i32 = row.try_get("answer_attempts")?;
        let left = i64::from(attempts) + 1 < i64::from(config.max_answer_attempts);
        let code = new_answer()?;
        sqlx::query("UPDATE native_login_challenges SET answer_attempts=answer_attempts+1, answer_hash=$2 WHERE id=$1")
            .bind(id)
            .bind(left.then(|| answer_hash(id, &code)))
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
    let bound = Bound::read(&row, &code_hash)?;
    let nonce = secret()?;
    let proof = sign_proof(config, &bound, &nonce)?;
    sqlx::query("UPDATE native_login_challenges SET completed_at=clock_timestamp(), proof_nonce_hash=$2, answer_hash=NULL WHERE id=$1")
        .bind(id)
        .bind(hash(&nonce))
        .execute(&mut *tx)
        .await?;
    event(&mut tx, Some(account), "native_challenge_completed").await?;
    tx.commit().await?;
    let mut target = url::Url::parse(&attempt.redirect_uri).map_err(|_| Error::InvalidRequest)?;
    target
        .query_pairs_mut()
        .append_pair("code", &proof)
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
