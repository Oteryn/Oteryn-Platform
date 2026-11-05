//! One-use verification tokens, persisted only as SHA-256 digests.
use crate::accounts::{AccountId, Accounts};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
/// Caller must hold the account lock when replacing an existing token.
pub async fn issue(
    tx: &mut Transaction<'_, Postgres>,
    id: AccountId,
    locale: &str,
) -> Result<(), sqlx::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| sqlx::Error::Protocol("secure randomness unavailable".into()))?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    sqlx::query("UPDATE account_email_tokens SET used_at=now(),updated_at=now() WHERE account_id=$1 AND purpose='verify_email' AND used_at IS NULL").bind(id).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO account_email_tokens (account_id,token_hash,purpose,expires_at) VALUES ($1,$2,'verify_email',now()+interval '24 hours')").bind(id).bind(token_hash(&token)).execute(&mut **tx).await?;
    let payload = serde_json::json!({"account_id":id,"locale":locale,"token":token}).to_string();
    oteryn_db::outbox::enqueue(
        tx,
        "email.verify",
        &payload,
        &uuid::Uuid::now_v7().to_string(),
        sqlx::types::time::OffsetDateTime::now_utc(),
        5,
    )
    .await?;
    Ok(())
}
pub async fn consume(pool: &PgPool, token: &str) -> Result<bool, sqlx::Error> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let hash = token_hash(token);
    let mut tx = pool.begin().await?;
    let id:Option<AccountId>=sqlx::query_scalar("SELECT account_id FROM account_email_tokens WHERE token_hash=$1 AND purpose='verify_email'").bind(&hash).fetch_optional(&mut *tx).await?;
    let Some(id) = id else {
        return Ok(false);
    };
    // Same lock order as resend. The token is checked again after acquiring the lock.
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let used=sqlx::query("UPDATE account_email_tokens SET used_at=now(),updated_at=now() WHERE token_hash=$1 AND used_at IS NULL AND expires_at>now() AND purpose='verify_email'").bind(hash).execute(&mut *tx).await?.rows_affected();
    if used != 1 || !Accounts::mark_verified(&mut tx, id).await? {
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}
pub async fn resend(pool: &PgPool, id: AccountId) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let locale:Option<String>=sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1 AND status='pending_verification' AND email_verified_at IS NULL FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?;
    let Some(locale) = locale else {
        return Ok(false);
    };
    issue(&mut tx, id, &locale).await?;
    tx.commit().await?;
    Ok(true)
}
