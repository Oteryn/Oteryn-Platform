//! Native recovery handles for the credential-change tests (gateway login
//! contract §19.3): a credential change ends every handle of the account.
use sqlx::PgPool;
use uuid::Uuid;

/// Inserts one live, unbound handle of `account` and returns its hash.
pub async fn handle(pool: &PgPool, account: Uuid) -> Result<Vec<u8>, sqlx::Error> {
    let hash = Uuid::now_v7().as_bytes().repeat(2);
    sqlx::query("INSERT INTO native_recovery_handles(handle_hash,account_id,character_id,world_id,channel_id,security_generation,play_generation,attempt_ref,issued_at,expires_at) VALUES($1,$2,uuidv7(),uuidv7(),uuidv7(),1,1,uuidv7(),1,9999999999)")
        .bind(&hash)
        .bind(account)
        .execute(pool)
        .await?;
    Ok(hash)
}

/// Whether the handle has been revoked.
pub async fn revoked(pool: &PgPool, hash: &[u8]) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT revoked_at IS NOT NULL FROM native_recovery_handles WHERE handle_hash=$1",
    )
    .bind(hash)
    .fetch_one(pool)
    .await
}
