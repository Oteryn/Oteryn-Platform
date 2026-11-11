//! Native device sessions. Production activation remains separately gated.
//!
//! `NativeAuthority` locks and revalidates the OAuth client, original generations
//! and native ticket policy on the same connection, with no external effects.
//! Deployment routes select it only behind the default-off native device flag.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::{future::Future, pin::Pin, sync::Arc};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Denied,
    Forbidden,
    Conflict,
    Unavailable,
    Invalid,
    TooLarge,
    Throttled,
    #[doc(hidden)]
    Retryable,
}
type Result<T> = std::result::Result<T, Failure>;
pub type NativeFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Trusted server-side context, never decoded from request authority fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    pub account_id: Uuid,
    pub client_id: Uuid,
    pub game_generation: i64,
    pub native_generation: i64,
}
/// Verified enrollment returned by the owning OAuth verifier under its locks.
pub struct Enrollment {
    pub authorization: Authorization,
    pub token_id: String,
}
#[derive(Debug, Clone, Copy)]
pub enum Purpose {
    Characters,
    Ticket,
}
/// Implementations must use only the supplied transaction connection. Enrollment
/// must resolve the bearer using the normal OAuth verifier, with game:ticket scope,
/// first-party public client, current account and generation checks. `verify` must
/// recheck the same current client and both immutable original generations.
pub trait NativeAuthority: Send + Sync {
    fn deployment_ready(&self) -> Result<()> {
        Ok(())
    }
    fn enroll<'a>(
        &'a self,
        db: &'a mut PgConnection,
        bearer: &'a str,
    ) -> NativeFuture<'a, Enrollment>;
    fn verify<'a>(
        &'a self,
        db: &'a mut PgConnection,
        authorization: &'a Authorization,
    ) -> NativeFuture<'a, ()>;
    /// Owner-scoped projection or native single-use ticket, with transaction-local effects.
    fn use_native<'a>(
        &'a self,
        db: &'a mut PgConnection,
        authorization: &'a Authorization,
        family: Uuid,
        purpose: Purpose,
    ) -> NativeFuture<'a, NativeResult>;
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub character_id: Uuid,
    pub world_id: Uuid,
    pub name: String,
    pub availability: String,
}
pub enum NativeResult {
    Characters(Vec<Character>),
    Ticket {
        ticket: String,
        expires_in: i64,
        expires_at: i64,
    },
}
struct UnavailableAuthority;
impl NativeAuthority for UnavailableAuthority {
    fn enroll<'a>(&'a self, _: &'a mut PgConnection, _: &'a str) -> NativeFuture<'a, Enrollment> {
        Box::pin(async { Err(Failure::Unavailable) })
    }
    fn verify<'a>(&'a self, _: &'a mut PgConnection, _: &'a Authorization) -> NativeFuture<'a, ()> {
        Box::pin(async { Err(Failure::Unavailable) })
    }
    fn use_native<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a Authorization,
        _: Uuid,
        _: Purpose,
    ) -> NativeFuture<'a, NativeResult> {
        Box::pin(async { Err(Failure::Unavailable) })
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub environment: String,
    pub absolute_seconds: i64,
    pub idle_seconds: i64,
    pub requests_per_minute: i32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            environment: String::new(),
            absolute_seconds: 30 * 86400,
            idle_seconds: 7 * 86400,
            requests_per_minute: 30,
        }
    }
}
impl Settings {
    fn check(&self) -> Result<()> {
        if self.enabled
            && matches!(self.environment.as_str(), "testing" | "preproduction")
            && (1..=30 * 86400).contains(&self.absolute_seconds)
            && (1..=7 * 86400).contains(&self.idle_seconds)
            && (1..=60).contains(&self.requests_per_minute)
        {
            Ok(())
        } else {
            Err(Failure::Unavailable)
        }
    }
}
/// Exporting a credential is an explicit response operation; Debug is redacted.
pub struct Credential {
    secret: String,
    pub family_id: Uuid,
    pub absolute_expires_at: i64,
    pub idle_expires_at: i64,
    pub rotation_nonce: String,
}
impl Credential {
    pub fn secret(&self) -> &str {
        &self.secret
    }
    fn payload(&self) -> Result<serde_json::Value> {
        serde_json::to_value(wire::CredentialResponse {
            protocol_version: 1,
            device_credential: self.secret.clone(),
            family_id: self.family_id,
            absolute_expires_at: self.absolute_expires_at,
            idle_expires_at: self.idle_expires_at,
            rotation_nonce: self.rotation_nonce.clone(),
        })
        .map_err(|_| Failure::Unavailable)
    }
}
fn secret() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Failure::Unavailable)?;
    Ok(format!("otd1.{}", URL_SAFE_NO_PAD.encode(bytes)))
}
fn hash(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}
fn valid_secret(secret: &str) -> bool {
    secret.len() == 48
        && secret.starts_with("otd1.")
        && secret.as_bytes()[5..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
fn unavailable(error: sqlx::Error) -> Failure {
    if error
        .as_database_error()
        .and_then(|e| e.code())
        .is_some_and(|code| matches!(code.as_ref(), "40001" | "40P01"))
    {
        Failure::Retryable
    } else {
        Failure::Unavailable
    }
}
async fn clock(db: &mut PgConnection) -> Result<i64> {
    sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(db)
        .await
        .map_err(unavailable)
}
async fn lock_account(db: &mut PgConnection, auth: &Authorization) -> Result<()> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT status, native_security_generation FROM accounts WHERE id=$1 FOR UPDATE",
    )
    .bind(auth.account_id)
    .fetch_optional(db)
    .await
    .map_err(unavailable)?;
    if row.is_some_and(|(status, generation)| {
        status == "active" && generation == auth.native_generation
    }) && auth.game_generation >= 0
    {
        Ok(())
    } else {
        Err(Failure::Denied)
    }
}
#[derive(Clone)]
pub struct Sessions {
    pool: PgPool,
    settings: Settings,
    authority: Arc<dyn NativeAuthority>,
}
impl Sessions {
    pub fn new(pool: PgPool, settings: Settings, authority: Arc<dyn NativeAuthority>) -> Self {
        Self {
            pool,
            settings,
            authority,
        }
    }
    pub async fn enroll(&self, bearer: &str, consent: bool) -> Result<Credential> {
        self.enroll_class(bearer, consent, None).await
    }
    pub async fn enroll_class(
        &self,
        bearer: &str,
        consent: bool,
        key: Option<DeviceKey>,
    ) -> Result<Credential> {
        for _ in 0..3 {
            match self.enroll_once(bearer, consent, key.as_ref()).await {
                Err(Failure::Retryable) => continue,
                result => return result,
            }
        }
        Err(Failure::Unavailable)
    }
    async fn enroll_once(
        &self,
        bearer: &str,
        consent: bool,
        key: Option<&DeviceKey>,
    ) -> Result<Credential> {
        self.settings.check()?;
        self.authority.deployment_ready()?;
        if key.is_none() {
            return Err(Failure::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let enrollment = self.authority.enroll(&mut tx, bearer).await?;
        let auth = enrollment.authorization;
        lock_account(&mut tx, &auth).await?;
        if enrollment.token_id.is_empty() || enrollment.token_id.len() > 8192 {
            return Err(Failure::Denied);
        }
        if let Some(key) = key {
            key.verify(bearer, auth.client_id, consent)
                .map_err(|error| {
                    if error == Failure::Denied {
                        Failure::Forbidden
                    } else {
                        error
                    }
                })?;
        }
        let at = clock(&mut tx).await?;
        let absolute = at
            + if consent {
                self.settings.absolute_seconds
            } else {
                43200
            };
        let credential = Credential {
            secret: secret()?,
            rotation_nonce: key::new_nonce()?,
            family_id: Uuid::now_v7(),
            absolute_expires_at: absolute,
            idle_expires_at: if consent {
                absolute.min(at + self.settings.idle_seconds)
            } else {
                absolute
            },
        };
        let inserted = sqlx::query("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(enrollment_token_id) DO NOTHING")
            .bind(credential.family_id).bind(auth.account_id).bind(auth.client_id).bind(enrollment.token_id).bind(auth.game_generation).bind(auth.native_generation).bind(absolute).bind(credential.idle_expires_at).execute(&mut *tx).await.map_err(unavailable)?;
        if inserted.rows_affected() != 1 {
            return Err(Failure::Conflict);
        }
        sqlx::query("UPDATE device_sessions SET persistence_class=$2,device_public_key=$3,device_key_thumbprint=$4,rotation_nonce=$5 WHERE id=$1")
            .bind(credential.family_id).bind(if consent { "remembered" } else { "session" })
            .bind(key.map(|k| k.public_key.clone())).bind(key.map(|k| URL_SAFE_NO_PAD.encode(hash_bytes(&k.public_key))))
            .bind(&credential.rotation_nonce)
            .execute(&mut *tx).await.map_err(|error| {
                if error.as_database_error().is_some_and(|error| {
                    error.code().as_deref() == Some("23505")
                        && error.constraint() == Some("device_sessions_device_key_thumbprint_key")
                }) {
                    Failure::Conflict
                } else {
                    unavailable(error)
                }
            })?;
        insert_credential(&mut tx, &credential, 0).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(credential)
    }
    pub async fn rotate(
        &self,
        secret: &str,
        client: Uuid,
        purpose: Purpose,
    ) -> Result<serde_json::Value> {
        self.operate(secret, client, Some(purpose), None).await
    }
    pub async fn rotate_with_proof(
        &self,
        secret: &str,
        client: Uuid,
        purpose: Purpose,
        proof: &RotationProof,
    ) -> Result<serde_json::Value> {
        self.operate(secret, client, Some(purpose), Some(proof))
            .await
    }
    pub async fn revoke(&self, secret: &str, client: Uuid) -> Result<()> {
        self.operate(secret, client, None, None).await.map(|_| ())
    }
    async fn operate(
        &self,
        presented: &str,
        client: Uuid,
        purpose: Option<Purpose>,
        proof: Option<&RotationProof>,
    ) -> Result<serde_json::Value> {
        // Only server-confirmed transaction aborts are retried. An ambiguous
        // commit or network error never retries a potentially consumed secret.
        for _ in 0..3 {
            match self.operate_once(presented, client, purpose, proof).await {
                Err(Failure::Retryable) => continue,
                result => return result,
            }
        }
        Err(Failure::Unavailable)
    }
    async fn operate_once(
        &self,
        presented: &str,
        client: Uuid,
        purpose: Option<Purpose>,
        proof: Option<&RotationProof>,
    ) -> Result<serde_json::Value> {
        self.settings.check()?;
        self.authority.deployment_ready()?;
        if !valid_secret(presented) {
            return Err(Failure::Denied);
        }
        let digest = hash(presented);
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        // Locator is not authority. Account first, then owning client verification,
        // then family and credential locks; all security facts are rechecked below.
        let locator: Option<(Uuid, Uuid, Uuid, i64, i64)> = sqlx::query_as("SELECT f.id,f.account_id,f.client_id,f.game_generation,f.native_generation FROM device_sessions f JOIN device_session_credentials c ON c.family_id=f.id WHERE c.secret_hash=$1")
            .bind(&digest).fetch_optional(&mut *tx).await.map_err(unavailable)?;
        let Some((family, account_id, bound_client, game_generation, native_generation)) = locator
        else {
            return Err(Failure::Denied);
        };
        if bound_client != client {
            return Err(Failure::Denied);
        }
        let auth = Authorization {
            account_id,
            client_id: client,
            game_generation,
            native_generation,
        };
        let account_check = lock_account(&mut tx, &auth).await;
        let authority_check = match account_check {
            Err(error @ (Failure::Unavailable | Failure::Retryable)) => return Err(error),
            _ if purpose.is_none() => Ok(()),
            Ok(()) => self.authority.verify(&mut tx, &auth).await,
            error => error,
        };
        if let Err(error @ (Failure::Unavailable | Failure::Retryable)) = authority_check {
            return Err(error);
        }
        let row = sqlx::query("SELECT f.device_public_key,f.rotation_nonce,c.secret_hash,f.persistence_class,f.sequence,f.absolute_expires_at,f.idle_expires_at,f.revoked_at,c.consumed_at,c.sequence AS credential_sequence FROM device_sessions f JOIN device_session_credentials c ON c.family_id=f.id WHERE f.id=$1 AND c.secret_hash=$2 FOR UPDATE OF f,c")
            .bind(family).bind(&digest).fetch_one(&mut *tx).await.map_err(unavailable)?;
        let stored: Vec<u8> = row.try_get("secret_hash").map_err(unavailable)?;
        if !bool::from(stored.ct_eq(&digest)) {
            return Err(Failure::Denied);
        }
        let at = clock(&mut tx).await?;
        if purpose.is_some() {
            let proof = proof.ok_or(Failure::Denied)?;
            let public_key: Option<Vec<u8>> =
                row.try_get("device_public_key").map_err(unavailable)?;
            let nonce: Option<String> = row.try_get("rotation_nonce").map_err(unavailable)?;
            let enrolled = public_key.ok_or(Failure::Denied)?;
            // Freshness and nonce checks precede key comparison: old proofs cannot
            // revoke a successor. The account/family locks serialize consumption.
            proof.check_fresh(family, nonce.as_deref(), at)?;
            let supplied = key::decode_bytes(&proof.device_public_key, 32)?;
            if !bool::from(enrolled.ct_eq(&supplied)) {
                sqlx::query("UPDATE device_sessions SET revoked_at=coalesce(revoked_at,$2),updated_at=now() WHERE id=$1")
                    .bind(family).bind(at).execute(&mut *tx).await.map_err(unavailable)?;
                tx.commit().await.map_err(unavailable)?;
                return Err(Failure::Denied);
            }
            proof.verify(&enrolled)?;
        }
        let sequence: i64 = row.try_get("sequence").map_err(unavailable)?;
        let absolute: i64 = row.try_get("absolute_expires_at").map_err(unavailable)?;
        let idle: i64 = row.try_get("idle_expires_at").map_err(unavailable)?;
        let revoked: Option<i64> = row.try_get("revoked_at").map_err(unavailable)?;
        let consumed: Option<i64> = row.try_get("consumed_at").map_err(unavailable)?;
        let credential_sequence: i64 = row.try_get("credential_sequence").map_err(unavailable)?;
        if purpose.is_none()
            || authority_check.is_err()
            || revoked.is_some()
            || consumed.is_some()
            || credential_sequence != sequence
            || sequence == i64::MAX
            || at >= absolute
            || at >= idle
        {
            sqlx::query("UPDATE device_sessions SET revoked_at=coalesce(revoked_at,$2),updated_at=now() WHERE id=$1").bind(family).bind(at).execute(&mut *tx).await.map_err(unavailable)?;
            tx.commit().await.map_err(unavailable)?;
            return if purpose.is_none() {
                serde_json::to_value(wire::RevokeResponse {
                    protocol_version: 1,
                    revoked: true,
                })
                .map_err(|_| Failure::Unavailable)
            } else {
                Err(Failure::Denied)
            };
        }
        let purpose = purpose.ok_or(Failure::Denied)?;
        let next_sequence = sequence.checked_add(1).ok_or(Failure::Unavailable)?;
        let credential = Credential {
            secret: secret()?,
            rotation_nonce: key::new_nonce()?,
            family_id: family,
            absolute_expires_at: absolute,
            idle_expires_at: if row
                .try_get::<String, _>("persistence_class")
                .map_err(unavailable)?
                == "session"
            {
                absolute
            } else {
                absolute.min(at + self.settings.idle_seconds)
            },
        };
        sqlx::query("UPDATE device_session_credentials SET consumed_at=$2,updated_at=now() WHERE secret_hash=$1").bind(digest).bind(at).execute(&mut *tx).await.map_err(unavailable)?;
        insert_credential(&mut tx, &credential, next_sequence).await?;
        sqlx::query("UPDATE device_sessions SET sequence=$2,idle_expires_at=$3,rotation_nonce=$4,updated_at=now() WHERE id=$1").bind(family).bind(next_sequence).bind(credential.idle_expires_at).bind(&credential.rotation_nonce).execute(&mut *tx).await.map_err(unavailable)?;
        let native = self
            .authority
            .use_native(&mut tx, &auth, family, purpose)
            .await?;
        let mut payload = credential.payload()?;
        match (purpose, native) {
            (Purpose::Characters, NativeResult::Characters(characters))
                if characters.len() <= 64
                    && characters.iter().all(|c| {
                        !c.name.is_empty()
                            && c.name.len() <= 96
                            && matches!(c.availability.as_str(), "AVAILABLE" | "UNAVAILABLE")
                    }) =>
            {
                payload["characters"] =
                    serde_json::to_value(characters).map_err(|_| Failure::Unavailable)?;
            }
            (
                Purpose::Ticket,
                NativeResult::Ticket {
                    ticket,
                    expires_in,
                    expires_at,
                },
            ) if !ticket.is_empty()
                && ticket.len() <= 256
                && (1..=60).contains(&expires_in)
                && expires_at > at
                && expires_at <= at + 60 =>
            {
                payload["ticket"] = ticket.into();
                payload["expires_in"] = expires_in.into();
                payload["expires_at"] = expires_at.into();
            }
            _ => return Err(Failure::Unavailable),
        }
        if serde_json::to_vec(&payload)
            .map_err(|_| Failure::Unavailable)?
            .len()
            > 16384
        {
            return Err(Failure::Unavailable);
        }
        let payload = wire::operation_response(payload, purpose)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(payload)
    }
    /// Browser callers must supply an authenticated owner after CSRF and recent
    /// authentication checks. No HTTP owner-enumeration surface is added here.
    pub async fn revoke_for_owner(&self, owner: Uuid, family: Uuid) -> Result<()> {
        self.settings.check()?;
        self.authority.deployment_ready()?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(owner)
                .fetch_optional(&mut *tx)
                .await
                .map_err(unavailable)?;
        if status.as_deref() != Some("active") {
            return Err(Failure::Denied);
        }
        let at = clock(&mut tx).await?;
        let result = sqlx::query("UPDATE device_sessions SET revoked_at=coalesce(revoked_at,$3),updated_at=now() WHERE id=$1 AND account_id=$2").bind(family).bind(owner).bind(at).execute(&mut *tx).await.map_err(unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Failure::Denied);
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(())
    }
    /// Credential-change integration: caller owns an account-locked transaction.
    pub async fn revoke_for_account(db: &mut PgConnection, account: Uuid) -> Result<()> {
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut *db)
            .await
            .map_err(unavailable)?
            .ok_or(Failure::Denied)?;
        sqlx::query("UPDATE device_sessions SET revoked_at=coalesce(revoked_at,floor(extract(epoch FROM clock_timestamp()))::bigint),updated_at=now() WHERE account_id=$1").bind(account).execute(db).await.map_err(unavailable)?;
        Ok(())
    }
}
async fn insert_credential(
    db: &mut PgConnection,
    credential: &Credential,
    sequence: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO device_session_credentials(secret_hash,family_id,sequence) VALUES($1,$2,$3)",
    )
    .bind(hash(credential.secret()))
    .bind(credential.family_id)
    .bind(sequence)
    .execute(db)
    .await
    .map_err(unavailable)?;
    Ok(())
}

#[path = "devices_http.rs"]
mod http;
pub use http::{router, routes};
#[path = "devices_tls.rs"]
pub mod tls;
pub use tls::VerifiedHttps;
#[path = "devices_wire.rs"]
pub mod wire;
pub use key::RotationProof;
#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;

impl std::fmt::Debug for Enrollment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Enrollment([redacted])")
    }
}
impl std::fmt::Debug for NativeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeResult([redacted])")
    }
}
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credential([redacted])")
    }
}
impl std::fmt::Debug for Sessions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Sessions([redacted])")
    }
}

#[path = "devices_authority.rs"]
pub(crate) mod native;
pub use native::PlatformAuthority;
#[path = "devices_key.rs"]
mod key;
pub use key::DeviceKey;
fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
