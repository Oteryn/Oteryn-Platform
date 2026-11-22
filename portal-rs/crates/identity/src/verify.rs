//! One-use verification tokens, persisted only as SHA-256 digests.
use crate::accounts::{AccountId, Accounts};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
/// Generate an opaque one-use email credential.
pub fn new_token() -> Result<String, sqlx::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| sqlx::Error::Protocol("secure randomness unavailable".into()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Caller must hold the account lock when replacing an existing token.
pub async fn issue(
    tx: &mut Transaction<'_, Postgres>,
    id: AccountId,
    locale: &str,
) -> Result<(), sqlx::Error> {
    let token = new_token()?;
    sqlx::query("UPDATE account_email_tokens SET used_at=now(),updated_at=now() WHERE account_id=$1 AND purpose='verify_email' AND used_at IS NULL").bind(id).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO account_email_tokens (account_id,token_hash,purpose,expires_at) VALUES ($1,$2,'verify_email',now()+interval '24 hours')").bind(id).bind(token_hash(&token)).execute(&mut **tx).await?;
    enqueue(tx, id, locale, &token).await
}
async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    id: AccountId,
    locale: &str,
    token: &str,
) -> Result<(), sqlx::Error> {
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
/// Check whether to present the confirmation form without consuming the link.
pub async fn available(pool: &PgPool, token: &str) -> Result<bool, sqlx::Error> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT EXISTS(SELECT FROM account_email_tokens t JOIN accounts a ON a.id=t.account_id WHERE t.token_hash=$1 AND t.purpose='verify_email' AND t.used_at IS NULL AND t.expires_at>clock_timestamp() AND a.status='pending_verification' AND a.email_verified_at IS NULL)")
        .bind(token_hash(token)).fetch_one(pool).await
}
pub async fn consume(pool: &PgPool, token: &str) -> Result<Option<AccountId>, sqlx::Error> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let hash = token_hash(token);
    let mut tx = pool.begin().await?;
    let id:Option<AccountId>=sqlx::query_scalar("SELECT account_id FROM account_email_tokens WHERE token_hash=$1 AND purpose='verify_email'").bind(&hash).fetch_optional(&mut *tx).await?;
    let Some(id) = id else {
        return Ok(None);
    };
    // Same lock order as resend. The token is checked again after acquiring the lock.
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let used=sqlx::query("UPDATE account_email_tokens SET used_at=now(),updated_at=now() WHERE token_hash=$1 AND used_at IS NULL AND expires_at>clock_timestamp() AND purpose='verify_email'").bind(hash).execute(&mut *tx).await?.rows_affected();
    if used != 1 || !Accounts::mark_verified(&mut tx, id).await? {
        return Ok(None);
    }
    tx.commit().await?;
    Ok(Some(id))
}
pub async fn resend(
    pool: &PgPool,
    id: impl Into<crate::mutation_actor::MutationActor>,
) -> Result<bool, sqlx::Error> {
    let actor = id.into();
    let id = actor.account;
    let mut tx = pool.begin().await?;
    let locale:Option<String>=sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1 AND status='pending_verification' AND email_verified_at IS NULL FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?;
    let Some(locale) = locale else {
        return Ok(false);
    };
    if !actor.live(&mut tx).await? {
        return Ok(false);
    }

    let recent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_email_tokens WHERE account_id=$1 AND purpose='verify_email' AND updated_at>clock_timestamp()-interval '60 seconds')").bind(id).fetch_one(&mut *tx).await?;
    if recent {
        return Ok(false);
    }
    let active: Option<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM account_email_tokens WHERE account_id=$1 AND purpose='verify_email' AND used_at IS NULL AND expires_at>clock_timestamp() ORDER BY created_at DESC LIMIT 1")
        .bind(id).fetch_optional(&mut *tx).await?;
    if let Some(hash) = active {
        // The delivery job already holds the bearer value. Never invalidate a live
        // link if its delivery record has been pruned.
        let token: Option<String> = sqlx::query_scalar("SELECT payload->>'token' FROM outbox WHERE kind='email.verify' AND payload->>'account_id'=$1 AND sha256(convert_to(payload->>'token','UTF8'))=$2 ORDER BY created_at DESC LIMIT 1")
            .bind(id.to_string()).bind(&hash).fetch_optional(&mut *tx).await?;
        let Some(token) = token else {
            return Ok(false);
        };
        enqueue(&mut tx, id, &locale, &token).await?;
        sqlx::query(
            "UPDATE account_email_tokens SET updated_at=clock_timestamp() WHERE token_hash=$1",
        )
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    } else {
        issue(&mut tx, id, &locale).await?;
    }
    tx.commit().await?;
    Ok(true)
}
