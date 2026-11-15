//! Lock order: moderation gate, listing, issuer, evidence/outbox. Receipt
//! verification reads immutable listing bindings without taking listing locks.
use sqlx::PgConnection;
pub(crate) async fn listing_gate(db: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared(64002)")
        .execute(db)
        .await?;
    Ok(())
}
