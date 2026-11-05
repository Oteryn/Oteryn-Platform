//! R1 player views. Only allowlisted fields reach templates; personal fields are
//! masked even when present in a verified response. Reads fail if auditing fails.
use super::access::{Backend, Console, Error, Page, language, tr};
use crate::{
    audit::{
        AdminControlAudit, AdminControlAuditWriter, AuditContext, AuditOutcome, ReadView, RiskClass,
    },
    rbac::CheckContext,
};
use axum::{
    Extension,
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub lang: Option<String>,
    pub q: Option<String>,
    pub character: Option<String>,
}
impl Input {
    pub fn validate(&self) -> Result<(), Error> {
        language(self.lang.as_deref())?;
        if self.q.is_some() && self.character.is_some() {
            return Err(Error::BadInput);
        }
        if let Some(q) = &self.q
            && (!(2..=64).contains(&q.chars().count())
                || !q
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'')))
        {
            return Err(Error::BadInput);
        }
        if let Some(id) = &self.character
            && (id.is_empty()
                || id.len() > 80
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub world: String,
    pub level: u32,
    pub dossier: Option<Dossier>,
}
/// No contact details, IP addresses, credentials or payment fields.
#[derive(Debug, Clone)]
pub struct Dossier {
    pub vocation: String,
    pub skills: Vec<(String, u32)>,
    pub position: String,
    pub online: bool,
    pub premium_until: Option<String>,
    pub guild: Option<String>,
    pub house: Option<String>,
    pub bank_balance: String,
    pub last_login_times: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Item {
    pub instance_id: String,
    pub type_id: String,
    pub location: String,
    pub count: String,
}
#[derive(Debug, Clone)]
pub struct Sanction {
    pub sanction_id: String,
    pub kind: String,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Correlated request id from the verified result, not a browser-supplied id.
    pub request_id: Uuid,
    pub observed_at: String,
    pub characters: Option<Vec<Character>>,
    pub inventory: Option<Vec<Item>>,
    pub sanctions: Option<Vec<Sanction>>,
}
/// The shared audit writer stores one R1 event before releasing the view.
pub(crate) async fn record_read(
    pool: &sqlx::PgPool,
    actor: CheckContext,
    request_id: Uuid,
    target: &str,
    operation: &str,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let record = AdminControlAudit {
        context: AuditContext {
            actor: Some(actor.account),
            request_id,
        },
        admin_operation_id: request_id,
        operation: operation.to_owned(),
        risk_class: RiskClass::R1,
        target: if operation == "player.search" {
            json!({"view":"search"})
        } else if operation == "admin.ledger.query" {
            json!({"view":"ledger"})
        } else {
            json!({"character_id":target})
        },
        reason: None,
        approvals: json!([]),
        ip: None,
        user_agent: None,
        correlation_id: request_id,
        outcome: AuditOutcome::Succeeded,
        game_ledger_id: None,
    };
    let writer = AdminControlAuditWriter { enabled: true };
    if operation == ReadView::Dossier.operation() {
        writer.read_audit(&mut tx, ReadView::Dossier, record).await
    } else {
        writer.append(&mut tx, record).await
    }
    .map_err(|_| Error::Unavailable)?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn view<B: Backend>(
    state: &Console<B>,
    actor: CheckContext,
    input: Input,
) -> Result<Page, Error> {
    state.check(actor, "game.player.read").await?;
    input.validate()?;
    let lang = language(input.lang.as_deref())?;
    let mut page = Page::new(lang, "players");
    page.search = true;
    let dossier = input.character.is_some();
    let operation = if dossier {
        "player.character.get"
    } else {
        "player.search"
    };
    let has_input = dossier || input.q.is_some();
    let result = if has_input && state.backend.advertised(operation).await {
        state.backend.players(actor, &input, operation).await?
    } else {
        None
    };
    if let Some(snapshot) = result {
        if snapshot.request_id.is_nil() {
            return Err(Error::Unavailable);
        }
        // One durable row for this view, regardless of how many sections it has.
        record_read(
            &state.pool,
            actor,
            snapshot.request_id,
            input.character.as_deref().unwrap_or("search"),
            operation,
        )
        .await?;
        page.source = format!(
            "{} · {} · {}",
            tr(lang, "game-source"),
            snapshot.observed_at,
            snapshot.request_id
        );
        page.section(
            "characters",
            snapshot.characters.map(|rows| {
                rows.into_iter()
                    .take(100)
                    .map(|c| {
                        let base = format!("{} · {} · {} · {}", c.id, c.name, c.world, c.level);
                        if let Some(d) = c.dossier.filter(|_| dossier) {
                            format!("{base} · {}: {} · {}: {:?} · {}: {} · {}: {} · {}: {} · {}: {} · {}: {} · {}: {} · {}: {}",
                                tr(lang,"vocation"),d.vocation,tr(lang,"skills"),d.skills,
                                tr(lang,"position"),d.position,tr(lang,"online"),tr(lang,if d.online {"yes"} else {"no"}),
                                tr(lang,"premium"),d.premium_until.unwrap_or_default(),tr(lang,"guild"),d.guild.unwrap_or_default(),
                                tr(lang,"house"),d.house.unwrap_or_default(),tr(lang,"bank"),d.bank_balance,tr(lang,"logins"),d.last_login_times.join(", "))
                        } else { base }
                    })
                    .collect()
            }),
        );
        page.section("personal", Some(vec![tr(lang, "masked")]));
        let inventory = if dossier && state.backend.advertised("player.inventory.get").await {
            state
                .backend
                .players(actor, &input, "player.inventory.get")
                .await?
                .filter(|s| !s.request_id.is_nil())
                .and_then(|s| s.inventory)
        } else {
            None
        };
        page.section(
            "inventory",
            inventory.map(|rows| {
                rows.into_iter()
                    .map(|i| {
                        format!(
                            "{} · {} · {} · {}",
                            i.instance_id, i.type_id, i.location, i.count
                        )
                    })
                    .collect()
            }),
        );
        let sanctions = if dossier
            && state.backend.permitted(actor, "game.sanction.read").await
            && state.backend.advertised("sanction.list").await
        {
            state
                .backend
                .players(actor, &input, "sanction.list")
                .await?
                .filter(|s| !s.request_id.is_nil())
                .and_then(|s| s.sanctions)
        } else {
            None
        };
        page.section(
            "sanctions",
            sanctions.map(|rows| {
                rows.into_iter()
                    .map(|s| format!("{} · {} · {}", s.sanction_id, s.kind, s.status))
                    .collect()
            }),
        );
    } else {
        for title in ["characters", "inventory", "sanctions"] {
            page.section(title, None);
        }
    }
    Ok(page)
}
pub(crate) async fn page<B: Backend>(
    State(state): State<Arc<Console<B>>>,
    Extension(actor): Extension<CheckContext>,
    Query(input): Query<Input>,
) -> Result<Response, Error> {
    view(&state, actor, input).await?.response()
}

#[cfg(test)]
#[path = "players_tests.rs"]
mod tests;
