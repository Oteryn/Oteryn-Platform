//! Platform-side adapters for the isolated testing environment.
use super::*;
use oteryn_wallet::{AdjustmentAudit, AuditHook, Ledger, WalletFlags, wire::DatabasePolicy};
use sqlx::{PgConnection, PgPool};

#[derive(Debug, Clone, Copy)]
pub(super) struct AccountAuthority;

impl Authority for AccountAuthority {
    async fn account(&self, tx: &mut Transaction<'_, Postgres>, account: Uuid) -> Result<()> {
        if oteryn_wallet::value_eligible_in(tx, account).await? {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }

    async fn publish(&self, _: &mut Transaction<'_, Postgres>, _: Uuid) -> Result<()> {
        Err(Error::Unavailable)
    }

    async fn event(&self, _: &mut Transaction<'_, Postgres>, _: &Event) -> Result<()> {
        Err(Error::Unavailable)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PurchaseAudit;

impl AuditHook for PurchaseAudit {
    async fn adjustment(&self, db: &mut PgConnection, event: &AdjustmentAudit) -> bool {
        if event.amount >= 0
            || event.actor_id != event.account_id
            || event.reason != format!("store-purchase:{}", event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'pass.purchase.debited',$1,$2,$3,$4)")
            .bind(event.account_id.to_string())
            .bind(serde_json::json!({"available": event.before.available, "reserved": event.before.reserved}))
            .bind(serde_json::json!({"available": event.after.available, "reserved": event.after.reserved, "coins": event.amount}))
            .bind(event.operation_id)
            .execute(db)
            .await
            .is_ok()
    }
}

pub(super) fn payment(
    pool: PgPool,
    enabled: bool,
) -> Wallet<DatabasePolicy, DatabasePolicy, PurchaseAudit> {
    Wallet::new(Ledger::new(
        pool,
        WalletFlags { enabled },
        DatabasePolicy,
        DatabasePolicy,
        PurchaseAudit,
    ))
}
