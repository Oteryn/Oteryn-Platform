//! Platform incident assignments and an allowlisted projection of durable ADM history.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, AccountStateReader, CheckContext, MutationAuthorization, Permission},
    web::{Panel, StaffSession, language, message, messages},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use time::{Date, Duration, OffsetDateTime, format_description::well_known::Iso8601};
use uuid::Uuid;

const CASE_LIMIT: usize = 50;
const EVENT_LIMIT: usize = 100;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("invalid incident input")]
    Invalid,
    #[error("incident access refused")]
    Denied,
    #[error("incident not found")]
    Missing,
    #[error("incident assignment conflicts with current state")]
    Conflict,
    #[error("incident service unavailable")]
    Unavailable,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Denied => StatusCode::FORBIDDEN,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if error.as_database_error().and_then(|e| e.constraint())
            == Some("admin_incident_cases_alarm_id_key")
        {
            return Self::Conflict;
        }
        Self::Unavailable
    }
}

impl From<rbac::Error> for Error {
    fn from(error: rbac::Error) -> Self {
        match error {
            rbac::Error::Denied => Self::Denied,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Search {
    case_id: Option<Uuid>,
    receipt: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Dates {
    from: Option<String>,
    through: Option<String>,
}

#[derive(Debug)]
struct Window {
    from: Date,
    through: Date,
    end: Date,
}

impl Dates {
    fn window(&self, today: Date) -> Result<Window, Error> {
        let parse = |value: &str| {
            if value.len() != 10
                || !value.bytes().enumerate().all(|(index, byte)| {
                    if matches!(index, 4 | 7) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_digit()
                    }
                })
            {
                return Err(Error::Invalid);
            }
            Date::parse(value, &Iso8601::DATE).map_err(|_| Error::Invalid)
        };
        let through = self
            .through
            .as_deref()
            .map(parse)
            .transpose()?
            .unwrap_or(today);
        let from = match self.from.as_deref() {
            Some(value) => parse(value)?,
            None => through
                .checked_sub(Duration::days(29))
                .ok_or(Error::Invalid)?,
        };
        let end = through.next_day().ok_or(Error::Invalid)?;
        if from.year() < 1 || from > through || through > today || (end - from).whole_days() > 31 {
            return Err(Error::Invalid);
        }
        Ok(Window { from, through, end })
    }
}

fn identifier(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

fn case_id(id: Uuid) -> Result<Uuid, Error> {
    if id.get_version() == Some(uuid::Version::SortRand)
        && id.get_variant() == uuid::Variant::RFC4122
    {
        Ok(id)
    } else {
        Err(Error::Invalid)
    }
}

fn optional_uuid(value: &str) -> Result<Option<Uuid>, Error> {
    if value.is_empty() {
        return Ok(None);
    }
    let id: Uuid = value.parse().map_err(|_| Error::Invalid)?;
    if id.is_nil() {
        return Err(Error::Invalid);
    }
    Ok(Some(id))
}

#[derive(Debug, Deserialize)]
pub(super) struct Create {
    investigator: Uuid,
    world_id: String,
    node_id: String,
    character_id: String,
    account_ref: String,
    alarm_id: String,
}

impl Create {
    fn validate(&self) -> Result<(), Error> {
        if self.investigator.is_nil()
            || !identifier(&self.world_id)
            || [&self.character_id, &self.node_id]
                .into_iter()
                .any(|id| !id.is_empty() && !identifier(id))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct Assign {
    case_id: Uuid,
    investigator: Uuid,
}

#[derive(sqlx::FromRow)]
struct Case {
    id: Uuid,
    world_id: String,
    node_id: Option<String>,
    character_id: Option<String>,
    account_ref: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct Event {
    at: OffsetDateTime,
    kind: String,
    actor: Option<Uuid>,
    request_id: Option<Uuid>,
    operation: Option<String>,
    status: Option<String>,
    approval_id: Option<Uuid>,
}

impl Event {
    fn timestamp(&self) -> String {
        self.at
            .to_offset(time::UtcOffset::UTC)
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    }
}

#[derive(Template)]
#[template(path = "admin/incidents.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    bundle: FluentBundle<FluentResource>,
    cases: Vec<Case>,
    detail: Option<(Case, Window)>,
    receipt: Option<Uuid>,
    events: Vec<Event>,
    more: bool,
    csrf: Option<String>,
    unavailable: String,
}

impl Page {
    fn new(headers: &HeaderMap) -> Result<Self, Error> {
        let lang = language(headers);
        Ok(Self {
            lang,
            bundle: messages(lang).ok_or(Error::Unavailable)?,
            cases: Vec::new(),
            detail: None,
            receipt: None,
            events: Vec::new(),
            more: false,
            csrf: None,
            unavailable: crate::console::access::tr(lang, "unavailable"),
        })
    }

    fn label(&self, key: &str) -> String {
        message(&self.bundle, key)
    }

    fn event_label(&self, kind: &str) -> String {
        self.label(&format!("incident-event-{}", kind.replace(['.', '_'], "-")))
    }

    fn status_label(&self, status: &str) -> String {
        self.label(&format!("incident-status-{}", status.replace('_', "-")))
    }
}

async fn manage_form<I, A: AccountStateReader + 'static>(
    state: &Panel<I, A>,
    context: CheckContext,
    session: &BrowserSession,
) -> Result<Option<String>, Error> {
    match state.auth.check(context, Permission::ManageRoles).await {
        Ok(()) => session
            .csrf_token()
            .await
            .map(Some)
            .map_err(|_| Error::Unavailable),
        Err(rbac::Error::Denied) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn index<I: StaffSession, A: AccountStateReader + 'static>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(query): Query<Search>,
) -> Result<Response, Error> {
    if let Some(id) = query.case_id {
        return Ok(Redirect::to(&format!("/admin/incidents/{}", case_id(id)?)).into_response());
    }
    let csrf = manage_form(&state, context, &session).await?;
    let receipt = if let Some(id) = query.receipt {
        let id = case_id(id)?;
        let recorded: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM audit_log WHERE target=$1 AND actor=$2
             AND action IN ('incident.created', 'incident.assigned'))",
        )
        .bind(id.to_string())
        .bind(context.account)
        .fetch_one(&state.pool)
        .await?;
        if !recorded {
            return Err(Error::Missing);
        }
        Some(id)
    } else {
        None
    };
    let mut cases: Vec<Case> = sqlx::query_as(
        "SELECT id, world_id, node_id, character_id, account_ref FROM admin_incident_cases
         WHERE investigator=$1 ORDER BY id DESC LIMIT $2",
    )
    .bind(context.account)
    .bind((CASE_LIMIT + 1) as i64)
    .fetch_all(&state.pool)
    .await?;
    let mut page = Page::new(&headers)?;
    page.csrf = csrf;
    page.receipt = receipt;
    page.more = cases.len() > CASE_LIMIT;
    cases.truncate(CASE_LIMIT);
    page.cases = cases;
    Ok(oteryn_web::render(&page))
}

async fn load_timeline(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: Uuid,
    window: &Window,
) -> Result<(Case, Vec<Event>, bool), Error> {
    let case = sqlx::query_as(
        "SELECT id, world_id, node_id, character_id, account_ref
         FROM admin_incident_cases WHERE id=$1 AND investigator=$2 FOR SHARE",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::Missing)?;
    let mut events: Vec<Event> = sqlx::query_as(include_str!("timeline.sql"))
        .bind(id)
        .bind(window.from.midnight().assume_utc())
        .bind(window.end.midnight().assume_utc())
        .bind((EVENT_LIMIT + 1) as i64)
        .fetch_all(&mut **tx)
        .await?;
    let more = events.len() > EVENT_LIMIT;
    events.truncate(EVENT_LIMIT);
    record(
        tx,
        actor,
        id,
        "incident.viewed",
        json!(null),
        json!({"from":window.from.to_string(),"through":window.through.to_string(),
               "returned":events.len(),"truncated":more,"projection":"platform-summary-v1"}),
    )
    .await?;
    Ok((case, events, more))
}

pub(super) async fn detail<I: StaffSession, A: AccountStateReader + 'static>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(dates): Query<Dates>,
) -> Result<Response, Error> {
    let id = case_id(id)?;
    let window = dates.window(OffsetDateTime::now_utc().date())?;
    let mut tx = state.pool.begin().await?;
    let (case, events, more) = load_timeline(&mut tx, id, context.account, &window).await?;
    tx.commit().await?;
    let mut page = Page::new(&headers)?;
    page.detail = Some((case, window));
    page.events = events;
    page.more = more;
    Ok(oteryn_web::render(&page))
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    id: Uuid,
    action: &str,
    before: serde_json::Value,
    after: serde_json::Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await?;
    Ok(())
}

const MANAGE: [Permission; 3] = [
    Permission::Access,
    Permission::ViewAudit,
    Permission::ManageRoles,
];

async fn eligible<A: AccountStateReader>(
    tx: &mut Transaction<'_, Postgres>,
    auth: &rbac::Authorizer<A>,
    investigator: Uuid,
) -> Result<(), Error> {
    for permission in [Permission::Access, Permission::ViewAudit] {
        auth.check_subject_in(tx, investigator, permission).await?;
    }
    Ok(())
}

pub(super) async fn create<I: StaffSession, A: AccountStateReader + 'static>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Form(form): Form<Create>,
) -> Result<Redirect, Error> {
    form.validate()?;
    let account_ref = optional_uuid(&form.account_ref)?;
    let alarm_id = optional_uuid(&form.alarm_id)?;
    let mut tx = state.pool.begin().await?;
    rbac::authorize_mutation(&mut tx, &[form.investigator], &auth, context, &MANAGE).await?;
    eligible(&mut tx, &state.auth, form.investigator).await?;
    if let Some(alarm) = alarm_id {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM audit_log WHERE id=$1 AND actor IS NULL
             AND action='ops.playerdata_binding_conflict')",
        )
        .bind(alarm)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::Invalid);
        }
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO admin_incident_cases (investigator, world_id, node_id, character_id, account_ref, alarm_id)
         VALUES($1,$2,NULLIF($3,''),NULLIF($4,''),$5,$6) RETURNING id",
    )
    .bind(form.investigator)
    .bind(form.world_id)
    .bind(form.node_id)
    .bind(form.character_id)
    .bind(account_ref)
    .bind(alarm_id)
    .fetch_one(&mut *tx)
    .await?;
    record(
        &mut tx,
        context.account,
        id,
        "incident.created",
        json!(null),
        json!({"investigator":form.investigator,"alarm_id":alarm_id}),
    )
    .await?;
    tx.commit().await?;
    if form.investigator == context.account {
        Ok(Redirect::to(&format!("/admin/incidents/{id}")))
    } else {
        Ok(Redirect::to(&format!("/admin/incidents?receipt={id}")))
    }
}

pub(super) async fn assign<I: StaffSession, A: AccountStateReader + 'static>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Form(form): Form<Assign>,
) -> Result<Redirect, Error> {
    let id = case_id(form.case_id)?;
    if form.investigator.is_nil() {
        return Err(Error::Invalid);
    }
    let mut tx = state.pool.begin().await?;
    rbac::authorize_mutation_with_resources(
        &mut tx,
        &[form.investigator],
        |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM admin_incident_cases WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &MANAGE,
    )
    .await?;
    eligible(&mut tx, &state.auth, form.investigator).await?;
    let previous: Uuid =
        sqlx::query_scalar("SELECT investigator FROM admin_incident_cases WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Missing)?;
    if previous == form.investigator {
        return Err(Error::Conflict);
    }
    sqlx::query("UPDATE admin_incident_cases SET investigator=$2 WHERE id=$1")
        .bind(id)
        .bind(form.investigator)
        .execute(&mut *tx)
        .await?;
    record(
        &mut tx,
        context.account,
        id,
        "incident.assigned",
        json!({"investigator":previous}),
        json!({"investigator":form.investigator}),
    )
    .await?;
    tx.commit().await?;
    if form.investigator == context.account {
        Ok(Redirect::to(&format!("/admin/incidents/{id}")))
    } else {
        Ok(Redirect::to(&format!("/admin/incidents?receipt={id}")))
    }
}

#[cfg(test)]
mod tests;
