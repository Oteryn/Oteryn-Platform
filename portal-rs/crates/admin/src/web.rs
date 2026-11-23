//! Staff surface. Authentication adapters are trusted server configuration, never request data.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{
        AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, Change,
        CheckContext, Error, Permission,
    },
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::{future::Future, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct StaffIdentity {
    pub account: Uuid,
    pub mfa_satisfied: bool,
}
/// Resolve only from the validated server-side browser session.
pub trait StaffSession: Clone + Send + Sync + 'static {
    fn resolve(
        &self,
        session: &BrowserSession,
    ) -> impl Future<Output = Option<StaffIdentity>> + Send;
}
#[derive(Debug, Clone)]
pub struct PendingAuthentication;
impl StaffSession for PendingAuthentication {
    async fn resolve(&self, _session: &BrowserSession) -> Option<StaffIdentity> {
        None
    }
}
impl AccountStateReader for PendingAuthentication {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Unknown)
    }
}
/// Identity read from the validated browser session and current account/MFA stores.
#[derive(Debug, Clone)]
pub struct PortalIdentity {
    pool: PgPool,
    mfa: Option<oteryn_identity::mfa::Mfa>,
}
impl PortalIdentity {
    pub fn configured(pool: PgPool) -> Self {
        Self::configured_from(pool, &|key| std::env::var(key).ok())
    }
    /// [`Self::configured`] reading its configuration through `lookup`.
    pub fn configured_from(pool: PgPool, lookup: &dyn Fn(&str) -> Option<String>) -> Self {
        let mfa = oteryn_identity::mfa::Mfa::configured_from(pool.clone(), lookup).ok();
        Self { pool, mfa }
    }
}
impl StaffSession for PortalIdentity {
    async fn resolve(&self, session: &BrowserSession) -> Option<StaffIdentity> {
        let account = session.account_id().await?;
        let key =
            oteryn_web::session::SessionKey::<String>::new(oteryn_identity::mfa::SESSION_GRANT_KEY);
        let mfa_satisfied =
            if let (Some(mfa), Ok(Some(grant))) = (&self.mfa, session.get(key).await) {
                mfa.enrolled(account).await.unwrap_or(false)
                    && mfa.satisfied(account, &grant).await.unwrap_or(false)
            } else {
                false
            };
        Some(StaffIdentity {
            account,
            mfa_satisfied,
        })
    }
}
impl AccountStateReader for PortalIdentity {
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: CheckContext,
    ) -> Result<bool, AccountStateUnavailable> {
        let Some(proof) = context.session_proof else {
            return Ok(false);
        };
        crate::rbac::durable_session_in(connection, context.account, proof).await
    }

    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> Result<AccountState, AccountStateUnavailable> {
        let state: Option<(String, bool)> = sqlx::query_as(
            "SELECT status,email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE",
        )
        .bind(account)
        .fetch_optional(connection)
        .await
        .map_err(|_| AccountStateUnavailable)?;
        Ok(match state {
            Some((status, true)) if status == "active" => AccountState::Active,
            Some(_) => AccountState::Disabled,
            None => AccountState::Unknown,
        })
    }
    async fn state(&self, account: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        let state: Option<(String, bool)> =
            sqlx::query_as("SELECT status,email_verified_at IS NOT NULL FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AccountStateUnavailable)?;
        Ok(match state {
            Some((status, true)) if status == "active" => AccountState::Active,
            Some(_) => AccountState::Disabled,
            None => AccountState::Unknown,
        })
    }
}
#[derive(Debug, Clone, Default)]
pub struct PanelConfig {
    pub admin_panel: bool,
    pub host: String,
}
impl PanelConfig {
    pub(crate) fn csrf_origin(&self) -> String {
        format!("https://{}", self.host.to_ascii_lowercase())
    }
}

pub(crate) struct Panel<I, A> {
    config: PanelConfig,
    /// Set for feature routes wrapped by [`guarded`]; the panel itself picks per path.
    required: Option<Permission>,
    players_enabled: bool,
    identity: I,
    pub(crate) auth: Arc<Authorizer<A>>,
    pub(crate) pool: PgPool,
}

pub(crate) fn config_from_env() -> Result<PanelConfig, Box<dyn std::error::Error>> {
    config_from(&|key| std::env::var(key).ok())
}

pub(crate) fn config_from(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<PanelConfig, Box<dyn std::error::Error>> {
    let enabled = lookup("OTERYN_ADMIN_PANEL").is_some_and(|v| v == "true");
    let host = lookup("OTERYN_ADMIN_HOST").unwrap_or_default();
    if enabled && (host.is_empty() || host.parse::<axum::http::uri::Authority>().is_err()) {
        return Err("invalid admin host".into());
    }
    Ok(PanelConfig {
        admin_panel: enabled,
        host,
    })
}

/// Indicates that the portal composes the reports backend alongside the staff panel.
#[derive(Clone, Copy)]
struct ReportsNavigation;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let reports_enabled = std::env::var("OTERYN_REPORTS_ENABLED").as_deref() == Ok("true")
        && std::env::var("OTERYN_REPORTS_PENDING_LIMIT").map_or(true, |v| {
            v.parse::<u8>().is_ok_and(|n| (1..=20).contains(&n))
        })
        && std::env::var("OTERYN_REPORTS_UNJUSTIFIED_LIMIT")
            .map_or(true, |v| v.parse::<u8>().is_ok_and(|n| n > 0));
    let router = panel_router(
        pool.clone(),
        config_from_env()?,
        PortalIdentity::configured(pool.clone()),
        PortalIdentity::configured(pool),
        std::env::var("OTERYN_ADMIN_PLAYERS").is_ok_and(|value| value == "true"),
    );
    Ok(if reports_enabled {
        router.layer(Extension(ReportsNavigation))
    } else {
        router
    })
}

/// [`guarded`] with the panel's environment configuration and the server-side
/// authentication adapters used by [`configured_router`].
pub fn configured_guard(
    routes: Router,
    pool: PgPool,
    permission: Permission,
) -> Result<Router, Box<dyn std::error::Error>> {
    configured_guard_from(routes, pool, permission, &|key| std::env::var(key).ok())
}

/// [`configured_guard`] reading its configuration through `lookup`.
pub fn configured_guard_from(
    routes: Router,
    pool: PgPool,
    permission: Permission,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(guarded(
        routes,
        pool.clone(),
        config_from(lookup)?,
        PortalIdentity::configured_from(pool.clone(), lookup),
        PortalIdentity::configured_from(pool, lookup),
        permission,
    ))
}

/// Wrap feature routes mounted under `/admin/...` in the panel's guard: admin host,
/// browser session with CSRF, staff MFA, `Permission::Access` and `permission`.
/// Refusals are audited like panel refusals. Handlers read the checked
/// [`CheckContext`] and [`crate::rbac::MutationAuthorization`] from request extensions.
/// Mutations recheck after resource locks and retain their transaction through commit.
pub fn guarded<I: StaffSession, A: AccountStateReader + 'static>(
    routes: Router,
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    permission: Permission,
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![config.csrf_origin()];
    let admin_csrf = session.csrf.clone();
    let state = Arc::new(Panel {
        config,
        required: Some(permission),
        players_enabled: false,
        identity,
        auth: Arc::new(Authorizer::new(pool.clone(), accounts)),
        pool,
    });
    routes
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate::<I, A>,
        ))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .route_layer(axum::Extension(admin_csrf))
        .route_layer(middleware::from_fn_with_state(state, host::<I, A>))
}
/// Mount under `/admin`; host checks run before session or CSRF processing.
pub fn router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    panel_router(pool, config, identity, accounts, false)
}

fn panel_router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    players_enabled: bool,
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![config.csrf_origin()];
    let admin_csrf = session.csrf.clone();
    let state = Arc::new(Panel {
        config,
        required: None,
        players_enabled,
        identity,
        auth: Arc::new(Authorizer::new(pool.clone(), accounts)),
        pool,
    });
    Router::new()
        .route("/admin", get(page::<I, A>))
        .route("/admin/roles", get(page::<I, A>).post(change::<I, A>))
        .route("/admin/mail-outbox", get(crate::mail_ops::page::<I, A>))
        .route("/admin/native-login", get(crate::native_login::page))
        .route(
            "/admin/mail-outbox/retry",
            post(crate::mail_ops::retry::<I, A>),
        )
        .route(
            "/admin/mail-outbox/retry-kind",
            post(crate::mail_ops::retry_kind::<I, A>),
        )
        .route(
            "/admin/mail-outbox/cancel",
            post(crate::mail_ops::cancel::<I, A>),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate::<I, A>,
        ))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .route_layer(axum::Extension(admin_csrf))
        .route_layer(middleware::from_fn_with_state(state.clone(), host::<I, A>))
        .with_state(state)
}
/// Compose adapters that must run before shared CSRF verification but after
/// the staff panel/host refusal boundary.
pub fn configured_host_boundary(router: Router) -> Result<Router, Box<dyn std::error::Error>> {
    configured_host_boundary_from(router, &|key| std::env::var(key).ok())
}
/// Configuration injection for applications mounting an outer request adapter.
pub fn configured_host_boundary_from(
    router: Router,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(router.layer(middleware::from_fn_with_state(
        (config_from(lookup)?, false),
        host_guard,
    )))
}

async fn host<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    request: Request,
    next: Next,
) -> Response {
    host_guard(State((s.config.clone(), false)), request, next).await
}

/// Shared outer host boundary: reject before session, authorization or audit work.
/// Analytics opts into private cache headers; the existing panel keeps no-store.
pub(crate) async fn host_guard(
    State((config, private)): State<(PanelConfig, bool)>,
    request: Request,
    next: Next,
) -> Response {
    let hosts = request.headers().get_all(header::HOST);
    if !config.admin_panel
        || config.host.is_empty()
        || hosts.iter().count() != 1
        || !hosts
            .iter()
            .next()
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case(&config.host))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        (
            "cache-control",
            if private {
                "no-store, private"
            } else {
                "no-store"
            },
        ),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    if private {
        response.headers_mut().insert(
            header::PRAGMA,
            axum::http::HeaderValue::from_static("no-cache"),
        );
    }
    response
}
async fn authenticate<I: StaffSession, A: AccountStateReader + 'static>(
    State(s): State<Arc<Panel<I, A>>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(session) = request.extensions().get::<BrowserSession>() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(identity) = s.identity.resolve(session).await else {
        return Redirect::to("/login").into_response();
    };
    if !identity.mfa_satisfied {
        return (StatusCode::FORBIDDEN, axum::response::Html(format!("<html lang=\"{}\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><main><h1>{}</h1></main></html>", language(request.headers()), text(language(request.headers()), "mfa")))).into_response();
    }
    let proof = match (
        session.identity_hash().await,
        session
            .get(oteryn_web::session::SessionKey::<String>::new(
                oteryn_identity::mfa::SESSION_GRANT_KEY,
            ))
            .await,
    ) {
        (Some(hash), Ok(Some(grant))) => {
            use sha2::{Digest, Sha256};
            hash.try_into()
                .ok()
                .map(|session_hash| crate::rbac::SessionProof {
                    session_hash,
                    grant_hash: Sha256::digest(grant.as_bytes()).into(),
                })
        }
        _ => None,
    };
    let context = CheckContext {
        account: identity.account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: proof,
    };
    let permission = s.required.unwrap_or_else(|| match request.uri().path() {
        "/admin/roles" => Permission::ManageRoles,
        "/admin/mail-outbox" => Permission::MailView,
        "/admin/native-login" => Permission::ViewAudit,
        "/admin/mail-outbox/retry" => Permission::MailRetry,
        "/admin/mail-outbox/retry-kind" => Permission::MailRetryBatch,
        "/admin/mail-outbox/cancel" => Permission::MailCancel,
        _ => Permission::Access,
    });
    for required in [Permission::Access, permission] {
        if let Err(error) = s.auth.check(context, required).await {
            if matches!(error, Error::Storage(_)) {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            return match refusal(&s.pool, context.account, request.uri().path()).await {
                Ok(()) => StatusCode::FORBIDDEN.into_response(),
                Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            };
        }
    }
    request.extensions_mut().insert(context);
    request
        .extensions_mut()
        .insert(crate::rbac::MutationAuthorization::new(s.auth.clone()));
    next.run(request).await
}
async fn refusal(pool: &PgPool, actor: Uuid, target: &str) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action: "admin.access.denied",
            target,
            before: json!(null),
            after: json!({"allowed":false}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) fn language(headers: &axum::http::HeaderMap) -> &'static str {
    if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    }
}
pub(crate) fn messages(
    lang: &str,
) -> Option<fluent_bundle::FluentBundle<fluent_bundle::FluentResource>> {
    let mut bundle = fluent_bundle::FluentBundle::new(vec![lang.parse().ok()?]);
    let source = if lang == "en" {
        include_str!("../../../templates/admin/en.ftl")
    } else {
        include_str!("../../../templates/admin/pl.ftl")
    };
    let resource = fluent_bundle::FluentResource::try_new(source.to_owned()).ok()?;
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

pub(crate) fn text(lang: &str, key: &str) -> String {
    messages(lang)
        .map(|bundle| message(&bundle, key))
        .unwrap_or_default()
}

pub(crate) fn message(
    bundle: &fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    key: &str,
) -> String {
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "admin/roles.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    roles_label: String,
    reports_label: String,
    moderate_reports: bool,
    mail_link: Option<String>,
    native_login_link: Option<String>,
    players_label: String,
    players: bool,
    account_label: String,
    role_label: String,
    grant: String,
    revoke: String,
    csrf: String,
    roles: Vec<(Uuid, String)>,
    manage: bool,
    next: Option<Uuid>,
}
impl Page {
    fn permission_catalog(&self) -> &'static [Permission] {
        Permission::ALL
    }
}
const ROLE_PAGE_SIZE: i64 = 50;
#[derive(Deserialize, Default)]
struct RolePageQuery {
    after: Option<Uuid>,
}
pub(crate) async fn role_page(
    pool: &PgPool,
    after: Option<Uuid>,
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,key FROM roles WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT $2",
    )
    .bind(after)
    .bind(ROLE_PAGE_SIZE)
    .fetch_all(pool)
    .await
}
pub(crate) async fn holder_batch(
    pool: &PgPool,
    role: Uuid,
    excluded: Uuid,
    after: Option<Uuid>,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT account_id FROM account_roles WHERE role_id=$1 AND account_id<>$2 AND ($3::uuid IS NULL OR account_id>$3) ORDER BY account_id LIMIT $4")
        .bind(role).bind(excluded).bind(after).bind(ROLE_PAGE_SIZE).fetch_all(pool).await
}
async fn page<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    reports_navigation: Option<Extension<ReportsNavigation>>,
    headers: axum::http::HeaderMap,
    Query(query): Query<RolePageQuery>,
) -> Response {
    let manage = match s.auth.check(context, Permission::ManageRoles).await {
        Ok(()) => true,
        Err(Error::Denied) => false,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let moderate_reports = if reports_navigation.is_some() {
        match s
            .auth
            .check(context, Permission::ManageSupportReports)
            .await
        {
            Ok(()) => true,
            Err(Error::Denied) => false,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        false
    };
    let players = if s.players_enabled {
        match s
            .auth
            .check(context, Permission::ManageSupportTickets)
            .await
        {
            Ok(()) => true,
            Err(Error::Denied) => false,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        false
    };
    let roles = if manage {
        match role_page(&s.pool, query.after).await {
            Ok(v) => v,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        vec![]
    };
    let csrf = if manage {
        match session.csrf_token().await {
            Ok(v) => v,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        String::new()
    };
    let lang = language(&headers);
    let mail_link = match s.auth.check(context, Permission::MailView).await {
        Ok(()) => Some(text(lang, "mail-title")),
        Err(Error::Denied) => None,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let native_login_link = match s.auth.check(context, Permission::ViewAudit).await {
        Ok(()) => Some(text(lang, "native-login-title")),
        Err(Error::Denied) => None,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    oteryn_web::render(&Page {
        lang,
        title: text(lang, "title"),
        roles_label: text(lang, "roles"),
        reports_label: text(lang, "reports"),
        moderate_reports,
        mail_link,
        native_login_link,
        players_label: text(lang, "players"),
        players,
        account_label: text(lang, "account"),
        role_label: text(lang, "role"),
        grant: text(lang, "grant"),
        revoke: text(lang, "revoke"),
        csrf,
        next: (roles.len() == ROLE_PAGE_SIZE as usize)
            .then(|| roles.last().map(|r| r.0))
            .flatten(),
        roles,
        manage,
    })
}
#[derive(Deserialize)]
struct RoleForm {
    #[serde(default)]
    account: Uuid,
    #[serde(default)]
    role: Uuid,
    action: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    permissions: String,
}
async fn change<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Form(form): Form<RoleForm>,
) -> Response {
    let result = match form.action.as_str() {
        "create" | "set-permissions" => {
            let permissions: Result<Vec<Permission>, _> = form
                .permissions
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|value| !value.is_empty())
                .map(str::parse)
                .collect();
            let Ok(permissions) = permissions else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            let change = if form.action == "create" {
                Change::CreateRole {
                    key: &form.key,
                    permissions: &permissions,
                }
            } else {
                Change::SetPermissions {
                    role: form.role,
                    permissions: &permissions,
                }
            };
            s.auth
                .change_from_panel(context, Uuid::now_v7(), change)
                .await
                .map(|_| ())
        }
        "grant" if !form.account.is_nil() && !form.role.is_nil() => s
            .auth
            .change_from_panel(
                context,
                Uuid::now_v7(),
                Change::Grant {
                    account: form.account,
                    role: form.role,
                },
            )
            .await
            .map(|_| ()),
        "revoke" => revoke(&s, context, form.account, form.role).await,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    match result {
        Ok(()) => Redirect::to("/admin/roles").into_response(),
        Err(Error::Denied) => match refusal(&s.pool, context.account, "/admin/roles").await {
            Ok(()) => StatusCode::FORBIDDEN.into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
        Err(Error::InvalidInput | Error::RoleNotFound) => StatusCode::BAD_REQUEST.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
/// Serialize with every T10 role writer and recheck authorization under that lock.
/// T10 has no owner key: its full-access platform administrator is also protected.
async fn revoke<I: StaffSession, A: AccountStateReader>(
    s: &Panel<I, A>,
    context: CheckContext,
    account: Uuid,
    role: Uuid,
) -> Result<(), Error> {
    if account.is_nil() || role.is_nil() {
        return Err(Error::InvalidInput);
    }
    // Discover a potential survivor with bounded keyset batches, without holding
    // the RBAC fence. Its state and membership are rechecked after account locks.
    let mut cursor = None;
    let mut survivor = None;
    loop {
        let batch = holder_batch(&s.pool, role, account, cursor).await?;
        if batch.is_empty() {
            break;
        }
        cursor = batch.last().copied();
        for candidate in batch {
            if s.auth.account_active(candidate).await {
                survivor = Some(candidate);
                break;
            }
        }
        if survivor.is_some() {
            break;
        }
    }
    let mut tx = s.pool.begin().await?;
    crate::rbac::mutation_budget(&mut tx).await?;
    let mut accounts = vec![context.account, account];
    accounts.extend(survivor);
    crate::rbac::authorize_mutation(
        &mut tx,
        &accounts,
        s.auth.as_ref(),
        context,
        &[Permission::Access, Permission::ManageRoles],
    )
    .await?;
    for permission in [Permission::Access, Permission::ManageRoles] {
        s.auth.check_in(&mut tx, context, permission).await?;
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE id=$1)")
        .bind(role)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        return Err(Error::RoleNotFound);
    }
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2)",
    )
    .bind(account)
    .bind(role)
    .fetch_one(&mut *tx)
    .await?;
    if assigned && !oteryn_db::protected_role_survives(&mut tx, account, Some(role)).await? {
        return Err(Error::Denied);
    }
    if assigned {
        sqlx::query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=$2")
            .bind(account)
            .bind(role)
            .execute(&mut *tx)
            .await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: "rbac.role.revoked",
                target: &account.to_string(),
                before: json!({"role":role,"assigned":true}),
                after: json!({"role":role,"assigned":false}),
            },
        )
        .await?;
    }
    if assigned && account == context.account {
        // Self-revocation removed the actor's grant; the retained RBAC fence
        // keeps roles fixed, so only time-bound session state can change.
        s.auth.session_in(&mut tx, context).await?;
    } else {
        for permission in [Permission::Access, Permission::ManageRoles] {
            s.auth.check_in(&mut tx, context, permission).await?;
        }
    }
    if assigned {
        Change::Revoke { account, role }
            .revoke_browser_credentials(&mut tx, context)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;
