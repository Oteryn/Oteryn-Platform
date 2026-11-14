//! Composition of native coin purchases (sandbox, OTERYN_COIN_PURCHASE_V1):
//! `game:account-services` OAuth tokens name the payer, the LCFA projection
//! checks character ownership (as `GET /api/v1/game-auth/native-characters`
//! does) and the wallet ledger credits the one coin balance through the top-up
//! adjustment; a refund or chargeback debits it back through a keyed reversal
//! adjustment that never goes below zero. Off unless
//! `PORTAL_COIN_PURCHASES_ENABLED=true` with `OTERYN_ENV` testing or preprod;
//! the sandbox HMAC key is generated per process.
use axum::Router;
use oteryn_characters::{lcfa, overview};
use oteryn_gameauth::oauth;
use oteryn_payments::{
    TestProvider,
    coin::CoinSettings,
    coin_http,
    coin_store::{
        CoinError, CoinHost, CoinPurchases, Debit, HostFuture, Ownership, WalletSnapshot,
    },
    provider_webhook,
};
use oteryn_wallet::{self as wallet, AdjustmentKind, Ledger, Refusal, WalletFlags};
use serde_json::json;
use sqlx::{Acquire, PgConnection, PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

use crate::wallet_topup::Policy;

pub fn configured_router(pool: PgPool) -> Router {
    let settings = CoinSettings::from_lookup(|key| std::env::var(key).ok());
    let host = Host {
        pool: pool.clone(),
        oauth: oauth::Config::from_env(),
        projection: lcfa::Projection::new(
            pool.clone(),
            overview::configured_projection(),
            Arc::new(overview::Accounts(pool.clone())),
        ),
    };
    let service = if settings.enabled {
        sandbox_provider().map(|provider| {
            let coins = Arc::new(CoinPurchases::new(
                pool.clone(),
                settings,
                provider.clone(),
                Arc::new(host),
            ));
            (coins, provider)
        })
    } else {
        None
    };
    let webhook = provider_webhook::configured_router(pool.clone(), service.clone());
    coin_http::router(pool, service.map(|(coins, _)| coins)).merge(webhook)
}

/// A fresh random signing key; only this process can sign sandbox events.
fn sandbox_provider() -> Option<TestProvider> {
    let mut key = [0_u8; 32];
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut key)
        .ok()?;
    TestProvider::new(&key).ok()
}

#[derive(Debug, Clone)]
pub(crate) struct Host {
    pub(crate) pool: PgPool,
    pub(crate) oauth: oauth::Config,
    pub(crate) projection: lcfa::Projection,
}
impl Host {
    fn ledger(&self) -> Ledger<Policy, Policy, Policy> {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            Policy,
            Policy,
            Policy,
        )
    }
}
impl CoinHost for Host {
    fn payer<'a>(&'a self, bearer: &'a str) -> HostFuture<'a, Result<Uuid, CoinError>> {
        Box::pin(async move {
            match oauth::verify_account_services(&self.pool, &self.oauth, bearer).await {
                Ok(account) => Ok(account),
                Err(oauth::Error::Unavailable | oauth::Error::Disabled) => {
                    Err(CoinError::Unavailable)
                }
                Err(_) => Err(CoinError::Unauthorized),
            }
        })
    }
    fn ownership(&self, payer: Uuid, character: Uuid) -> HostFuture<'_, Ownership> {
        Box::pin(async move {
            let Ok(account) = lcfa::AccountId::parse(&payer.to_string()) else {
                return Ownership::Unavailable;
            };
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            match self.projection.list_characters(account, now).await {
                Ok(view) => match view.state {
                    lcfa::ReadState::Ready
                        if view
                            .characters
                            .iter()
                            .any(|c| c.character_id.as_uuid() == character) =>
                    {
                        Ownership::Owned
                    }
                    lcfa::ReadState::Ready | lcfa::ReadState::Missing | lcfa::ReadState::Denied => {
                        Ownership::NotOwned
                    }
                    _ => Ownership::Unavailable,
                },
                Err(_) => Ownership::Unavailable,
            }
        })
    }
    fn restriction_in<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        payer: Uuid,
    ) -> HostFuture<'a, Result<Option<String>, CoinError>> {
        Box::pin(async move {
            // Serializes restriction/closure against order creation and delivery.
            // Every restriction state the identity domain has: account status,
            // unverified email, the email-change security lock, a pending account
            // termination (erasure) and an effective restriction or suspension
            // sanction. A missing account is restricted.
            let reason: Option<Option<String>> = sqlx::query_scalar(
                "SELECT CASE \
                   WHEN a.status<>'active' THEN 'account_'||a.status \
                   WHEN a.email_verified_at IS NULL THEN 'email_unverified' \
                   WHEN a.email_change_blocked_until>clock_timestamp() THEN 'email_change_lock' \
                   WHEN EXISTS(SELECT 1 FROM account_terminations t WHERE t.account_id=a.id \
                     AND t.cancelled_at IS NULL AND t.finalized_at IS NULL) THEN 'termination_pending' \
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.status='active' \
                     AND s.category IN ('restriction','suspension') AND s.effective_at<=clock_timestamp() \
                     AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())) THEN 'sanction' \
                 END FROM accounts a WHERE a.id=$1 FOR SHARE OF a",
            )
            .bind(payer)
            .fetch_optional(connection)
            .await?;
            Ok(reason.unwrap_or_else(|| Some("account_missing".to_owned())))
        })
    }
    fn credit_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<(), CoinError>> {
        Box::pin(async move {
            // The intent ID is the global ledger key: a replay never credits twice.
            self.ledger()
                .with_adjustment_kind(AdjustmentKind::TopUp)
                .adjust_in(
                    tx,
                    key,
                    payer,
                    coins,
                    payer,
                    &format!("wallet-top-up:{key}"),
                )
                .await
                .map(|_| ())
                .map_err(|_| CoinError::Unavailable)
        })
    }
    fn reverse_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<Debit, CoinError>> {
        Box::pin(async move {
            if coins <= 0 {
                return Err(CoinError::Invalid);
            }
            // A savepoint: an insufficient balance rolls back only this attempt
            // (its operation claim must not survive), so the caller can hold
            // the order instead of failing the verified event.
            let mut attempt = (&mut **tx).begin().await?;
            let debited = Ledger::new(
                self.pool.clone(),
                WalletFlags { enabled: true },
                Reversal,
                Reversal,
                Reversal,
            )
            .adjust_in(
                &mut attempt,
                key,
                payer,
                -coins,
                payer,
                &format!("coin-purchase-reversal:{key}"),
            )
            .await;
            match debited {
                Ok(_) => {
                    attempt.commit().await?;
                    Ok(Debit::Debited)
                }
                Err(wallet::Error::Refused(Refusal::InsufficientFunds)) => {
                    attempt.rollback().await?;
                    Ok(Debit::Insufficient)
                }
                Err(_) => Err(CoinError::Unavailable),
            }
        })
    }
    fn wallet(&self, payer: Uuid) -> HostFuture<'_, Result<WalletSnapshot, CoinError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await?;
            let (balance, revision) = self
                .ledger()
                .snapshot_in(&mut tx, payer)
                .await
                .map_err(|_| CoinError::Unavailable)?;
            tx.commit().await?;
            Ok(WalletSnapshot {
                revision,
                total: balance
                    .available
                    .checked_add(balance.reserved)
                    .ok_or(CoinError::Unavailable)?,
                available: balance.available,
            })
        })
    }
}

/// Wallet policy of a purchase reversal only. A debit that returns paid coins
/// applies whatever the payer's restrictions (a banned or closing account still
/// owes them) and takes no account lock; a missing account is refused. The
/// audit hook accepts only the reversal shape and appends its balances.
#[derive(Debug, Clone, Copy)]
struct Reversal;
impl wallet::AccountStateReader for Reversal {
    async fn state(&self, conn: &mut PgConnection, account: Uuid) -> wallet::AccountState {
        match sqlx::query_scalar::<_, bool>("SELECT true FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(conn)
            .await
        {
            Ok(Some(true)) => wallet::AccountState::Active,
            _ => wallet::AccountState::Unknown,
        }
    }
}
impl wallet::TransferBlockCheck for Reversal {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for Reversal {
    async fn adjustment(&self, conn: &mut PgConnection, event: &wallet::AdjustmentAudit) -> bool {
        if event.amount >= 0
            || event.actor_id != event.account_id
            || event.reason != format!("coin-purchase-reversal:{}", event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'wallet.coin_purchase_reversal.debited',$1,$2,$3,$4)")
            .bind(event.account_id.to_string())
            .bind(json!({"available":event.before.available,"reserved":event.before.reserved}))
            .bind(json!({"available":event.after.available,"reserved":event.after.reserved,"coins":event.amount}))
            .bind(event.operation_id).execute(conn).await.is_ok()
    }
}

#[cfg(test)]
#[path = "coin_purchases_tests.rs"]
mod tests;
