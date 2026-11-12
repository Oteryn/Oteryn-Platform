//! Production adapters of the Store purchase route: the buyer gate over the
//! `accounts` row and the Wallet policy for the buyer's own purchase debit.
use super::{AccountState, Accounts, Error, Purchases, Settings};
use oteryn_wallet::{
    AccountStateReader, AdjustmentAudit, AuditHook, Ledger, TransferBlockCheck, TransferPermission,
    WalletFlags,
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// The purchase service over the production adapters.
pub type DatabasePurchases =
    Purchases<DatabaseAccounts, DatabaseAccounts, DatabaseAccounts, DatabaseAccounts>;

/// Buyable means `active`, email verified, outside the email-change cooldown
/// (`email_change_blocked_until`) and barred by nothing the support access
/// guard (`oteryn_support` `require_game_access`) refuses: a sanction of an
/// unknown category or status, an effective `suspension`, or an effective
/// typed ban (latest revision active and unexpired); an effective `restriction`
/// bars buying too.
///
/// Lock order: when the referral campaign is installed, the purchase first
/// calls `referral_campaign_prepare`, which locks the buyer and inviter rows
/// `FOR UPDATE` in id order; this gate's `FOR SHARE` then adds nothing. Without
/// it this gate takes the first lock on the buyer row, `FOR SHARE`. Either lock
/// is held until the purchase transaction ends, so a writer that locks the
/// account row (closure, suspension, ban) cannot interleave.
#[derive(Debug, Clone, Copy)]
pub struct DatabaseAccounts;

const BUYABLE: &str = "SELECT a.status='active' AND a.email_verified_at IS NOT NULL AND a.email_change_blocked_until<=clock_timestamp() \
AND NOT EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND (s.category NOT IN ('warning','restriction','suspension') OR s.status NOT IN ('pending','active','expired','revoked') \
OR (s.category IN ('restriction','suspension') AND s.status='active' AND s.effective_at<=clock_timestamp() AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())))) \
AND NOT EXISTS(SELECT 1 FROM (SELECT DISTINCT ON (r.sanction_id) r.active,r.expires_at FROM support_sanction_revisions r JOIN support_sanction_identities i USING(sanction_id) \
WHERE r.account_id=a.id AND i.kind='ban' ORDER BY r.sanction_id,r.revision DESC) latest WHERE latest.active AND (latest.expires_at IS NULL OR latest.expires_at>clock_timestamp())) \
FROM accounts a WHERE a.id=$1 FOR SHARE OF a";

async fn buyable(connection: &mut PgConnection, account: Uuid) -> Result<Option<bool>, Error> {
    Ok(sqlx::query_scalar::<_, bool>(BUYABLE)
        .bind(account)
        .fetch_optional(connection)
        .await?)
}

impl Accounts for DatabaseAccounts {
    async fn state(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> Result<AccountState, Error> {
        Ok(match buyable(connection, account).await? {
            Some(true) => AccountState::Active,
            Some(false) => AccountState::Restricted,
            None => AccountState::Missing,
        })
    }
}

impl AccountStateReader for DatabaseAccounts {
    async fn state(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> oteryn_wallet::AccountState {
        match buyable(connection, account).await {
            Ok(Some(true)) => oteryn_wallet::AccountState::Active,
            Ok(Some(false)) => oteryn_wallet::AccountState::Inactive,
            Ok(None) => oteryn_wallet::AccountState::Unknown,
            Err(_) => oteryn_wallet::AccountState::Unavailable,
        }
    }
}

/// The purchase route never transfers coins between accounts.
impl TransferBlockCheck for DatabaseAccounts {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Unknown
    }
}

/// Authorizes only the buyer's own purchase debit and audits it in the same
/// transaction; any other adjustment is denied.
impl AuditHook for DatabaseAccounts {
    async fn adjustment(&self, connection: &mut PgConnection, event: &AdjustmentAudit) -> bool {
        if event.amount >= 0
            || event.actor_id != event.account_id
            || event.reason != format!("store-purchase:{}", event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'store.purchase.debited',$1,$2,$3,$4)")
            .bind(event.operation_id.to_string())
            .bind(json!({"available":event.before.available,"reserved":event.before.reserved}))
            .bind(json!({"available":event.after.available,"reserved":event.after.reserved,"coins":event.amount}))
            .bind(event.operation_id)
            .execute(connection)
            .await
            .is_ok()
    }
}

/// The Wallet is on exactly when the purchase surface is valid; the route
/// refuses before the Wallet otherwise.
pub fn database(pool: PgPool, settings: Settings) -> DatabasePurchases {
    let wallet = Ledger::new(
        pool.clone(),
        WalletFlags {
            enabled: settings.valid(),
        },
        DatabaseAccounts,
        DatabaseAccounts,
        DatabaseAccounts,
    );
    Purchases::new(pool, settings, wallet, DatabaseAccounts)
}
