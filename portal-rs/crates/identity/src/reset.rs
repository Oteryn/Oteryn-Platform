//! Single-use recovery credentials. The delivery queue contains authenticated ciphertext only.
use crate::{AccountId, accounts::normalize_email, password, verify::token_hash};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use subtle::ConstantTimeEq;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Input {
    pub email: String,
    pub token: String,
    pub password: String,
    pub password_confirmation: String,
}
impl Input {
    pub fn valid(&self) -> bool {
        valid_email(&self.email)
            && self.token.len() == 64
            && self.token.bytes().all(|b| b.is_ascii_hexdigit())
            && password::valid(&self.password)
            && self.password == self.password_confirmation
    }
}
pub fn valid_email(email: &str) -> bool {
    let email = normalize_email(email);
    email.len() <= 254 && email.is_ascii() && email_address::EmailAddress::is_valid(&email)
}
/// Protects queued recovery links; debug output is always redacted.
#[derive(Clone)]
pub struct DeliveryKey([u8; 32]);
impl DeliveryKey {
    pub fn parse(value: &str) -> Option<Self> {
        if value.len() != 64 {
            return None;
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(value.get(index * 2..index * 2 + 2)?, 16).ok()?;
        }
        Some(Self(bytes))
    }
    /// Encrypt a one-use token for storage in a mail delivery job.
    pub fn seal(&self, token: &str) -> Result<Vec<u8>, sqlx::Error> {
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| unavailable())?;
        let mut ciphertext = ChaCha20Poly1305::new((&self.0).into())
            .encrypt(&Nonce::from(nonce), token.as_bytes())
            .map_err(|_| unavailable())?;
        let mut result = nonce.to_vec();
        result.append(&mut ciphertext);
        Ok(result)
    }
    pub fn open(&self, ciphertext: &[u8]) -> Option<String> {
        if ciphertext.len() < 28 {
            return None;
        }
        let plain = ChaCha20Poly1305::new((&self.0).into())
            .decrypt(
                &Nonce::from(<[u8; 12]>::try_from(&ciphertext[..12]).ok()?),
                &ciphertext[12..],
            )
            .ok()?;
        String::from_utf8(plain).ok()
    }
}
fn unavailable() -> sqlx::Error {
    sqlx::Error::Protocol("recovery unavailable".into())
}
pub async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    account: Option<AccountId>,
    event: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES ($1,$2,'password-reset','{}','{}',$3)")
        .bind(account).bind(event).bind(uuid::Uuid::now_v7()).execute(&mut **tx).await?;
    Ok(())
}
/// Always returns the same public result, including missing and ineligible accounts.
pub async fn request(pool: &PgPool, email: &str, key: &DeliveryKey) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !valid_email(email) {
        audit(&mut tx, None, "identity.password_reset_bad_input").await?;
        return tx.commit().await;
    }
    let email = normalize_email(email);
    // A backup can be removed while this statement waits for the account lock.
    let candidate: Option<AccountId> = sqlx::query_scalar(
        "SELECT a.id FROM accounts a WHERE lower(a.email)=$1 OR EXISTS (
            SELECT FROM account_backup_emails b WHERE b.account_id=a.id AND lower(b.email)=$1 AND b.verified_at<=clock_timestamp()
        ) ORDER BY a.id LIMIT 1 FOR UPDATE OF a",
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;
    let mut actor = None;
    if let Some(id) = candidate {
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(|_| unavailable())?;
        let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
        // Use one post-lock snapshot for both eligibility and the delivery witness.
        let issued: Option<bool> = sqlx::query_scalar(
            "WITH owners AS (
                SELECT a.id,a.status,b.verified_at FROM accounts a
                LEFT JOIN account_backup_emails b ON b.account_id=a.id AND lower(b.email)=$3
                    AND b.verified_at<=clock_timestamp()
                WHERE lower(a.email)=$3 OR b.account_id IS NOT NULL
            ), eligible AS (
                SELECT id,verified_at FROM owners WHERE id=$1 AND status IN ('active','pending_verification')
                    AND NOT EXISTS(SELECT FROM owners WHERE id<>$1)
            ), issued AS (
                INSERT INTO account_password_resets(account_id,token_hash,expires_at,recovery_email,backup_verified_at)
                SELECT id,$2,clock_timestamp()+interval '60 minutes',$3,verified_at FROM eligible
                WHERE NOT EXISTS(SELECT FROM account_password_resets WHERE account_id=$1 AND created_at>now()-interval '60 seconds')
                ON CONFLICT(account_id) DO UPDATE SET token_hash=EXCLUDED.token_hash,expires_at=EXCLUDED.expires_at,
                    recovery_email=EXCLUDED.recovery_email,backup_verified_at=EXCLUDED.backup_verified_at,
                    used_at=NULL,created_at=clock_timestamp(),updated_at=clock_timestamp() RETURNING account_id
            ) SELECT EXISTS(SELECT FROM issued) FROM eligible",
        )
        .bind(id)
        .bind(token_hash(&token))
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;
        actor = issued.map(|_| id);
        if issued == Some(true) {
            let payload =
                serde_json::json!({"account_id": id,"ciphertext": key.seal(&token)?}).to_string();
            oteryn_db::outbox::enqueue(
                &mut tx,
                "email.password-reset",
                &payload,
                &uuid::Uuid::now_v7().to_string(),
                sqlx::types::time::OffsetDateTime::now_utc(),
                5,
            )
            .await?;
        }
    }
    audit(&mut tx, actor, "identity.password_reset_requested").await?;
    tx.commit().await
}
/// Result of a reset submission. Only `Completed` changes the password or uses the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    Completed,
    Refused,
    /// The new password appears in known data breaches.
    Breached,
    /// The breached-password check could not run.
    Unavailable,
}
pub async fn complete(pool: &PgPool, input: Input) -> Result<Completion, sqlx::Error> {
    complete_with(pool, &crate::breach::Checker::from_env(pool.clone()), input).await
}
/// [`complete`] with an explicit breached-password range source.
pub async fn complete_with(
    pool: &PgPool,
    checker: &crate::breach::Checker,
    input: Input,
) -> Result<Completion, sqlx::Error> {
    if !input.valid() {
        refusal(pool, "identity.password_reset_bad_input").await?;
        return Ok(Completion::Refused);
    }
    // Every well-formed submission is checked before the account or token is read, so
    // the answer reveals nothing about either, and nothing is consumed on refusal.
    match crate::breach::check(checker, &input.password).await {
        Ok(()) => {}
        Err(crate::breach::BreachError::Breached) => return Ok(Completion::Breached),
        Err(crate::breach::BreachError::Unavailable) => return Ok(Completion::Unavailable),
    }
    let hash = tokio::task::spawn_blocking(move || password::hash(&input.password))
        .await
        .map_err(|_| unavailable())?
        .map_err(sqlx::Error::Protocol)?;
    let mut tx = pool.begin().await?;
    let id: Option<AccountId> = sqlx::query_scalar("SELECT a.id FROM accounts a WHERE (lower(a.email)=$1 OR EXISTS(SELECT 1 FROM account_backup_emails b WHERE b.account_id=a.id AND lower(b.email)=$1 AND b.verified_at<=clock_timestamp())) AND NOT EXISTS(SELECT 1 FROM accounts other WHERE other.id<>a.id AND (lower(other.email)=$1 OR EXISTS(SELECT 1 FROM account_backup_emails b WHERE b.account_id=other.id AND lower(b.email)=$1 AND b.verified_at<=clock_timestamp()))) AND a.status IN ('active','pending_verification') FOR UPDATE OF a")
        .bind(normalize_email(&input.email)).fetch_optional(&mut *tx).await?;
    let mut accepted = false;
    if let Some(id) = id {
        let stored: Option<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM account_password_resets WHERE account_id=$1 AND used_at IS NULL AND expires_at>clock_timestamp() AND (recovery_email IS NULL OR recovery_email=$2) AND (backup_verified_at IS NULL OR EXISTS(SELECT 1 FROM account_backup_emails b WHERE b.account_id=$1 AND lower(b.email)=recovery_email AND b.verified_at=backup_verified_at)) FOR UPDATE")
            .bind(id).bind(normalize_email(&input.email)).fetch_optional(&mut *tx).await?;
        if stored.is_some_and(|s| bool::from(s.ct_eq(&token_hash(&input.token)))) {
            sqlx::query("UPDATE accounts SET password_hash=$2,native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1")
                .bind(id)
                .bind(hash)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE account_password_resets SET used_at=clock_timestamp(),last_reset_at=clock_timestamp(),updated_at=now() WHERE account_id=$1").bind(id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            crate::recovery_key::revoke_authorizations(&mut tx, id).await?;
            crate::email_change::cancel_pending(&mut tx, id).await?;
            sqlx::query("INSERT INTO credential_security_events(account_id,kind) VALUES($1,'password_changed')").bind(id).execute(&mut *tx).await?;
            accepted = true;
        }
    }
    audit(
        &mut tx,
        id,
        if accepted {
            "identity.password_reset_completed"
        } else {
            "identity.password_reset_refused"
        },
    )
    .await?;
    tx.commit().await?;
    Ok(if accepted {
        Completion::Completed
    } else {
        Completion::Refused
    })
}
async fn refusal(pool: &PgPool, event: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    audit(&mut tx, None, event).await?;
    tx.commit().await
}
#[cfg(test)]
#[path = "reset_tests.rs"]
mod tests;

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResetInput([redacted])")
    }
}
impl std::fmt::Debug for DeliveryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeliveryKey([redacted])")
    }
}
