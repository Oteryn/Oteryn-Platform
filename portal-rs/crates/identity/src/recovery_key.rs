//! One-time account recovery. All mutations serialize on the account row.
use crate::{
    accounts::{Account, AccountId, normalize_email},
    password,
    verify::token_hash,
};
use sqlx::{PgConnection, PgPool};
use std::{future::Future, pin::Pin};
use subtle::ConstantTimeEq;

/// Off unless explicitly enabled by the composition root.
#[derive(Clone, Copy, Debug, Default)]
pub struct Enabled(pub bool);
/// Composition-root feature registry; absent or unknown entries never enable recovery.
pub type FeatureFlags = std::collections::BTreeSet<String>;
pub const FEATURE: &str = "account_recovery_keys";

#[derive(Debug)]
pub enum Error {
    Disabled,
    InvalidInput,
    Rejected,
    Unavailable,
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "recovery key: {self:?}")
    }
}
impl std::error::Error for Error {}

pub type ResetFuture<'a> = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
/// Runs under the account lock in the same transaction. Implementations must revoke
/// password resets, MFA secrets/codes, trusted devices, OAuth families and device sessions.
pub type SecurityReset = for<'a> fn(&'a mut PgConnection, AccountId) -> ResetFuture<'a>;

/// The dependency modules are absent from the base. Refuse if their tables appear
/// until the composition root supplies their transactional reset implementation.
pub fn deny_unintegrated_security(conn: &mut PgConnection, _id: AccountId) -> ResetFuture<'_> {
    Box::pin(async move {
        for table in [
            "account_password_resets",
            "account_mfa_totp",
            "account_recovery_codes",
            "account_trusted_devices",
            "trusted_devices",
            "oauth_tokens",
            "device_sessions",
        ] {
            let present: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
                .bind(table)
                .fetch_one(&mut *conn)
                .await?;
            if present {
                return Err(Error::Unavailable);
            }
        }
        Ok(())
    })
}
/// Unconditional base revocation; extensions cannot omit these effects.
async fn reset_base(conn: &mut PgConnection, id: AccountId) -> Result<(), Error> {
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1, updated_at=now() WHERE id=$1").bind(id).execute(conn).await?;
    Ok(())
}
#[derive(Clone, Debug)]
pub struct RecoveryKeys {
    pub pool: PgPool,
    pub enabled: Enabled,
    pub reset: SecurityReset,
}
impl RecoveryKeys {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            enabled: Enabled::default(),
            reset: deny_unintegrated_security,
        }
    }
    fn gate(&self) -> Result<(), Error> {
        if self.enabled.0 {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    async fn locked(conn: &mut PgConnection, id: AccountId) -> Result<Account, Error> {
        let account: Option<Account> =
            sqlx::query_as("SELECT * FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(conn)
                .await?;
        account
            .filter(|a| matches!(a.status.as_str(), "active" | "pending_verification"))
            .ok_or(Error::Rejected)
    }
    async fn authorize(conn: &mut PgConnection, id: AccountId, current: &str) -> Result<(), Error> {
        if current.is_empty() || current.len() > 1024 {
            return Err(Error::InvalidInput);
        }
        let account = Self::locked(conn, id).await?;
        let current = current.to_owned();
        let verified =
            tokio::task::spawn_blocking(move || password::verify(&current, &account.password_hash))
                .await
                .map_err(|_| Error::Unavailable)?;
        if verified {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }
    pub async fn generate(&self, id: AccountId, current: &str) -> Result<String, Error> {
        self.gate()?;
        let mut tx = self.pool.begin().await?;
        Self::authorize(&mut tx, id, current).await?;
        let mut bytes = [0u8; 20];
        getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
        let mut raw = String::from("OTERYN");
        for group in bytes.chunks(4) {
            raw.push('-');
            for byte in group {
                raw.push_str(&format!("{byte:02X}"));
            }
        }
        sqlx::query("INSERT INTO account_recovery_keys (account_id,key_hash) VALUES ($1,$2) ON CONFLICT (account_id) DO UPDATE SET key_hash=EXCLUDED.key_hash,confirmed_at=NULL,used_at=NULL,revoked_at=NULL,created_at=now(),updated_at=now()")
            .bind(id).bind(token_hash(&normalize(&raw))).execute(&mut *tx).await?;
        event(&mut tx, id, "generated").await?;
        tx.commit().await?;
        Ok(raw)
    }
    pub async fn confirm(&self, id: AccountId, current: &str, raw: &str) -> Result<(), Error> {
        self.gate()?;
        validate_key(raw)?;
        let mut tx = self.pool.begin().await?;
        Self::authorize(&mut tx, id, current).await?;
        let digest: Option<Vec<u8>>=sqlx::query_scalar("SELECT key_hash FROM account_recovery_keys WHERE account_id=$1 AND confirmed_at IS NULL AND used_at IS NULL AND revoked_at IS NULL").bind(id).fetch_optional(&mut *tx).await?;
        if !matches_digest(digest, raw) {
            return Err(Error::Rejected);
        }
        sqlx::query("UPDATE account_recovery_keys SET confirmed_at=now(),updated_at=now() WHERE account_id=$1").bind(id).execute(&mut *tx).await?;
        event(&mut tx, id, "confirmed").await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn revoke(&self, id: AccountId, current: &str) -> Result<(), Error> {
        self.gate()?;
        let mut tx = self.pool.begin().await?;
        Self::authorize(&mut tx, id, current).await?;
        let result=sqlx::query("UPDATE account_recovery_keys SET revoked_at=now(),updated_at=now() WHERE account_id=$1 AND used_at IS NULL AND revoked_at IS NULL").bind(id).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        event(&mut tx, id, "revoked").await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn recover(
        &self,
        email: &str,
        raw: &str,
        new_password: &str,
        confirmation: &str,
    ) -> Result<AccountId, Error> {
        self.gate()?;
        let email = normalize_email(email);
        if email.len() > 254
            || !email.is_ascii()
            || !email_address::EmailAddress::is_valid(&email)
            || !password::valid(new_password)
            || new_password != confirmation
        {
            return Err(Error::InvalidInput);
        }
        validate_key(raw)?;
        // Hash on every well-formed request, including unknown accounts.
        let new_password = new_password.to_owned();
        let hash = tokio::task::spawn_blocking(move || password::hash(&new_password))
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(|_| Error::Unavailable)?;
        let mut tx = self.pool.begin().await?;
        let id: Option<AccountId> =
            sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1 FOR UPDATE")
                .bind(email)
                .fetch_optional(&mut *tx)
                .await?;
        let id = id.ok_or(Error::Rejected)?;
        Self::locked(&mut tx, id).await?;
        let digest:Option<Vec<u8>>=sqlx::query_scalar("SELECT key_hash FROM account_recovery_keys WHERE account_id=$1 AND confirmed_at IS NOT NULL AND used_at IS NULL AND revoked_at IS NULL").bind(id).fetch_optional(&mut *tx).await?;
        if !matches_digest(digest, raw) {
            return Err(Error::Rejected);
        }
        reset_base(&mut tx, id).await?;
        (self.reset)(&mut tx, id).await?;
        sqlx::query("UPDATE accounts SET password_hash=$2,updated_at=now() WHERE id=$1")
            .bind(id)
            .bind(hash)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE account_recovery_keys SET used_at=now(),updated_at=now() WHERE account_id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        event(&mut tx, id, "used").await?;
        tx.commit().await?;
        Ok(id)
    }
}
async fn event(conn: &mut PgConnection, id: AccountId, kind: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO account_recovery_key_events (account_id,kind) VALUES ($1,$2)")
        .bind(id)
        .bind(kind)
        .execute(conn)
        .await?;
    Ok(())
}
pub fn normalize(raw: &str) -> String {
    raw.bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|b| char::from(b.to_ascii_uppercase()))
        .collect()
}
fn validate_key(raw: &str) -> Result<(), Error> {
    if raw.is_empty() || raw.len() > 160 {
        Err(Error::InvalidInput)
    } else if normalize(raw).is_empty() {
        Err(Error::Rejected)
    } else {
        Ok(())
    }
}
fn matches_digest(digest: Option<Vec<u8>>, raw: &str) -> bool {
    let candidate = token_hash(&normalize(raw));
    let found = digest.is_some();
    let stored = digest.unwrap_or_else(|| vec![0; 32]);
    bool::from(stored.ct_eq(&candidate)) && found
}
#[cfg(test)]
#[path = "recovery_key_tests.rs"]
mod tests;
