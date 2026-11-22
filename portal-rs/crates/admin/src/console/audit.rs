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

// Match the stored action, even when the list uses a separate operation label.
fn fields(action: &str) -> &'static [&'static str] {
    match action {
        "rbac.role.created" => &["key", "permissions"],
        "rbac.role.permissions_changed" => &["permissions"],
        "rbac.role.granted" | "rbac.role.revoked" => &["role", "assigned"],
        "players.kick" => &["native_security_generation"],
        "support.enforcement_created"
        | "support.enforcement_updated"
        | "support.enforcement_appeal_updated" => &[
            "category",
            "status",
            "pending",
            "effective_at_ms",
            "expires_at_ms",
            "appeal_status",
            "lock_version",
        ],
        "support.enforcement_approved" | "support.enforcement_revoked" => {
            &["status", "lock_version"]
        }
        "cms.news_created" | "cms.news_updated" | "cms.news_deleted" => &[
            "slug",
            "title",
            "title_pl",
            "published_at",
            "translation_published_at",
        ],
        "cms.page.saved" | "cms.page.deleted" => &[
            "locale",
            "title",
            "published_at",
            "legal_version",
            "legal_effective_date",
        ],
        "events.event_created" | "events.event_updated" => &["status"],
        "events.status_changed" => &["status", "version"],
        "portal.announcement_saved" => &["state"],
        _ => &[],
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
        (
            "key"
            | "role"
            | "category"
            | "status"
            | "appeal_status"
            | "state"
            | "slug"
            | "title"
            | "title_pl"
            | "published_at"
            | "translation_published_at"
            | "locale"
            | "legal_version"
            | "legal_effective_date",
            Value::String(text),
        ) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn changes(lang: &str, action: &str, before: &Value, after: &Value) -> Vec<Change> {
    // Older page deletions stored the submitted form in the after snapshot.
    let after = if action == "cms.page.deleted" {
        &Value::Null
    } else {
        after
    };
    // Only the explicitly named top-level email field may be displayed masked.
    fields(action)
        .iter()
        .copied()
        .chain(["email"])
        .filter_map(|field| {
            let previous = before.get(field).unwrap_or(&Value::Null);
            let current = after.get(field).unwrap_or(&Value::Null);
            if previous == current {
                return None;
            }
            let before = display(lang, field, previous);
            let after = display(lang, field, current);
            if (before.is_none() && after.is_none()) || (field != "email" && before == after) {
                return None;
            }
            Some(Change {
                field: tr(lang, &format!("audit-field-{}", field.replace('_', "-"))),
                before: before.unwrap_or_else(|| "—".into()),
                after: after.unwrap_or_else(|| "—".into()),
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
    target: String,
    recorded_at: String,
    before: Value,
    after: Value,
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
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT request_id, actor, action, COALESCE(operation,action) AS operation,
                    COALESCE(admin_control_target->>'character_id',admin_control_target->>'view',target) AS target,
                    created_at::text AS recorded_at, before, after
             FROM audit_log
             WHERE ($1::uuid IS NULL OR request_id=$1)
               AND ($2::uuid IS NULL OR actor=$2)
               AND ($3::text IS NULL OR COALESCE(operation,action)=$3)
               AND ($4::text IS NULL OR COALESCE(admin_control_target->>'character_id',admin_control_target->>'view',target)=$4)
             ORDER BY created_at DESC, id DESC LIMIT 100",
        )
        .bind(filter.request_id)
        .bind(filter.admin)
        .bind(&filter.operation)
        .bind(&filter.target)
        .fetch_all(&state.pool)
        .await?;
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
                            "{} · {} · {} · {} · {}",
                            row.request_id,
                            row.actor.map(|id| id.to_string()).unwrap_or_default(),
                            row.operation,
                            row.target,
                            row.recorded_at,
                        ),
                        changes: changes(lang, &row.action, &row.before, &row.after),
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diff_masks_email_and_omits_credentials_unknown_fields_and_unchanged_values() {
        let before = json!({"email":"alice@example.invalid", "title":"Old", "slug":"same"});
        let after = json!({
            "email":"amber@example.invalid",
            "title":"New",
            "slug":"same",
            "password_hash":"placeholder",
            "access_token":"placeholder",
            "totp_secret":"placeholder",
            "recovery_codes":["placeholder"],
            "body":"free text"
        });
        assert_eq!(
            changes("en", "cms.news_updated", &before, &after),
            [
                Change {
                    field: "Title".into(),
                    before: "Old".into(),
                    after: "New".into()
                },
                Change {
                    field: "Email".into(),
                    before: "a•••@•••".into(),
                    after: "a•••@•••".into()
                },
            ]
        );
    }

    #[test]
    fn diff_refuses_wrong_action_and_nested_or_malformed_values() {
        assert!(changes("en", "unknown", &Value::Null, &json!({"title":"private"})).is_empty());
        assert!(
            changes(
                "en",
                "rbac.role.created",
                &Value::Null,
                &json!({
                    "key":{"token":"placeholder"},
                    "permissions":["admin.access", {"token":"placeholder"}],
                    "email":"invalid"
                })
            )
            .is_empty()
        );
    }

    #[test]
    fn diff_treats_legacy_page_deletion_forms_as_an_absent_after_value() {
        assert_eq!(
            changes(
                "en",
                "cms.page.deleted",
                &json!({"title":"Original"}),
                &json!({"title":"Submitted"})
            ),
            [Change {
                field: "Title".into(),
                before: "Original".into(),
                after: "—".into()
            }]
        );
    }

    #[test]
    fn diff_distinguishes_a_missing_value_from_an_empty_permission_set() {
        assert_eq!(
            changes(
                "pl",
                "rbac.role.created",
                &Value::Null,
                &json!({"permissions":[]})
            ),
            [Change {
                field: "Uprawnienia".into(),
                before: "—".into(),
                after: "[]".into()
            }]
        );
    }
}
