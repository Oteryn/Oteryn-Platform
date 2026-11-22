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
    target: String,
    recorded_at: String,
    before: Value,
    after: Value,
}

impl AuditRow {
    fn target_summary(&self, lang: &str) -> std::borrow::Cow<'_, str> {
        if EDITORIAL_PAGE_ACTIONS.contains(&self.action.as_str())
            && !LEGAL_PAGE_TARGETS.contains(&self.target.as_str())
        {
            tr(lang, "audit-target-hidden").into()
        } else {
            self.target.as_str().into()
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
                        changes: changes(lang, &row.action, &row.target, &row.before, &row.after),
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

    fn diff(lang: &str, action: &str, before: &Value, after: &Value) -> Vec<Change> {
        changes(lang, action, "terms-of-service", before, after)
    }

    #[tokio::test]
    async fn postgres_audit_projects_only_fields_for_the_stored_action_and_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let _guard = oteryn_db::test_support::global_lock_guard().await?;
        let db = super::super::players::tests::Database::open(&std::env::var(
            "PORTAL_TEST_DATABASE_URL",
        )?)
        .await?;
        let request_id = Uuid::now_v7();
        for (action, target, before, after) in [
            (
                "cms.news_updated",
                "news",
                json!({"body":"x".repeat(100_000),"body_pl":"y".repeat(100_000),"status":"active","title":"old"}),
                json!({"body":"x".repeat(100_000),"body_pl":"y".repeat(100_000),"title_pl":"new","totp_secret":"excluded"}),
            ),
            (
                "cms.page.saved",
                "draft-client-case-4821",
                Value::Null,
                json!({"body":"excluded", "title":"draft","legal_version":"discarded","legal_effective_date":"2020-01-01"}),
            ),
            (
                "cms.page.saved",
                "privacy-policy",
                json!({"body":"excluded"}),
                json!({"legal_version":"v1","legal_effective_date":"2020-01-01","title":"Policy"}),
            ),
            (
                "rbac.role.granted",
                "account",
                json!({"assigned":false,"permissions":["excluded"]}),
                json!({"assigned":true,"role":"00000000-0000-7000-8000-000000000001"}),
            ),
        ] {
            let mut tx = db.pool.begin().await?;
            crate::audit::append(
                &mut tx,
                crate::audit::Entry {
                    context: crate::audit::AuditContext {
                        actor: None,
                        request_id,
                    },
                    action,
                    target,
                    before,
                    after,
                },
            )
            .await?;
            tx.commit().await?;
        }
        let rows = platform_rows(
            &db.pool,
            &Filter {
                request_id: Some(request_id),
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(rows.len(), 4);
        assert_eq!(
            (&rows[3].before, &rows[3].after),
            (&json!({"title":"old"}), &json!({"title_pl":"new"}))
        );
        assert_eq!(
            (&rows[2].before, &rows[2].after),
            (&Value::Null, &json!({"title":"draft"}))
        );
        assert_eq!(
            (&rows[1].before, &rows[1].after),
            (
                &json!({}),
                &json!({"legal_version":"v1","legal_effective_date":"2020-01-01","title":"Policy"})
            )
        );
        assert_eq!(
            (&rows[0].before, &rows[0].after),
            (
                &json!({"assigned":false}),
                &json!({"assigned":true,"role":"00000000-0000-7000-8000-000000000001"})
            )
        );
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn postgres_audit_normalizes_timestamp_offsets_and_fractional_seconds()
    -> Result<(), Box<dyn std::error::Error>> {
        let _guard = oteryn_db::test_support::global_lock_guard().await?;
        let db = super::super::players::tests::Database::open(&std::env::var(
            "PORTAL_TEST_DATABASE_URL",
        )?)
        .await?;
        let request_id = Uuid::now_v7();
        for (legacy, canonical) in [
            ("2020-01-01 00:00:00+0000", "2020-01-01T00:00:00Z"),
            ("2020-01-01 00:00:00-0230", "2020-01-01T02:30:00Z"),
            ("2020-01-01 00:00:00+01:30:59", "2019-12-31T22:29:01Z"),
            (
                "2020-01-01 00:00:00.1234565Z",
                "2020-01-01T00:00:00.123456Z",
            ),
            ("2020-01-01 00:00:00.0002495Z", "2020-01-01T00:00:00.00025Z"),
            ("2020-01-01 23:59:59.9999995Z", "2020-01-02T00:00:00Z"),
        ] {
            let mut tx = db.pool.begin().await?;
            crate::audit::append(
                &mut tx,
                crate::audit::Entry {
                    context: crate::audit::AuditContext {
                        actor: None,
                        request_id,
                    },
                    action: "cms.news_updated",
                    target: legacy,
                    before: json!({"published_at":canonical,"translation_published_at":canonical}),
                    after: json!({"published_at":legacy,"translation_published_at":legacy}),
                },
            )
            .await?;
            tx.commit().await?;
        }
        let mut tx = db.pool.begin().await?;
        crate::audit::append(&mut tx, crate::audit::Entry {
            context: crate::audit::AuditContext { actor: None, request_id },
            action: "cms.news_updated",
            target: "invalid",
            before: json!({"published_at":null,"translation_published_at":null}),
            after: json!({"published_at":"today","translation_published_at":"2020-02-30T00:00:00+0000"}),
        }).await?;
        tx.commit().await?;

        let rows = platform_rows(
            &db.pool,
            &Filter {
                request_id: Some(request_id),
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(rows.len(), 7);
        for row in rows {
            let changes = changes("en", &row.action, &row.target, &row.before, &row.after);
            if row.target == "invalid" {
                assert_eq!(changes.len(), 2);
                assert!(
                    changes
                        .iter()
                        .all(|change| change.before == "—" && change.after == "Unknown")
                );
            } else {
                assert_eq!(row.before, row.after, "{}", row.target);
                assert!(changes.is_empty(), "{}", row.target);
            }
        }
        db.close().await?;
        Ok(())
    }

    #[test]
    fn diff_masks_email_and_omits_credentials_unknown_fields_and_unchanged_values() {
        let before = json!({"email":"alice@example.invalid", "title":"Private draft alice@example.invalid", "slug":"same"});
        let after = json!({
            "email":"amber@example.invalid",
            "title":"Private draft 192.0.2.42",
            "slug":"same",
            "password_hash":"placeholder",
            "access_token":"placeholder",
            "totp_secret":"placeholder",
            "recovery_codes":["placeholder"],
            "body":"free text"
        });
        assert_eq!(
            diff("en", "cms.news_updated", &before, &after),
            [
                Change {
                    field: "Title".into(),
                    before: "Value hidden".into(),
                    after: "Changed (value hidden)".into()
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
        assert!(diff("en", "unknown", &Value::Null, &json!({"title":"private"})).is_empty());
        assert!(
            diff(
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
            diff(
                "en",
                "cms.page.deleted",
                &json!({"title":"Original"}),
                &json!({"title":"Submitted"})
            ),
            [Change {
                field: "Title".into(),
                before: "Value hidden".into(),
                after: "—".into()
            }]
        );
    }

    #[test]
    fn equivalent_legacy_publication_timestamps_do_not_create_a_change() {
        let before = json!({"published_at":"2020-01-01T00:00:00+00:00", "translation_published_at":"2020-01-01 00:00:00+00"});
        let after = json!({"published_at":"2020-01-01T00:00:00Z", "translation_published_at":"2020-01-01T01:00:00+01:00"});
        assert!(diff("en", "cms.news_updated", &before, &after).is_empty());
        assert!(diff("en", "cms.page.saved", &before, &after).is_empty());
        assert_eq!(
            diff(
                "en",
                "cms.page.saved",
                &before,
                &json!({"published_at":"2020-01-02T01:00:00+01:00"})
            ),
            [Change {
                field: "Publication time".into(),
                before: "2020-01-01T00:00:00Z".into(),
                after: "2020-01-02T00:00:00Z".into(),
            }]
        );
    }

    #[test]
    fn diff_hides_embedded_pii_in_legal_versions() {
        assert_eq!(
            diff(
                "en",
                "cms.page.saved",
                &json!({"legal_version":"v1 reviewer@example.invalid"}),
                &json!({"legal_version":"v2 2001:db8::42"}),
            ),
            [Change {
                field: "Legal version".into(),
                before: "Value hidden".into(),
                after: "Changed (value hidden)".into(),
            }]
        );
    }

    #[test]
    fn diff_hides_editorial_slugs_and_custom_role_keys() {
        for (action, field, label) in [
            ("cms.news_updated", "slug", "URL slug"),
            ("rbac.role.created", "key", "Role key"),
        ] {
            assert_eq!(
                diff(
                    "en",
                    action,
                    &Value::Null,
                    &json!({(field):"private-case-42"})
                ),
                [Change {
                    field: label.into(),
                    before: "—".into(),
                    after: "Changed (value hidden)".into(),
                }]
            );
        }
    }

    #[test]
    fn diff_distinguishes_unparseable_historical_timestamps_from_absence() {
        for (before, after, previous, current) in [
            (Value::Null, json!("today"), "—", "Unknown"),
            (json!("today"), json!("tomorrow"), "Unknown", "Unknown"),
            (
                json!("today"),
                json!("2020-01-01T00:00:00Z"),
                "Unknown",
                "2020-01-01T00:00:00Z",
            ),
        ] {
            assert_eq!(
                diff(
                    "en",
                    "cms.news_updated",
                    &json!({"published_at":before}),
                    &json!({"published_at":after})
                ),
                [Change {
                    field: "Publication time".into(),
                    before: previous.into(),
                    after: current.into()
                }]
            );
        }
    }

    #[test]
    fn diff_ignores_legacy_legal_inputs_on_non_legal_pages() {
        let before = json!({"locale":"en","title":"Previous"});
        let after = json!({"locale":"en","title":"Updated","legal_version":"discarded","legal_effective_date":"2020-01-01"});
        let changes = changes("en", "cms.page.saved", "support", &before, &after);
        assert_eq!(
            changes,
            [Change {
                field: "Title".into(),
                before: "Value hidden".into(),
                after: "Changed (value hidden)".into()
            }]
        );
        let legal_changes = diff("en", "cms.page.saved", &before, &after);
        assert_eq!(legal_changes.len(), 3);
        assert_eq!(legal_changes[2].after, "2020-01-01");
    }

    #[test]
    fn diff_localizes_known_workflow_values_in_both_languages() {
        for (lang, values) in [
            (
                "en",
                [
                    "Draft",
                    "Published",
                    "Under review",
                    "Accepted",
                    "Warning",
                    "Restriction",
                ],
            ),
            (
                "pl",
                [
                    "Szkic",
                    "Opublikowany",
                    "W trakcie rozpatrywania",
                    "Uwzględnione",
                    "Ostrzeżenie",
                    "Ograniczenie",
                ],
            ),
        ] {
            for ((field, value), label) in [
                ("state", "draft"),
                ("status", "published"),
                ("appeal_status", "reviewing"),
                ("appeal_status", "accepted"),
                ("category", "warning"),
                ("category", "restriction"),
            ]
            .into_iter()
            .zip(values)
            {
                assert_eq!(display(lang, field, &json!(value)).as_deref(), Some(label));
            }
        }
    }

    #[test]
    fn a_legacy_sanction_edit_does_not_infer_an_appeal_status_change() {
        let before = json!({"status":"active"});
        let after = json!({"status":"revoked","appeal_status":"reviewing"});
        assert_eq!(
            diff("en", "support.enforcement_updated", &before, &after),
            [Change {
                field: "Status".into(),
                before: "Active".into(),
                after: "Revoked".into()
            }]
        );
    }

    #[test]
    fn a_legacy_appeal_snapshot_marks_missing_prior_status_unknown() {
        assert_eq!(
            diff(
                "en",
                "support.enforcement_appeal_updated",
                &json!({"status":"active"}),
                &json!({"status":"active","appeal_status":"reviewing"})
            ),
            [Change {
                field: "Appeal status".into(),
                before: "Unknown".into(),
                after: "Under review".into()
            }]
        );
    }

    #[test]
    fn diff_distinguishes_a_missing_value_from_an_empty_permission_set() {
        assert_eq!(
            diff(
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
