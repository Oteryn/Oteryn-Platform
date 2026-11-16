//! Lock order: moderation gate before accounts, listing, issuer and evidence/outbox.
//! Staff holds this gate exclusively before target and account UPDATE locks. Receipt
//! verification reads immutable listing bindings without taking listing locks.
use sqlx::PgConnection;
pub(crate) async fn listing_gate(db: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared(64002)")
        .execute(db)
        .await?;
    Ok(())
}

/// All identity SHARE locks precede issuer/wallet effects, in UUID order.
pub(crate) async fn accounts(
    db: &mut PgConnection,
    accounts: &[uuid::Uuid],
) -> Result<(), sqlx::Error> {
    listing_gate(db).await?;
    sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE")
        .bind(accounts)
        .execute(db)
        .await?;
    Ok(())
}
