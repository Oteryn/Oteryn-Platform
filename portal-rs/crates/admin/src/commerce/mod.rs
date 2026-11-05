//! Staff commerce orchestration. Domain effects and audit commit together.
//!
//! Sources: premium snapshot contract §2.5, Rust plan §5.4, Laravel
//! AdminPremiumTimeController, AdjustWalletBalance and ResolvePaymentReconciliation.
//! Authentication and transactional account authority are trusted adapters. The
//! configured surface keeps the pending authentication adapters from the staff shell.
mod effects;
mod http;
pub use effects::*;
pub use http::{configured_router, router};

use crate::{audit, rbac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::future::Future;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub premium: bool,
    pub wallet: bool,
    pub payments: bool,
    pub refunds: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Grant {
        days: u16,
    },
    Revoke,
    Adjust {
        amount: i64,
    },
    /// Review is deliberately incapable of changing payment state.
    Reconcile,
    /// Only full refunds are supported by the current payment provider contract.
    Refund,
}
impl Command {
    pub fn permission(&self) -> rbac::Permission {
        match self {
            Self::Grant { .. } | Self::Revoke => rbac::Permission::ManagePremiumTime,
            Self::Adjust { .. } => rbac::Permission::ManageMarketplace,
            Self::Reconcile | Self::Refund => rbac::Permission::ReconcilePayments,
        }
    }
    fn approval(&self) -> bool {
        // No approved threshold exists yet: every correction needs four eyes.
        matches!(self, Self::Adjust { .. } | Self::Refund)
    }
    fn validate(&self, flags: Flags) -> Result<(), Error> {
        let enabled = match self {
            Self::Grant { days } => {
                if !(1..=366).contains(days) {
                    return Err(Error::Invalid);
                }
                flags.premium
            }
            Self::Revoke => flags.premium,
            Self::Adjust { amount } => {
                if *amount == 0 || !(-1_000_000_000..=1_000_000_000).contains(amount) {
                    return Err(Error::Invalid);
                }
                flags.wallet
            }
            Self::Reconcile => flags.payments,
            Self::Refund => flags.payments && flags.refunds,
        };
        if flags.enabled && enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub request_id: Uuid,
    /// Immutable account id, or payment intent id for payment operations.
    pub target: Uuid,
    pub reason: String,
    pub command: Command,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub id: Uuid,
    pub pending: bool,
    pub replayed: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("commerce disabled")]
    Disabled,
    #[error("invalid commerce input")]
    Invalid,
    #[error("commerce access denied")]
    Denied,
    #[error("commerce request conflicts with existing state")]
    Conflict,
    #[error("commerce dependency unavailable")]
    Unavailable,
    #[error("commerce storage unavailable")]
    Storage(#[from] sqlx::Error),
}
impl From<rbac::Error> for Error {
    fn from(error: rbac::Error) -> Self {
        match error {
            rbac::Error::Storage(e) => Self::Storage(e),
            _ => Self::Denied,
        }
    }
}
/// Implementations validate target authority in this transaction. No external
/// effects may occur here; use the transactional outbox for provider requests.
pub trait Effects: Send + Sync {
    /// Read current target authority before accepting a pending request.
    fn inspect(&self, request: &Request) -> impl Future<Output = Result<Value, Error>> + Send;

    fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        request: &Request,
        reason_hash: &str,
    ) -> impl Future<Output = Result<Value, Error>> + Send;
}
#[derive(Debug)]
pub struct Commerce<A, E> {
    pub(crate) pool: PgPool,
    pub(crate) auth: rbac::Authorizer<A>,
    pub(crate) effects: E,
    pub(crate) flags: Flags,
}
impl<A: rbac::AccountStateReader, E: Effects> Commerce<A, E> {
    pub fn new(pool: PgPool, accounts: A, effects: E, flags: Flags) -> Self {
        Self {
            auth: rbac::Authorizer::new(pool.clone(), accounts),
            pool,
            effects,
            flags,
        }
    }
    async fn authorize(&self, context: rbac::CheckContext, command: &Command) -> Result<(), Error> {
        // Caller-controlled relaxation of the MFA requirement is never honored.
        if !context.mfa_verified || !valid_id(context.account) {
            return Err(Error::Denied);
        }
        let context = rbac::CheckContext {
            require_staff_mfa: true,
            ..context
        };
        for permission in [rbac::Permission::Access, command.permission()] {
            self.auth.check(context, permission).await?;
        }
        Ok(())
    }
    pub async fn submit(
        &self,
        context: rbac::CheckContext,
        mut request: Request,
    ) -> Result<Outcome, Error> {
        request.command.validate(self.flags)?;
        if !valid_id(request.request_id) || !valid_id(request.target) {
            return Err(Error::Invalid);
        }
        request.reason = request.reason.trim().to_owned();
        if !(10..=500).contains(&request.reason.chars().count()) {
            return Err(Error::Invalid);
        }
        let hash = format!("{:x}", Sha256::digest(request.reason.as_bytes()));
        let command = serde_json::to_value(&request.command).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await?;
        // Serialize with T10/T10b role writers. Check both staff permissions under
        // the lock, including on idempotent replays and approval requests.
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        if let Err(error) = self.authorize(context, &request.command).await {
            if matches!(error, Error::Denied) {
                record(
                    &mut tx,
                    context.account,
                    &request,
                    "commerce.denied",
                    json!(null),
                    json!({"allowed":false}),
                )
                .await?;
                tx.commit().await?;
            }
            return Err(error);
        }
        let inserted = sqlx::query("INSERT INTO commerce_operations(id,actor_id,target_id,command,reason_sha256,requires_approval) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(request.request_id).bind(context.account).bind(request.target).bind(&command).bind(&hash).bind(request.command.approval()).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            let row = sqlx::query("SELECT actor_id,target_id,command,reason_sha256,result FROM commerce_operations WHERE id=$1 FOR UPDATE")
                .bind(request.request_id).fetch_one(&mut *tx).await?;
            if row.try_get::<Uuid, _>("actor_id")? != context.account
                || row.try_get::<Uuid, _>("target_id")? != request.target
                || row.try_get::<Value, _>("command")? != command
                || row.try_get::<String, _>("reason_sha256")? != hash
            {
                return Err(Error::Conflict);
            }
            let pending = row.try_get::<Option<Value>, _>("result")?.is_none();
            tx.commit().await?;
            return Ok(Outcome {
                id: request.request_id,
                pending,
                replayed: true,
            });
        }
        let review = self.effects.inspect(&request).await?;
        sqlx::query("UPDATE commerce_operations SET review=$2 WHERE id=$1")
            .bind(request.request_id)
            .bind(&review)
            .execute(&mut *tx)
            .await?;
        let pending = request.command.approval();
        let result = if !pending {
            let result = self
                .effects
                .apply(&mut tx, context.account, &request, &hash)
                .await?;
            complete(&mut tx, request.request_id, None, result.clone()).await?;
            result
        } else {
            json!({"pending":true})
        };
        record(
            &mut tx,
            context.account,
            &request,
            "commerce.requested",
            review,
            json!({"pending":pending,"reason_sha256":hash,"result":result}),
        )
        .await?;
        tx.commit().await?;
        Ok(Outcome {
            id: request.request_id,
            pending,
            replayed: false,
        })
    }
    pub async fn approve(&self, context: rbac::CheckContext, id: Uuid) -> Result<Outcome, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        if !valid_id(id) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query("SELECT actor_id,target_id,command,reason_sha256,requires_approval,approved_by,result FROM commerce_operations WHERE id=$1 FOR UPDATE")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::Invalid)?;
        let actor: Uuid = row.try_get("actor_id")?;
        let command: Command =
            serde_json::from_value(row.try_get("command")?).map_err(|_| Error::Unavailable)?;
        command.validate(self.flags)?;
        self.authorize(context, &command).await?;
        if actor == context.account || !row.try_get::<bool, _>("requires_approval")? {
            return Err(Error::Denied);
        }
        if row.try_get::<Option<Value>, _>("result")?.is_some() {
            if row.try_get::<Option<Uuid>, _>("approved_by")? != Some(context.account) {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(Outcome {
                id,
                pending: false,
                replayed: true,
            });
        }
        // The original actor must still be active and hold the permission. The
        // second actor supplies fresh MFA; the original MFA was verified at submit.
        self.authorize(
            rbac::CheckContext {
                account: actor,
                require_staff_mfa: true,
                mfa_verified: true,
            },
            &command,
        )
        .await?;
        let request = Request {
            request_id: id,
            target: row.try_get("target_id")?,
            reason: String::new(),
            command,
        };
        let hash: String = row.try_get("reason_sha256")?;
        let result = self.effects.apply(&mut tx, actor, &request, &hash).await?;
        complete(&mut tx, id, Some(context.account), result.clone()).await?;
        record(
            &mut tx,
            context.account,
            &request,
            "commerce.approved",
            json!({"pending":true}),
            json!({"requested_by":actor,"reason_sha256":hash,"pending":false,"result":result}),
        )
        .await?;
        tx.commit().await?;
        Ok(Outcome {
            id,
            pending: false,
            replayed: false,
        })
    }
}
async fn complete(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    approver: Option<Uuid>,
    result: Value,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE commerce_operations SET approved_by=$2,result=$3,updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .bind(approver)
    .bind(result)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    request: &Request,
    action: &str,
    before: Value,
    after: Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        audit::Entry {
            context: audit::AuditContext {
                actor: Some(actor),
                request_id: request.request_id,
            },
            action,
            target: &request.target.to_string(),
            before,
            after: json!({"command":request.command,"details":after}),
        },
    )
    .await?;
    Ok(())
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
#[cfg(test)]
mod tests;
