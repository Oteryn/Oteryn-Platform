//! Staff support boundary. T33c/T34b supply the authoritative storage adapter.
//!
//! No support schema is duplicated here. Adapters must lock the domain row, validate
//! its version and transition, persist the change and enqueue public notifications
//! in the supplied transaction. Internal notes must never enqueue a notification.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Tickets,
    Reports,
    Enforcement,
}
impl Area {
    pub fn permission(self) -> Permission {
        match self {
            Self::Tickets => Permission::ManageSupportTickets,
            Self::Reports => Permission::ManageSupportReports,
            Self::Enforcement => Permission::ManageSupportEnforcement,
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Reply {
        body: String,
        #[serde(default, deserialize_with = "form_bool")]
        internal: bool,
    },
    TicketStatus {
        status: String,
    },
    Moderate {
        status: String,
        public_outcome: String,
        moderator_notes: String,
    },
    Sanction {
        #[serde(default, deserialize_with = "form_bool")]
        create: bool,
        status: String,
        target: Uuid,
        category: String,
        public_reason: String,
        moderator_notes: String,
        #[serde(deserialize_with = "form_integer")]
        effective_at: i64,
        #[serde(deserialize_with = "form_integer")]
        expires_at: i64,
    },
    Appeal {
        appeal_status: String,
        appeal_outcome: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unavailable")]
    Unavailable,
    #[error("denied")]
    Denied,
    #[error("not found")]
    NotFound,
    #[error("invalid input")]
    Invalid,
    #[error("conflict")]
    Conflict,
}
impl Command {
    pub fn area(&self) -> Area {
        match self {
            Self::Reply { .. } | Self::TicketStatus { .. } => Area::Tickets,
            Self::Moderate { .. } => Area::Reports,
            Self::Sanction { .. } | Self::Appeal { .. } => Area::Enforcement,
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        let valid = match self {
            Self::Reply { body, .. } => text(body, 8000, true),
            Self::TicketStatus { status } => ticket_status(status),
            Self::Moderate {
                status,
                public_outcome,
                moderator_notes,
            } => {
                matches!(
                    status.as_str(),
                    "submitted" | "reviewing" | "actioned" | "rejected" | "closed"
                ) && text(
                    public_outcome,
                    4000,
                    matches!(status.as_str(), "actioned" | "rejected" | "closed"),
                ) && text(moderator_notes, 8000, false)
            }
            Self::Sanction {
                status,
                target,
                category,
                public_reason,
                moderator_notes,
                effective_at,
                expires_at,
                ..
            } => {
                matches!(status.as_str(), "active" | "expired" | "revoked")
                    && !target.is_nil()
                    && matches!(category.as_str(), "warning" | "restriction" | "suspension")
                    && text(public_reason, 4000, true)
                    && text(moderator_notes, 8000, false)
                    && *effective_at > 0
                    && expires_at > effective_at
            }
            Self::Appeal {
                appeal_status,
                appeal_outcome,
            } => {
                matches!(
                    appeal_status.as_str(),
                    "reviewing" | "accepted" | "rejected"
                ) && text(appeal_outcome, 4000, appeal_status != "reviewing")
            }
        };
        if valid { Ok(()) } else { Err(Error::Invalid) }
    }
    /// Called against the locked authoritative row, never a browser-supplied state.
    pub fn transition(&self, current: &str) -> Result<(), Error> {
        self.validate()?;
        let valid = match self {
            Self::Reply { .. } => matches!(current, "open" | "waiting_staff" | "waiting_user"),
            Self::TicketStatus { .. } => ticket_status(current),
            Self::Moderate { status, .. } => match current {
                "submitted" => matches!(
                    status.as_str(),
                    "submitted" | "reviewing" | "actioned" | "rejected"
                ),
                "reviewing" => matches!(status.as_str(), "reviewing" | "actioned" | "rejected"),
                "actioned" | "rejected" => status == "closed",
                _ => false,
            },
            Self::Sanction { create: true, .. } => current == "absent",
            Self::Sanction { .. } => matches!(current, "active" | "expired" | "revoked"),
            Self::Appeal { .. } => matches!(current, "requested" | "reviewing"),
        };
        if valid { Ok(()) } else { Err(Error::Conflict) }
    }
    fn audit_action(&self) -> &'static str {
        match self {
            Self::Reply { internal: true, .. } => "support.ticket_internal_note_added",
            Self::Reply { .. } => "support.ticket_replied",
            Self::TicketStatus { .. } => "support.ticket_status_changed",
            Self::Moderate { .. } => "support.report_status_changed",
            Self::Sanction { create: true, .. } => "support.enforcement_created",
            Self::Sanction { .. } => "support.enforcement_updated",
            Self::Appeal { .. } => "support.enforcement_appeal_updated",
        }
    }
}
fn text(value: &str, max: usize, required: bool) -> bool {
    (!required || !value.trim().is_empty()) && value.chars().count() <= max && !value.contains('\0')
}
fn ticket_status(value: &str) -> bool {
    matches!(
        value,
        "open" | "waiting_staff" | "waiting_user" | "resolved" | "closed"
    )
}

/// Public display fields only. Private messages are provided solely to this guarded
/// staff surface; there is deliberately no player serialization implementation.
#[derive(Debug, Clone)]
pub struct Record {
    pub id: Uuid,
    pub version: i64,
    pub title: String,
    pub status: String,
    pub messages: Vec<Message>,
}
#[derive(Debug, Clone)]
pub struct Message {
    pub body: String,
    pub internal: bool,
}

pub trait Backend: Clone + Send + Sync + 'static {
    fn list(
        &self,
        area: Area,
        page: u32,
    ) -> impl Future<Output = Result<Vec<Record>, Error>> + Send;
    fn load(&self, area: Area, id: Uuid) -> impl Future<Output = Result<Record, Error>> + Send;
    /// Creation requires an absent ID and an active authoritative target account.
    /// Updates must never change the target account of an existing sanction.
    /// Must reject stale versions; accepted appeals revoke the sanction atomically.
    /// Return redacted before/after metadata; never return message bodies in audit data.
    fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        id: Uuid,
        version: i64,
        command: &Command,
    ) -> impl Future<Output = Result<(serde_json::Value, serde_json::Value), Error>> + Send;
}
#[derive(Debug, Clone)]
pub struct PendingSupport;
impl Backend for PendingSupport {
    async fn list(&self, _: Area, _: u32) -> Result<Vec<Record>, Error> {
        Err(Error::Unavailable)
    }
    async fn load(&self, _: Area, _: Uuid) -> Result<Record, Error> {
        Err(Error::Unavailable)
    }
    async fn apply(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: Uuid,
        _: Uuid,
        _: i64,
        _: &Command,
    ) -> Result<(serde_json::Value, serde_json::Value), Error> {
        Err(Error::Unavailable)
    }
}
/// Authorization is rechecked while holding the role-writer lock. Domain writes and
/// audit commit together; an unavailable dependency cannot produce a success.
pub async fn execute<A: AccountStateReader, B: Backend>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    backend: &B,
    context: CheckContext,
    id: Uuid,
    version: i64,
    command: &Command,
) -> Result<(), Error> {
    if id.is_nil()
        || version < 1
        || (matches!(command, Command::Sanction { create: true, .. }) && version != 1)
    {
        return Err(Error::Invalid);
    }
    command.validate()?;
    if !context.require_staff_mfa || !context.mfa_verified {
        return Err(Error::Denied);
    }
    let mut tx = pool.begin().await.map_err(|_| Error::Unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    for permission in [Permission::Access, command.area().permission()] {
        auth.check(context, permission).await.map_err(|e| match e {
            crate::rbac::Error::Storage(_) => Error::Unavailable,
            _ => Error::Denied,
        })?;
    }
    let (before, after) = backend
        .apply(&mut tx, context.account, id, version, command)
        .await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: command.audit_action(),
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await
    .map_err(|_| Error::Unavailable)?;
    tx.commit().await.map_err(|_| Error::Unavailable)
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
#[path = "support_web.rs"]
pub mod web;

fn form_bool<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Bool(bool),
        Text(String),
    }
    match Value::deserialize(deserializer)? {
        Value::Bool(value) => Ok(value),
        Value::Text(value) => match value.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(serde::de::Error::custom("invalid boolean")),
        },
    }
}
fn form_integer<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Integer(i64),
        Text(String),
    }
    match Value::deserialize(deserializer)? {
        Value::Integer(value) => Ok(value),
        Value::Text(value) => value.parse().map_err(serde::de::Error::custom),
    }
}
