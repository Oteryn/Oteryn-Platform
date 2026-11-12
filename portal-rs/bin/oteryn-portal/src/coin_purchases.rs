//! Composition of native coin purchases (sandbox, OTERYN_COIN_PURCHASE_V1):
//! `game:account-services` OAuth tokens name the payer, the LCFA projection
//! checks character ownership (as `GET /api/v1/game-auth/native-characters`
//! does) and the wallet ledger credits transferable coins through the top-up
//! adjustment. Off unless `PORTAL_COIN_PURCHASES_ENABLED=true` with `OTERYN_ENV`
//! testing or preprod; the sandbox HMAC key is generated per process.
use axum::Router;
use oteryn_characters::{lcfa, overview};
use oteryn_gameauth::oauth;
use oteryn_payments::{
    TestProvider,
    coin::CoinSettings,
    coin_http,
    coin_store::{CoinError, CoinHost, CoinPurchases, HostFuture, Ownership, WalletSnapshot},
};
use oteryn_wallet::{AdjustmentKind, Ledger, WalletFlags};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
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
            Arc::new(CoinPurchases::new(
                pool.clone(),
                settings,
                provider,
                Arc::new(host),
            ))
        })
    } else {
        None
    };
    coin_http::router(pool, service)
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
    fn eligible_in<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        payer: Uuid,
    ) -> HostFuture<'a, Result<bool, CoinError>> {
        Box::pin(async move {
            // Serializes restriction/closure against order creation and delivery.
            let eligible: Option<bool> = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE")
                .bind(payer).fetch_optional(connection).await?;
            Ok(eligible == Some(true))
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
                transferable: balance.transferable,
            })
        })
    }
}

#[cfg(test)]
#[path = "coin_purchases_tests.rs"]
mod tests;
