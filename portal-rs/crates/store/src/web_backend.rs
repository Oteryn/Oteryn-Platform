//! Explicit composition seams keep browser authorization separate from internal mTLS.
use std::sync::Arc;

use oteryn_wallet::{AccountStateReader, AuditHook, Ledger, TransferBlockCheck};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Error, FutureResult, Package, PurchaseForm};
use crate::{
    catalog::{Catalog, CatalogFlags, CatalogStore},
    purchase,
};

/// Implementations must resolve current account restrictions and ownership. Unknown denies.
pub trait Access: Send + Sync {
    fn active(&self, account: Uuid) -> FutureResult<'_, bool>;
    fn target(&self, account: Uuid, character: Uuid, world: Uuid) -> FutureResult<'_, bool>;
}

/// The caller binds a package to server-side pricing and conversion policy. Creation
/// must deduplicate the operation key and must never credit coins from a browser return.
pub trait TopUps: Send + Sync {
    fn test_checkout(&self) -> bool {
        false
    }
    fn checkout(
        &self,
        _account: Uuid,
        _intent: Uuid,
        _action: Option<String>,
    ) -> FutureResult<'_, Option<super::Checkout>> {
        Box::pin(async { Err(Error::Unavailable) })
    }
    fn packages(&self) -> Vec<Package>;
    fn create(&self, account: Uuid, operation: Uuid, package: String) -> FutureResult<'_, Uuid>;
}

/// Purchase adapter uses the public T29 API, preserving its transactional decisions.
pub trait Buy: Send + Sync {
    fn buy(&self, bytes: Vec<u8>) -> FutureResult<'_, Vec<u8>>;
    fn authority(&self) -> &str;
}
impl<A, B, H, C> Buy for purchase::Purchases<A, B, H, C>
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
    C: purchase::Accounts + 'static,
{
    fn buy(&self, bytes: Vec<u8>) -> FutureResult<'_, Vec<u8>> {
        Box::pin(async move { self.purchase(&bytes).await.map_err(Error::from) })
    }
    fn authority(&self) -> &str {
        &self.settings().source_authority
    }
}

/// Object-safe boundary for HTTP tests and composition without cyclic crate dependencies.
pub trait Backend: Send + Sync {
    fn active(&self, account: Uuid) -> FutureResult<'_, bool>;
    fn catalog(&self) -> FutureResult<'_, Option<Catalog>>;
    fn balance(&self, account: Uuid) -> FutureResult<'_, oteryn_wallet::Balance>;
    fn history(
        &self,
        account: Uuid,
        before: Option<Uuid>,
    ) -> FutureResult<'_, Vec<oteryn_wallet::HistoryEntry>>;
    fn transfer(
        &self,
        account: Uuid,
        operation: Uuid,
        recipient: Uuid,
        amount: i64,
    ) -> FutureResult<'_, Uuid>;
    fn purchase(&self, account: Uuid, form: PurchaseForm) -> FutureResult<'_, ()>;
    fn test_checkout(&self) -> bool {
        false
    }
    fn checkout(
        &self,
        _account: Uuid,
        _intent: Uuid,
        _action: Option<String>,
    ) -> FutureResult<'_, Option<super::Checkout>> {
        Box::pin(async { Err(Error::Unavailable) })
    }
    fn packages(&self) -> Vec<Package>;
    fn top_up(&self, account: Uuid, operation: Uuid, package: String) -> FutureResult<'_, Uuid>;
}

pub struct Domain<A, B, H> {
    pub wallet: Ledger<A, B, H>,
    pub catalog: CatalogStore,
    pub access: Arc<dyn Access>,
    pub purchases: Option<Arc<dyn Buy>>,
    pub top_ups: Option<Arc<dyn TopUps>>,
}
impl<A, B, H> std::fmt::Debug for Domain<A, B, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Domain").finish_non_exhaustive()
    }
}
impl<A, B, H> Backend for Domain<A, B, H>
where
    A: AccountStateReader + 'static,
    B: TransferBlockCheck + 'static,
    H: AuditHook + 'static,
{
    fn active(&self, account: Uuid) -> FutureResult<'_, bool> {
        self.access.active(account)
    }
    fn catalog(&self) -> FutureResult<'_, Option<Catalog>> {
        Box::pin(async { self.catalog.active().await.map_err(|_| Error::Unavailable) })
    }
    fn balance(&self, account: Uuid) -> FutureResult<'_, oteryn_wallet::Balance> {
        Box::pin(async move { self.wallet.balance(account).await.map_err(Error::from) })
    }
    fn history(
        &self,
        account: Uuid,
        before: Option<Uuid>,
    ) -> FutureResult<'_, Vec<oteryn_wallet::HistoryEntry>> {
        Box::pin(async move {
            self.wallet
                .history(account, oteryn_wallet::HistoryPage { before, limit: 25 })
                .await
                .map_err(Error::from)
        })
    }
    fn transfer(
        &self,
        account: Uuid,
        operation: Uuid,
        recipient: Uuid,
        amount: i64,
    ) -> FutureResult<'_, Uuid> {
        Box::pin(async move {
            self.wallet
                .transfer(operation, account, recipient, amount)
                .await
                .map(|r| r.operation_id)
                .map_err(Error::from)
        })
    }
    fn purchase(&self, account: Uuid, form: PurchaseForm) -> FutureResult<'_, ()> {
        Box::pin(async move {
            if !self
                .access
                .target(account, form.character, form.world)
                .await?
            {
                return Err(Error::Denied);
            }
            let purchases = self.purchases.as_ref().ok_or(Error::Unavailable)?;
            let request = purchase::PurchaseRequest {
                contract_version: 1,
                operation: "PurchaseStoreOfferV1".into(),
                source_authority: purchases.authority().into(),
                purchase_request_id: form.operation.to_string(),
                account_id: account.to_string(),
                character_id: form.character.to_string(),
                world_id: form.world.to_string(),
                catalog_revision: form.revision,
                offer_key: form.offer,
                currency: "COINS".into(),
                unit_price: form.price,
                quantity: form.quantity,
            };
            // Struct member order matches the canonical T29 request contract.
            let bytes = serde_json::to_vec(&request).map_err(|_| Error::Invalid)?;
            purchase::PurchaseRequest::decode(&bytes).map_err(Error::from)?;
            let bytes = purchases.buy(bytes).await?;
            let result: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
            match result.get("result").and_then(|v| v.as_str()) {
                Some("ACCEPTED") => Ok(()),
                Some("STALE_QUOTE") => Err(Error::Conflict),
                Some(_) => Err(Error::Denied),
                None => Err(Error::Unavailable),
            }
        })
    }
    fn test_checkout(&self) -> bool {
        self.top_ups.as_ref().is_some_and(|t| t.test_checkout())
    }
    fn checkout(
        &self,
        account: Uuid,
        intent: Uuid,
        action: Option<String>,
    ) -> FutureResult<'_, Option<super::Checkout>> {
        Box::pin(async move {
            self.top_ups
                .as_ref()
                .ok_or(Error::Unavailable)?
                .checkout(account, intent, action)
                .await
        })
    }
    fn packages(&self) -> Vec<Package> {
        self.top_ups
            .as_ref()
            .map_or_else(Vec::new, |p| p.packages())
    }
    fn top_up(&self, account: Uuid, operation: Uuid, package: String) -> FutureResult<'_, Uuid> {
        Box::pin(async move {
            self.top_ups
                .as_ref()
                .ok_or(Error::Unavailable)?
                .create(account, operation, package)
                .await
        })
    }
}

/// Current identity state and D14 restrictions are read under the ledger transaction.
#[derive(Clone)]
struct ReadAccounts(PgPool);
impl AccountStateReader for ReadAccounts {
    async fn state(
        &self,
        conn: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> oteryn_wallet::AccountState {
        match sqlx::query_scalar::<_, bool>("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE").bind(account).fetch_optional(conn).await {
            Ok(Some(true)) => oteryn_wallet::AccountState::Active,
            _ => oteryn_wallet::AccountState::Unknown,
        }
    }
}
impl TransferBlockCheck for ReadAccounts {
    async fn permission(
        &self,
        conn: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> oteryn_wallet::TransferPermission {
        match oteryn_identity::email_change::trade_allowed(conn, account).await {
            Ok(true) => oteryn_wallet::TransferPermission::Allowed,
            Ok(false) => oteryn_wallet::TransferPermission::Blocked,
            Err(_) => oteryn_wallet::TransferPermission::Unknown,
        }
    }
}
impl AuditHook for ReadAccounts {
    async fn adjustment(
        &self,
        _: &mut sqlx::PgConnection,
        _: &oteryn_wallet::AdjustmentAudit,
    ) -> bool {
        false
    }
}
impl Access for ReadAccounts {
    fn active(&self, account: Uuid) -> FutureResult<'_, bool> {
        Box::pin(active(&self.0, account))
    }
    fn target(&self, _: Uuid, _: Uuid, _: Uuid) -> FutureResult<'_, bool> {
        Box::pin(async { Ok(false) })
    }
}
async fn active(pool: &PgPool, account: Uuid) -> Result<bool, Error> {
    oteryn_identity::accounts::Accounts { pool: pool.clone() }
        .find_by_id(account)
        .await
        .map(|a| a.is_some_and(|a| a.status == "active" && a.email_verified_at.is_some()))
        .map_err(|_| Error::Unavailable)
}
pub fn configured(pool: PgPool, top_ups: Option<Arc<dyn TopUps>>) -> Arc<dyn Backend> {
    let accounts = ReadAccounts(pool.clone());
    Arc::new(Domain {
        wallet: Ledger::new(
            pool.clone(),
            oteryn_wallet::WalletFlags { enabled: true },
            accounts.clone(),
            accounts.clone(),
            accounts.clone(),
        ),
        catalog: CatalogStore::new(pool, CatalogFlags { enabled: true }),
        access: Arc::new(accounts),
        purchases: None,
        top_ups,
    })
}
