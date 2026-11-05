//! R0 console. All Game reads and query submissions use the Platform channel store.
//! P1A/P2A are absent: the configured channel fails closed until their adapters exist.
//! Adapters must resolve current hello ownership, verify results, bound R0 fields and
//! enforce request ownership. No network client or guessed channel schema lives here.
pub mod access;
pub mod approvals;
pub mod audit;
pub mod players;

mod dashboard;
mod database;
mod nodes;
mod worlds;

use crate::{
    rbac::{self, AccountStateReader, CheckContext, Permission},
    web,
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    future::Future,
    sync::Arc,
    time::{Duration, SystemTime},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Operation {
    #[serde(rename = "ops.overview.get")]
    Overview,
    #[serde(rename = "world.list")]
    Worlds,
    #[serde(rename = "world.get")]
    World,
    #[serde(rename = "node.list")]
    Nodes,
    #[serde(rename = "node.get")]
    Node,
    #[serde(rename = "db.status")]
    Database,
    #[serde(rename = "db.migrations.list")]
    Migrations,
}
impl Operation {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Overview => "ops.overview.get",
            Self::Worlds => "world.list",
            Self::World => "world.get",
            Self::Nodes => "node.list",
            Self::Node => "node.get",
            Self::Database => "db.status",
            Self::Migrations => "db.migrations.list",
        }
    }
    pub const fn permission(self) -> &'static str {
        match self {
            Self::Overview => dashboard::PERMISSION,
            Self::Worlds | Self::World => worlds::PERMISSION,
            Self::Nodes | Self::Node => nodes::PERMISSION,
            Self::Database | Self::Migrations => database::PERMISSION,
        }
    }
    fn page(self) -> &'static str {
        match self {
            Self::Overview => "dashboard",
            Self::Worlds | Self::World => "worlds",
            Self::Nodes | Self::Node => "nodes",
            Self::Database | Self::Migrations => "database",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "dashboard",
            Self::Worlds => "worlds",
            Self::World => "world",
            Self::Nodes => "nodes",
            Self::Node => "node",
            Self::Database => "database",
            Self::Migrations => "migrations",
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("permission denied")]
    Denied,
    #[error("invalid query")]
    Invalid,
    #[error("channel unavailable")]
    Unavailable,
    #[error("storage unavailable")]
    Storage(#[from] sqlx::Error),
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Denied => StatusCode::FORBIDDEN,
            Self::Invalid => StatusCode::BAD_REQUEST,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}
/// Narrow permission interface pending P2A. The permission is always selected by a route.
pub trait Permissions: Send + Sync + 'static {
    fn check(
        &self,
        context: CheckContext,
        operation: Operation,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
#[derive(Debug, Clone)]
pub struct PgPermissions<A> {
    pub pool: PgPool,
    pub accounts: A,
}
impl<A: AccountStateReader + 'static> Permissions for PgPermissions<A> {
    async fn check(&self, context: CheckContext, operation: Operation) -> Result<(), Error> {
        if context.account.is_nil()
            || !context.require_staff_mfa
            || !context.mfa_verified
            || !matches!(
                self.accounts.state(context.account).await,
                Ok(rbac::AccountState::Active)
            )
        {
            return Err(Error::Denied);
        }
        let access: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_roles a JOIN role_permissions p ON p.role_id=a.role_id WHERE a.account_id=$1 AND p.permission='admin.access')").bind(context.account).fetch_one(&self.pool).await?;
        if !access {
            return Err(Error::Denied);
        }
        let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_roles a JOIN role_permissions p ON p.role_id=a.role_id WHERE a.account_id=$1 AND p.permission=$2)").bind(context.account).bind(operation.permission()).fetch_one(&self.pool).await?;
        if allowed { Ok(()) } else { Err(Error::Denied) }
    }
}
/// Only R0, allowlisted, privacy-safe fields may be supplied by a channel adapter.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: FieldLabel,
    pub value: String,
}
#[derive(Debug, Clone, Copy)]
pub enum FieldLabel {
    Players,
    Peak,
    Health,
    Build,
    Assignment,
    Cpu,
    Memory,
    Disk,
    Tick,
    State,
    Revision,
    Rates,
    Channels,
    Size,
    Connections,
    Locks,
    Lag,
    Bloat,
    OldestTransaction,
    CacheHit,
    BackupAge,
    Alerts,
    Applied,
    Pending,
    Drift,
    OnlineSample,
}
impl FieldLabel {
    fn key(self) -> &'static str {
        match self {
            Self::Players => "players",
            Self::Peak => "peak",
            Self::Health => "health",
            Self::Build => "build",
            Self::Assignment => "assignment",
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Disk => "disk",
            Self::Tick => "tick",
            Self::State => "state",
            Self::Revision => "revision",
            Self::Rates => "rates",
            Self::Channels => "channels",
            Self::Size => "size",
            Self::Connections => "connections",
            Self::Locks => "locks",
            Self::Lag => "lag",
            Self::Bloat => "bloat",
            Self::OldestTransaction => "oldest-transaction",
            Self::CacheHit => "cache-hit",
            Self::BackupAge => "backup-age",
            Self::Alerts => "alerts",
            Self::Applied => "applied",
            Self::Pending => "migration-pending",
            Self::Drift => "drift",
            Self::OnlineSample => "online-sample",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub owner_epoch: u64,
    pub request_id: Uuid,
    pub actor: Uuid,
    pub agent: String,
    pub operation: Operation,
    pub target: Option<String>,
    pub observed_at: SystemTime,
    pub revision: String,
    pub records: Vec<Record>,
}
#[derive(Debug, Clone)]
pub struct AgentView {
    pub owner_epoch: u64,
    pub last_contact: SystemTime,
    pub current_owner: bool,
    pub operations: Vec<(Operation, u16)>,
    pub result: Option<CachedResult>,
    pub status: QueryStatus,
}
/// The store returns status only after checking actor, target and request ownership.
#[derive(Debug, Clone, Copy)]
pub enum QueryStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Rejected,
    Expired,
    Cancelled,
}
impl QueryStatus {
    fn key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in-progress",
            Self::Succeeded => "unavailable",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }
}
/// Reads only admin_control_agents and stored results/rollups; never contacts Game.
/// enqueue must use P1A's signed R0 query store, fresh authority/capability checks,
/// a UUIDv7 request id, 60s expiry, audit_log, and per-actor concurrency limits.
pub trait Channel: Send + Sync + 'static {
    fn read(
        &self,
        actor: Uuid,
        agent: &str,
        operation: Operation,
        target: Option<&str>,
        request: Option<Uuid>,
    ) -> impl Future<Output = Result<AgentView, Error>> + Send;
    fn enqueue(
        &self,
        context: CheckContext,
        session: &BrowserSession,
        agent: &str,
        operation: Operation,
        target: Option<&str>,
    ) -> impl Future<Output = Result<Uuid, Error>> + Send;
}
#[derive(Debug, Clone, Copy)]
pub struct UnavailableChannel;
impl Channel for UnavailableChannel {
    async fn read(
        &self,
        _: Uuid,
        _: &str,
        _: Operation,
        _: Option<&str>,
        _: Option<Uuid>,
    ) -> Result<AgentView, Error> {
        Err(Error::Unavailable)
    }
    async fn enqueue(
        &self,
        _: CheckContext,
        _: &BrowserSession,
        _: &str,
        _: Operation,
        _: Option<&str>,
    ) -> Result<Uuid, Error> {
        Err(Error::Unavailable)
    }
}
#[derive(Debug)]
pub struct Console<P, C> {
    pub enabled: bool,
    pub permissions: P,
    pub channel: C,
    pub pool: PgPool,
}
/// Off by default; additionally requires the existing admin host/session/MFA guard.
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let console = Console {
        enabled: std::env::var("OTERYN_ADMIN_CONSOLE").is_ok_and(|v| v == "true"),
        permissions: PgPermissions {
            pool: pool.clone(),
            accounts: web::PendingAuthentication,
        },
        channel: UnavailableChannel,
        pool: pool.clone(),
    };
    web::configured_guard(routes(console), pool, Permission::Access)
}
/// Embedders must wrap these routes with web::guarded (trusted staff context + CSRF).
fn routes<P: Permissions, C: Channel>(console: Console<P, C>) -> Router {
    Router::new()
        .route(
            "/console/dashboard",
            get(dashboard_page::<P, C>).post(submit::<P, C>),
        )
        .route(
            "/console/worlds",
            get(worlds_page::<P, C>).post(submit::<P, C>),
        )
        .route(
            "/console/nodes",
            get(nodes_page::<P, C>).post(submit::<P, C>),
        )
        .route(
            "/console/database",
            get(database_page::<P, C>).post(submit::<P, C>),
        )
        .with_state(Arc::new(console))
}
/// Configured embedding with actual staff adapters, retaining the existing guard.
pub fn router<P: Permissions, C: Channel, I: web::StaffSession, A: AccountStateReader + 'static>(
    console: Console<P, C>,
    panel: web::PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let pool = console.pool.clone();
    web::guarded(
        routes(console),
        pool,
        panel,
        identity,
        accounts,
        Permission::Access,
    )
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    agent: Option<String>,
    target: Option<String>,
    request: Option<Uuid>,
    lang: Option<String>,
}
impl Selection {
    fn validate(&self) -> Result<(), Error> {
        for id in [&self.agent, &self.target].into_iter().flatten() {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            {
                return Err(Error::Invalid);
            }
        }
        if self.request.is_some_and(|id| id.get_version_num() != 7)
            || self
                .lang
                .as_deref()
                .is_some_and(|l| !matches!(l, "pl" | "en"))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryForm {
    operation: Operation,
    agent: String,
    target: Option<String>,
    lang: String,
    _csrf: String,
}
fn lang(headers: &HeaderMap, selection: &Selection) -> &'static str {
    if selection.lang.as_deref() == Some("en")
        || (selection.lang.is_none()
            && headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("en")))
    {
        "en"
    } else {
        "pl"
    }
}
fn text(lang: &str, key: &str) -> String {
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn connected(view: &AgentView, op: Operation, now: SystemTime) -> bool {
    view.owner_epoch > 0
        && view.current_owner
        && now
            .duration_since(view.last_contact)
            .is_ok_and(|age| age < Duration::from_secs(30))
        && view.operations.iter().filter(|(o, _)| *o == op).count() == 1
        && view.operations.contains(&(op, 1))
}
struct Section {
    title: String,
    source: String,
    freshness: String,
    state: String,
    revision: String,
    records: Vec<DisplayRecord>,
    operation: &'static str,
    can_query: bool,
}
struct DisplayRecord {
    name: String,
    fields: Vec<(String, String)>,
}
#[derive(Template)]
#[template(path = "page.html", config = "src/console/templates/askama.toml")]
struct Page {
    lang: &'static str,
    language_label: String,
    title: String,
    nav: Vec<(String, String)>,
    sections: Vec<Section>,
    source_label: String,
    freshness_label: String,
    revision_label: String,
    agent_label: String,
    target_label: String,
    select_label: String,
    query_label: String,
    poll_label: String,
    skip: String,
    note: String,
    csrf: String,
    agent: String,
    target: String,
    path: String,
    poll: String,
}
async fn render<P: Permissions, C: Channel>(
    s: Arc<Console<P, C>>,
    context: CheckContext,
    session: BrowserSession,
    headers: HeaderMap,
    q: Selection,
    operations: &[Operation],
) -> Result<Response, Error> {
    if !s.enabled {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    q.validate()?;
    if q.target.is_some()
        && !operations
            .iter()
            .all(|op| matches!(op, Operation::World | Operation::Node))
    {
        return Err(Error::Invalid);
    }
    for op in operations {
        s.permissions.check(context, *op).await?;
    }
    let language = lang(&headers, &q);
    let mut sections = Vec::new();
    for op in operations {
        let mut section = Section {
            title: text(language, op.title()),
            source: text(language, "game-source"),
            freshness: text(language, "unknown"),
            state: text(language, "unavailable"),
            revision: String::new(),
            records: vec![],
            operation: op.name(),
            can_query: false,
        };
        if let Some(agent) = &q.agent
            && let Ok(view) = s
                .channel
                .read(context.account, agent, *op, q.target.as_deref(), q.request)
                .await
            && connected(&view, *op, SystemTime::now())
        {
            section.can_query = true;
            section.state = text(language, view.status.key());
            if matches!(view.status, QueryStatus::Succeeded)
                && let Some(result) = view.result
                && result.owner_epoch == view.owner_epoch
                && result.request_id.get_version_num() == 7
                && result.actor == context.account
                && result.agent == *agent
                && result.operation == *op
                && result.target == q.target
                && q.request.is_none_or(|id| id == result.request_id)
                && !result.revision.is_empty()
                && let Ok(age) = SystemTime::now().duration_since(result.observed_at)
            {
                section.freshness = format!("{} {}", age.as_secs(), text(language, "seconds"));
                section.state = text(
                    language,
                    if age < Duration::from_secs(60) {
                        "fresh"
                    } else {
                        "stale"
                    },
                );
                section.revision = result.revision;
                section.records = result
                    .records
                    .into_iter()
                    .take(200)
                    .map(|r| DisplayRecord {
                        name: r.name,
                        fields: r
                            .fields
                            .into_iter()
                            .take(32)
                            .map(|f| (text(language, f.label.key()), f.value))
                            .collect(),
                    })
                    .collect();
            }
        }
        sections.push(section);
    }
    if operations.contains(&Operation::Overview) {
        sections.push(dashboard::section(&s.pool, language).await);
    }
    let mut nav = Vec::new();
    for op in [
        Operation::Overview,
        Operation::Worlds,
        Operation::Nodes,
        Operation::Database,
    ] {
        if s.permissions.check(context, op).await.is_ok() {
            nav.push((
                format!("/console/{}?lang={language}", op.page()),
                text(language, op.title()),
            ));
        }
    }
    let op = operations[0];
    let path = format!("/console/{}", op.page());
    let poll = format!(
        "{path}?lang={language}{}{}{}",
        q.agent
            .as_ref()
            .map(|a| format!("&agent={a}"))
            .unwrap_or_default(),
        q.target
            .as_ref()
            .map(|t| format!("&target={t}"))
            .unwrap_or_default(),
        q.request
            .map(|id| format!("&request={id}"))
            .unwrap_or_default()
    );
    let csrf = session.csrf_token().await.map_err(|_| Error::Unavailable)?;
    Ok(oteryn_web::render(&Page {
        lang: language,
        language_label: text(language, "language"),
        title: text(language, op.page()),
        nav,
        sections,
        source_label: text(language, "source"),
        freshness_label: text(language, "freshness"),
        revision_label: text(language, "revision"),
        agent_label: text(language, "agent"),
        target_label: text(language, "target"),
        select_label: text(language, "select"),
        query_label: text(language, "query"),
        poll_label: text(language, "poll"),
        skip: text(language, "skip"),
        note: text(language, "note"),
        csrf,
        agent: q.agent.unwrap_or_default(),
        target: q.target.unwrap_or_default(),
        path,
        poll,
    }))
}
macro_rules! page_handler {
    ($name:ident, $base:expr, $detail:expr) => {
        async fn $name<P: Permissions, C: Channel>(
            State(s): State<Arc<Console<P, C>>>,
            Extension(context): Extension<CheckContext>,
            Extension(session): Extension<BrowserSession>,
            headers: HeaderMap,
            Query(mut q): Query<Selection>,
        ) -> Result<Response, Error> {
            if q.agent.as_deref() == Some("") {
                q.agent = None;
            }
            if q.target.as_deref() == Some("") {
                q.target = None;
            }
            let op = if q.target.is_some() { $detail } else { $base };
            render(s, context, session, headers, q, &[op]).await
        }
    };
}
page_handler!(dashboard_page, Operation::Overview, Operation::Overview);
page_handler!(worlds_page, Operation::Worlds, Operation::World);
page_handler!(nodes_page, Operation::Nodes, Operation::Node);
async fn database_page<P: Permissions, C: Channel>(
    State(s): State<Arc<Console<P, C>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(mut q): Query<Selection>,
) -> Result<Response, Error> {
    if q.agent.as_deref() == Some("") {
        q.agent = None;
    }
    if q.target.as_deref() == Some("") {
        q.target = None;
    }
    render(
        s,
        context,
        session,
        headers,
        q,
        &[Operation::Database, Operation::Migrations],
    )
    .await
}
async fn submit<P: Permissions, C: Channel>(
    State(s): State<Arc<Console<P, C>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    uri: axum::http::Uri,
    Form(form): Form<QueryForm>,
) -> Result<Response, Error> {
    if !s.enabled {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let target = form.target.filter(|s| !s.is_empty());
    let q = Selection {
        agent: Some(form.agent.clone()),
        target: target.clone(),
        request: None,
        lang: Some(form.lang.clone()),
    };
    q.validate()?;
    if uri.path() != format!("/console/{}", form.operation.page())
        || matches!(form.operation, Operation::World | Operation::Node) != target.is_some()
    {
        return Err(Error::Invalid);
    }
    s.permissions.check(context, form.operation).await?;
    let view = s
        .channel
        .read(
            context.account,
            &form.agent,
            form.operation,
            target.as_deref(),
            None,
        )
        .await?;
    if !connected(&view, form.operation, SystemTime::now()) {
        return Err(Error::Unavailable);
    }
    let id = s
        .channel
        .enqueue(
            context,
            &session,
            &form.agent,
            form.operation,
            target.as_deref(),
        )
        .await?;
    if id.get_version_num() != 7 {
        return Err(Error::Unavailable);
    }
    Ok(Redirect::to(&format!(
        "/console/{}?lang={}&agent={}&target={}&request={id}",
        form.operation.page(),
        form.lang,
        form.agent,
        target.unwrap_or_default()
    ))
    .into_response())
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
