//! Production domain composition. Domain flags are independent of the staff route flag.
use super::*;
use oteryn_store::premium;
use oteryn_wallet as wallet;
use sqlx::PgConnection;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct DomainAuthority;
impl wallet::AccountStateReader for DomainAuthority {
    async fn state(&self, connection: &mut PgConnection, account: Uuid) -> wallet::AccountState {
        match sqlx::query_scalar::<_, bool>("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account).fetch_optional(connection).await {
            Ok(Some(true)) => wallet::AccountState::Active,
            _ => wallet::AccountState::Unknown,
        }
    }
}
impl wallet::TransferBlockCheck for DomainAuthority {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        // Commerce never uses player-to-player transfers.
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for DomainAuthority {
    async fn adjustment(
        &self,
        connection: &mut PgConnection,
        event: &wallet::AdjustmentAudit,
    ) -> bool {
        let allowed = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM accounts a JOIN account_roles ar ON ar.account_id=a.id JOIN role_permissions rp ON rp.role_id=ar.role_id WHERE a.id=$1 AND a.status='active' AND rp.permission='marketplace.manage')")
            .bind(event.actor_id).fetch_one(&mut *connection).await.unwrap_or(false);
        if !allowed {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'commerce.wallet.adjusted',$2,$3,$4,$5)")
            .bind(event.actor_id).bind(event.account_id.to_string())
            .bind(json!({"available":event.before.available,"reserved":event.before.reserved}))
            .bind(json!({"available":event.after.available,"reserved":event.after.reserved,"amount":event.amount}))
            .bind(event.operation_id).execute(connection).await.is_ok()
    }
}
impl premium::Accounts for DomainAuthority {
    fn state<'a>(
        &'a self,
        account: Uuid,
        connection: &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<premium::AccountState, premium::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            Ok(
                match wallet::AccountStateReader::state(self, connection, account).await {
                    wallet::AccountState::Active => premium::AccountState::Known,
                    _ => premium::AccountState::Unknown,
                },
            )
        })
    }
}
impl PaymentActions for Option<TestPaymentAdapter> {
    async fn inspect(
        &self,
        connection: &mut PgConnection,
        request: &Request,
    ) -> Result<Value, Error> {
        self.as_ref()
            .ok_or(Error::Unavailable)?
            .inspect(connection, request)
            .await
    }
    async fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &Request,
    ) -> Result<Value, Error> {
        self.as_ref()
            .ok_or(Error::Unavailable)?
            .apply(tx, request)
            .await
    }
}
pub type ConfiguredEffects =
    LedgerEffects<DomainAuthority, DomainAuthority, DomainAuthority, Option<TestPaymentAdapter>>;

/// Shared by configured HTTP composition and the binary worker. The existing test
/// provider is available only outside production and only with explicit custody.
pub fn configured_payment_adapter(
    pool: PgPool,
) -> Result<Option<TestPaymentAdapter>, Box<dyn std::error::Error>> {
    if std::env::var("OTERYN_PAYMENTS_ENABLED").as_deref() != Ok("true")
        || std::env::var("OTERYN_ENV").as_deref() == Ok("production")
    {
        return Ok(None);
    }
    const KEY_FILE: &str = "OTERYN_COMMERCE_TEST_PROVIDER_KEY_FILE";
    let Some(path) = std::env::var_os(KEY_FILE) else {
        return Ok(None);
    };
    let mut key = zeroize::Zeroizing::new(Vec::new());
    std::io::Read::read_to_end(
        &mut oteryn_core::secret_file::open(KEY_FILE, path)?,
        &mut key,
    )
    .map_err(|_| "could not read OTERYN_COMMERCE_TEST_PROVIDER_KEY_FILE")?;
    let provider = oteryn_payments::TestProvider::new(&key)?;
    Ok(Some(TestPaymentAdapter::new(
        pool,
        provider,
        oteryn_payments::PaymentFlags { enabled: true },
    )))
}
pub fn domain_effects(
    pool: PgPool,
    flags: Flags,
    payments: Option<TestPaymentAdapter>,
) -> ConfiguredEffects {
    LedgerEffects {
        wallet: wallet::Ledger::new(
            pool.clone(),
            wallet::WalletFlags {
                enabled: flags.wallet,
            },
            DomainAuthority,
            DomainAuthority,
            DomainAuthority,
        ),
        premium: premium::repository::Ledger::new(pool, Arc::new(DomainAuthority), flags.premium),
        payments,
    }
}
