//! Credential changes serialize on the account row. Authentication adapters must apply
//! CSRF and rate limits before calling this API; no account ID is accepted from a form.
use crate::{AccountId, password};
use sqlx::{PgPool, Postgres, Transaction};
use std::{future::Future, pin::Pin, sync::Arc};

pub type EffectFuture<'a> = Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>>;
/// T16/T21 composition seam. Implementations must revoke authorization codes, OAuth
/// access/refresh families, remembered devices and trusted devices in this transaction.
/// They must also enforce any activated native non-rollback witness. Never commit here.
pub trait AuthorizationRevoker: Send + Sync {
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> EffectFuture<'a>;
}
/// Missing dependencies are unavailable, rather than silently treated as empty tables.
#[derive(Debug)]
pub struct UnavailableRevoker;
impl AuthorizationRevoker for UnavailableRevoker {
    fn revoke<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: AccountId,
    ) -> EffectFuture<'a> {
        Box::pin(async {
            Err(sqlx::Error::Protocol(
                "authorization revocation unavailable".into(),
            ))
        })
    }
}
#[derive(Clone)]
pub struct Credentials {
    pub pool: PgPool,
    pub enabled: bool,
    pub revoker: Arc<dyn AuthorizationRevoker>,
    pub notifications: Arc<oteryn_notify::Notifications>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Changed,
    Refused,
    Invalid,
}
/// Sensitive fields are always redacted in Debug output.
pub struct PasswordChange {
    pub current_password: String,
    pub password: String,
    pub password_confirmation: String,
}
impl PasswordChange {
    pub fn valid(&self) -> bool {
        !self.current_password.is_empty()
            && self.current_password.len() <= 1024
            && password::valid(&self.password)
            && self.password == self.password_confirmation
            && self.password != self.current_password
    }
}
impl Credentials {
    /// `session_hash` is SHA-256 of the server-issued browser cookie, never an account ID.
    pub async fn change(
        &self,
        session_hash: &[u8],
        input: PasswordChange,
    ) -> Result<Outcome, sqlx::Error> {
        if !self.enabled {
            return Ok(Outcome::Refused);
        }
        if !input.valid() {
            return Ok(Outcome::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let Some((account, hash)) = authenticated(&mut tx, session_hash).await? else {
            return Ok(Outcome::Refused);
        };
        let verified = verify(input.current_password, hash).await?;
        if !verified {
            return Ok(Outcome::Refused);
        }
        let hash = tokio::task::spawn_blocking(move || password::hash(&input.password))
            .await
            .map_err(|_| sqlx::Error::Protocol("password hashing unavailable".into()))?
            .map_err(sqlx::Error::Protocol)?;
        // Revocation and the witness must succeed before any credential update commits.
        self.revoker.revoke(&mut tx, account).await?;
        sqlx::query("UPDATE accounts SET password_hash=$2,native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1")
            .bind(account).bind(hash).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
            .bind(account)
            .execute(&mut *tx)
            .await?;
        self.notifications
            .enqueue(
                &mut tx,
                account,
                &oteryn_notify::EmailTemplate::PasswordChanged,
            )
            .await
            .map_err(|_| sqlx::Error::Protocol("security notification unavailable".into()))?;
        sqlx::query(
            "INSERT INTO credential_security_events(account_id,kind) VALUES($1,'password_changed')",
        )
        .bind(account)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Outcome::Changed)
    }
    /// Returns the authenticated account only for a live, recently confirmed session.
    pub async fn require_sudo(
        &self,
        session_hash: &[u8],
    ) -> Result<Option<AccountId>, sqlx::Error> {
        if !self.enabled {
            return Ok(None);
        }
        sqlx::query_scalar("SELECT s.account_id FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.idle_expires_at>now() AND s.absolute_expires_at>now() AND s.sudo_until>now() AND s.sudo_until<=now()+interval '15 minutes' AND a.status='active' AND a.email_verified_at IS NOT NULL")
            .bind(session_hash).fetch_optional(&self.pool).await
    }
}
/// Shared with the binary sudo adapter. Hold this transaction's account lock until
/// identifier/CSRF rotation completes so a password change cannot race confirmation.
pub async fn authenticated(
    tx: &mut Transaction<'_, Postgres>,
    session_hash: &[u8],
) -> Result<Option<(AccountId, String)>, sqlx::Error> {
    if session_hash.len() != 32 {
        return Ok(None);
    }
    // Account first, session second: every credential mutation uses the same lock order.
    let row: Option<(AccountId,String)> = sqlx::query_as("SELECT a.id,a.password_hash FROM accounts a JOIN web_sessions s ON s.account_id=a.id WHERE s.id_hash=$1 AND a.status='active' AND a.email_verified_at IS NOT NULL FOR UPDATE OF a")
        .bind(session_hash).fetch_optional(&mut **tx).await?;
    let Some((id, hash)) = row else {
        return Ok(None);
    };
    let live: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2 AND idle_expires_at>now() AND absolute_expires_at>now())")
        .bind(session_hash).bind(id).fetch_one(&mut **tx).await?;
    Ok(live.then_some((id, hash)))
}
pub async fn verify(secret: String, hash: String) -> Result<bool, sqlx::Error> {
    if secret.is_empty() || secret.len() > 1024 {
        return Ok(false);
    }
    tokio::task::spawn_blocking(move || password::verify(&secret, &hash))
        .await
        .map_err(|_| sqlx::Error::Protocol("password verification unavailable".into()))
}
#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for PasswordChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordChange([redacted])")
    }
}
