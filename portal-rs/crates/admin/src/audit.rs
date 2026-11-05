//! Append-only staff audit records. Call inside the transaction that changes domain state.

use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Metadata supplied by a trusted authentication/request layer, never by a request body.
#[derive(Debug, Clone, Copy)]
pub struct AuditContext {
    pub actor: Option<Uuid>,
    pub request_id: Uuid,
}

/// Values must contain only the relevant domain diff, never credentials or session secrets.
#[derive(Debug)]
pub struct Entry<'a> {
    pub context: AuditContext,
    pub action: &'a str,
    pub target: &'a str,
    pub before: Value,
    pub after: Value,
}

/// Appends through the caller's transaction, so a failed audit insert rolls back the change.
pub async fn append(
    transaction: &mut Transaction<'_, Postgres>,
    entry: Entry<'_>,
) -> Result<Uuid, crate::rbac::Error> {
    if entry.context.request_id.is_nil()
        || entry.context.actor.is_some_and(|id| id.is_nil())
        || entry.action.trim().is_empty()
        || entry.action.chars().count() > 128
        || entry.target.trim().is_empty()
        || entry.target.chars().count() > 256
    {
        return Err(crate::rbac::Error::InvalidInput);
    }
    let id = sqlx::query_scalar(
        "INSERT INTO audit_log (actor, action, target, before, after, request_id)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(entry.context.actor)
    .bind(entry.action)
    .bind(entry.target)
    .bind(entry.before)
    .bind(entry.after)
    .bind(entry.context.request_id)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(id)
}
