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
    fn seal(&self, token: &str) -> Result<Vec<u8>, sqlx::Error> {
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
    let id: Option<AccountId> = sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1 AND status IN ('active','pending_verification') FOR UPDATE")
        .bind(normalize_email(email)).fetch_optional(&mut *tx).await?;
    if let Some(id) = id {
        let recent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_password_resets WHERE account_id=$1 AND created_at>now()-interval '60 seconds')")
            .bind(id).fetch_one(&mut *tx).await?;
        if !recent {
            let mut random = [0; 32];
            getrandom::fill(&mut random).map_err(|_| unavailable())?;
            let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
            sqlx::query("INSERT INTO account_password_resets(account_id,token_hash,expires_at) VALUES ($1,$2,now()+interval '60 minutes') ON CONFLICT(account_id) DO UPDATE SET token_hash=$2,expires_at=now()+interval '60 minutes',used_at=NULL,created_at=now(),updated_at=now()")
                .bind(id).bind(token_hash(&token)).execute(&mut *tx).await?;
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
    audit(&mut tx, id, "identity.password_reset_requested").await?;
    tx.commit().await
}
pub async fn complete(pool: &PgPool, input: Input) -> Result<bool, sqlx::Error> {
    if !input.valid() {
        return refusal(pool, "identity.password_reset_bad_input").await;
    }
    let hash = tokio::task::spawn_blocking(move || password::hash(&input.password))
        .await
        .map_err(|_| unavailable())?
        .map_err(sqlx::Error::Protocol)?;
    let mut tx = pool.begin().await?;
    let id: Option<AccountId> = sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1 AND status IN ('active','pending_verification') FOR UPDATE")
        .bind(normalize_email(&input.email)).fetch_optional(&mut *tx).await?;
    let mut accepted = false;
    if let Some(id) = id {
        let stored: Option<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM account_password_resets WHERE account_id=$1 AND used_at IS NULL AND expires_at>clock_timestamp() FOR UPDATE")
            .bind(id).fetch_optional(&mut *tx).await?;
        if stored.is_some_and(|s| bool::from(s.ct_eq(&token_hash(&input.token)))) {
            sqlx::query("UPDATE accounts SET password_hash=$2,updated_at=now() WHERE id=$1")
                .bind(id)
                .bind(hash)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE account_password_resets SET used_at=clock_timestamp(),last_reset_at=clock_timestamp(),updated_at=now() WHERE account_id=$1").bind(id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
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
    Ok(accepted)
}
async fn refusal(pool: &PgPool, event: &str) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    audit(&mut tx, None, event).await?;
    tx.commit().await?;
    Ok(false)
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
