//! ADM-01 audit storage. Callers authorize before reading and commit before rendering.
use super::{AuditContext, Postgres, Transaction, Uuid, Value};
use crate::rbac::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
}
impl RiskClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::R0 => "R0",
            Self::R1 => "R1",
            Self::R2 => "R2",
            Self::R3 => "R3",
            Self::R4 => "R4",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AuditOutcome {
    Accepted,
    InProgress,
    Succeeded,
    Failed,
    Rejected,
    Expired,
    Cancelled,
}
impl AuditOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::InProgress => "in_progress",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Disabled by default. This storage helper grants no permission or agent capability.
#[derive(Debug, Default, Clone, Copy)]
pub struct AdminControlAuditWriter {
    pub enabled: bool,
}

/// Trusted request metadata and bounded domain evidence, never credentials or results.
/// `target` is stored as JSONB in `admin_control_target` to retain the legacy text column.
#[derive(Debug, Clone)]
pub struct AdminControlAudit {
    pub context: AuditContext,
    pub admin_operation_id: Uuid,
    pub operation: String,
    pub risk_class: RiskClass,
    pub target: Value,
    pub reason: Option<Value>,
    pub approvals: Value,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub correlation_id: Uuid,
    pub outcome: AuditOutcome,
    pub game_ledger_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReadView {
    Dossier,
    Inventory,
    History,
    Relink,
    Export,
    DatabaseSql,
    AnalyticsSql,
}
impl ReadView {
    pub const fn operation(self) -> &'static str {
        match self {
            Self::Dossier => "player.character.get",
            Self::Inventory => "player.inventory.get",
            Self::History => "player.history.get",
            Self::Relink => "analytics.identity.relink",
            Self::Export => "analytics.export",
            Self::DatabaseSql => "db.query.read",
            Self::AnalyticsSql => "analytics.sql.read",
        }
    }
}

impl AdminControlAuditWriter {
    /// Appends one event in the caller's transaction; never updates an earlier event.
    pub async fn append(
        self,
        transaction: &mut Transaction<'_, Postgres>,
        record: AdminControlAudit,
    ) -> Result<Uuid, Error> {
        if !self.enabled {
            return Err(Error::Denied);
        }
        if record.context.actor.is_none_or(|id| id.is_nil())
            || record.context.request_id.is_nil()
            || record.admin_operation_id.is_nil()
            || record.correlation_id.is_nil()
            || record.operation.split('.').any(str::is_empty)
            || record.operation.len() > 128
            || !record
                .operation
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c == b'.' || c == b'_')
            || !record.target.is_object()
            || record.target.as_object().is_none_or(|v| v.is_empty())
            || !record.approvals.is_array()
            || record.reason.as_ref().is_some_and(|v| !v.is_object())
            || record
                .ip
                .as_ref()
                .is_some_and(|v| v.parse::<std::net::IpAddr>().is_err())
            || record
                .user_agent
                .as_ref()
                .is_some_and(|v| v.len() > 2048 || v.chars().any(char::is_control))
            || record
                .game_ledger_id
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.len() > 256)
        {
            return Err(Error::InvalidInput);
        }
        Ok(sqlx::query_scalar(
            "INSERT INTO audit_log
             (actor, action, target, before, after, request_id, admin_operation_id,
              operation, risk_class, admin_control_target, reason, approvals, ip,
              user_agent, correlation_id, outcome, game_ledger_id)
             VALUES ($1,$2,$3,'null'::jsonb,'null'::jsonb,$4,$5,$2,$6,$7,$8,$9,$10,$11,$12,$13,$14)
             RETURNING id",
        )
        .bind(record.context.actor)
        .bind(&record.operation)
        .bind(record.admin_operation_id.to_string())
        .bind(record.context.request_id)
        .bind(record.admin_operation_id)
        .bind(record.risk_class.as_str())
        .bind(record.target)
        .bind(record.reason)
        .bind(record.approvals)
        .bind(record.ip)
        .bind(record.user_agent)
        .bind(record.correlation_id)
        .bind(record.outcome.as_str())
        .bind(record.game_ledger_id)
        .fetch_one(&mut **transaction)
        .await?)
    }

    /// Call once for each view, including cached views. SQL text belongs in reason.sql.
    /// Export is audited here as a view; its command retains the catalog's R2 class.
    pub async fn read_audit(
        self,
        transaction: &mut Transaction<'_, Postgres>,
        view: ReadView,
        mut record: AdminControlAudit,
    ) -> Result<Uuid, Error> {
        if record.operation != view.operation()
            || record.risk_class
                != if matches!(view, ReadView::Export) {
                    RiskClass::R2
                } else {
                    RiskClass::R1
                }
            || (matches!(view, ReadView::Relink) && record.reason.is_none())
            || (matches!(view, ReadView::DatabaseSql | ReadView::AnalyticsSql)
                && record
                    .reason
                    .as_ref()
                    .and_then(|v| v.get("sql"))
                    .and_then(Value::as_str)
                    .is_none_or(|v| v.trim().is_empty()))
        {
            return Err(Error::InvalidInput);
        }
        // View evidence carries no returned player data. Preserve full SQL audit text.
        record.target = mask_player_fields(&record.target);
        self.append(transaction, record).await
    }
}

/// Full redaction also covers malformed input and avoids leaking address fragments.
pub fn mask_email(_: &str) -> &'static str {
    "[masked]"
}
pub fn mask_ip(_: &str) -> &'static str {
    "[masked]"
}
pub fn mask_payment(_: &Value) -> Value {
    Value::String("[masked]".into())
}

/// Recursively masks sensitive fields, including nested objects and arrays.
/// Callers must still use an operation-specific allowlist for displayable fields.
pub fn mask_player_fields(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(mask_player_fields).collect()),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| {
                    let normalized = key.to_ascii_lowercase().replace('-', "_");
                    let masked = if normalized.contains("email")
                        || normalized.contains("payment")
                        || normalized.contains("card")
                        || normalized.contains("iban")
                        || normalized == "ip"
                        || normalized.starts_with("ip_")
                        || normalized.ends_with("_ip")
                        || normalized.contains("ip_address")
                        || normalized.contains("ipaddress")
                        || matches!(
                            normalized.as_str(),
                            "lastip" | "remoteip" | "clientip" | "sourceip"
                        )
                        || normalized.starts_with("billing")
                        || normalized.contains("bank_account")
                        || normalized == "cvv"
                        || normalized == "cvc"
                    {
                        Value::String("[masked]".into())
                    } else {
                        mask_player_fields(value)
                    };
                    (key.clone(), masked)
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}
