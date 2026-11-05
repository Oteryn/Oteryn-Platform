//! Read-only pending R4 review. No mutation endpoint is mounted until the
//! authoritative approval service supplies step-up, distinct actor and digest checks.
use super::access::{Backend, Console, Error, Locale, Page, language, tr};
use crate::rbac::CheckContext;
use axum::{
    Extension,
    extract::{Query, State},
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct Pending {
    pub request_id: Uuid,
    pub requester: Uuid,
    pub operation: String,
    pub target: String,
    pub expires_at: String,
    pub preview_digest: String,
    pub preview: Vec<String>,
}
pub(crate) async fn page<B: Backend>(
    State(state): State<Arc<Console<B>>>,
    Extension(actor): Extension<CheckContext>,
    Query(input): Query<Locale>,
) -> Result<Response, Error> {
    state.check(actor, "admin.roles.manage").await?;
    let lang = language(input.lang.as_deref())?;
    let mut page = Page::new(lang, "approvals");
    let pending = state.backend.approvals(actor).await?;
    let lines = if let Some(rows) = pending {
        let mut lines = Vec::new();
        for row in rows.into_iter().take(100) {
            let preview = if state.backend.advertised(&row.operation).await {
                format!("{} · {}", row.preview_digest, row.preview.join(" · "))
            } else {
                tr(lang, "unavailable")
            };
            lines.push(format!(
                "R4 · {} · {} · {} · {} · {} · {}",
                row.request_id, row.requester, row.operation, row.target, row.expires_at, preview
            ));
        }
        Some(lines)
    } else {
        None
    };
    page.section("pending", lines);
    page.section("review", Some(vec![tr(lang, "read-only")]));
    page.response()
}
