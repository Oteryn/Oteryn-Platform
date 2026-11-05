//! Read-only premium projection for the authenticated account panel.
use crate::premium::{Entitlement, Error};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// No snapshot issuance: viewing the panel never changes authority revisions.
pub async fn premium(pool: &PgPool, account: Uuid) -> Result<Option<Entitlement>, Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *transaction)
        .await?;
    let row = sqlx::query("SELECT entitlement_id,event_type,lifecycle_revision,effective_from,effective_until FROM premium_ledger WHERE account_id=$1 ORDER BY lifecycle_revision DESC LIMIT 1")
        .bind(account).fetch_optional(&mut *transaction).await?;
    let Some(row) = row else { return Ok(None) };
    let value = Entitlement {
        id: row.try_get("entitlement_id")?,
        revoked: row.try_get::<String, _>("event_type")? == "REVOKE",
        revision: row.try_get("lifecycle_revision")?,
        from: row
            .try_get::<OffsetDateTime, _>("effective_from")?
            .unix_timestamp(),
        until: row
            .try_get::<OffsetDateTime, _>("effective_until")?
            .unix_timestamp(),
    };
    value.validate()?;
    let (seconds, count, identities): (i64,i64,i64) = sqlx::query_as("SELECT COALESCE(SUM(delta_seconds),0)::bigint,COUNT(*),COUNT(DISTINCT entitlement_id) FROM premium_ledger WHERE account_id=$1")
        .bind(account).fetch_one(&mut *transaction).await?;
    if value.from.checked_add(seconds) != Some(value.until)
        || count != value.revision
        || identities != 1
    {
        return Err(Error::Unavailable);
    }
    Ok(Some(value))
}
