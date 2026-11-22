//! WebAuthn verification and durable, single-use browser ceremonies.
//! Serialized library state never leaves PostgreSQL. Browser bindings are session CSRF secrets.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction, types::time::OffsetDateTime};
use uuid::Uuid;
use webauthn_rs::prelude::*;

#[derive(Clone, Debug)]
pub struct Passkeys {
    pub pool: PgPool,
    webauthn: Webauthn,
}
#[derive(Debug)]
pub enum Error {
    Refused,
    Storage(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Storage(value)
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::Refused)
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|_| Error::Refused)
}
fn hash(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}
fn token() -> Result<String, Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Refused)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
#[derive(Serialize)]
pub struct Challenge {
    pub token: String,
    pub options: Value,
}
impl std::fmt::Debug for Challenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Challenge([redacted])")
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub token: String,
    pub credential: Value,
}
impl std::fmt::Debug for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Completion([redacted])")
    }
}
impl Passkeys {
    /// RP identity comes only from deployment configuration, never request headers.
    pub fn new(pool: PgPool, rp_id: &str, origin: &str) -> Result<Self, Error> {
        let origin = Url::parse(origin).map_err(|_| Error::Refused)?;
        if origin.scheme() != "https"
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
        {
            return Err(Error::Refused);
        }
        let webauthn = WebauthnBuilder::new(rp_id, &origin)
            .map_err(|_| Error::Refused)?
            .rp_name("Oteryn")
            .build()
            .map_err(|_| Error::Refused)?;
        Ok(Self { pool, webauthn })
    }
    async fn save<T: Serialize>(
        &self,
        binding: &str,
        account: Option<Uuid>,
        purpose: &str,
        state: &T,
        options: Value,
    ) -> Result<Challenge, Error> {
        if binding.len() != 64 {
            return Err(Error::Refused);
        }
        let token = token()?;
        sqlx::query("INSERT INTO passkey_ceremonies (token_hash,session_hash,account_id,purpose,state) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (session_hash,purpose) DO UPDATE SET token_hash=EXCLUDED.token_hash,account_id=EXCLUDED.account_id,state=EXCLUDED.state,expires_at=now()+interval '5 minutes',updated_at=now()")
            .bind(hash(&token)).bind(hash(binding)).bind(account).bind(purpose).bind(encode(state)?).execute(&self.pool).await?;
        self.audit(account, "passkey.challenge").await?;
        Ok(Challenge { token, options })
    }
    /// A DELETE statement commits before parsing or verification. Even invalid proofs burn state.
    /// Returns the ceremony expiry too: callers recheck it after their account lock wait.
    async fn consume<T: serde::de::DeserializeOwned>(
        &self,
        binding: &str,
        token: &str,
        purpose: &str,
        account: Option<Uuid>,
    ) -> Result<(T, OffsetDateTime), Error> {
        if token.len() != 64 || binding.len() != 64 {
            return Err(Error::Refused);
        }
        let row: Option<(Value, Option<Uuid>, bool, OffsetDateTime)> = sqlx::query_as("DELETE FROM passkey_ceremonies WHERE token_hash=$1 AND session_hash=$2 AND purpose=$3 RETURNING state,account_id,expires_at>clock_timestamp(),expires_at")
            .bind(hash(token)).bind(hash(binding)).bind(purpose).fetch_optional(&self.pool).await?;
        let Some((state, owner, true, expires_at)) = row else {
            return Err(Error::Refused);
        };
        if owner != account {
            return Err(Error::Refused);
        }
        Ok((decode(state)?, expires_at))
    }
    /// Browser credential mutations require a fresh password proof for the session owner.
    pub async fn confirm_password(
        &self,
        account: Uuid,
        password: String,
    ) -> Result<crate::login::Authorization, Error> {
        let email: Option<String> = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(&self.pool)
            .await?;
        let verified = crate::login::authenticate_authorized(
            &self.pool,
            crate::login::Credentials {
                email: email.ok_or(Error::Refused)?,
                password,
            },
        )
        .await?;
        verified
            .filter(|proof| proof.account == account)
            .ok_or(Error::Refused)
    }
    pub async fn begin_registration_authorized(
        &self,
        binding: &str,
        proof: crate::login::Authorization,
    ) -> Result<Challenge, Error> {
        let account = proof.account;
        let mut tx = self.pool.begin().await?;
        eligible(&mut tx, account).await?;
        mutation_authorized(&mut tx, binding, account, proof.generation).await?;
        let rows: Vec<(Value,)> =
            sqlx::query_as("SELECT credential FROM account_passkeys WHERE account_id=$1")
                .bind(account)
                .fetch_all(&mut *tx)
                .await?;
        if rows.len() >= 10 {
            return Err(Error::Refused);
        }
        let keys: Vec<Passkey> = rows
            .into_iter()
            .map(|(v,)| decode(v))
            .collect::<Result<_, _>>()?;
        let name = account.to_string();
        let (options, state) = self
            .webauthn
            .start_passkey_registration(
                account,
                &name,
                &name,
                Some(keys.iter().map(|k| k.cred_id().clone()).collect()),
            )
            .map_err(|_| Error::Refused)?;
        tx.commit().await?;
        let mut options = encode(&options)?;
        // Request discoverable credentials for account-free login. UV remains required by the library.
        options["publicKey"]["authenticatorSelection"]["residentKey"] = "required".into();
        options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = true.into();
        self.save(
            binding,
            Some(account),
            "register",
            &(state, proof.generation),
            options,
        )
        .await
    }
    pub async fn finish_registration(
        &self,
        binding: &str,
        account: Uuid,
        input: Completion,
    ) -> Result<(), Error> {
        let ((state, generation), expires_at): ((PasskeyRegistration, i64), _) = self
            .consume(binding, &input.token, "register", Some(account))
            .await?;
        let response: RegisterPublicKeyCredential = decode(input.credential)?;
        let key = self
            .webauthn
            .finish_passkey_registration(&response, &state)
            .map_err(|_| Error::Refused)?;
        let mut tx = self.pool.begin().await?;
        eligible(&mut tx, account).await?;
        live(&mut tx, expires_at).await?;
        mutation_authorized(&mut tx, binding, account, generation).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM account_passkeys WHERE account_id=$1")
                .bind(account)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 10 {
            return Err(Error::Refused);
        }
        let inserted = sqlx::query("INSERT INTO account_passkeys(account_id,credential_id,credential) VALUES ($1,$2,$3) ON CONFLICT (credential_id) DO NOTHING")
            .bind(account).bind(key.cred_id().as_ref() as &[u8]).bind(encode(&key)?).execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(Error::Refused);
        }
        revoke(&mut tx, account).await?;
        audit_tx(&mut tx, Some(account), "passkey.registered").await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn begin_authentication(&self, binding: &str) -> Result<Challenge, Error> {
        let (options, state) = self
            .webauthn
            .start_discoverable_authentication()
            .map_err(|_| Error::Refused)?;
        self.save(binding, None, "authenticate", &state, encode(&options)?)
            .await
    }
    pub async fn finish_authentication(
        &self,
        binding: &str,
        input: Completion,
    ) -> Result<Uuid, Error> {
        Ok(self.finish_authorized(binding, input).await?.account)
    }
    pub async fn finish_authorized(
        &self,
        binding: &str,
        input: Completion,
    ) -> Result<Authentication, Error> {
        let (state, expires_at): (DiscoverableAuthentication, _) = self
            .consume(binding, &input.token, "authenticate", None)
            .await?;
        let response: PublicKeyCredential = decode(input.credential)?;
        let (account, credential_id) = self
            .webauthn
            .identify_discoverable_authentication(&response)
            .map_err(|_| Error::Refused)?;
        let mut tx = self.pool.begin().await?;
        eligible(&mut tx, account).await?;
        live(&mut tx, expires_at).await?;
        let value: Option<Value> = sqlx::query_scalar("SELECT credential FROM account_passkeys WHERE account_id=$1 AND credential_id=$2 FOR UPDATE")
            .bind(account).bind(credential_id).fetch_optional(&mut *tx).await?;
        let mut key: Passkey = decode(value.ok_or(Error::Refused)?)?;
        let result = self
            .webauthn
            .finish_discoverable_authentication_v2(&response, state, &key)
            .map_err(|_| Error::Refused)?;
        if !result.user_verified() || key.update_credential(&result).is_none() {
            return Err(Error::Refused);
        }
        sqlx::query("UPDATE account_passkeys SET credential=$3,updated_at=now() WHERE account_id=$1 AND credential_id=$2")
            .bind(account).bind(credential_id).bind(encode(&key)?).execute(&mut *tx).await?;
        let generation =
            sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&mut *tx)
                .await?;
        audit_tx(&mut tx, Some(account), "passkey.authenticated").await?;
        tx.commit().await?;
        Ok(Authentication {
            account,
            generation,
            credential_id: credential_id.to_vec(),
        })
    }
    pub async fn authentication_current(&self, proof: &Authentication) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a JOIN account_passkeys p ON p.account_id=a.id WHERE a.id=$1 AND a.native_security_generation=$2 AND p.credential_id=$3 AND a.status='active' AND a.email_verified_at IS NOT NULL AND NOT a.login_mfa_required)")
            .bind(proof.account).bind(proof.generation).bind(&proof.credential_id).fetch_one(&self.pool).await?)
    }
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
    pub async fn list(&self, account: Uuid) -> Result<Vec<Uuid>, Error> {
        let mut tx = self.pool.begin().await?;
        eligible(&mut tx, account).await?;
        let ids = sqlx::query_scalar(
            "SELECT id FROM account_passkeys WHERE account_id=$1 ORDER BY created_at,id",
        )
        .bind(account)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ids)
    }
    pub async fn remove_authorized(
        &self,
        binding: &str,
        proof: crate::login::Authorization,
        id: Uuid,
    ) -> Result<(), Error> {
        let account = proof.account;
        let mut tx = self.pool.begin().await?;
        eligible(&mut tx, account).await?;
        mutation_authorized(&mut tx, binding, account, proof.generation).await?;
        let deleted = sqlx::query("DELETE FROM account_passkeys WHERE account_id=$1 AND id=$2")
            .bind(account)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() != 1 {
            return Err(Error::Refused);
        }
        revoke(&mut tx, account).await?;
        audit_tx(&mut tx, Some(account), "passkey.removed").await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn audit(&self, account: Option<Uuid>, action: &str) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        audit_tx(&mut tx, account, action).await?;
        tx.commit().await?;
        Ok(())
    }
}
/// Account policy for passkey management, shared with navigation.
pub fn account_eligible(status: &str, email_verified: bool, login_mfa_required: bool) -> bool {
    status == "active" && email_verified && !login_mfa_required
}

async fn eligible(tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<(), Error> {
    let state: Option<(String, bool, bool)> = sqlx::query_as(
        "SELECT status, email_verified_at IS NOT NULL, login_mfa_required
         FROM accounts WHERE id=$1 FOR UPDATE",
    )
    .bind(account)
    .fetch_optional(&mut **tx)
    .await?;
    if !state.is_some_and(|(status, verified, mfa_required)| {
        account_eligible(&status, verified, mfa_required)
    }) {
        return Err(Error::Refused);
    }
    Ok(())
}
/// A ceremony consumed before the account lock wait must still be live after it.
async fn live(tx: &mut Transaction<'_, Postgres>, expires_at: OffsetDateTime) -> Result<(), Error> {
    let live: bool = sqlx::query_scalar("SELECT $1::timestamptz>clock_timestamp()")
        .bind(expires_at)
        .fetch_one(&mut **tx)
        .await?;
    if !live {
        return Err(Error::Refused);
    }
    Ok(())
}
// The account lock held by eligible serializes this check with every revocation.
async fn mutation_authorized(
    tx: &mut Transaction<'_, Postgres>,
    binding: &str,
    account: Uuid,
    generation: i64,
) -> Result<(), Error> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a JOIN web_sessions s ON s.account_id=a.id WHERE a.id=$1 AND a.native_security_generation=$2 AND s.csrf_token=$3 AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp())")
        .bind(account).bind(generation).bind(binding).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Refused);
    }
    Ok(())
}
async fn revoke(tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1,updated_at=clock_timestamp() WHERE id=$1").bind(account).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM passkey_ceremonies WHERE account_id=$1")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    // End native recovery handles in the same transaction (gateway login contract §19.3).
    sqlx::query("UPDATE native_recovery_handles SET revoked_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE account_id=$1 AND revoked_at IS NULL")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn audit_tx(
    tx: &mut Transaction<'_, Postgres>,
    account: Option<Uuid>,
    action: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES ($1,$2,'passkey','{}','{}',$3)")
        .bind(account).bind(action).bind(Uuid::now_v7()).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
#[path = "passkeys_tests.rs"]
mod tests;

/// Bounded deletion of expired ceremony state, including abandoned browser flows.
#[derive(Clone, Debug)]
pub struct PasskeyCleanup {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for PasskeyCleanup {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            sqlx::query("DELETE FROM passkey_ceremonies WHERE token_hash IN (SELECT token_hash FROM passkey_ceremonies WHERE expires_at<=now() LIMIT 1000 FOR UPDATE SKIP LOCKED)")
                .execute(&self.pool).await.map_err(|e| e.to_string())?;
            Ok(())
        })
    }
}

pub struct Authentication {
    pub account: Uuid,
    generation: i64,
    credential_id: Vec<u8>,
}

impl std::fmt::Debug for Authentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Authentication([redacted])")
    }
}

impl Authentication {
    pub fn generation(&self) -> i64 {
        self.generation
    }
}
