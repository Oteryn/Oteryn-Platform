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
    Read(super::GameReadError, &'static str),
}
impl Error {
    fn from_query(error: super::Error, lang: &'static str) -> Self {
        match error {
            super::Error::Read(error) => Self::Read(error, lang),
            _ => Self::Unavailable,
        }
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Denied => StatusCode::FORBIDDEN,
            Self::BadInput => StatusCode::BAD_REQUEST,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Read(error, lang) => {
                return super::locale::error_page(lang, error.status(), tr(lang, error.key()));
            }
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

/// Trusted bridge to the channel, permission and approval services.
/// Every call rechecks current authoritative permissions. `advertised` is true only
/// for a unique owning agent, live heartbeat/poll <30s, and operation version 1.
/// Results must be verified, correlated to this actor/input, unexpired and bounded.
/// None means unavailable, never an empty successful response. No raw result JSON
/// or unfiltered free-text secrets may be passed to these presentation types.
pub trait Backend: Clone + Send + Sync + 'static {
    fn for_session(&self, _: oteryn_web::session::BrowserSession) -> Self {
        self.clone()
    }
    fn for_navigation(
        &self,
        session: oteryn_web::session::BrowserSession,
        headers: &axum::http::HeaderMap,
    ) -> Self {
        if super::trusted_navigation(headers)
            && !["sec-purpose", "purpose"].iter().any(|name| {
                headers.get_all(*name).iter().any(|value| {
                    value
                        .as_bytes()
                        .split(|byte| !byte.is_ascii_alphanumeric())
                        .any(|token| {
                            token.eq_ignore_ascii_case(b"prefetch")
                                || token.eq_ignore_ascii_case(b"prerender")
                        })
                })
            })
        {
            self.for_session(session)
        } else {
            self.clone()
        }
    }
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
    ) -> impl Future<Output = Result<Option<super::audit::LedgerSnapshot>, Error>> + Send;
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
    ) -> Result<Option<super::audit::LedgerSnapshot>, Error> {
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
/// Bridge to current RBAC, verified query results and immutable approval evidence.
#[derive(Debug, Clone)]
pub struct StoredMetadata<A> {
    pool: PgPool,
    auth: crate::rbac::Authorizer<A>,
    store: crate::control::Store,
    channel: Option<super::StoredChannel>,
    session: Option<oteryn_web::session::BrowserSession>,
}
impl<A: crate::rbac::AccountStateReader> StoredMetadata<A> {
    pub fn with_channel(mut self, channel: super::StoredChannel) -> Self {
        self.channel = Some(channel);
        self
    }

    pub fn new(pool: PgPool, flags: Flags, accounts: A, config: crate::control::Config) -> Self {
        Self {
            pool: pool.clone(),
            auth: crate::rbac::Authorizer::new(pool.clone(), accounts).with_game_flags(
                crate::rbac::GamePermissionFlags {
                    enabled: flags.console,
                    deploy: false,
                },
            ),
            store: crate::control::Store::new(pool, config),
            channel: None,
            session: None,
        }
    }
}
impl<A: crate::rbac::AccountStateReader + Clone + 'static> Backend for StoredMetadata<A> {
    fn for_session(&self, session: oteryn_web::session::BrowserSession) -> Self {
        let mut backend = self.clone();
        backend.session = Some(session);
        backend
    }
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
        actor: CheckContext,
        input: &super::players::Input,
        operation: &'static str,
    ) -> Result<Option<super::players::Snapshot>, Error> {
        input.validate()?;
        match operation {
            "player.search" if input.q.is_some() => (),
            "player.character.get" | "player.inventory.get" | "sanction.list"
                if input.character.is_some() => {}
            _ => return Err(Error::BadInput),
        }
        let permission = if operation == "sanction.list" {
            "game.sanction.read"
        } else {
            "game.player.read"
        };
        if !self.permitted(actor, permission).await {
            return Err(Error::Denied);
        }
        let Some(channel) = &self.channel else {
            return Err(Error::Unavailable);
        };
        let params = match operation {
            "player.search" => serde_json::json!({"limit":100,"name_prefix":input.q}),
            "player.inventory.get" | "sanction.list" => serde_json::json!({"limit":100}),
            _ => serde_json::json!({}),
        };
        let lang = language(input.lang.as_deref())?;
        let Some((request_id, observed_at, value)) = channel
            .snapshot(
                actor,
                self.session.as_ref(),
                operation,
                permission,
                input.character.as_deref(),
                params,
            )
            .await
            .map_err(|error| Error::from_query(error, lang))?
        else {
            return Ok(None);
        };
        Ok(Some(super::projection::snapshot(
            operation,
            request_id,
            observed_at,
            &value,
        )?))
    }
    async fn ledger(
        &self,
        actor: CheckContext,
        filter: &super::audit::Filter,
    ) -> Result<Option<super::audit::LedgerSnapshot>, Error> {
        if !self.permitted(actor, "game.audit.read").await {
            return Err(Error::Denied);
        }
        let Some(channel) = &self.channel else {
            return Err(Error::Unavailable);
        };
        let lang = language(filter.lang.as_deref())?;
        let day = time::OffsetDateTime::now_utc().replace_time(time::Time::MIDNIGHT);
        let timestamp = |date: time::OffsetDateTime| {
            date.format(&time::format_description::well_known::Rfc3339)
                .map_err(|_| Error::Unavailable)
        };
        let params = serde_json::json!({
            "limit":100,"admin_id":filter.admin,"operation":filter.operation,
            "from":timestamp(day - time::Duration::days(30))?,
            "to":timestamp(day + time::Duration::days(1))?
        });
        let Some((query_id, _, value)) = channel
            .snapshot(
                actor,
                self.session.as_ref(),
                "admin.ledger.query",
                "game.audit.read",
                None,
                params,
            )
            .await
            .map_err(|error| Error::from_query(error, lang))?
        else {
            return Ok(None);
        };
        Ok(Some(super::audit::LedgerSnapshot {
            query_id,
            rows: super::projection::ledger(&value)?,
        }))
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
    async fn policy(&self, actor: CheckContext) -> Result<Option<Vec<Policy>>, Error> {
        if !self.permitted(actor, "game.ops.read").await {
            return Err(Error::Denied);
        }
        let Some(channel) = &self.channel else {
            return Err(Error::Unavailable);
        };
        let Some((_, _, value)) = channel
            .snapshot(
                actor,
                self.session.as_ref(),
                "admin.policy.get",
                "game.ops.read",
                None,
                serde_json::json!({}),
            )
            .await
            .map_err(|error| Error::from_query(error, "en"))?
        else {
            return Ok(None);
        };
        Ok(Some(super::projection::policy(&value)?))
    }

    async fn approvals(
        &self,
        actor: CheckContext,
    ) -> Result<Option<Vec<super::approvals::Pending>>, Error> {
        if !self.permitted(actor, "admin.roles.manage").await {
            return Err(Error::Denied);
        }
        let rows = sqlx::query("SELECT r.request_id,r.actor,r.operation,r.target,r.expires_at::text,p.preview_digest,p.preview,p.permission FROM admin_control_requests r JOIN admin_control_previews p ON p.id=r.request_id WHERE r.risk_class='R4' AND r.dry_run AND r.status='succeeded' AND p.requester=r.actor AND p.operation=r.operation AND EXISTS(SELECT 1 FROM admin_control_agents g WHERE g.agent_id=r.agent_id AND g.last_seen_at>clock_timestamp()-interval '30 seconds' AND g.last_seen_at<=clock_timestamp() AND (r.target->>'world_id' IS NULL OR r.target->>'world_id'=ANY(g.served_worlds)) AND (r.target->>'node_id' IS NULL OR r.target->>'node_id'=ANY(g.served_nodes)) AND g.advertised_operations @> jsonb_build_array(jsonb_build_object('operation',r.operation,'operation_version',r.operation_version,'kind',r.kind)) AND NOT EXISTS(SELECT 1 FROM admin_control_agents other WHERE other.agent_id<>g.agent_id AND other.owner_epoch>=g.owner_epoch AND (r.target->>'world_id'=ANY(other.served_worlds) OR r.target->>'node_id'=ANY(other.served_nodes)))) AND r.expires_at>clock_timestamp() AND p.requester<>$1 AND p.created_at>clock_timestamp()-interval '30 minutes' AND NOT EXISTS(SELECT 1 FROM admin_control_approvals a WHERE a.preview_id=p.id) ORDER BY p.created_at DESC LIMIT 100")
            .bind(actor.account).fetch_all(&self.pool).await?;
        use sqlx::Row;
        let mut pending = Vec::new();
        for row in rows {
            let operation: String = row.try_get("operation")?;
            let permission: String = row.try_get("permission")?;
            let permission: &'static str = match permission.parse::<Permission>() {
                Ok(p) => p.as_str(),
                Err(_) => continue,
            };
            if !self.permitted(actor, permission).await {
                continue;
            }
            let target: serde_json::Value = row.try_get("target")?;
            pending.push(super::approvals::Pending {
                request_id: row.try_get("request_id")?,
                requester: row.try_get("actor")?,
                operation,
                target: super::projection::target(&target)?,
                expires_at: row.try_get("expires_at")?,
                preview_digest: row.try_get("preview_digest")?,
                preview: super::projection::preview(
                    &row.try_get::<serde_json::Value, _>("preview")?,
                )?,
            });
        }
        Ok(Some(pending))
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
        ("/admin/console/players", Permission::Access, "players"),
        ("/admin/console/audit", Permission::Access, "audit"),
        ("/admin/console/access", Permission::ManageRoles, "access"),
        (
            "/admin/console/approvals",
            Permission::ManageRoles,
            "approvals",
        ),
    ] {
        let page = match area {
            "players" => Router::new().route(
                path,
                get(super::players::page::<B>).post(super::players::search::<B>),
            ),
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
    routes.route_layer(axum::middleware::from_fn(super::locale::response_language))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    // Channel operations remain unavailable until a verified agent is connected.
    let flags = Flags {
        console: std::env::var("OTERYN_ADMIN_CONSOLE").is_ok_and(|v| v == "true"),
    };
    if !flags.console {
        return Ok(Router::new());
    }
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
    )
    .with_channel(super::StoredChannel::configured(pool.clone())?);
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
    super::locale::message(lang, key, None, true).unwrap_or_default()
}
#[derive(Template)]
#[template(path = "read.html", config = "src/console/templates/askama.toml")]
pub(crate) struct Page {
    pub lang: &'static str,
    pub title: String,
    pub path: String,
    pub language_label: String,
    pub skip: String,
    pub audit_rows: Option<super::audit::Rows>,
    pub source: String,
    pub sections: Vec<Section>,
    pub players: String,
    pub audit: String,
    pub access: String,
    pub invitations: Option<String>,
    pub approvals: String,
    pub search: bool,
    pub csrf: String,
    pub audit_filter: Option<super::audit::Filter>,
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
            path: format!("/admin/console/{title}"),
            language_label: tr(lang, "language"),
            skip: tr(lang, "skip"),
            audit_rows: None,
            source: tr(lang, "source"),
            sections: vec![],
            players: tr(lang, "players"),
            audit: tr(lang, "audit"),
            access: tr(lang, "access"),
            invitations: None,
            approvals: tr(lang, "approvals"),
            search: false,
            csrf: String::new(),
            audit_filter: None,
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
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
    headers: axum::http::HeaderMap,
    Query(input): Query<Locale>,
) -> Result<Response, Error> {
    state.check(actor, "admin.roles.manage").await?;
    let lang = super::locale::select(input.lang.as_deref(), &headers).ok_or(Error::BadInput)?;
    let roles: Vec<(String,String)> = sqlx::query_as("SELECT r.key, COALESCE(rp.permission,'') FROM roles r LEFT JOIN role_permissions rp ON rp.role_id = r.id ORDER BY r.key, rp.permission LIMIT 1000").fetch_all(&state.pool).await?;
    let metadata = state.backend.access().await?;
    let mut page = Page::new(lang, "access");
    if state
        .backend
        .permitted(actor, Permission::ListStaffInvites.as_str())
        .await
        || state
            .backend
            .permitted(actor, Permission::CreateStaffInvite.as_str())
            .await
    {
        page.invitations = Some(crate::web::text(lang, "invite-title"));
    }
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
    let backend = state.backend.for_navigation(session, &headers);
    let policy = if backend.permitted(actor, "game.ops.read").await
        && backend.advertised("admin.policy.get").await
    {
        backend.policy(actor).await?
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
