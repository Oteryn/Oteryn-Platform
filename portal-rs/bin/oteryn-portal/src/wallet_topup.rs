//! Checkout-bound paid coin delivery. No conversion is inferred from money units.
use crate::worker::{Handler, HandlerFuture, Registry};
use oteryn_db::outbox::{Job, PgPool};
use oteryn_payments::TopUpJob;
use oteryn_wallet::{self as wallet, AdjustmentKind, Ledger, WalletFlags};
use serde_json::json;
use sqlx::{PgConnection, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Delivery {
    pub pool: PgPool,
    pub payments_enabled: bool,
    pub wallet_enabled: bool,
}
pub fn register(registry: &mut Registry, pool: PgPool) -> Result<(), String> {
    registry.register(
        "wallet.top_up",
        Arc::new(Delivery {
            pool,
            payments_enabled: std::env::var("OTERYN_PAYMENTS_ENABLED").as_deref() == Ok("true"),
            wallet_enabled: std::env::var("OTERYN_WALLET_ENABLED").as_deref() == Ok("true"),
        }),
    )
}
impl Handler for Delivery {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            match self.credit(job).await {
                Ok(()) => Ok(()),
                Err(reason) => {
                    // The credit transaction has rolled back. Denials must survive
                    // that rollback; retry/dead-letter remains owned by the worker.
                    let mut tx = self
                        .pool
                        .begin()
                        .await
                        .map_err(|_| "top-up denial audit unavailable")?;
                    oteryn_admin::audit::append(
                        &mut tx,
                        oteryn_admin::audit::Entry {
                            context: oteryn_admin::audit::AuditContext {
                                actor: None,
                                request_id: job.id,
                            },
                            action: "wallet.top_up.denied",
                            target: &job.id.to_string(),
                            before: serde_json::Value::Null,
                            after: json!({"reason":reason}),
                        },
                    )
                    .await
                    .map_err(|_| "top-up denial audit unavailable")?;
                    tx.commit()
                        .await
                        .map_err(|_| "top-up denial audit unavailable")?;
                    Err(reason.into())
                }
            }
        })
    }
}
impl Delivery {
    async fn credit(&self, job: &Job) -> Result<(), &'static str> {
        if job.kind != "wallet.top_up" || job.payload.len() > 1024 {
            return Err("invalid top-up job");
        }
        let payload: TopUpJob =
            serde_json::from_str(&job.payload).map_err(|_| "invalid top-up job")?;
        if payload.intent_id.is_nil()
            || payload.intent_id.get_version_num() != 7
            || payload.account_id.is_nil()
            || payload.amount <= 0
        {
            return Err("invalid top-up job");
        }
        if !self.payments_enabled || !self.wallet_enabled {
            return Err("top-up features disabled");
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| "top-up storage unavailable")?;
        // Lock the intent first, as webhook processing does. A refund that wins
        // this lock prevents later delivery; a credit never rewrites refunds.
        let intent = sqlx::query(
            "SELECT account_id,amount,currency,state FROM payment_intents WHERE id=$1 FOR UPDATE",
        )
        .bind(payload.intent_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| "top-up intent unavailable")?
        .ok_or("top-up intent missing")?;
        let topup = sqlx::query("SELECT account_id,coins,minor_units,currency FROM store_web_topups WHERE payment_intent_id=$1 FOR SHARE")
            .bind(payload.intent_id).fetch_optional(&mut *tx).await.map_err(|_| "top-up checkout unavailable")?.ok_or("top-up checkout missing")?;
        let matches = intent.try_get::<Uuid, _>("account_id").ok() == Some(payload.account_id)
            && topup.try_get::<Uuid, _>("account_id").ok() == Some(payload.account_id)
            && intent.try_get::<i64, _>("amount").ok() == Some(payload.amount)
            && topup.try_get::<i64, _>("minor_units").ok() == Some(payload.amount)
            && intent.try_get::<String, _>("currency").ok().as_deref()
                == Some(payload.currency.as_str())
            && topup.try_get::<String, _>("currency").ok().as_deref()
                == Some(payload.currency.as_str())
            && intent.try_get::<String, _>("state").ok().as_deref() == Some("succeeded");
        if !matches {
            return Err("top-up checkout/intent/job mismatch");
        }
        let coins: i64 = topup.try_get("coins").map_err(|_| "invalid top-up coins")?;
        if coins <= 0 {
            return Err("invalid top-up coins");
        }
        let ledger = Ledger::new(
            self.pool.clone(),
            WalletFlags {
                enabled: self.wallet_enabled,
            },
            Policy,
            Policy,
            Policy,
        )
        .with_adjustment_kind(AdjustmentKind::TopUp);
        // Intent IDs are UUIDv7 and the global ledger key. Replays (even with
        // different outbox IDs) claim the identical payload under wallet locks.
        ledger
            .adjust_in(
                &mut tx,
                payload.intent_id,
                payload.account_id,
                coins,
                payload.account_id,
                &format!("wallet-top-up:{}", payload.intent_id),
            )
            .await
            .map_err(|_| "top-up wallet refused")?;
        tx.commit().await.map_err(|_| "top-up commit unavailable")?;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
struct Policy;
impl wallet::AccountStateReader for Policy {
    async fn state(&self, conn: &mut PgConnection, account: Uuid) -> wallet::AccountState {
        // Serialize account restriction/closure against delivery. Match payments'
        // active + verified policy; database errors and missing rows deny.
        match sqlx::query_scalar::<_,bool>("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account).fetch_optional(conn).await {
            Ok(Some(true)) => wallet::AccountState::Active,
            _ => wallet::AccountState::Unknown,
        }
    }
}
impl wallet::TransferBlockCheck for Policy {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for Policy {
    async fn adjustment(&self, conn: &mut PgConnection, event: &wallet::AdjustmentAudit) -> bool {
        if event.amount <= 0
            || event.actor_id != event.account_id
            || event.reason != format!("wallet-top-up:{}", event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'wallet.top_up.credited',$1,$2,$3,$4)")
            .bind(event.operation_id.to_string())
            .bind(json!({"available":event.before.available,"reserved":event.before.reserved}))
            .bind(json!({"available":event.after.available,"reserved":event.after.reserved,"coins":event.amount}))
            .bind(event.operation_id).execute(conn).await.is_ok()
    }
}
