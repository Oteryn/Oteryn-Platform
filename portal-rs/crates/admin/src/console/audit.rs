//! Combined Platform audit and verified Game ledger, joined visually by request id.
use super::access::{Backend, Console, Error, Page, language, tr};
use crate::rbac::CheckContext;
use axum::{
    Extension,
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
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
pub struct LedgerRow {
    pub request_id: Uuid,
    pub ledger_id: String,
    pub admin: String,
    pub operation: String,
    pub target: String,
    pub status: String,
    pub observed_at: String,
}
pub(crate) async fn page<B: Backend>(
    State(state): State<Arc<Console<B>>>,
    Extension(actor): Extension<CheckContext>,
    Query(filter): Query<Filter>,
) -> Result<Response, Error> {
    state.check(actor, "audit.view").await?;
    filter.validate()?;
    let lang = language(filter.lang.as_deref())?;
    let rows: Vec<(Uuid,Option<Uuid>,String,String,String)> = sqlx::query_as("SELECT request_id, actor, COALESCE(operation,action), COALESCE(admin_control_target->>'character_id',admin_control_target->>'view',target), created_at::text FROM audit_log WHERE ($1::uuid IS NULL OR request_id=$1) AND ($2::uuid IS NULL OR actor=$2) AND ($3::text IS NULL OR COALESCE(operation,action)=$3) AND ($4::text IS NULL OR COALESCE(admin_control_target->>'character_id',admin_control_target->>'view',target)=$4) ORDER BY created_at DESC, id DESC LIMIT 100")
        .bind(filter.request_id).bind(filter.admin).bind(&filter.operation).bind(&filter.target).fetch_all(&state.pool).await?;
    let mut page = Page::new(lang, "audit");
    page.filters = true;
    page.section(
        "platform-source",
        Some(
            rows.into_iter()
                .map(|(request, admin, operation, target, time)| {
                    format!(
                        "{request} · {} · {operation} · {target} · {time}",
                        admin.map(|id| id.to_string()).unwrap_or_default()
                    )
                })
                .collect(),
        ),
    );
    let ledger = if state.backend.permitted(actor, "game.audit.read").await
        && state.backend.advertised("admin.ledger.query").await
    {
        state.backend.ledger(actor, &filter).await?
    } else {
        None
    };
    if ledger.is_some() {
        super::players::record_read(
            &state.pool,
            actor,
            Uuid::now_v7(),
            "ledger",
            "admin.ledger.query",
        )
        .await?;
    }
    page.section(
        "game-source",
        ledger.map(|rows| {
            rows.into_iter()
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
    page.source = tr(lang, "combined-source");
    page.response()
}
