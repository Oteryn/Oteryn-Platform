//! Read-only console adapters. Implementations consume verified Platform channel state;
//! they must never open a connection to Game or infer grants from role names.
use crate::rbac::{CheckContext, Permission};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{future::Future, sync::Arc};

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub console: bool,
}
#[derive(Debug, Clone, Copy)]
pub enum Error {
    Denied,
    BadInput,
    Unavailable,
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Denied => StatusCode::FORBIDDEN,
            Self::BadInput => StatusCode::BAD_REQUEST,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

/// Safe, public metadata only. There is deliberately no key-material field.
#[derive(Debug, Clone)]
pub struct SignerStatus {
    pub key_id: String,
    pub valid_from: String,
    pub valid_until: String,
}
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub agent_id: String,
    pub operations: Vec<String>,
    pub observed_at: String,
}
#[derive(Debug, Clone, Default)]
pub struct AccessSnapshot {
    pub signers: Option<Vec<SignerStatus>>,
    pub agents: Option<Vec<AgentStatus>>,
}
#[derive(Debug, Clone)]
pub struct Policy {
    pub operation: String,
    pub minimum_risk: String,
    pub permission: String,
}

/// Narrow bridge until the channel, permission and approval services are integrated.
/// Every call rechecks current authoritative permissions. `advertised` is true only
/// for a unique owning agent, live heartbeat/poll <30s, and operation version 1.
/// Results must be verified, correlated to this actor/input, unexpired and bounded.
/// None means unavailable, never an empty successful response. No raw result JSON
/// or unfiltered free-text secrets may be passed to these presentation types.
pub trait Backend: Clone + Send + Sync + 'static {
    fn permitted(
        &self,
        actor: CheckContext,
        permission: &'static str,
    ) -> impl Future<Output = bool> + Send;
    fn advertised(&self, operation: &str) -> impl Future<Output = bool> + Send;
    fn players(
        &self,
        actor: CheckContext,
        input: &super::players::Input,
        operation: &'static str,
    ) -> impl Future<Output = Result<Option<super::players::Snapshot>, Error>> + Send;
    fn ledger(
        &self,
        actor: CheckContext,
        filter: &super::audit::Filter,
    ) -> impl Future<Output = Result<Option<Vec<super::audit::LedgerRow>>, Error>> + Send;
    fn access(&self) -> impl Future<Output = Result<AccessSnapshot, Error>> + Send;
    fn policy(
        &self,
        actor: CheckContext,
    ) -> impl Future<Output = Result<Option<Vec<Policy>>, Error>> + Send;
    fn approvals(
        &self,
        actor: CheckContext,
    ) -> impl Future<Output = Result<Option<Vec<super::approvals::Pending>>, Error>> + Send;
}
#[derive(Debug, Clone, Copy)]
pub struct PendingBackend;
impl Backend for PendingBackend {
    async fn permitted(&self, _: CheckContext, _: &'static str) -> bool {
        false
    }
    async fn advertised(&self, _: &str) -> bool {
        false
    }
    async fn players(
        &self,
        _: CheckContext,
        _: &super::players::Input,
        _: &'static str,
    ) -> Result<Option<super::players::Snapshot>, Error> {
        Ok(None)
    }
    async fn ledger(
        &self,
        _: CheckContext,
        _: &super::audit::Filter,
    ) -> Result<Option<Vec<super::audit::LedgerRow>>, Error> {
        Ok(None)
    }
    async fn access(&self) -> Result<AccessSnapshot, Error> {
        Ok(AccessSnapshot::default())
    }
    async fn policy(&self, _: CheckContext) -> Result<Option<Vec<Policy>>, Error> {
        Ok(None)
    }
    async fn approvals(
        &self,
        _: CheckContext,
    ) -> Result<Option<Vec<super::approvals::Pending>>, Error> {
        Ok(None)
    }
}
/// Read-only bridge to merged RBAC and request-store metadata. Query results
/// remain unavailable until a verified query service is supplied through Backend.
#[derive(Debug, Clone)]
pub struct StoredMetadata<A> {
    auth: crate::rbac::Authorizer<A>,
    store: crate::control::Store,
}
impl<A: crate::rbac::AccountStateReader> StoredMetadata<A> {
    pub fn new(pool: PgPool, flags: Flags, accounts: A, config: crate::control::Config) -> Self {
        Self {
            auth: crate::rbac::Authorizer::new(pool.clone(), accounts).with_game_flags(
                crate::rbac::GamePermissionFlags {
                    enabled: flags.console,
                    deploy: false,
                },
            ),
            store: crate::control::Store::new(pool, config),
        }
    }
}
impl<A: crate::rbac::AccountStateReader + Clone + 'static> Backend for StoredMetadata<A> {
    async fn permitted(&self, actor: CheckContext, permission: &'static str) -> bool {
        let Ok(permission) = permission.parse::<Permission>() else {
            return false;
        };
        self.auth.check(actor, permission).await.is_ok()
    }
    async fn advertised(&self, operation: &str) -> bool {
        let Ok(agents) = self.store.agents().await else {
            return false;
        };
        // Without a target-specific query adapter, multiple live owners are ambiguous.
        let mut live = agents.into_iter().filter(|a| a.connected);
        let Some(agent) = live.next() else {
            return false;
        };
        live.next().is_none()
            && agent
                .advertised_operations
                .iter()
                .any(|op| op.operation == operation && op.operation_version == 1)
    }
    async fn players(
        &self,
        _: CheckContext,
        _: &super::players::Input,
        _: &'static str,
    ) -> Result<Option<super::players::Snapshot>, Error> {
        Ok(None)
    }
    async fn ledger(
        &self,
        _: CheckContext,
        _: &super::audit::Filter,
    ) -> Result<Option<Vec<super::audit::LedgerRow>>, Error> {
        Ok(None)
    }
    async fn access(&self) -> Result<AccessSnapshot, Error> {
        let agents = match self.store.agents().await {
            Ok(agents) => agents,
            Err(crate::control::Error::Disabled) => return Ok(AccessSnapshot::default()),
            Err(_) => return Err(Error::Unavailable),
        };
        Ok(AccessSnapshot {
            signers: None,
            agents: Some(
                agents
                    .into_iter()
                    .filter(|agent| agent.connected)
                    .map(|agent| AgentStatus {
                        agent_id: agent.agent_id,
                        observed_at: agent.last_seen_at.to_string(),
                        operations: agent
                            .advertised_operations
                            .iter()
                            .map(|op| {
                                format!(
                                    "{}@{} ({})",
                                    op.operation,
                                    op.operation_version,
                                    op.kind.as_str()
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            ),
        })
    }
    async fn policy(&self, _: CheckContext) -> Result<Option<Vec<Policy>>, Error> {
        Ok(None)
    }
    async fn approvals(
        &self,
        _: CheckContext,
    ) -> Result<Option<Vec<super::approvals::Pending>>, Error> {
        Ok(None)
    }
}
pub(crate) struct Console<B> {
    pub pool: PgPool,
    pub flags: Flags,
    pub backend: B,
}
impl<B: Backend> Console<B> {
    pub async fn check(&self, actor: CheckContext, permission: &'static str) -> Result<(), Error> {
        if !self.flags.console
            || actor.account.is_nil()
            || !actor.mfa_verified
            || !self.backend.permitted(actor, permission).await
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
/// All adapters are trusted application wiring, never request extensions or input.
pub fn router<I, A, B>(
    pool: PgPool,
    flags: Flags,
    panel: crate::web::PanelConfig,
    identity: I,
    accounts: A,
    backend: B,
) -> Router
where
    I: crate::web::StaffSession,
    A: crate::rbac::AccountStateReader + Clone + 'static,
    B: Backend,
{
    let state = Arc::new(Console {
        pool: pool.clone(),
        flags,
        backend,
    });
    let mut routes = Router::new();
    for (path, permission, area) in [
        (
            "/admin/console/players",
            Permission::ManageSupportTickets,
            "players",
        ),
        ("/admin/console/audit", Permission::ViewAudit, "audit"),
        ("/admin/console/access", Permission::ManageRoles, "access"),
        (
            "/admin/console/approvals",
            Permission::ManageRoles,
            "approvals",
        ),
    ] {
        let page = match area {
            "players" => Router::new().route(path, get(super::players::page::<B>)),
            "audit" => Router::new().route(path, get(super::audit::page::<B>)),
            "approvals" => Router::new().route(path, get(super::approvals::page::<B>)),
            _ => Router::new().route(path, get(page::<B>)),
        }
        .with_state(state.clone());
        routes = routes.merge(crate::web::guarded(
            page,
            pool.clone(),
            panel.clone(),
            identity.clone(),
            accounts.clone(),
            permission,
        ));
    }
    routes
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    // Channel operations remain unavailable until a verified agent is connected.
    let flags = Flags {
        console: std::env::var("OTERYN_ADMIN_CONSOLE").is_ok_and(|v| v == "true"),
    };
    let panel = crate::web::PanelConfig {
        admin_panel: std::env::var("OTERYN_ADMIN_PANEL").is_ok_and(|v| v == "true"),
        host: std::env::var("OTERYN_ADMIN_HOST").unwrap_or_default(),
    };
    if panel.admin_panel
        && (panel.host.is_empty() || panel.host.parse::<axum::http::uri::Authority>().is_err())
    {
        return Err("invalid admin host".into());
    }
    let backend = StoredMetadata::new(
        pool.clone(),
        flags,
        crate::web::PortalIdentity::configured(pool.clone()),
        crate::control::Config::from_env(),
    );
    Ok(router(
        pool.clone(),
        flags,
        panel,
        crate::web::PortalIdentity::configured(pool.clone()),
        crate::web::PortalIdentity::configured(pool.clone()),
        backend,
    ))
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locale {
    pub lang: Option<String>,
}
pub(crate) fn language(value: Option<&str>) -> Result<&'static str, Error> {
    match value {
        None | Some("pl") => Ok("pl"),
        Some("en") => Ok("en"),
        _ => Err(Error::BadInput),
    }
}
pub(crate) fn tr(lang: &str, key: &str) -> String {
    let locale = if lang == "en" { "en" } else { "pl" };
    let Ok(id) = locale.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "en" {
        include_str!("access.en.ftl")
    } else {
        include_str!("access.pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) else {
        return String::new();
    };
    bundle
        .format_pattern(pattern, None, &mut vec![])
        .into_owned()
}
#[derive(Template)]
#[template(path = "read.html", config = "src/console/templates/askama.toml")]
pub(crate) struct Page {
    pub lang: &'static str,
    pub title: String,
    pub source: String,
    pub sections: Vec<Section>,
    pub players: String,
    pub audit: String,
    pub access: String,
    pub approvals: String,
    pub search: bool,
    pub filters: bool,
    pub submit: String,
    pub query_label: String,
    pub character_label: String,
    pub request_label: String,
    pub admin_label: String,
    pub operation_label: String,
    pub target_label: String,
}
#[derive(Debug)]
pub(crate) struct Section {
    pub title: String,
    pub lines: Vec<String>,
}
impl Page {
    pub fn new(lang: &'static str, title: &str) -> Self {
        Self {
            lang,
            title: tr(lang, title),
            source: tr(lang, "source"),
            sections: vec![],
            players: tr(lang, "players"),
            audit: tr(lang, "audit"),
            access: tr(lang, "access"),
            approvals: tr(lang, "approvals"),
            search: false,
            filters: false,
            submit: tr(lang, "submit"),
            query_label: tr(lang, "query"),
            character_label: tr(lang, "character"),
            request_label: tr(lang, "request"),
            admin_label: tr(lang, "admin"),
            operation_label: tr(lang, "operation"),
            target_label: tr(lang, "target"),
        }
    }
    pub fn section(&mut self, title: &str, lines: Option<Vec<String>>) {
        self.sections.push(Section {
            title: tr(self.lang, title),
            lines: match lines {
                None => vec![tr(self.lang, "unavailable")],
                Some(lines) if lines.is_empty() => vec![tr(self.lang, "empty")],
                Some(lines) => lines,
            },
        });
    }
    pub fn response(self) -> Result<Response, Error> {
        let html = self.render().map_err(|_| Error::Unavailable)?;
        Ok((
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "no-referrer"),
            ],
            html,
        )
            .into_response())
    }
}
pub(crate) async fn page<B: Backend>(
    State(state): State<Arc<Console<B>>>,
    Extension(actor): Extension<CheckContext>,
    Query(input): Query<Locale>,
) -> Result<Response, Error> {
    state.check(actor, "admin.roles.manage").await?;
    let lang = language(input.lang.as_deref())?;
    let roles: Vec<(String,String)> = sqlx::query_as("SELECT r.key, COALESCE(rp.permission,'') FROM roles r LEFT JOIN role_permissions rp ON rp.role_id = r.id ORDER BY r.key, rp.permission LIMIT 1000").fetch_all(&state.pool).await?;
    let metadata = state.backend.access().await?;
    let mut page = Page::new(lang, "access");
    page.section(
        "roles",
        Some(
            roles
                .into_iter()
                .map(|(role, permission)| format!("{role}: {permission}"))
                .collect(),
        ),
    );
    page.section(
        "signers",
        metadata.signers.map(|keys| {
            keys.into_iter()
                .map(|k| format!("{}: {} – {}", k.key_id, k.valid_from, k.valid_until))
                .collect()
        }),
    );
    page.section(
        "agents",
        metadata.agents.map(|agents| {
            agents
                .into_iter()
                .map(|a| {
                    format!(
                        "{} · {} · {}",
                        a.agent_id,
                        a.observed_at,
                        a.operations.join(", ")
                    )
                })
                .collect()
        }),
    );
    let policy = if state.backend.permitted(actor, "game.ops.read").await
        && state.backend.advertised("admin.policy.get").await
    {
        state.backend.policy(actor).await?
    } else {
        None
    };
    page.section(
        "policy",
        policy.map(|rows| {
            rows.into_iter()
                .map(|p| format!("{} · {} · {}", p.operation, p.minimum_risk, p.permission))
                .collect()
        }),
    );
    page.response()
}
