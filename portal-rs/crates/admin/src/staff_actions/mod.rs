//! Transactional staff form mutations. Mount routes behind the existing staff guard.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, Change, CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use axum::{http::StatusCode, response::Response};
use oteryn_ui::Chrome;
use oteryn_web::csrf::Verified;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum StepUp {
    SessionMfa,
    RecentPassword,
}

async fn check_step_up(
    connection: &mut PgConnection,
    actor: CheckContext,
    requirement: StepUp,
) -> Result<(), Error> {
    if matches!(requirement, StepUp::RecentPassword) {
        let proof = actor.session_proof.ok_or(Error::StepUp)?;
        // RBAC has already locked and verified the durable session and MFA grant.
        let fresh: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM web_sessions
             WHERE account_id=$1 AND id_hash=$2 AND sudo_until>clock_timestamp())",
        )
        .bind(actor.account)
        .bind(proof.session_hash.as_slice())
        .fetch_one(connection)
        .await?;
        if !fresh {
            return Err(Error::StepUp);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Reason,
    StepUp,
    Invalid,
    Conflict,
    Unavailable,
}

impl Error {
    pub fn response(self, chrome: &Chrome) -> Response {
        let status = match self {
            Self::Forbidden | Self::StepUp => StatusCode::FORBIDDEN,
            Self::Reason | Self::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        };
        if matches!(self, Self::Reason) {
            oteryn_ui::error_with_message(
                chrome,
                status,
                &chrome.text("staff-action-reason-invalid"),
            )
        } else if matches!(self, Self::StepUp) {
            oteryn_ui::error_with_message(
                chrome,
                status,
                &chrome.text("staff-action-step-up-required"),
            )
        } else {
            oteryn_ui::error(chrome, status)
        }
    }
}

impl From<rbac::Error> for Error {
    fn from(error: rbac::Error) -> Self {
        match error {
            rbac::Error::Denied => Self::Forbidden,
            rbac::Error::InvalidInput | rbac::Error::RoleNotFound => Self::Invalid,
            rbac::Error::Storage(error) => error.into(),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            Self::Conflict
        } else {
            Self::Unavailable
        }
    }
}

/// Closed audit projections. Each variant chooses its event name, fields and
/// value types. Form data and arbitrary runtime values never reach this API.
#[derive(Debug)]
pub enum AuditChange {
    RoleGrant {
        account: Uuid,
        role: Uuid,
    },
    RoleCreated {
        role: Uuid,
        permissions: Vec<Permission>,
    },
    RolePermissions {
        role: Uuid,
        before: Vec<Permission>,
        after: Vec<Permission>,
    },
    #[cfg(test)]
    TestLocale {
        account: Uuid,
    },
}

impl AuditChange {
    fn project(self) -> (&'static str, Uuid, Value, Value) {
        let permissions = |permissions: Vec<Permission>| {
            let mut names: Vec<_> = permissions.into_iter().map(Permission::as_str).collect();
            names.sort_unstable();
            json!({"permissions": names})
        };
        match self {
            Self::RoleGrant { account, role } => (
                "rbac.role.granted",
                account,
                json!({"role": role, "assigned": false}),
                json!({"role": role, "assigned": true}),
            ),
            Self::RoleCreated {
                role,
                permissions: assigned,
            } => (
                "rbac.role.created",
                role,
                Value::Null,
                permissions(assigned),
            ),
            Self::RolePermissions {
                role,
                before,
                after,
            } => (
                "rbac.role.permissions_changed",
                role,
                permissions(before),
                permissions(after),
            ),
            #[cfg(test)]
            Self::TestLocale { account } => (
                "staff.action.test",
                account,
                json!({"locale": "pl"}),
                json!({"locale": "en"}),
            ),
        }
    }
}

pub struct Action<'a> {
    pub permission: Permission,
    pub reason: &'a str,
    pub reason_codes: &'static [&'static str],
    pub step_up: StepUp,
}

impl std::fmt::Debug for Action<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Action")
            .field("permission", &self.permission)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct Guard<'a> {
    pub actor: CheckContext,
    pub authority: &'a MutationAuthorization,
    pub csrf: Option<Verified>,
}

pub type Mutation<'a> =
    Pin<Box<dyn Future<Output = Result<Option<AuditChange>, Error>> + Send + 'a>>;

/// Validate the form, lock its accounts and run the mutation under durable staff
/// authorization. Return `None` from the mutation only for a successful no-op.
pub async fn run<T, F>(
    pool: &PgPool,
    guard: Guard<'_>,
    action: Action<'_>,
    validate: impl FnOnce() -> Result<(Vec<Uuid>, T), Error>,
    mutate: F,
) -> Result<(), Error>
where
    F: for<'a> FnOnce(&'a mut PgConnection, T) -> Mutation<'a>,
{
    let reason = validated_reason(&guard, &action)?;
    let (mut accounts, input) = validate()?;
    accounts.push(guard.actor.account);
    let mut tx = pool.begin().await?;
    rbac::lock_accounts_and_mutation(&mut tx, &accounts).await?;
    run_locked(tx, guard, action, reason, input, mutate).await
}

/// Validate the request proof and reason before making a staff change.
pub(crate) fn validated_reason(
    guard: &Guard<'_>,
    action: &Action<'_>,
) -> Result<&'static str, Error> {
    let same_session = guard
        .csrf
        .zip(guard.actor.session_proof)
        .is_some_and(|(csrf, proof)| csrf.matches_session(&proof.session_hash));
    if !same_session
        || !guard.actor.require_staff_mfa
        || !guard.actor.mfa_verified
        || action.permission == Permission::Access
    {
        return Err(Error::Forbidden);
    }
    action
        .reason_codes
        .iter()
        .copied()
        .find(|code| *code == action.reason && (8..=500).contains(&code.chars().count()))
        .ok_or(Error::Reason)
}

/// Execute a validated action while retaining the caller's account and role locks.
pub(crate) async fn run_locked<T, F>(
    mut tx: Transaction<'_, Postgres>,
    guard: Guard<'_>,
    action: Action<'_>,
    reason: &'static str,
    input: T,
    mutate: F,
) -> Result<(), Error>
where
    F: for<'a> FnOnce(&'a mut PgConnection, T) -> Mutation<'a>,
{
    let permissions = [Permission::Access, action.permission];
    for permission in permissions {
        guard
            .authority
            .check(&mut tx, guard.actor, permission)
            .await?;
    }
    check_step_up(&mut tx, guard.actor, action.step_up).await?;
    let change = mutate(&mut tx, input).await?;
    let affected = match &change {
        Some(AuditChange::RoleGrant { account, .. }) => Some((Some(*account), None)),
        Some(AuditChange::RolePermissions { role, .. }) => Some((None, Some(*role))),
        _ => None,
    };
    let changed = change.is_some();
    if let Some(change) = change {
        record(&mut tx, guard.actor, Uuid::now_v7(), change, Some(reason)).await?;
    }
    for permission in permissions {
        guard
            .authority
            .check(&mut tx, guard.actor, permission)
            .await?;
    }
    check_step_up(&mut tx, guard.actor, action.step_up).await?;
    if changed {
        if let Some((account, role)) = affected {
            Change::revoke_affected_browser_credentials(&mut tx, guard.actor, account, role)
                .await?;
        }
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(())
}

pub(crate) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: CheckContext,
    request_id: Uuid,
    change: AuditChange,
    reason: Option<&str>,
) -> Result<(), rbac::Error> {
    let (event, target, before, after) = change.project();
    audit::append_with_reason(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor.account),
                request_id,
            },
            action: event,
            target: &target.to_string(),
            before,
            after,
        },
        reason,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
