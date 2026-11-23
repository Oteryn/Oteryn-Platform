//! Transactional staff form mutations. Mount routes behind the existing staff guard.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, Change, CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use axum::{http::StatusCode, response::Response};
use oteryn_ui::Chrome;
use oteryn_web::csrf::Verified;
use serde_json::{Value, json};

pub(crate) const ROLE_REASON: &str = "role_assignment";
pub(crate) const ACCOUNT_REASON: &str = "support_case";
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
    if matches!(requirement, StepUp::RecentPassword)
        && !recent_password_in(connection, actor).await?
    {
        return Err(Error::StepUp);
    }
    Ok(())
}

/// Check the password-confirmation deadline on the already authorized session.
pub(crate) async fn recent_password_in(
    connection: &mut PgConnection,
    actor: CheckContext,
) -> Result<bool, sqlx::Error> {
    let Some(proof) = actor.session_proof else {
        return Ok(false);
    };
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM web_sessions
         WHERE account_id=$1 AND id_hash=$2 AND sudo_until>clock_timestamp())",
    )
    .bind(actor.account)
    .bind(proof.session_hash.as_slice())
    .fetch_one(connection)
    .await
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
                &chrome.text("staff-action-approved-reason-required"),
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
    Commerce(crate::commerce::audit_change::CommerceAudit),
    RoleGrant {
        account: Uuid,
        role: Uuid,
    },
    RoleCreated {
        role: Uuid,
        key: String,
        permissions: Vec<Permission>,
    },
    RolePermissions {
        role: Uuid,
        before: Vec<Permission>,
        after: Vec<Permission>,
    },
    AccountNote {
        account: Uuid,
        note: Uuid,
    },
    ReviewFlag {
        account: Uuid,
        before: bool,
        after: bool,
    },
    AccountStatus {
        account: Uuid,
        suspended: bool,
    },
    AccountEmail {
        account: Uuid,
        request: Uuid,
    },
    #[cfg(test)]
    TestLocale {
        account: Uuid,
    },
}

impl AuditChange {
    fn request_id(&self) -> Uuid {
        match self {
            Self::Commerce(change) => change.request_id(),
            _ => Uuid::now_v7(),
        }
    }

    fn project(self) -> (&'static str, Uuid, Value, Value) {
        let permissions = |permissions: Vec<Permission>| {
            let mut names: Vec<_> = permissions.into_iter().map(Permission::as_str).collect();
            names.sort_unstable();
            names
        };
        match self {
            Self::Commerce(change) => change.project(),
            Self::RoleGrant { account, role } => (
                "rbac.role.granted",
                account,
                json!({"role": role, "assigned": false}),
                json!({"role": role, "assigned": true}),
            ),
            Self::RoleCreated {
                role,
                key,
                permissions: assigned,
            } => (
                "rbac.role.created",
                role,
                Value::Null,
                json!({"key": key, "permissions": permissions(assigned)}),
            ),
            Self::RolePermissions {
                role,
                before,
                after,
            } => (
                "rbac.role.permissions_changed",
                role,
                json!({"permissions": permissions(before)}),
                json!({"permissions": permissions(after)}),
            ),
            Self::AccountNote { account, note } => (
                "players.note.added",
                account,
                json!({}),
                json!({"note": note}),
            ),
            Self::ReviewFlag {
                account,
                before,
                after,
            } => (
                "players.flags.changed",
                account,
                json!({"needs_review": before}),
                json!({"needs_review": after}),
            ),
            Self::AccountStatus { account, suspended } => {
                let (before, after) = if suspended {
                    ("active", "suspended")
                } else {
                    ("suspended", "active")
                };
                (
                    "players.status.changed",
                    account,
                    json!({"status": before}),
                    json!({"status": after}),
                )
            }
            Self::AccountEmail { account, request } => (
                "players.email.change_requested",
                account,
                json!({"pending": false}),
                json!({"pending": true, "request": request}),
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

pub(crate) fn verify_session(actor: CheckContext, csrf: Option<Verified>) -> Result<(), Error> {
    let same_session = csrf
        .zip(actor.session_proof)
        .is_some_and(|(csrf, proof)| csrf.matches_session(&proof.session_hash));
    if same_session && actor.require_staff_mfa && actor.mfa_verified {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}

pub(crate) fn validated_code(
    reason: &str,
    allowed: &'static [&'static str],
) -> Result<&'static str, Error> {
    allowed
        .iter()
        .copied()
        .find(|code| *code == reason && (8..=500).contains(&code.chars().count()))
        .ok_or(Error::Reason)
}

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
    rbac::mutation_budget(&mut tx).await?;
    rbac::lock_accounts_and_mutation(&mut tx, &accounts).await?;
    run_locked(tx, guard, action, reason, input, mutate).await
}

/// Validate the request proof and reason before making a staff change.
pub(crate) fn validated_reason(
    guard: &Guard<'_>,
    action: &Action<'_>,
) -> Result<&'static str, Error> {
    verify_session(guard.actor, guard.csrf)?;
    if action.permission == Permission::Access {
        return Err(Error::Forbidden);
    }
    validated_code(action.reason, action.reason_codes)
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
    let result = async {
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
            let request_id = change.request_id();
            record(&mut tx, guard.actor, request_id, change, Some(reason)).await?;
        }
        for permission in permissions {
            guard
                .authority
                .check(&mut tx, guard.actor, permission)
                .await?;
        }
        check_step_up(&mut tx, guard.actor, action.step_up).await?;
        if let Some((account, role)) = affected {
            Change::revoke_affected_browser_credentials(&mut tx, guard.actor, account, role)
                .await?;
        }
        Ok::<_, Error>(changed)
    }
    .await;
    match result {
        Ok(true) => tx.commit().await?,
        Ok(false) => tx.rollback().await?,
        Err(error) => {
            tx.rollback().await?;
            return Err(error);
        }
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
