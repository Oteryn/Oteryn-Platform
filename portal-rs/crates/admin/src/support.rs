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
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;
use uuid::Uuid;

const MAX_SANCTION_DURATION_HOURS: i64 = 8760;
const SECONDS_PER_HOUR: i64 = 3600;

/// Duration of a temporary staff sanction after resolving its effective times.
pub fn valid_sanction_duration(seconds: i64) -> bool {
    (SECONDS_PER_HOUR..=MAX_SANCTION_DURATION_HOURS * SECONDS_PER_HOUR).contains(&seconds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Tickets,
    Reports,
    Enforcement,
}
impl Area {
    pub(crate) fn permissions(self) -> &'static [Permission] {
        match self {
            Self::Tickets => &[Permission::ManageSupportTickets],
            Self::Reports => &[
                Permission::ReviewSupportReports,
                Permission::ManageSupportReports,
            ],
            Self::Enforcement => &[Permission::ManageSupportEnforcement],
        }
    }

    /// Check an area grant after locking the resources and authorizing panel access.
    pub async fn check_in<A: AccountStateReader>(
        self,
        tx: &mut Transaction<'_, Postgres>,
        auth: &Authorizer<A>,
        context: CheckContext,
    ) -> Result<(), Error> {
        for &permission in self.permissions() {
            match auth.check_in(tx, context, permission).await {
                Err(crate::rbac::Error::Denied) => continue,
                result => return result.map_err(authorization_error),
            }
        }
        Err(Error::Denied)
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    AssignTicket {
        #[serde(default, deserialize_with = "form_optional_uuid")]
        assignee: Option<Uuid>,
    },
    VerifyReportTarget {
        target: Uuid,
    },
    LinkSanction {
        sanction: Uuid,
        public_outcome: String,
    },
    ApplySanction {
        target: Uuid,
        category: String,
        public_reason: String,
        moderator_notes: String,
        #[serde(deserialize_with = "form_integer")]
        duration_hours: i64,
    },
    RevokeSanction {
        public_reason: String,
    },
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
            Self::Reply { .. } | Self::TicketStatus { .. } | Self::AssignTicket { .. } => {
                Area::Tickets
            }
            Self::Moderate { .. } | Self::VerifyReportTarget { .. } | Self::LinkSanction { .. } => {
                Area::Reports
            }
            Self::Sanction { .. }
            | Self::Appeal { .. }
            | Self::ApplySanction { .. }
            | Self::RevokeSanction { .. } => Area::Enforcement,
        }
    }
    pub fn permissions(&self) -> &[Permission] {
        match self {
            Self::Moderate { .. } => &[Permission::Access],
            Self::LinkSanction { .. } => &[
                Permission::Access,
                Permission::ManageSupportReports,
                Permission::ManageSupportEnforcement,
            ],
            _ => match self.area() {
                Area::Tickets => &[Permission::Access, Permission::ManageSupportTickets],
                Area::Reports => &[Permission::Access, Permission::ManageSupportReports],
                Area::Enforcement => &[Permission::Access, Permission::ManageSupportEnforcement],
            },
        }
    }
    /// Report status review accepts either report grant. Target verification and
    /// sanction linking retain their separate, mandatory mutation permissions.
    pub async fn check_report_review_in<A: AccountStateReader>(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        auth: &Authorizer<A>,
        context: CheckContext,
    ) -> Result<(), Error> {
        if matches!(self, Self::Moderate { .. }) {
            Area::Reports.check_in(tx, auth, context).await?;
        }
        Ok(())
    }

    fn creates_sanction(&self) -> bool {
        matches!(
            self,
            Self::Sanction { create: true, .. } | Self::ApplySanction { .. }
        )
    }
    pub fn validate(&self) -> Result<(), Error> {
        let valid = match self {
            Self::AssignTicket { assignee } => assignee.is_none_or(|id| !id.is_nil()),
            Self::VerifyReportTarget { target } => !target.is_nil(),
            Self::LinkSanction {
                sanction,
                public_outcome,
            } => !sanction.is_nil() && text(public_outcome, 4000, true),
            Self::ApplySanction {
                target,
                category,
                public_reason,
                moderator_notes,
                duration_hours,
            } => {
                !target.is_nil()
                    && matches!(category.as_str(), "warning" | "restriction" | "suspension")
                    && text(public_reason, 4000, true)
                    && text(moderator_notes, 8000, false)
                    && (1..=MAX_SANCTION_DURATION_HOURS).contains(duration_hours)
            }
            Self::RevokeSanction { public_reason } => text(public_reason, 4000, true),
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
            Self::Reply { .. } | Self::AssignTicket { .. } => {
                matches!(current, "open" | "waiting_staff" | "waiting_user")
            }
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
            Self::VerifyReportTarget { .. } | Self::LinkSanction { .. } => {
                matches!(current, "submitted" | "reviewing")
            }
            Self::ApplySanction { .. } | Self::Sanction { create: true, .. } => current == "absent",
            Self::RevokeSanction { .. } => current == "active",
            Self::Sanction { status, .. } if status == "revoked" => current == "active",
            Self::Sanction { .. } => matches!(current, "active" | "expired" | "revoked"),
            Self::Appeal { .. } => matches!(current, "requested" | "reviewing"),
        };
        if valid { Ok(()) } else { Err(Error::Conflict) }
    }
    fn audit_action(&self) -> &'static str {
        match self {
            Self::AssignTicket { .. } => "support.ticket_assigned",
            Self::VerifyReportTarget { .. } => "support.report_target_verified",
            Self::LinkSanction { .. } => "support.report_sanction_linked",
            Self::ApplySanction { .. } => "support.enforcement_created",
            Self::RevokeSanction { .. } => "support.enforcement_updated",
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
const TICKET_STATUSES: &[&str] = &[
    "open",
    "waiting_staff",
    "waiting_user",
    "resolved",
    "closed",
];
const TICKET_CATEGORIES: &[&str] = &["account", "character", "technical", "payment", "other"];

fn ticket_status(value: &str) -> bool {
    TICKET_STATUSES.contains(&value)
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct TicketFilter {
    pub status: String,
    pub category: String,
}
impl TicketFilter {
    fn is_empty(&self) -> bool {
        self.status.is_empty() && self.category.is_empty()
    }
    fn valid(&self) -> bool {
        (self.status.is_empty() || ticket_status(&self.status))
            && (self.category.is_empty() || TICKET_CATEGORIES.contains(&self.category.as_str()))
    }
}

#[derive(Debug, Default)]
pub struct Details {
    pub assigned_to: Option<Uuid>,
    pub linked_sanction: Option<Uuid>,
    pub player_report: bool,
    pub verified_target: Option<Uuid>,
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
    pub next_message: Option<Uuid>,
}
#[derive(Debug, Clone)]
pub struct Message {
    pub id: Option<Uuid>,
    pub body: String,
    pub internal: bool,
}

pub trait Backend: Clone + Send + Sync + 'static {
    /// Acquire domain account locks before the RBAC fence and its actor SHARE lock.
    fn prepare(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        _context: CheckContext,
        _id: Uuid,
        _command: &Command,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        async { Ok(()) }
    }

    fn list(
        &self,
        area: Area,
        page: u32,
    ) -> impl Future<Output = Result<Vec<Record>, Error>> + Send;
    fn list_filtered(
        &self,
        area: Area,
        page: u32,
        filter: &TicketFilter,
    ) -> impl Future<Output = Result<Vec<Record>, Error>> + Send {
        async move {
            if !filter.is_empty() {
                return Err(Error::Unavailable);
            }
            self.list(area, page).await
        }
    }
    fn details(
        &self,
        _area: Area,
        _record: &Record,
    ) -> impl Future<Output = Result<Details, Error>> + Send {
        async { Ok(Details::default()) }
    }
    fn load(
        &self,
        area: Area,
        id: Uuid,
        after: Option<Uuid>,
    ) -> impl Future<Output = Result<Record, Error>> + Send;
    /// Creation requires an absent ID and an active authoritative target account.
    /// Updates must never change the target account of an existing sanction.
    /// Must reject stale versions; accepted appeals revoke the sanction atomically.
    /// Return redacted before/after metadata; never return message bodies in audit data.
    fn apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: CheckContext,
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
    async fn load(&self, _: Area, _: Uuid, _: Option<Uuid>) -> Result<Record, Error> {
        Err(Error::Unavailable)
    }
    async fn apply(
        &self,
        _: &mut Transaction<'_, Postgres>,
        _: CheckContext,
        _: Uuid,
        _: i64,
        _: &Command,
    ) -> Result<(serde_json::Value, serde_json::Value), Error> {
        Err(Error::Unavailable)
    }
}
fn authorization_error(error: crate::rbac::Error) -> Error {
    match error {
        crate::rbac::Error::Storage(_) => Error::Unavailable,
        _ => Error::Denied,
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
    execute_audited(pool, auth, backend, context, (id, version), command, None).await
}

async fn execute_audited<A: AccountStateReader, B: Backend>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    backend: &B,
    context: CheckContext,
    (id, version): (Uuid, i64),
    command: &Command,
    reason: Option<&str>,
) -> Result<(), Error> {
    if id.is_nil() || version < 1 || (command.creates_sanction() && version != 1) {
        return Err(Error::Invalid);
    }
    command.validate()?;
    if !context.require_staff_mfa || !context.mfa_verified {
        return Err(Error::Denied);
    }
    let permissions = command.permissions();
    for &permission in permissions {
        auth.check(context, permission)
            .await
            .map_err(authorization_error)?;
    }
    if matches!(command, Command::Moderate { .. }) {
        auth.check_any(context, Area::Reports.permissions())
            .await
            .map_err(authorization_error)?;
    }
    let mut tx = pool.begin().await.map_err(|_| Error::Unavailable)?;
    // Preparation precedes the RBAC fence, but retains its bounded lock waits.
    crate::rbac::mutation_budget(&mut tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    backend.prepare(&mut tx, context, id, command).await?;
    crate::rbac::authorize_mutation(&mut tx, &[context.account], auth, context, permissions)
        .await
        .map_err(authorization_error)?;
    command
        .check_report_review_in(&mut tx, auth, context)
        .await?;
    if let Command::AssignTicket {
        assignee: Some(assignee),
    } = command
    {
        for &permission in permissions {
            auth.check_subject_in(&mut tx, *assignee, permission)
                .await
                .map_err(authorization_error)?;
        }
    }
    let (before, mut after) = backend
        .apply(&mut tx, context, id, version, command)
        .await?;
    if let Some(reason) = reason {
        let after = after.as_object_mut().ok_or(Error::Unavailable)?;
        after.insert(
            "reason_sha256".into(),
            serde_json::json!(format!("{:x}", Sha256::digest(reason.trim().as_bytes()))),
        );
    }
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
    crate::rbac::authorize_mutation(&mut tx, &[context.account], auth, context, permissions)
        .await
        .map_err(authorization_error)?;
    command
        .check_report_review_in(&mut tx, auth, context)
        .await?;
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

fn form_optional_uuid<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Uuid>, D::Error> {
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().map_err(serde::de::Error::custom))
        .transpose()
}
