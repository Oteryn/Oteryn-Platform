//! Shared transaction-local gate for new spending, acquisitions and rewards.
use sqlx::PgConnection;
use uuid::Uuid;

/// Locks the account until transaction end and checks all identity restrictions.
/// Existing reservations and refunds must use the safe-return gate instead.
pub async fn value_eligible_in(db: &mut PgConnection, account: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT account_value_eligible($1)")
        .bind(account)
        .fetch_one(db)
        .await
}

/// Lock an existing holder without authorizing new spending.
pub async fn safe_return_in(db: &mut PgConnection, account: Uuid) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT status IN ('active','suspended') FROM accounts WHERE id=$1 FOR SHARE",
    )
    .bind(account)
    .fetch_optional(db)
    .await?
    .unwrap_or(false))
}
