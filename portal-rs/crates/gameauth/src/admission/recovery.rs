//! Gateway login contract §19.4 recovery issuance (`native-recoveries`), §19.7
//! handle revocation and the §19.8 recovery errors on the private issuer.
//! Testing only and default off (owner decision U21): recovery runs only while
//! protocol 3 and recovery handles are on and §19.15 device binding is off;
//! with binding on it stays `NATIVE_LOGIN_UNAVAILABLE` until the §19.15 device
//! proof is built. Recovery attempts share the `attempt_ref` namespace and the
//! advisory attempt lock with fresh admissions, and are stored like them: the
//! signing input only, never the token (§6.1).
use super::{
    AccountState, AccountStateLookup, Endpoint, Error, GrantSigner, grant, protocol3, store,
    tickets, wire,
};
use crate::runtime;
use oteryn_internal_api::canonical::{self, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Largest recovery response (§19.9).
pub const MAX_RESPONSE: usize = 6144;
/// Committed recovery attempts per `AccountId` per hour (§19.9).
const ACCOUNT_ATTEMPTS_PER_HOUR: i64 = 20;
const ISSUER: &str = crate::keys::RECOVERY_ISSUER;
const AUDIENCE: &str = "urn:oteryn:game:recovery";
const PROFILE: &str = crate::keys::RECOVERY_PROFILE;
const PURPOSE: &str = "existing_actor_recovery";
/// The `revoked` answer of §19.7, for every well-formed request.
pub const REVOKED: &[u8] = br#"{"protocol_version":3,"revoked":true}"#;

/// Recovery issuance policy beside the protocol 3 settings. The binary composes
/// `signer` from the §19.5 recovery custody only when issuance is permitted.
#[derive(Debug, Clone)]
pub struct RecoveryIssuer {
    /// The recovery-scope signer (`KeyringSigner::recovery`); never the fresh one.
    pub signer: Option<Arc<dyn GrantSigner>>,
    /// `source_authority` of the §5 read model; empty is no Character source,
    /// which refuses with `NATIVE_RECOVERY_ROUTE_UNAVAILABLE` (§19.4 step 4).
    pub projection_authority: String,
    /// Grant TTL, 5..=30 (§8.2, U4).
    pub ttl_seconds: u8,
    /// Platform clock uncertainty for retirement, 0..=5.
    pub clock_uncertainty_seconds: u8,
}
impl RecoveryIssuer {
    fn signer(&self) -> Result<&dyn GrantSigner, RecoveryError> {
        match &self.signer {
            Some(signer)
                if (5..=30).contains(&self.ttl_seconds) && self.clock_uncertainty_seconds <= 5 =>
            {
                Ok(signer.as_ref())
            }
            _ => Err(RecoveryError::Unavailable),
        }
    }
}

/// §19.8 recovery errors and the protocol 2 codes protocol 3 reuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryError {
    Malformed,
    UnsupportedVersion,
    OfferUnsupported,
    HandleRejected,
    DeviceProofRejected,
    SecurityDenied,
    AttemptConflict,
    AttemptPending,
    CharacterConflict,
    WorldStale,
    RouteUnavailable,
    GrantExpired,
    ReconciliationRequired,
    RateLimited,
    Unavailable,
}
impl RecoveryError {
    pub fn status(self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::Malformed | Self::UnsupportedVersion => StatusCode::BAD_REQUEST,
            Self::HandleRejected | Self::DeviceProofRejected | Self::SecurityDenied => {
                StatusCode::UNAUTHORIZED
            }
            Self::OfferUnsupported
            | Self::AttemptConflict
            | Self::AttemptPending
            | Self::CharacterConflict
            | Self::WorldStale
            | Self::GrantExpired => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::RouteUnavailable | Self::ReconciliationRequired | Self::Unavailable => {
                StatusCode::SERVICE_UNAVAILABLE
            }
        }
    }
    /// Public code and class; `SECURITY_TERMINAL` rows collapse (§19.8).
    fn public(self) -> (&'static str, &'static str) {
        match self {
            Self::Malformed => ("NATIVE_RECOVERY_REQUEST_MALFORMED", "RETRY_LOGIN"),
            Self::UnsupportedVersion => {
                ("NATIVE_LOGIN_UNSUPPORTED_VERSION", "CLIENT_UPDATE_REQUIRED")
            }
            Self::OfferUnsupported => ("NATIVE_LOGIN_OFFER_UNSUPPORTED", "CLIENT_UPDATE_REQUIRED"),
            Self::HandleRejected | Self::DeviceProofRejected | Self::SecurityDenied => (
                "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
                "AUTHENTICATION_REQUIRED",
            ),
            Self::AttemptConflict => ("NATIVE_RECOVERY_ATTEMPT_CONFLICT", "RETRY_LOGIN"),
            Self::AttemptPending => ("NATIVE_RECOVERY_ATTEMPT_PENDING", "TEMPORARILY_UNAVAILABLE"),
            Self::CharacterConflict => {
                ("NATIVE_RECOVERY_CHARACTER_CONFLICT", "SESSION_UNAVAILABLE")
            }
            Self::WorldStale => ("NATIVE_RECOVERY_WORLD_STALE", "RETRY_LOGIN"),
            Self::RouteUnavailable => (
                "NATIVE_RECOVERY_ROUTE_UNAVAILABLE",
                "TEMPORARILY_UNAVAILABLE",
            ),
            Self::GrantExpired => ("NATIVE_RECOVERY_GRANT_EXPIRED", "RETRY_LOGIN"),
            Self::ReconciliationRequired => (
                "RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED",
                "TEMPORARILY_UNAVAILABLE",
            ),
            Self::RateLimited => ("NATIVE_LOGIN_RATE_LIMITED", "TEMPORARILY_UNAVAILABLE"),
            Self::Unavailable => ("NATIVE_LOGIN_UNAVAILABLE", "TEMPORARILY_UNAVAILABLE"),
        }
    }
    /// The §11.1 body with `protocol_version` `protocol` (3, or 2 while the
    /// protocol 3 switch is off).
    pub fn body(self, attempt: Option<&str>, protocol: u8) -> Vec<u8> {
        let (code, class) = self.public();
        canonical::encode(&Value::Object(vec![
            (
                "protocol_version".into(),
                Value::Unsigned(u64::from(protocol)),
            ),
            (
                "error".into(),
                Value::Object(vec![
                    ("code".into(), Value::String(code.into())),
                    ("public_class".into(), Value::String(class.into())),
                ]),
            ),
            (
                "attempt_ref".into(),
                attempt.map_or(Value::Null, |s| Value::String(s.to_owned())),
            ),
        ]))
        .unwrap_or_default()
    }
}
impl From<Error> for RecoveryError {
    fn from(error: Error) -> Self {
        match error {
            Error::Malformed => Self::Malformed,
            Error::UnsupportedVersion => Self::UnsupportedVersion,
            Error::OfferUnsupported => Self::OfferUnsupported,
            Error::TicketRejected => Self::HandleRejected,
            Error::SecurityDenied => Self::SecurityDenied,
            Error::AttemptConflict => Self::AttemptConflict,
            Error::CharacterConflict => Self::CharacterConflict,
            Error::RouteUnavailable | Error::ChannelUnavailable => Self::RouteUnavailable,
            Error::RateLimited => Self::RateLimited,
            Error::GrantExpired => Self::GrantExpired,
            Error::ReconciliationRequired => Self::ReconciliationRequired,
            Error::Unavailable => Self::Unavailable,
        }
    }
}
/// A refusal and, for `AttemptPending`, the whole seconds until retirement.
#[derive(Debug)]
pub(super) struct Refusal {
    pub error: RecoveryError,
    pub retry_after: Option<u64>,
}
impl From<RecoveryError> for Refusal {
    fn from(error: RecoveryError) -> Self {
        Self {
            error,
            retry_after: None,
        }
    }
}
impl From<Error> for Refusal {
    fn from(error: Error) -> Self {
        RecoveryError::from(error).into()
    }
}

/// A decoded §19.4 request. The handle is kept as its hash only.
pub(super) struct RecoveryRequest {
    pub attempt_ref: Uuid,
    pub character_id: Uuid,
    handle_hash: [u8; 32],
    digest: [u8; 32],
}
impl std::fmt::Debug for RecoveryRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryRequest")
            .field("attempt_ref", &self.attempt_ref)
            .finish_non_exhaustive()
    }
}
impl RecoveryRequest {
    /// The §19.4 member set with `device_proof` absent (§19.15 binding off).
    pub(super) fn decode(raw: &[u8]) -> Result<Self, RecoveryError> {
        let value = canonical::decode(raw, wire::MAX_BODY, 5).map_err(|_| Error::Malformed)?;
        if wire::number(value.get("protocol_version"))? != 3 {
            return Err(RecoveryError::UnsupportedVersion);
        }
        wire::keys(
            &value,
            &[
                "protocol_version",
                "recovery_handle",
                "attempt_ref",
                "character_id",
                "offer",
            ],
        )?;
        let handle_hash = protocol3::handle_hash(wire::string(value.get("recovery_handle"))?)
            .ok_or(RecoveryError::Malformed)?;
        let attempt_ref = wire::uuid(wire::string(value.get("attempt_ref"))?)?;
        let character_id = wire::uuid(wire::string(value.get("character_id"))?)?;
        let offer = wire::offer(value.get("offer"))?;
        // §6.2 step 2 with the handle hash in place of the ticket hash: the
        // remaining bound members are the Character and the offer.
        let binding = Value::Array(vec![
            Value::String("recovery".into()),
            Value::String(character_id.to_string()),
            offer,
        ]);
        let digest =
            Sha256::digest(canonical::encode(&binding).map_err(|_| Error::Malformed)?).into();
        Ok(Self {
            attempt_ref,
            character_id,
            handle_hash,
            digest,
        })
    }
}
/// The §19.7 body: exactly `protocol_version` 3 and a well-formed handle.
pub(super) fn decode_revoke(raw: &[u8]) -> Result<[u8; 32], RecoveryError> {
    let value = canonical::decode(raw, wire::MAX_BODY, 5).map_err(|_| Error::Malformed)?;
    if wire::number(value.get("protocol_version"))? != 3 {
        return Err(RecoveryError::UnsupportedVersion);
    }
    wire::keys(&value, &["protocol_version", "recovery_handle"])?;
    protocol3::handle_hash(wire::string(value.get("recovery_handle"))?)
        .ok_or(RecoveryError::Malformed)
}

#[derive(Serialize, Deserialize)]
struct Grant {
    profile: String,
    token: String,
    valid_for_seconds: i64,
}
#[derive(Serialize, Deserialize)]
struct Success {
    protocol_version: u8,
    attempt_ref: Uuid,
    world_id: Uuid,
    channel_id: Uuid,
    endpoint: Endpoint,
    grant: Grant,
}
fn encode(success: &Success) -> Result<Vec<u8>, RecoveryError> {
    let raw = serde_json::to_vec(success).map_err(|_| RecoveryError::Unavailable)?;
    let value = canonical::decode(&raw, MAX_RESPONSE, 5).map_err(|_| RecoveryError::Unavailable)?;
    canonical::encode(&value).map_err(|_| RecoveryError::Unavailable)
}
/// The §19.4 success body, in the contract's member order.
pub(super) fn success(
    attempt: Uuid,
    world: Uuid,
    channel: Uuid,
    endpoint: &Endpoint,
    token: &str,
    valid_for_seconds: i64,
) -> Result<Vec<u8>, RecoveryError> {
    encode(&Success {
        protocol_version: 3,
        attempt_ref: attempt,
        world_id: world,
        channel_id: channel,
        endpoint: endpoint.clone(),
        grant: Grant {
            profile: PROFILE.into(),
            token: token.into(),
            valid_for_seconds,
        },
    })
}
fn stored(raw: &[u8]) -> Result<Success, RecoveryError> {
    serde_json::from_slice(raw).map_err(|_| RecoveryError::ReconciliationRequired)
}

/// Everything §19.4 step 6 binds.
struct Claims<'a> {
    iat: u64,
    ttl: u8,
    jti: String,
    attempt: Uuid,
    account: Uuid,
    character: Uuid,
    world: Uuid,
    security_generation: i64,
    report: &'a Value,
}
/// §19.4 step 6 claims, exactly in this order and without whitespace. No
/// channel, route, observation, ownership or offer claim (recovery profile §7).
fn payload(claims: &Claims<'_>) -> Result<Vec<u8>, Error> {
    if claims.security_generation <= 0 || claims.jti.len() != 43 || !(5..=30).contains(&claims.ttl)
    {
        return Err(Error::Unavailable);
    }
    let text = |s: &str| Value::String(s.into());
    let id = |id: Uuid| Value::String(id.to_string());
    let mut members = vec![
        ("iss", text(ISSUER)),
        ("aud", text(AUDIENCE)),
        ("iat", Value::Unsigned(claims.iat)),
        ("nbf", Value::Unsigned(claims.iat)),
        (
            "exp",
            Value::Unsigned(
                claims
                    .iat
                    .checked_add(claims.ttl.into())
                    .ok_or(Error::Unavailable)?,
            ),
        ),
        ("jti", text(&claims.jti)),
        ("profile", text(PROFILE)),
        ("purpose", text(PURPOSE)),
        ("attempt_ref", id(claims.attempt)),
        ("account_id", id(claims.account)),
        ("character_id", id(claims.character)),
        ("world_id", id(claims.world)),
        (
            "account_security_generation",
            Value::String(claims.security_generation.to_string()),
        ),
        ("protocol_major", Value::Unsigned(1)),
        ("transport_profile", Value::Unsigned(1)),
    ];
    for name in [
        "ruleset_revision",
        "content_revision",
        "map_revision",
        "world_policy_revision",
    ] {
        members.push((name, grant::revision(claims.report, name)?));
    }
    let bytes = canonical::encode(&Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    ))
    .map_err(|_| Error::Unavailable)?;
    if bytes.len() > 3072 {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}

/// What a handle binds, after every §19.4 step 2 check passed.
struct Bound {
    account: Uuid,
    world: Uuid,
    channel: Uuid,
    security_generation: i64,
}
/// §19.4 step 2 against current state, in the contract's lock order: handle,
/// Identity, device family (shared), play generation. A family-bound handle
/// also needs its family live and current (§19.15.1: an expired family's
/// handles count as revoked on every path); nothing here rotates or touches it.
async fn authenticate(
    tx: &mut PgConnection,
    request: &RecoveryRequest,
    accounts: &dyn AccountStateLookup,
    now: i64,
) -> Result<Bound, RecoveryError> {
    let get = |_: sqlx::Error| RecoveryError::Unavailable;
    let row = sqlx::query("SELECT account_id, character_id, world_id, channel_id, security_generation, play_generation, device_family_id, expires_at, revoked_at IS NULL AS live FROM native_recovery_handles WHERE handle_hash=$1 FOR UPDATE")
        .bind(&request.handle_hash[..]).fetch_optional(&mut *tx).await.map_err(get)?
        .ok_or(RecoveryError::HandleRejected)?;
    let live: bool = row.try_get("live").map_err(get)?;
    let expires: i64 = row.try_get("expires_at").map_err(get)?;
    if !live || now >= expires {
        return Err(RecoveryError::HandleRejected);
    }
    let account: Uuid = row.try_get("account_id").map_err(get)?;
    let security_generation: i64 = row.try_get("security_generation").map_err(get)?;
    let identity = sqlx::query("SELECT status, native_security_generation::text AS native FROM accounts WHERE id=$1 FOR SHARE")
        .bind(account).fetch_optional(&mut *tx).await.map_err(get)?
        .ok_or(RecoveryError::HandleRejected)?;
    if identity.try_get::<String, _>("status").map_err(get)? != "active" {
        return Err(RecoveryError::HandleRejected);
    }
    match accounts.state(tx, account).await? {
        AccountState::Active => {}
        _ => return Err(RecoveryError::SecurityDenied),
    }
    if identity.try_get::<String, _>("native").map_err(get)? != security_generation.to_string() {
        return Err(RecoveryError::SecurityDenied);
    }
    let family: Option<Uuid> = row.try_get("device_family_id").map_err(get)?;
    if let Some(family) = family {
        let current: Option<i32> = sqlx::query_scalar("SELECT 1 FROM device_sessions d JOIN accounts a ON a.id=d.account_id WHERE d.id=$1 AND d.account_id=$2 AND d.revoked_at IS NULL AND d.absolute_expires_at > $3 AND d.idle_expires_at > $3 AND d.game_generation=a.game_auth_generation AND d.native_generation=a.native_security_generation FOR SHARE OF d")
            .bind(family).bind(account).bind(now)
            .fetch_optional(&mut *tx).await.map_err(get)?;
        if current.is_none() {
            return Err(RecoveryError::HandleRejected);
        }
    }
    let play: Option<i64> = sqlx::query_scalar(
        "SELECT generation FROM native_play_generations WHERE account_id=$1 FOR SHARE",
    )
    .bind(account)
    .fetch_optional(&mut *tx)
    .await
    .map_err(get)?;
    let character: Uuid = row.try_get("character_id").map_err(get)?;
    if play != Some(row.try_get::<i64, _>("play_generation").map_err(get)?)
        || character != request.character_id
    {
        return Err(RecoveryError::HandleRejected);
    }
    Ok(Bound {
        account,
        world: row.try_get("world_id").map_err(get)?,
        channel: row.try_get("channel_id").map_err(get)?,
        security_generation,
    })
}

/// §19.4 steps 4 and 5: ownership before World, then the locator entry of the
/// handle's own account or the bound channel, which must be a §7.4 candidate.
/// The selected route record stays share-locked until commit.
async fn target(
    tx: &mut PgConnection,
    issuer: &RecoveryIssuer,
    runtime: &runtime::Settings,
    request: &RecoveryRequest,
    bound: &Bound,
    now: i64,
) -> Result<grant::Candidate, RecoveryError> {
    let listed = tickets::listing(
        tx,
        &issuer.projection_authority,
        bound.account,
        request.character_id,
    )
    .await?;
    match listed {
        None => return Err(RecoveryError::CharacterConflict),
        Some((world, _)) if world != bound.world => return Err(RecoveryError::WorldStale),
        Some(_) => {}
    }
    let get = |_: sqlx::Error| RecoveryError::Unavailable;
    // §19.6.3 restore reset: an entry of a lower epoch than the highest
    // assignment epoch is invalidated and steers nothing.
    let entry = sqlx::query("SELECT account_id, world_id, channel_id, present, conflicted FROM native_recovery_locators WHERE character_id=$1 AND assignment_epoch >= (SELECT max(assignment_epoch) FROM native_scope_assignments)")
        .bind(request.character_id).fetch_optional(&mut *tx).await.map_err(get)?;
    let mut channel = bound.channel;
    if let Some(entry) = entry
        && entry.try_get::<Uuid, _>("account_id").map_err(get)? == bound.account
    {
        if entry.try_get::<Uuid, _>("world_id").map_err(get)? != bound.world {
            return Err(RecoveryError::WorldStale);
        }
        if entry.try_get::<bool, _>("conflicted").map_err(get)? {
            return Err(RecoveryError::RouteUnavailable);
        }
        if !entry.try_get::<bool, _>("present").map_err(get)? {
            return Err(RecoveryError::CharacterConflict);
        }
        channel = entry.try_get("channel_id").map_err(get)?;
    }
    // Never a substitute channel (FND-04B §23).
    let candidate = grant::candidates(tx, runtime, bound.world, now)
        .await?
        .into_iter()
        .find(|c| c.route.channel == channel)
        .ok_or(RecoveryError::RouteUnavailable)?;
    let route = &candidate.route;
    grant::current(tx, bound.world, channel, &route.endpoint, &route.revision).await?;
    Ok(candidate)
}

/// §19.4 in one transaction. Lock order: attempt (advisory, shared with fresh
/// admissions), epoch, then `authenticate`'s order.
pub(super) async fn recover(
    pool: &PgPool,
    request: &RecoveryRequest,
    accounts: &dyn AccountStateLookup,
    issuer: &RecoveryIssuer,
    runtime: &runtime::Settings,
) -> Result<Vec<u8>, Refusal> {
    let signer = issuer.signer()?;
    let mut tx = pool.begin().await.map_err(|_| Error::Unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    store::lock(&mut tx, request.attempt_ref.as_bytes()).await?;
    // One attempt_ref namespace with fresh admissions (§19.4 step 1).
    let fresh: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_admissions WHERE attempt_ref=$1)")
            .bind(request.attempt_ref)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
    if fresh {
        return Err(RecoveryError::AttemptConflict.into());
    }
    let existing = sqlx::query("SELECT handle_hash, request_digest, response_body, expires_at_ms FROM native_recoveries WHERE attempt_ref=$1 FOR UPDATE")
        .bind(request.attempt_ref).fetch_optional(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    let epoch: Option<i16> =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
    if epoch.is_none() {
        return Err(RecoveryError::Unavailable.into());
    }
    let now_ms = store::now(&mut tx).await?;
    let now = now_ms.div_euclid(1000);
    if let Some(row) = existing {
        let get = |_: sqlx::Error| RecoveryError::ReconciliationRequired;
        let handle: Vec<u8> = row.try_get("handle_hash").map_err(get)?;
        if !bool::from(handle.ct_eq(&request.handle_hash)) {
            return Err(RecoveryError::AttemptConflict.into());
        }
        // A retry authenticates exactly like a first request (§19.4 step 1).
        authenticate(&mut tx, request, accounts, now).await?;
        let digest: Vec<u8> = row.try_get("request_digest").map_err(get)?;
        if !bool::from(digest.ct_eq(&request.digest)) {
            return Err(RecoveryError::AttemptConflict.into());
        }
        let body: Vec<u8> = row.try_get("response_body").map_err(get)?;
        let expires: i64 = row.try_get("expires_at_ms").map_err(get)?;
        if expires - now_ms < 1000 {
            return Err(RecoveryError::GrantExpired.into());
        }
        let mut success = stored(&body)?;
        let input = success.grant.token.clone();
        if grant::typed_key_id(&input, grant::RECOVERY_TYPE).is_none() {
            return Err(RecoveryError::ReconciliationRequired.into());
        }
        signer
            .prepare()
            .await
            .map_err(|_| RecoveryError::ReconciliationRequired)?;
        let at = store::now(&mut tx).await?;
        let reconcile = |_| RecoveryError::ReconciliationRequired;
        success.grant.token = signer
            .resign(
                &input,
                u64::try_from(expires / 1000).map_err(reconcile)?,
                u64::try_from(at / 1000).map_err(reconcile)?,
            )
            .map_err(|_| RecoveryError::ReconciliationRequired)?;
        if expires - at < 1000 {
            return Err(RecoveryError::GrantExpired.into());
        }
        success.grant.valid_for_seconds = (expires - at) / 1000;
        let response = encode(&success)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        return Ok(response);
    }
    let bound = authenticate(&mut tx, request, accounts, now).await?;
    // §19.9: at most 20 committed recovery attempts per account per hour.
    let committed: i64 = sqlx::query_scalar("SELECT count(*) FROM native_recoveries WHERE account_id=$1 AND created_at >= clock_timestamp() - interval '1 hour'")
        .bind(bound.account).fetch_one(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    if committed >= ACCOUNT_ATTEMPTS_PER_HOUR {
        return Err(RecoveryError::RateLimited.into());
    }
    // §19.4 step 3: one unretired recovery attempt per account.
    let pending: Option<i64> = sqlx::query_scalar(
        "SELECT max(retires_at_ms) FROM native_recoveries WHERE account_id=$1 AND retires_at_ms > $2",
    )
    .bind(bound.account)
    .bind(now_ms)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| Error::Unavailable)?;
    if let Some(retires) = pending {
        let seconds = u64::try_from((retires - now_ms + 999) / 1000).unwrap_or(1);
        return Err(Refusal {
            error: RecoveryError::AttemptPending,
            retry_after: Some(seconds.max(1)),
        });
    }
    let candidate = target(&mut tx, issuer, runtime, request, &bound, now).await?;
    // The trust self-check is refreshed before the issuer clock is read.
    signer.prepare().await?;
    let iat = u64::try_from(store::now(&mut tx).await? / 1000).map_err(|_| Error::Unavailable)?;
    let payload = payload(&Claims {
        iat,
        ttl: issuer.ttl_seconds,
        jti: grant::jti()?,
        attempt: request.attempt_ref,
        account: bound.account,
        character: request.character_id,
        world: bound.world,
        security_generation: bound.security_generation,
        report: &candidate.report,
    })?;
    let exp = iat + u64::from(issuer.ttl_seconds);
    let token = signer.sign(&payload, exp, iat)?;
    let input = token
        .rsplit_once('.')
        .map(|(input, _)| input)
        .filter(|input| grant::typed_key_id(input, grant::RECOVERY_TYPE).is_some())
        .ok_or(RecoveryError::Unavailable)?;
    let expires_ms = i64::try_from(exp)
        .ok()
        .and_then(|s| s.checked_mul(1000))
        .ok_or(RecoveryError::Unavailable)?;
    let at = store::now(&mut tx).await?;
    if expires_ms - at < 1000 {
        return Err(RecoveryError::GrantExpired.into());
    }
    let route = &candidate.route;
    let response = success(
        request.attempt_ref,
        bound.world,
        route.channel,
        &route.endpoint,
        &token,
        (expires_ms - at) / 1000,
    )?;
    let record = success(
        request.attempt_ref,
        bound.world,
        route.channel,
        &route.endpoint,
        input,
        (expires_ms - at) / 1000,
    )?;
    let retires_ms = expires_ms + (5 + i64::from(issuer.clock_uncertainty_seconds)) * 1000;
    sqlx::query("INSERT INTO native_recoveries (attempt_ref, handle_hash, request_digest, account_id, response_body, expires_at_ms, retires_at_ms) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(request.attempt_ref).bind(&request.handle_hash[..]).bind(&request.digest[..])
        .bind(bound.account).bind(&record).bind(expires_ms).bind(retires_ms)
        .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    tx.commit()
        .await
        .map_err(|_| RecoveryError::ReconciliationRequired)?;
    Ok(response)
}

/// §19.7: revokes only that handle, idempotently; unknown, dead and live
/// handles get the same answer, and no generation advances.
pub(super) async fn revoke(pool: &PgPool, hash: &[u8; 32]) -> Result<(), RecoveryError> {
    sqlx::query("UPDATE native_recovery_handles SET revoked_at = floor(extract(epoch FROM clock_timestamp()))::bigint WHERE handle_hash=$1 AND revoked_at IS NULL")
        .bind(&hash[..]).execute(pool).await.map_err(|_| RecoveryError::Unavailable)?;
    Ok(())
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
