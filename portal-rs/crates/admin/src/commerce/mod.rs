//! Staff commerce orchestration. Domain effects and audit commit together.
//!
//! Sources: premium snapshot contract §2.5, Rust plan §5.4, Laravel
//! AdminPremiumTimeController, AdjustWalletBalance and ResolvePaymentReconciliation.
//! Authentication and transactional account authority are trusted adapters. The
//! configured surface uses current staff identity and the owning domain ledgers.
pub mod configured;
mod effects;
mod http;
mod qualification;
mod reconcile;
pub use configured::*;
pub use effects::*;
pub use http::{configured_router, router};
pub use reconcile::*;

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
        // No approved threshold exists yet: every correction and every staff
        // premium grant needs four eyes. Revocation only removes value.
        matches!(
            self,
            Self::Grant { .. } | Self::Adjust { .. } | Self::Refund
        )
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
    fn inspect(
        &self,
        connection: &mut sqlx::PgConnection,
        request: &Request,
    ) -> impl Future<Output = Result<Value, Error>> + Send;

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
        let mut connection = self.pool.acquire().await?;
        self.authorize_in(&mut connection, context, command).await
    }
    async fn authorize_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: rbac::CheckContext,
        command: &Command,
    ) -> Result<(), Error> {
        // Caller-controlled relaxation of the MFA requirement is never honored.
        if !context.mfa_verified || !valid_id(context.account) {
            return Err(Error::Denied);
        }
        let context = rbac::CheckContext {
            require_staff_mfa: true,
            ..context
        };
        for permission in [rbac::Permission::Access, command.permission()] {
            self.auth.check_in(connection, context, permission).await?;
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
        rbac::mutation_budget(&mut tx).await?;
        // Serialize with T10/T10b role writers. Check both staff permissions under
        // the lock, including on idempotent replays and approval requests.
        let owner = payment_owner(&mut tx, &request.command, request.target).await?;
        let operation_id = request.request_id;
        let authorization = rbac::authorize_mutation_with_resources(
            &mut tx,
            &[context.account, owner],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM commerce_operations WHERE id=$1 FOR UPDATE")
                        .bind(operation_id)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            &self.auth,
            context,
            &[rbac::Permission::Access, request.command.permission()],
        )
        .await
        .map_err(Error::from);
        // Preserve the input refusal boundary; a captured positive MFA flag is
        // insufficient without the helper's durable proof validation above.
        // Staff never act on their own account's premium, balance or payments.
        let authorization = authorization.and_then(|()| {
            if context.mfa_verified
                && valid_id(context.account)
                && (owner != context.account || matches!(request.command, Command::Reconcile))
            {
                Ok(())
            } else {
                Err(Error::Denied)
            }
        });
        // A pending grant must not wait on an account that cannot hold value;
        // the approval applies the same check again under its own locks. A
        // replay of a recorded operation (row locked above) reports its
        // recorded state instead of a fresh refusal.
        let authorization = match authorization {
            Ok(())
                if matches!(request.command, Command::Grant { .. })
                    && !recorded(&mut tx, request.request_id).await?
                    && !oteryn_wallet::value_eligible_in(&mut tx, request.target).await? =>
            {
                Err(Error::Denied)
            }
            other => other,
        };
        if let Err(error) = authorization {
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
            self.authorize_in(&mut tx, context, &request.command)
                .await?;
            tx.commit().await?;
            return Ok(Outcome {
                id: request.request_id,
                pending,
                replayed: true,
            });
        }
        let review = self.effects.inspect(&mut tx, &request).await?;
        self.authorize_in(&mut tx, context, &request.command)
            .await?;
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
        self.authorize_in(&mut tx, context, &request.command)
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
        rbac::mutation_budget(&mut tx).await?;
        let (original, target): (Uuid, Uuid) =
            sqlx::query_as("SELECT actor_id,target_id FROM commerce_operations WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::Invalid)?;
        let saved: Value =
            sqlx::query_scalar("SELECT command FROM commerce_operations WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        let saved: Command = serde_json::from_value(saved).map_err(|_| Error::Unavailable)?;
        let owner = payment_owner(&mut tx, &saved, target).await?;
        rbac::authorize_mutation_with_resources(
            &mut tx,
            &[context.account, original, owner],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM commerce_operations WHERE id=$1 FOR UPDATE")
                        .bind(id)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            &self.auth,
            context,
            &[rbac::Permission::Access, saved.permission()],
        )
        .await?;
        let row = sqlx::query("SELECT actor_id,target_id,command,reason_sha256,requires_approval,approved_by,result FROM commerce_operations WHERE id=$1 FOR UPDATE")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::Invalid)?;
        let actor: Uuid = row.try_get("actor_id")?;
        let command: Command =
            serde_json::from_value(row.try_get("command")?).map_err(|_| Error::Unavailable)?;
        command.validate(self.flags)?;
        self.authorize_in(&mut tx, context, &command).await?;
        if actor == context.account
            || owner == context.account
            || !row.try_get::<bool, _>("requires_approval")?
        {
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
        // Approval rechecks the proposer's eligibility and current grants; the
        // approver's live session and MFA were checked above under these locks.
        for permission in [rbac::Permission::Access, command.permission()] {
            self.auth
                .check_subject_in(&mut tx, actor, permission)
                .await?;
        }
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
        self.authorize_in(&mut tx, context, &request.command)
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
async fn recorded(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM commerce_operations WHERE id=$1)")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?,
    )
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
#[cfg(test)]
mod tests;

async fn payment_owner(
    tx: &mut Transaction<'_, Postgres>,
    command: &Command,
    target: Uuid,
) -> Result<Uuid, Error> {
    if matches!(command, Command::Reconcile | Command::Refund) {
        Ok(
            sqlx::query_scalar("SELECT account_id FROM payment_intents WHERE id=$1")
                .bind(target)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(Error::Invalid)?,
        )
    } else {
        Ok(target)
    }
}
