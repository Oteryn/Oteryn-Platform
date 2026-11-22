//! Combined Platform audit and verified Game ledger, joined visually by request id.
use super::access::{Backend, Console, Error, Page, SectionLine, language, tr};
use crate::rbac::CheckContext;
use axum::{
    Extension,
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use uuid::Uuid;
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub lang: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub request_id: Option<Uuid>,
    #[serde(default, deserialize_with = "optional")]
    pub admin: Option<Uuid>,
    #[serde(default, deserialize_with = "optional")]
    pub operation: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub target: Option<String>,
}
fn optional<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().map_err(serde::de::Error::custom))
        .transpose()
}
impl Filter {
    pub fn validate(&self) -> Result<(), Error> {
        language(self.lang.as_deref())?;
        if self.request_id.is_some_and(|id| id.is_nil()) || self.admin.is_some_and(|id| id.is_nil())
        {
            return Err(Error::BadInput);
        }
        for value in [&self.operation, &self.target].into_iter().flatten() {
            if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                return Err(Error::BadInput);
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct LedgerSnapshot {
    pub query_id: Uuid,
    pub rows: Vec<LedgerRow>,
}
#[derive(Debug, Clone)]
pub struct LedgerRow {
    pub request_id: Uuid,
    pub ledger_id: String,
    pub admin: String,
    pub operation: String,
    pub target: String,
    pub status: String,
    pub observed_at: String,
}
pub(crate) struct DiffLabels {
    pub field: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Change {
    pub field: String,
    pub before: String,
    pub after: String,
}

// Keep the legal targets aligned with content::pages::legal.
const LEGAL_PAGE_TARGETS: &[&str] = &["terms-of-service", "privacy-policy", "cookie-policy"];

const EDITORIAL_PAGE_ACTIONS: &[&str] = &["cms.page.saved", "cms.page.deleted"];
const LEGAL_PAGE_FIELDS: &[&str] = &["legal_version", "legal_effective_date"];

// The same allowlist drives snapshot projection and localized rendering.
const ACTION_FIELDS: &[(&[&str], &[&str])] = &[
    (&["rbac.role.created"], &["key", "permissions"]),
    (&["rbac.role.permissions_changed"], &["permissions"]),
    (
        &["rbac.role.granted", "rbac.role.revoked"],
        &["assigned", "role"],
    ),
    (&["players.kick"], &["native_security_generation"]),
    (
        &[
            "support.enforcement_created",
            "support.enforcement_updated",
            "support.enforcement_appeal_updated",
        ],
        &[
            "category",
            "status",
            "pending",
            "effective_at_ms",
            "expires_at_ms",
            "appeal_status",
            "lock_version",
        ],
    ),
    (
        &[
            "support.enforcement_approved",
            "support.enforcement_revoked",
        ],
        &["status", "lock_version"],
    ),
    (
        &["cms.news_created", "cms.news_updated", "cms.news_deleted"],
        &[
            "slug",
            "title",
            "title_pl",
            "published_at",
            "translation_published_at",
        ],
    ),
    (EDITORIAL_PAGE_ACTIONS, &["locale", "title", "published_at"]),
    (
        &["events.event_created", "events.event_updated"],
        &["status"],
    ),
    (&["events.status_changed"], &["status", "version"]),
    (&["portal.announcement_saved"], &["state"]),
    (
        &[
            "staff.invite.created",
            "staff.invite.revoked",
            "staff.invite.accepted",
        ],
        &["status"],
    ),
    (&["staff.invite.approved"], &["approved_by"]),
];

fn fields(action: &str, target: &str) -> impl Iterator<Item = &'static str> {
    let base = ACTION_FIELDS
        .iter()
        .find(|(actions, _)| actions.contains(&action))
        .map_or(&[][..], |(_, fields)| *fields);
    let legal = if EDITORIAL_PAGE_ACTIONS.contains(&action) && LEGAL_PAGE_TARGETS.contains(&target)
    {
        LEGAL_PAGE_FIELDS
    } else {
        &[]
    };
    base.iter().chain(legal).copied().chain(["email"])
}

#[derive(PartialEq, Eq)]
enum PublicationTime {
    Absent,
    Known(OffsetDateTime),
    Unknown,
}

fn publication_time(value: &Value) -> PublicationTime {
    if value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty()) {
        return PublicationTime::Absent;
    }
    let Some(text) = value.as_str() else {
        return PublicationTime::Unknown;
    };
    // The old news editor round-tripped PostgreSQL text such as "2020-01-01 00:00:00+00".
    let mut timestamp = text.replacen(' ', "T", 1);
    if timestamp
        .as_bytes()
        .iter()
        .rev()
        .nth(2)
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        timestamp.push_str(":00");
    }
    OffsetDateTime::parse(&timestamp, &Rfc3339)
        .map_or(PublicationTime::Unknown, PublicationTime::Known)
}

impl PublicationTime {
    fn display(self, lang: &str) -> Option<String> {
        match self {
            Self::Absent => None,
            Self::Known(time) => time.to_offset(UtcOffset::UTC).format(&Rfc3339).ok(),
            Self::Unknown => Some(tr(lang, "audit-diff-unknown")),
        }
    }
}

fn hidden_field(field: &str) -> bool {
    matches!(
        field,
        "title" | "title_pl" | "legal_version" | "slug" | "key"
    )
}

fn workflow_label(lang: &str, field: &str, text: &str) -> String {
    let known = match field {
        "category" => matches!(text, "warning" | "restriction" | "suspension"),
        "appeal_status" => matches!(
            text,
            "none" | "requested" | "reviewing" | "accepted" | "rejected"
        ),
        "state" => matches!(text, "draft" | "published"),
        "status" => matches!(
            text,
            "draft"
                | "scheduled"
                | "published"
                | "archived"
                | "pending"
                | "active"
                | "expired"
                | "revoked"
                | "accepted"
        ),
        _ => false,
    };
    if known {
        tr(lang, &format!("audit-value-{text}"))
    } else {
        tr(lang, "audit-diff-unknown")
    }
}

fn display(lang: &str, field: &str, value: &Value) -> Option<String> {
    match (field, value) {
        ("email", Value::String(email)) => {
            let (local, domain) = email.split_once('@')?;
            let initial = local.chars().next()?;
            (!domain.is_empty() && !domain.contains('@') && !initial.is_control())
                .then(|| format!("{initial}•••@•••"))
        }
        ("permissions", Value::Array(values)) => {
            let permissions = values
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()?;
            Some(if permissions.is_empty() {
                "[]".into()
            } else {
                permissions.join(", ")
            })
        }
        ("assigned" | "pending", Value::Bool(value)) => Some(tr(
            lang,
            if *value {
                "audit-diff-yes"
            } else {
                "audit-diff-no"
            },
        )),
        (
            "effective_at_ms"
            | "expires_at_ms"
            | "lock_version"
            | "native_security_generation"
            | "version",
            Value::Number(value),
        ) => Some(value.to_string()),
        (field, Value::String(text)) if hidden_field(field) && !text.is_empty() => {
            Some(tr(lang, "audit-diff-hidden"))
        }
        ("category" | "status" | "appeal_status" | "state", Value::String(text))
            if !text.is_empty() =>
        {
            Some(workflow_label(lang, field, text))
        }
        ("approved_by", Value::String(text)) => Uuid::parse_str(text).ok().map(|id| id.to_string()),
        ("locale" | "legal_effective_date", Value::String(text)) if !text.is_empty() => {
            Some(text.clone())
        }
        _ => None,
    }
}

fn changes(lang: &str, action: &str, target: &str, before: &Value, after: &Value) -> Vec<Change> {
    // Older page deletions stored the submitted form in the after snapshot.
    let after = if action == "cms.page.deleted" {
        &Value::Null
    } else {
        after
    };
    fields(action, target)
        .filter_map(|field| {
            let previous = before.get(field).unwrap_or(&Value::Null);
            let current = after.get(field).unwrap_or(&Value::Null);
            if previous == current {
                return None;
            }
            let (mut previous_display, mut current_display) =
                if matches!(field, "published_at" | "translation_published_at") {
                    let previous = publication_time(previous);
                    let current = publication_time(current);
                    if previous == current && previous != PublicationTime::Unknown {
                        return None;
                    }
                    (previous.display(lang), current.display(lang))
                } else {
                    (
                        display(lang, field, previous),
                        display(lang, field, current),
                    )
                };
            if hidden_field(field) && current_display.is_some() {
                current_display = Some(tr(lang, "audit-diff-hidden-change"));
            }
            if field == "appeal_status"
                && before.is_object()
                && previous.is_null()
                && current_display.is_some()
            {
                // Legacy sanction edits recorded the current workflow status only in after.
                // An appeal decision can still show its recorded outcome with an unknown prior value.
                if action != "support.enforcement_appeal_updated" {
                    return None;
                }
                previous_display = Some(tr(lang, "audit-diff-unknown"));
            }
            if (previous_display.is_none() && current_display.is_none())
                || (!matches!(field, "email" | "published_at" | "translation_published_at")
                    && previous_display == current_display)
            {
                return None;
            }
            Some(Change {
                field: tr(lang, &format!("audit-field-{}", field.replace('_', "-"))),
                before: previous_display.unwrap_or_else(|| "—".into()),
                after: current_display.unwrap_or_else(|| "—".into()),
            })
        })
        .collect()
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    request_id: Uuid,
    actor: Option<Uuid>,
    action: String,
    operation: String,
    target: Option<String>,
    recorded_at: String,
    before: Value,
    after: Value,
}

impl AuditRow {
    fn target_summary(&self, lang: &str) -> std::borrow::Cow<'_, str> {
        match &self.target {
            Some(target) => target.as_str().into(),
            None => tr(lang, "audit-target-hidden").into(),
        }
    }

    fn role_context(&self, lang: &str) -> String {
        if !matches!(
            self.action.as_str(),
            "rbac.role.granted" | "rbac.role.revoked"
        ) {
            return String::new();
        }
        [&self.after, &self.before]
            .into_iter()
            .filter_map(|snapshot| snapshot.get("role")?.as_str())
            .find_map(|role| Uuid::parse_str(role).ok())
            .map(|role| format!(" · {} {role}", tr(lang, "audit-field-role")))
            .unwrap_or_default()
    }
}

async fn platform_rows(pool: &sqlx::PgPool, filter: &Filter) -> Result<Vec<AuditRow>, sqlx::Error> {
    let allowed: serde_json::Map<String, Value> = ACTION_FIELDS
        .iter()
        .flat_map(|(actions, fields)| {
            let fields = serde_json::json!(fields);
            actions
                .iter()
                .map(move |action| ((*action).to_owned(), fields.clone()))
        })
        .collect();
    sqlx::query_as(include_str!("audit_rows.sql"))
        .bind(filter.request_id)
        .bind(filter.admin)
        .bind(&filter.operation)
        .bind(&filter.target)
        .bind(Value::Object(allowed))
        .bind(EDITORIAL_PAGE_ACTIONS)
        .bind(LEGAL_PAGE_TARGETS)
        .bind(serde_json::json!(LEGAL_PAGE_FIELDS))
        .bind(crate::audit::EDITORIAL_PAGE_PATH_PREFIX)
        .bind(crate::audit::EDITORIAL_PAGE_ROUTE)
        .fetch_all(pool)
        .await
}

pub(crate) async fn page<B: Backend>(
    State(state): State<Arc<Console<B>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    headers: axum::http::HeaderMap,
    Query(filter): Query<Filter>,
) -> Result<Response, Error> {
    let platform = state.check(actor, "audit.view").await.is_ok();
    let game = state.check(actor, "game.audit.read").await.is_ok();
    if !platform && !game {
        return Err(Error::Denied);
    }
    filter.validate()?;
    let lang = language(filter.lang.as_deref())?;
    let mut page = Page::new(lang, "audit");
    page.filters = true;
    if platform {
        let rows = platform_rows(&state.pool, &filter).await?;
        page.audit_diff = Some(DiffLabels {
            field: tr(lang, "audit-diff-field"),
            before: tr(lang, "audit-diff-before"),
            after: tr(lang, "audit-diff-after"),
        });
        page.section_with_lines(
            "platform-source",
            Some(
                rows.into_iter()
                    .map(|row| SectionLine {
                        text: format!(
                            "{} · {} · {} · {} · {}{}",
                            row.request_id,
                            row.actor.map(|id| id.to_string()).unwrap_or_default(),
                            row.operation,
                            row.target_summary(lang),
                            row.recorded_at,
                            row.role_context(lang),
                        ),
                        changes: changes(
                            lang,
                            &row.action,
                            row.target.as_deref().unwrap_or_default(),
                            &row.before,
                            &row.after,
                        ),
                    })
                    .collect(),
            ),
        );
    }
    let ledger = if game && state.backend.advertised("admin.ledger.query").await {
        state
            .backend
            .for_navigation(session, &headers)
            .ledger(actor, &filter)
            .await?
    } else {
        None
    };
    if let Some(snapshot) = &ledger {
        super::players::record_read(
            &state.pool,
            actor,
            snapshot.query_id,
            "ledger",
            "admin.ledger.query",
        )
        .await?;
    }
    page.section(
        "game-source",
        ledger.map(|snapshot| {
            snapshot
                .rows
                .into_iter()
                .filter(|r| {
                    filter.request_id.is_none_or(|id| id == r.request_id)
                        && filter.admin.is_none_or(|id| id.to_string() == r.admin)
                        && filter
                            .operation
                            .as_ref()
                            .is_none_or(|op| op == &r.operation)
                        && filter
                            .target
                            .as_ref()
                            .is_none_or(|target| target == &r.target)
                })
                .take(100)
                .map(|r| {
                    format!(
                        "{} · {} · {} · {} · {} · {} · {}",
                        r.request_id,
                        r.ledger_id,
                        r.admin,
                        r.operation,
                        r.target,
                        r.status,
                        r.observed_at
                    )
                })
                .collect()
        }),
    );
    page.source = tr(
        lang,
        if platform {
            "combined-source"
        } else {
            "game-source"
        },
    );
    page.response()
}

#[cfg(test)]
#[path = "audit_tests.rs"]
mod tests;
