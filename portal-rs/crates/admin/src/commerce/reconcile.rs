//! Operator reconciliation of refunds whose `commerce.refund` outbox job is
//! dead (Issue #1922). It revives the same job; the worker then runs the one
//! refund path (claim, provider call keyed by the intent id, finalize).
use super::*;

/// Rows revived per run; a larger backlog needs another run.
pub const RECONCILE_BATCH: i64 = 100;

/// An unsettled refund whose outbox job is dead, as found before any change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadRefund {
    pub operation: Uuid,
    pub job: Uuid,
    pub state: String,
    pub attempt: Option<Uuid>,
}

/// Finds unsettled refunds whose `commerce.refund` job is dead and, unless
/// `dry_run`, returns that same job (same key and payload) to `pending` with
/// its attempts reset, writing one `commerce.refund.reconciled` audit row per
/// revival in the same transaction. A refund whose intent the payment ledger
/// already shows refunded is left untouched. A `refunding` row is skipped while its
/// claim is live under the delivery path's own staleness rule. The actor must
/// be an active staff account holding payment reconciliation.
pub async fn reconcile_dead_refunds(
    pool: &PgPool,
    actor: Uuid,
    dry_run: bool,
) -> Result<Vec<DeadRefund>, Error> {
    if actor.is_nil() {
        return Err(Error::Denied);
    }
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    rbac::lock_accounts_and_mutation(&mut tx, &[actor]).await?;
    let permissions = [
        rbac::Permission::Access.as_str(),
        rbac::Permission::ReconcilePayments.as_str(),
    ];
    let permitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active') AND (SELECT count(DISTINCT rp.permission) FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id WHERE ar.account_id=$1 AND rp.permission=ANY($2))=cardinality($2)")
        .bind(actor).bind(&permissions[..]).fetch_one(&mut *tx).await?;
    if !permitted {
        return Err(Error::Denied);
    }
    let found: Vec<(Uuid, Uuid, String, Option<Uuid>)> = sqlx::query_as("SELECT r.operation_id,o.id,r.state,r.attempt_id FROM commerce_refunds r JOIN outbox o ON o.kind='commerce.refund' AND o.idempotency_key=r.intent_id::text AND o.payload->>'operation'=r.operation_id::text WHERE o.state='dead' AND r.provider_reference IS NULL AND NOT EXISTS(SELECT 1 FROM payment_intents i WHERE i.id=r.intent_id AND i.state='refunded') AND NOT (r.state='refunding' AND r.claimed_at > now() - make_interval(secs => $1)) ORDER BY r.operation_id LIMIT $2 FOR UPDATE OF r,o")
        .bind(REFUND_CLAIM_STALE_AFTER.as_secs_f64()).bind(RECONCILE_BATCH).fetch_all(&mut *tx).await?;
    let found: Vec<DeadRefund> = found
        .into_iter()
        .map(|(operation, job, state, attempt)| DeadRefund {
            operation,
            job,
            state,
            attempt,
        })
        .collect();
    if dry_run {
        tx.rollback().await?;
        return Ok(found);
    }
    for refund in &found {
        sqlx::query("UPDATE outbox SET state='pending',attempts=0,run_after=now(),leased_until=NULL,updated_at=now() WHERE id=$1 AND state='dead'")
            .bind(refund.job).execute(&mut *tx).await?;
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(actor),
                    request_id: refund.operation,
                },
                action: "commerce.refund.reconciled",
                target: &refund.operation.to_string(),
                before: json!({"state":refund.state,"attempt_id":refund.attempt,"job":refund.job,"job_state":"dead"}),
                after: json!({"operation":refund.operation,"job":refund.job,"job_state":"pending"}),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(found)
}
