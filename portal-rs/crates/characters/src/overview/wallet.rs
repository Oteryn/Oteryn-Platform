//! Read-only adapter to the wallet's locked balance API.
use super::Accounts;
use oteryn_wallet::{
    AccountState, AccountStateReader, AdjustmentAudit, AuditHook, TransferBlockCheck,
    TransferPermission,
};
use sqlx::PgConnection;
use uuid::Uuid;
impl AccountStateReader for Accounts {
    async fn state(&self, connection: &mut PgConnection, account: Uuid) -> AccountState {
        match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account)
            .fetch_optional(connection)
            .await
        {
            Ok(Some(status)) if status == "active" => AccountState::Active,
            Ok(_) => AccountState::Inactive,
            Err(_) => AccountState::Unknown,
        }
    }
}
impl TransferBlockCheck for Accounts {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Unknown
    }
}
impl AuditHook for Accounts {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        false
    }
}
pub(super) async fn balance(pool: &sqlx::PgPool, account: Uuid) -> Option<i64> {
    let adapter = Accounts(pool.clone());
    let ledger = oteryn_wallet::Ledger::new(
        pool.clone(),
        oteryn_wallet::WalletFlags {
            enabled: std::env::var("OTERYN_ACCOUNT_BALANCE_ENABLED").as_deref() == Ok("true"),
        },
        adapter.clone(),
        adapter.clone(),
        adapter,
    );
    ledger
        .balance(account)
        .await
        .ok()
        .map(|balance| balance.available)
}
