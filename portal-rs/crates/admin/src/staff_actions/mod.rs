//! Transactional staff form mutations. Mount routes behind the existing staff guard.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use axum::{http::StatusCode, response::Response};
use oteryn_ui::Chrome;
use oteryn_web::csrf::Verified;
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use std::{collections::BTreeMap, future::Future, pin::Pin};
use uuid::Uuid;

pub const MIN_REASON_CHARS: usize = 8;
pub const MAX_REASON_CHARS: usize = 500;

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Reason,
    Invalid,
    Conflict,
    Unavailable,
}

impl Error {
    pub fn response(self, chrome: &Chrome) -> Response {
        let status = match self {
            Self::Forbidden => StatusCode::FORBIDDEN,
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

/// Audit projections accept only explicitly selected, non-secret domain values.
/// Use opaque domain IDs and stable enums represented by trusted static labels.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AuditValue {
    Id(Uuid),
    Flag(bool),
    Count(i64),
    Label(&'static str),
}

pub type Snapshot = BTreeMap<&'static str, AuditValue>;

#[derive(Debug)]
pub struct AuditChange {
    pub target: Uuid,
    pub before: Snapshot,
    pub after: Snapshot,
}

pub struct Action<'a> {
    pub permission: Permission,
    pub action: &'static str,
    pub reason: &'a str,
}

impl std::fmt::Debug for Action<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Action")
            .field("permission", &self.permission)
            .field("action", &self.action)
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
    if guard.csrf.is_none()
        || !guard.actor.require_staff_mfa
        || !guard.actor.mfa_verified
        || action.permission == Permission::Access
    {
        return Err(Error::Forbidden);
    }
    let reason = action.reason.trim();
    if !(MIN_REASON_CHARS..=MAX_REASON_CHARS).contains(&reason.chars().count())
        || reason.contains('\0')
    {
        return Err(Error::Reason);
    }
    let (accounts, input) = validate()?;
    let mut tx = pool.begin().await?;
    let result = async {
        rbac::mutation_budget(&mut tx).await?;
        let permissions = [Permission::Access, action.permission];
        rbac::authorize_mutation(
            &mut tx,
            &accounts,
            guard.authority,
            guard.actor,
            &permissions,
        )
        .await?;
        let change = mutate(&mut tx, input).await?;
        let changed = change.is_some();
        if let Some(change) = change {
            audit::append_with_reason(
                &mut tx,
                Entry {
                    context: AuditContext {
                        actor: Some(guard.actor.account),
                        request_id: Uuid::now_v7(),
                    },
                    action: action.action,
                    target: &change.target.to_string(),
                    before: serde_json::to_value(change.before).map_err(|_| Error::Unavailable)?,
                    after: serde_json::to_value(change.after).map_err(|_| Error::Unavailable)?,
                },
                Some(reason),
            )
            .await?;
        }
        for permission in permissions {
            guard
                .authority
                .check(&mut tx, guard.actor, permission)
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

#[cfg(test)]
mod tests;
