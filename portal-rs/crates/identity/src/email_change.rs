//! D14 email changes. All entry points require an explicit feature flag.
//! Security proofs and global authorization revocation are transactional ports:
//! T06/T05b and OAuth/device owners must supply them before enabling requests.
//! Unknown proof or revocation state denies; there is no production allow adapter.
use crate::{AccountId, accounts::normalize_email, password, verify::token_hash};
use sqlx::{PgConnection, PgPool, Postgres, Transaction, types::time::OffsetDateTime};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

#[path = "email_change_mail.rs"]
mod mail;
pub use mail::{Worker, text};

type FutureResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, sqlx::Error>> + Send + 'a>>;
/// Verify account-bound MFA or consume a recovery key in the supplied transaction.
/// A true result must be based on authoritative, replay-protected evidence.
/// Revocation must cover OAuth families and device sessions, or return false.
pub trait Security: Send + Sync {
    fn proof<'a>(
        &'a self,
        db: &'a mut PgConnection,
        account: AccountId,
        recovery: bool,
        proof: &'a str,
    ) -> FutureResult<'a, bool>;
    fn revoke<'a>(&'a self, db: &'a mut PgConnection, account: AccountId)
    -> FutureResult<'a, bool>;
}
#[derive(Debug)]
pub struct DenySecurity;
impl Security for DenySecurity {
    fn proof<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: AccountId,
        _: bool,
        _: &'a str,
    ) -> FutureResult<'a, bool> {
        Box::pin(async { Ok(false) })
    }
    fn revoke<'a>(&'a self, _: &'a mut PgConnection, _: AccountId) -> FutureResult<'a, bool> {
        Box::pin(async { Ok(false) })
    }
}
/// Transactional proof consumption and shared authorization revocation.
#[derive(Debug)]
pub struct TransactionalSecurity(pub crate::mfa::Mfa);
impl Security for TransactionalSecurity {
    fn proof<'a>(
        &'a self,
        db: &'a mut PgConnection,
        account: AccountId,
        recovery: bool,
        proof: &'a str,
    ) -> FutureResult<'a, bool> {
        Box::pin(async move {
            if !recovery {
                return self.0.verify_totp_in_connection(db, account, proof).await;
            }
            let stored: Option<String> = sqlx::query_scalar("SELECT key_hash FROM account_recovery_keys WHERE account_id=$1 AND confirmed_at IS NOT NULL AND used_at IS NULL AND revoked_at IS NULL FOR UPDATE").bind(account).fetch_optional(&mut *db).await?;
            let raw = crate::recovery_key::normalize(proof);
            let valid = tokio::task::spawn_blocking(move || {
                stored.is_some_and(|hash| password::verify(&raw, &hash))
            })
            .await
            .map_err(|_| sqlx::Error::Protocol("proof verification unavailable".into()))?;
            if valid {
                sqlx::query("UPDATE account_recovery_keys SET used_at=now(),updated_at=now() WHERE account_id=$1").bind(account).execute(db).await?;
            }
            Ok(valid)
        })
    }
    fn revoke<'a>(
        &'a self,
        db: &'a mut PgConnection,
        account: AccountId,
    ) -> FutureResult<'a, bool> {
        Box::pin(async move {
            crate::recovery_key::revoke_authorizations(db, account).await?;
            Ok(true)
        })
    }
}
#[derive(Clone)]
pub struct Service {
    pub pool: PgPool,
    pub enabled: bool,
    pub security: Arc<dyn Security>,
    /// Canonical HTTPS portal origin, supplied by deployment configuration.
    pub origin: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Requested,
    Confirmed,
    Changed,
    Cancelled,
    Reverted,
    Refused,
}
#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct Request {
    pub email: String,
    pub email_confirmation: String,
    pub current_password: String,
    pub proof: String,
    pub recovery: bool,
    pub backup: bool,
}
impl Request {
    fn valid(&self) -> bool {
        valid_email(&normalize_email(&self.email))
            && normalize_email(&self.email) == normalize_email(&self.email_confirmation)
            && !self.current_password.is_empty()
            && self.current_password.len() <= 1024
            && !self.proof.is_empty()
            && self.proof.len() <= 1024
    }
}
fn valid_email(email: &str) -> bool {
    email.len() <= 254 && email.is_ascii() && email_address::EmailAddress::is_valid(email)
}
pub fn valid_token(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn random_token() -> Result<String, sqlx::Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| sqlx::Error::Protocol("secure randomness unavailable".into()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
#[derive(sqlx::FromRow)]
struct Change {
    id: Uuid,
    account_id: Uuid,
    kind: String,
    old_email: String,
    new_email: String,
    confirmed_at: Option<OffsetDateTime>,
    applied_at: Option<OffsetDateTime>,
    cancelled_at: Option<OffsetDateTime>,
    reverted_at: Option<OffsetDateTime>,
    apply_after: OffsetDateTime,
    expires_at: OffsetDateTime,
    revert_until: OffsetDateTime,
}
impl Service {
    pub fn ready(&self) -> bool {
        self.enabled
            && self.origin.starts_with("https://")
            && self.origin.len() <= 2048
            && !self.origin[8..].is_empty()
            && !self.origin[8..].contains(['/', '?', '#', '<', '>', '"', '\'', ' ', '\r', '\n'])
    }
    pub async fn request(
        &self,
        account: AccountId,
        input: Request,
    ) -> Result<Outcome, sqlx::Error> {
        if !self.ready() || !input.valid() {
            return Ok(Outcome::Refused);
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,String,String)> = sqlx::query_as("SELECT email,password_hash,locale FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=now() FOR UPDATE")
            .bind(account).fetch_optional(&mut *tx).await?;
        let Some((old, hash, locale)) = row else {
            return Ok(Outcome::Refused);
        };
        let new = normalize_email(&input.email);
        if new == old {
            return Ok(Outcome::Refused);
        }
        let password = input.current_password;
        let verified = tokio::task::spawn_blocking(move || password::verify(&password, &hash))
            .await
            .map_err(|_| sqlx::Error::Protocol("password verifier unavailable".into()))?;
        if !verified
            || !self
                .security
                .proof(&mut tx, account, input.recovery, &input.proof)
                .await?
        {
            return Ok(Outcome::Refused);
        }
        // Serialize cross-account reservations without relying on check-then-insert.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(&new)
            .execute(&mut *tx)
            .await?;
        if !available(&mut tx, account, &new).await? {
            return Ok(Outcome::Refused);
        }
        let kind = if input.backup { "backup" } else { "primary" };
        sqlx::query("UPDATE account_email_changes SET cancelled_at=now(),updated_at=now() WHERE account_id=$1 AND kind=$2 AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL").bind(account).bind(kind).execute(&mut *tx).await?;
        let confirmation = random_token()?;
        let revert = random_token()?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO account_email_changes(id,account_id,kind,old_email,new_email,confirmation_hash,revert_hash,immediate,apply_after,expires_at,revert_until) VALUES($1,$2,$3,$4,$5,$6,$7,$8,now()+CASE WHEN $8 THEN interval '0' ELSE interval '7 days' END,now()+interval '8 days',now()+interval '30 days')")
            .bind(id).bind(account).bind(kind).bind(&old).bind(&new).bind(token_hash(&confirmation)).bind(token_hash(&revert)).bind(input.recovery || input.backup).execute(&mut *tx).await?;
        mail::enqueue(
            &mut tx,
            id,
            &new,
            &locale,
            "confirm",
            &format!("{}/email-change/confirm/{confirmation}", self.origin),
        )
        .await?;
        mail::enqueue(
            &mut tx,
            id,
            &old,
            &locale,
            "revert",
            &format!("{}/email-change/revert/{revert}", self.origin),
        )
        .await?;
        tx.commit().await?;
        Ok(Outcome::Requested)
    }
    pub async fn confirm(&self, token: &str) -> Result<Outcome, sqlx::Error> {
        self.token(token, false).await
    }
    pub async fn revert(&self, token: &str) -> Result<Outcome, sqlx::Error> {
        self.token(token, true).await
    }
    async fn token(&self, token: &str, revert: bool) -> Result<Outcome, sqlx::Error> {
        if !self.ready() || !valid_token(token) {
            return Ok(Outcome::Refused);
        }
        let mut tx = self.pool.begin().await?;
        let id: Option<Uuid> = sqlx::query_scalar("SELECT account_id FROM account_email_changes WHERE (NOT $2 AND confirmation_hash=$1) OR ($2 AND revert_hash=$1)").bind(token_hash(token)).bind(revert).fetch_optional(&mut *tx).await?;
        let Some(account) = id else {
            return Ok(Outcome::Refused);
        };
        // Every path locks account first, then the change, including the worker.
        let active: Option<String> = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE").bind(account).fetch_optional(&mut *tx).await?;
        let Some(current) = active else {
            return Ok(Outcome::Refused);
        };
        let change: Change = sqlx::query_as("SELECT * FROM account_email_changes WHERE (NOT $2 AND confirmation_hash=$1) OR ($2 AND revert_hash=$1) FOR UPDATE").bind(token_hash(token)).bind(revert).fetch_one(&mut *tx).await?;
        let now: OffsetDateTime = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *tx)
            .await?;
        if change.cancelled_at.is_some() || change.reverted_at.is_some() {
            return Ok(Outcome::Refused);
        }
        let result = if revert {
            if change.revert_until <= now {
                return Ok(Outcome::Refused);
            }
            if change.applied_at.is_none() {
                sqlx::query("UPDATE account_email_changes SET cancelled_at=now(),updated_at=now() WHERE id=$1").bind(change.id).execute(&mut *tx).await?;
                Outcome::Cancelled
            } else if change.kind == "backup" {
                let removed = sqlx::query("DELETE FROM account_backup_emails WHERE account_id=$1 AND email=$2 AND verified_at=$3")
                    .bind(account).bind(&change.new_email).bind(change.applied_at).execute(&mut *tx).await?.rows_affected();
                if removed != 1 {
                    return Ok(Outcome::Refused);
                }
                mark_reverted(&mut tx, change.id).await?;
                Outcome::Reverted
            } else {
                if current != change.new_email || !self.replace(&mut tx, &change, true).await? {
                    return Ok(Outcome::Refused);
                }
                mark_reverted(&mut tx, change.id).await?;
                Outcome::Reverted
            }
        } else {
            if change.confirmed_at.is_some()
                || change.applied_at.is_some()
                || change.expires_at <= now
                || current != change.old_email
            {
                return Ok(Outcome::Refused);
            }
            sqlx::query(
                "UPDATE account_email_changes SET confirmed_at=now(),updated_at=now() WHERE id=$1",
            )
            .bind(change.id)
            .execute(&mut *tx)
            .await?;
            if change.kind == "backup" {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
                    .bind(&change.new_email)
                    .execute(&mut *tx)
                    .await?;
                if !available(&mut tx, account, &change.new_email).await? {
                    return Ok(Outcome::Refused);
                }
                sqlx::query("INSERT INTO account_backup_emails(account_id,email,verified_at) VALUES($1,$2,now()) ON CONFLICT(account_id) DO UPDATE SET email=$2,verified_at=now(),updated_at=now()") .bind(account).bind(&change.new_email).execute(&mut *tx).await?;
                mark_applied(&mut tx, change.id).await?;
                Outcome::Changed
            } else if change.apply_after <= now {
                if !self.replace(&mut tx, &change, false).await? {
                    return Ok(Outcome::Refused);
                }
                mark_applied(&mut tx, change.id).await?;
                Outcome::Changed
            } else {
                Outcome::Confirmed
            }
        };
        tx.commit().await?;
        Ok(result)
    }
    async fn replace(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        c: &Change,
        revert: bool,
    ) -> Result<bool, sqlx::Error> {
        let email = if revert { &c.old_email } else { &c.new_email };
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(email)
            .execute(&mut **tx)
            .await?;
        if !available(tx, c.account_id, email).await?
            || !self.security.revoke(tx, c.account_id).await?
        {
            return Ok(false);
        }
        sqlx::query("UPDATE accounts SET email=$2,email_verified_at=now(),email_change_blocked_until=now()+interval '72 hours',native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1").bind(c.account_id).bind(email).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
            .bind(c.account_id)
            .execute(&mut **tx)
            .await?;
        // Swapping with a verified backup preserves the previous primary as backup.
        sqlx::query("UPDATE account_backup_emails SET email=$3,verified_at=now(),updated_at=now() WHERE account_id=$1 AND email=$2").bind(c.account_id).bind(email).bind(if revert { &c.new_email } else { &c.old_email }).execute(&mut **tx).await?;
        sqlx::query("UPDATE account_email_changes SET cancelled_at=now(),updated_at=now() WHERE account_id=$1 AND id<>$2 AND applied_at IS NULL AND cancelled_at IS NULL").bind(c.account_id).bind(c.id).execute(&mut **tx).await?;
        Ok(true)
    }
    /// Worker applies only confirmed requests, with authoritative revocation available.
    pub async fn apply_due(&self) -> Result<usize, sqlx::Error> {
        if !self.ready() {
            return Ok(0);
        }
        let ids: Vec<(Uuid,Uuid)> = sqlx::query_as("SELECT id,account_id FROM account_email_changes WHERE kind='primary' AND confirmed_at IS NOT NULL AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL AND apply_after<=now() AND expires_at>now() ORDER BY apply_after LIMIT 100").fetch_all(&self.pool).await?;
        let mut count = 0;
        for (id, account) in ids {
            let mut tx = self.pool.begin().await?;
            let current: Option<String> = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=now() FOR UPDATE").bind(account).fetch_optional(&mut *tx).await?;
            let c: Change =
                sqlx::query_as("SELECT * FROM account_email_changes WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            let now: OffsetDateTime = sqlx::query_scalar("SELECT now()")
                .fetch_one(&mut *tx)
                .await?;
            if current.as_deref() != Some(&c.old_email)
                || c.confirmed_at.is_none()
                || c.applied_at.is_some()
                || c.cancelled_at.is_some()
                || c.reverted_at.is_some()
                || c.apply_after > now
                || c.expires_at <= now
            {
                continue;
            }
            if self.replace(&mut tx, &c, false).await? {
                mark_applied(&mut tx, id).await?;
                tx.commit().await?;
                count += 1;
            }
        }
        Ok(count)
    }
}
async fn available(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
    email: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM accounts WHERE lower(email)=lower($2) AND id<>$1) AND NOT EXISTS(SELECT 1 FROM account_backup_emails WHERE lower(email)=lower($2) AND account_id<>$1) AND NOT EXISTS(SELECT 1 FROM account_email_changes WHERE lower(new_email)=lower($2) AND account_id<>$1 AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL AND expires_at>now())").bind(account).bind(email).fetch_one(&mut **tx).await
}
async fn mark_applied(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE account_email_changes SET applied_at=now(),updated_at=now() WHERE id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn mark_reverted(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE account_email_changes SET reverted_at=now(),updated_at=now() WHERE id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
/// Consumer seam for wallet and bazaar: call inside their transaction before trade.
/// The shared account lock serializes the check with an email change. Missing state denies.
pub async fn trade_allowed(db: &mut PgConnection, account: AccountId) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar::<_,bool>("SELECT status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=now() FROM accounts WHERE id=$1 FOR SHARE").bind(account).fetch_optional(db).await?.unwrap_or(false))
}
#[cfg(test)]
#[path = "email_change_tests.rs"]
mod tests;

impl std::fmt::Debug for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EmailChangeService { redacted }")
    }
}
impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EmailChangeRequest { redacted }")
    }
}
/// Wallet adapter for D14. The wallet's independent account and feature guards
/// still apply; database errors and absent identity rows deny transfer.
#[derive(Debug)]
pub struct TransferRestrictions;
impl oteryn_wallet::TransferBlockCheck for TransferRestrictions {
    async fn permission(
        &self,
        db: &mut PgConnection,
        account: Uuid,
    ) -> oteryn_wallet::TransferPermission {
        match trade_allowed(db, account).await {
            Ok(true) => oteryn_wallet::TransferPermission::Allowed,
            Ok(false) => oteryn_wallet::TransferPermission::Blocked,
            Err(_) => oteryn_wallet::TransferPermission::Unknown,
        }
    }
}
