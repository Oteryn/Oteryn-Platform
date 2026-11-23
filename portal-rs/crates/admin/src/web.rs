//! Staff surface. Authentication adapters are trusted server configuration, never request data.
use crate::staff_actions;
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{
        AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, Change,
        CheckContext, Error, Permission,
    },
    support::{
        Area,
        web::{REPORTS_FLAG, REPORTS_PATH},
    },
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Query, Request, State, rejection::FormRejection},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_ui::panels::{self, Panel as PanelKind};
use oteryn_web::session::{BrowserSession, SessionLayer, SessionMutationDeadline};
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
    fn landing_url(&self) -> Option<String> {
        self.admin_panel
            .then(|| format!("{}/admin", self.csrf_origin()))
    }

    pub(crate) fn csrf_origin(&self) -> String {
        format!(
            "https://{}",
            oteryn_web::staff_host::canonical_authority(&self.host).unwrap_or_default()
        )
    }
}

struct Panel<I, A> {
    config: PanelConfig,
    /// Set for feature routes wrapped by [`guarded`]; the panel itself picks per path.
    required: Option<Vec<Permission>>,
    reports_enabled: bool,
    grant_confirmation: bool,
    identity: I,
    auth: Arc<Authorizer<A>>,
    pool: PgPool,
}

pub(crate) fn config_from_env() -> Result<PanelConfig, Box<dyn std::error::Error>> {
    config_from(&|key| std::env::var(key).ok())
}

pub(crate) fn config_from(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<PanelConfig, Box<dyn std::error::Error>> {
    let enabled = lookup("OTERYN_ADMIN_PANEL").is_some_and(|v| v == "true");
    let host = if enabled {
        let host = lookup("OTERYN_ADMIN_HOST").unwrap_or_default();
        oteryn_web::staff_host::canonical_authority(&host).ok_or("invalid admin host")?
    } else {
        String::new()
    };
    Ok(PanelConfig {
        admin_panel: enabled,
        host,
    })
}

/// Register the dashboard destination from the same host and flag as the panel.
pub fn configured_dashboard_host() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = config_from_env()?;
    Ok(config.admin_panel.then_some(config.host))
}

pub fn configured_router(
    pool: PgPool,
    grant_confirmation: bool,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(router_with_reports(
        pool.clone(),
        config_from_env()?,
        PortalIdentity::configured(pool.clone()),
        PortalIdentity::configured(pool),
        std::env::var(REPORTS_FLAG).is_ok_and(|value| value == "true"),
        grant_confirmation,
    ))
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
    configured_guard_any_from(routes, pool, &[permission], lookup)
}

pub(crate) fn configured_guard_any_from(
    routes: Router,
    pool: PgPool,
    permissions: &[Permission],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(guarded_any(
        routes,
        pool.clone(),
        config_from(lookup)?,
        PortalIdentity::configured_from(pool.clone(), lookup),
        PortalIdentity::configured_from(pool, lookup),
        permissions,
    ))
}

/// Check the staff identity and permissions before an upload body is read.
#[derive(Clone)]
pub struct StaffPreflight {
    state: Arc<Panel<PortalIdentity, PortalIdentity>>,
    sessions: SessionLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadPreflight {
    Short,
    Long,
    Denied,
}

#[derive(Debug)]
pub enum StaffGuardError {
    Status(StatusCode),
    Login,
    Mfa(&'static str),
}

impl From<StatusCode> for StaffGuardError {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl IntoResponse for StaffGuardError {
    fn into_response(self) -> Response {
        match self {
            Self::Status(status) => status.into_response(),
            Self::Login => Redirect::to("/login").into_response(),
            Self::Mfa(lang) => mfa_refusal(lang),
        }
    }
}

impl std::fmt::Debug for StaffPreflight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaffPreflight").finish_non_exhaustive()
    }
}

pub fn configured_staff_preflight_from(
    pool: PgPool,
    permission: Permission,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<StaffPreflight, Box<dyn std::error::Error>> {
    let config = config_from(lookup)?;
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = vec![config.csrf_origin()];
    let identity = PortalIdentity::configured_from(pool.clone(), lookup);
    let state = Arc::new(Panel {
        config,
        required: Some(vec![permission]),
        reports_enabled: false,
        grant_confirmation: false,
        auth: Arc::new(Authorizer::new(pool.clone(), identity.clone())),
        identity,
        pool,
    });
    Ok(StaffPreflight { state, sessions })
}

impl StaffPreflight {
    pub fn wrap(&self, routes: Router) -> Router {
        guard_routes(routes, self.sessions.clone(), self.state.clone())
    }

    /// A matching Origin and a live authorized staff session qualify for a
    /// longer body deadline. The full guard still checks CSRF after the read.
    pub async fn upload_body_access(
        &self,
        headers: &axum::http::HeaderMap,
        path: &str,
        locale: Option<oteryn_web::Language>,
    ) -> Result<UploadPreflight, StaffGuardError> {
        oteryn_web::csrf::verify_origin(headers, &self.sessions.csrf.trusted_origins)?;
        if !headers.contains_key(header::ORIGIN) || !headers.contains_key(header::COOKIE) {
            return Ok(UploadPreflight::Short);
        }
        let mut tx = self.state.pool.begin().await.map_err(preflight_error)?;
        // Let PostgreSQL finish or cancel its work before the HTTP preflight
        // deadline. These limits stay on this transaction when the pool reuses it.
        sqlx::raw_sql("SET TRANSACTION READ ONLY; SET LOCAL lock_timeout='1s'; SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await
            .map_err(preflight_error)?;
        let session = oteryn_web::session::signed_in_session(&self.sessions, &mut tx, headers)
            .await
            .map_err(preflight_error)?;
        let Some(session) = session else {
            return Ok(UploadPreflight::Short);
        };
        let Some(account) = session.account_id().await else {
            return Ok(UploadPreflight::Short);
        };
        let lang = locale.map_or_else(|| language(headers), oteryn_web::Language::code);
        let grant = session
            .get_in(&mut tx, mfa_grant_key())
            .await
            .map_err(preflight_error)?;
        let Some(proof) = session_proof(&session, grant).await else {
            return Err(StaffGuardError::Mfa(lang));
        };
        if self.state.identity.mfa.is_none()
            || !crate::rbac::valid_durable_session_in(&mut tx, account, proof, None)
                .await
                .map_err(preflight_error)?
        {
            return Err(StaffGuardError::Mfa(lang));
        }
        for &required in [Permission::Access]
            .iter()
            .chain(staff_permissions(&self.state, path))
        {
            let check = match self.state.auth.check_permission_flag(required) {
                Ok(()) => {
                    Authorizer::<PortalIdentity>::check_permission(&mut tx, account, required).await
                }
                Err(error) => Err(error),
            };
            match check {
                Ok(()) => {}
                Err(Error::Storage(error)) => return Err(preflight_error(error)),
                Err(_) => {
                    tx.commit().await.map_err(preflight_error)?;
                    // The normal guard audits a denied actor only after CSRF verification.
                    return Ok(UploadPreflight::Denied);
                }
            }
        }
        tx.commit().await.map_err(preflight_error)?;
        Ok(UploadPreflight::Long)
    }
}

fn preflight_error(error: sqlx::Error) -> StaffGuardError {
    let status = match error {
        sqlx::Error::Protocol(message) if message == "invalid session cookie" => {
            StatusCode::BAD_REQUEST
        }
        sqlx::Error::Database(error)
            if matches!(error.code().as_deref(), Some("55P03" | "57014")) =>
        {
            StatusCode::REQUEST_TIMEOUT
        }
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    status.into()
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
    guarded_any(routes, pool, config, identity, accounts, &[permission])
}

pub(crate) fn guarded_any<I: StaffSession, A: AccountStateReader + 'static>(
    routes: Router,
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    permissions: &[Permission],
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    if permissions.contains(&Permission::ManageSupportEnforcement) {
        session = session.with_bounded_refresh();
    }
    session.csrf.trusted_origins = vec![config.csrf_origin()];
    let state = Arc::new(Panel {
        config,
        required: Some(permissions.to_vec()),
        reports_enabled: false,
        grant_confirmation: false,
        identity,
        auth: Arc::new(Authorizer::new(pool.clone(), accounts)),
        pool,
    });
    guard_routes(routes, session, state)
}

fn guard_routes<I: StaffSession, A: AccountStateReader + 'static>(
    routes: Router,
    session: SessionLayer,
    state: Arc<Panel<I, A>>,
) -> Router {
    let admin_csrf = session.csrf.clone();
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
    router_with_reports(pool, config, identity, accounts, false, true)
}

fn router_with_reports<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    reports_enabled: bool,
    grant_confirmation: bool,
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![config.csrf_origin()];
    let admin_csrf = session.csrf.clone();
    let state = Arc::new(Panel {
        config,
        required: None,
        reports_enabled,
        grant_confirmation,
        identity,
        auth: Arc::new(Authorizer::new(pool.clone(), accounts)),
        pool,
    });
    Router::new()
        .route("/admin", get(page::<I, A>))
        .route("/admin/roles", get(page::<I, A>).post(change::<I, A>))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate::<I, A>,
        ))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .route_layer(Extension(oteryn_web::csrf::LocalizedRefusal))
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
    if !config.admin_panel || !oteryn_web::staff_host::matches(request.headers(), &config.host) {
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
    let headers = response.headers_mut();
    if headers
        .get(header::REFERRER_POLICY)
        .is_none_or(|policy| policy != "no-referrer" && policy != "same-origin")
    {
        headers.insert(
            header::REFERRER_POLICY,
            axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
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
    let deadline = request.extensions().get::<SessionMutationDeadline>();
    let lang = request
        .extensions()
        .get::<oteryn_web::Language>()
        .copied()
        .map_or_else(
            || {
                page_language(
                    request.headers(),
                    request.extensions().get::<oteryn_ui::Chrome>(),
                )
            },
            oteryn_web::Language::code,
        );
    let context = match request_staff_context(&s, session, lang, request.uri().path(), deadline)
        .await
    {
        Ok(context) => context,
        Err(StaffGuardError::Login)
            if request.method() == axum::http::Method::GET && request.uri().path() == "/admin" =>
        {
            return Redirect::to(oteryn_web::staff_host::DASHBOARD_LOGIN_PATH).into_response();
        }
        Err(StaffGuardError::Status(StatusCode::FORBIDDEN)) => {
            let fallback = oteryn_ui::Registry::default().chrome(
                lang,
                request.uri().path(),
                oteryn_ui::Account::Anonymous,
            );
            return oteryn_ui::error(
                request
                    .extensions()
                    .get::<oteryn_ui::Chrome>()
                    .unwrap_or(&fallback),
                StatusCode::FORBIDDEN,
            );
        }
        Err(error) => return error.into_response(),
    };
    request.extensions_mut().insert(context);
    request
        .extensions_mut()
        .insert(crate::rbac::MutationAuthorization::new(s.auth.clone()));
    next.run(request).await
}
async fn request_staff_context<I: StaffSession, A: AccountStateReader + 'static>(
    s: &Panel<I, A>,
    session: &BrowserSession,
    lang: &'static str,
    path: &str,
    deadline: Option<&SessionMutationDeadline>,
) -> Result<CheckContext, StaffGuardError> {
    let Some(identity) = read_request(deadline, s.identity.resolve(session)).await? else {
        return Err(StaffGuardError::Login);
    };
    if !identity.mfa_satisfied {
        return Err(StaffGuardError::Mfa(lang));
    }
    let context = read_request(deadline, staff_context(session, identity)).await?;
    let required = staff_permissions(s, path);
    let authorized = read_request(deadline, async {
        match s.auth.check(context, Permission::Access).await {
            Ok(()) if !required.is_empty() => s.auth.check_any(context, required).await,
            result => result,
        }
    })
    .await?;
    if let Err(error) = authorized {
        if matches!(error, Error::Storage(_)) {
            return Err(StatusCode::SERVICE_UNAVAILABLE.into());
        }
        let target = if path.starts_with(audit::EDITORIAL_PAGE_PATH_PREFIX) {
            audit::EDITORIAL_PAGE_ROUTE
        } else {
            path
        };
        return Err(
            match refusal(&s.pool, context.account, target, deadline).await {
                Ok(()) => StatusCode::FORBIDDEN,
                Err(_) => StatusCode::SERVICE_UNAVAILABLE,
            }
            .into(),
        );
    }
    Ok(context)
}

pub(crate) async fn staff_context(
    session: &BrowserSession,
    identity: StaffIdentity,
) -> CheckContext {
    let grant = session.get(mfa_grant_key()).await.ok().flatten();
    CheckContext {
        account: identity.account,
        require_staff_mfa: true,
        mfa_verified: identity.mfa_satisfied,
        session_proof: session_proof(session, grant).await,
    }
}

fn mfa_grant_key() -> oteryn_web::session::SessionKey<String> {
    oteryn_web::session::SessionKey::new(oteryn_identity::mfa::SESSION_GRANT_KEY)
}

async fn session_proof(
    session: &BrowserSession,
    grant: Option<String>,
) -> Option<crate::rbac::SessionProof> {
    use sha2::{Digest, Sha256};
    let grant = grant?;
    Some(crate::rbac::SessionProof {
        session_hash: session.identity_hash().await?.try_into().ok()?,
        grant_hash: Sha256::digest(grant.as_bytes()).into(),
    })
}

fn staff_permissions<'a, I, A>(s: &'a Panel<I, A>, path: &str) -> &'a [Permission] {
    s.required.as_deref().unwrap_or_else(|| {
        if path == "/admin/roles" {
            &[Permission::ManageRoles]
        } else {
            &[]
        }
    })
}

#[derive(Template)]
#[template(
    source = r#"<html lang="{{ lang }}"><meta name="viewport" content="width=device-width, initial-scale=1"><main><h1>{{ message }}</h1></main></html>"#,
    ext = "html"
)]
struct MfaRefusal<'a> {
    lang: &'a str,
    message: String,
}

fn mfa_refusal(lang: &str) -> Response {
    match (MfaRefusal {
        lang,
        message: text(lang, "mfa"),
    })
    .render()
    {
        Ok(html) => (StatusCode::FORBIDDEN, axum::response::Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Presentation check for the shared account menu.
pub fn configured_dashboard_link(
    pool: PgPool,
) -> Result<Option<oteryn_web::chrome::StaffDashboard>, Box<dyn std::error::Error>> {
    let Some(url) = config_from_env()?.landing_url() else {
        return Ok(None);
    };
    let auth = Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured(pool),
    ));
    Ok(Some(oteryn_web::chrome::StaffDashboard {
        url,
        allows: Arc::new(move |account| {
            let auth = auth.clone();
            Box::pin(async move {
                auth.check(
                    CheckContext {
                        require_staff_mfa: false,
                        ..CheckContext::new(account)
                    },
                    Permission::Access,
                )
                .await
                .is_ok()
            })
        }),
    }))
}

pub(crate) async fn read_request<T>(
    deadline: Option<&SessionMutationDeadline>,
    read: impl Future<Output = T>,
) -> Result<T, StatusCode> {
    match deadline {
        Some(deadline) => deadline.read_request(read).await,
        None => Ok(read.await),
    }
}

async fn refusal(
    pool: &PgPool,
    actor: Uuid,
    target: &str,
    deadline: Option<&SessionMutationDeadline>,
) -> Result<(), Error> {
    let mut tx = match deadline {
        Some(deadline) => deadline.begin(pool).await?,
        None => pool.begin().await?,
    };
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
    if deadline.is_some() && !oteryn_db::transaction::arm_commit(&mut tx, None).await? {
        return Err(sqlx::Error::PoolTimedOut.into());
    }
    tx.commit().await?;
    Ok(())
}
pub(crate) fn page_language(
    headers: &axum::http::HeaderMap,
    chrome: Option<&oteryn_ui::Chrome>,
) -> &'static str {
    chrome.map_or_else(
        || language(headers),
        |chrome| if chrome.lang == "en" { "en" } else { "pl" },
    )
}

pub(crate) fn language(headers: &axum::http::HeaderMap) -> &'static str {
    oteryn_web::negotiate(headers).code()
}
pub(crate) fn selected_language(
    headers: &axum::http::HeaderMap,
    lang: Option<&str>,
) -> &'static str {
    match lang {
        Some("pl") => "pl",
        Some("en") => "en",
        _ => language(headers),
    }
}
pub(crate) fn text(lang: &str, key: &str) -> String {
    text_with_args(lang, key, None)
}
pub(crate) fn text_with_args(
    lang: &str,
    key: &str,
    args: Option<&fluent_bundle::FluentArgs<'_>>,
) -> String {
    translated_with_args(translations(lang).as_ref(), key, args)
}
pub(crate) fn translations(
    lang: &str,
) -> Option<fluent_bundle::FluentBundle<fluent_bundle::FluentResource>> {
    let locale = lang.parse().ok()?;
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("../../../templates/admin/en.ftl")
    } else {
        include_str!("../../../templates/admin/pl.ftl")
    };
    let resource = fluent_bundle::FluentResource::try_new(source.to_owned()).ok()?;
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}
pub(crate) fn translated(
    bundle: Option<&fluent_bundle::FluentBundle<fluent_bundle::FluentResource>>,
    key: &str,
) -> String {
    translated_with_args(bundle, key, None)
}

fn translated_with_args(
    bundle: Option<&fluent_bundle::FluentBundle<fluent_bundle::FluentResource>>,
    key: &str,
    args: Option<&fluent_bundle::FluentArgs<'_>>,
) -> String {
    bundle
        .and_then(|bundle| {
            let pattern = bundle.get_message(key)?.value()?;
            Some(
                bundle
                    .format_pattern(pattern, args, &mut vec![])
                    .into_owned(),
            )
        })
        .unwrap_or_default()
}
#[derive(Template)]
#[template(path = "admin/roles.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    roles_label: String,
    invites_label: String,
    reports_label: String,
    reports_url: Option<&'static str>,
    account_label: String,
    role_label: String,
    grant: String,
    can_grant: bool,
    reason_label: String,
    reason_assignment: String,
    step_up_label: String,
    navigation: Vec<panels::Link>,
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
    lang: Option<String>,
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
    headers: axum::http::HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
    Query(query): Query<RolePageQuery>,
) -> Response {
    let manage = match s.auth.check(context, Permission::ManageRoles).await {
        Ok(()) => true,
        Err(Error::Denied) => false,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let reports_url = if s.reports_enabled {
        match s.auth.check_any(context, Area::Reports.permissions()).await {
            Ok(()) => Some(REPORTS_PATH),
            Err(Error::Denied) => None,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        None
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
    let lang = if query.lang.is_some() {
        selected_language(&headers, query.lang.as_deref())
    } else {
        page_language(&headers, chrome.as_deref())
    };
    let mut registry = panels::Registry::default();
    for &(route, label, permission, flag) in crate::rbac::ADMIN_PANEL_SECTIONS {
        registry.register(
            flag.is_none_or(|key| std::env::var(key).as_deref() == Ok("true"))
                && s.auth.check(context, permission).await.is_ok(),
            panels::Entry {
                panel: PanelKind::Admin,
                route,
                label,
                permission: panels::Permission::Staff(permission.as_str()),
            },
        );
    }
    if std::env::var("OTERYN_ADMIN_ANALYTICS_PRESENT").as_deref() == Ok("true") {
        let analytics_auth = crate::analytics::web::view_authorizer(
            s.pool.clone(),
            PortalIdentity::configured(s.pool.clone()),
            true,
        );
        if let Some(permission) =
            crate::analytics::web::view_permission(&analytics_auth, context).await
        {
            registry.register(
                true,
                panels::Entry {
                    panel: PanelKind::Admin,
                    route: crate::analytics::web::OVERVIEW_ROUTE,
                    label: "ops-analytics",
                    permission: panels::Permission::Staff(permission.as_str()),
                },
            );
        }
    }
    if std::env::var("OTERYN_ADMIN_CONSOLE").as_deref() == Ok("true") {
        let game_auth = Authorizer::new(s.pool.clone(), PortalIdentity::configured(s.pool.clone()))
            .with_game_flags(crate::rbac::GamePermissionFlags {
                enabled: true,
                deploy: false,
            });
        for &(_, _, permission, _) in crate::rbac::GM_PANEL_SECTIONS {
            if game_auth.check(context, permission).await.is_ok() {
                registry.register(
                    true,
                    panels::Entry {
                        panel: PanelKind::Admin,
                        route: PanelKind::Gm.root(),
                        label: PanelKind::Gm.label(),
                        permission: panels::Permission::Staff(permission.as_str()),
                    },
                );
                break;
            }
        }
    }
    oteryn_web::render(&Page {
        lang,
        title: text(lang, "title"),
        roles_label: text(lang, "roles"),
        invites_label: text(lang, "invite-title"),
        reports_label: text(lang, "player-reports"),
        reports_url,
        account_label: text(lang, "account"),
        role_label: text(lang, "role"),
        grant: text(lang, "grant"),
        can_grant: manage
            && s.grant_confirmation
            && s.auth.check(context, Permission::GrantRoles).await.is_ok(),
        reason_label: oteryn_ui::text(lang, "staff-action-reason"),
        reason_assignment: oteryn_ui::text(lang, "staff-action-role-assignment"),
        step_up_label: oteryn_ui::text(lang, "staff-action-step-up"),
        navigation: registry.links(PanelKind::Admin, lang, |_| true),
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
    #[serde(default)]
    reason: String,
}
async fn change<I: StaffSession, A: AccountStateReader + 'static>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(authority): Extension<crate::rbac::MutationAuthorization>,
    csrf: Option<Extension<oteryn_web::csrf::Verified>>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
    headers: axum::http::HeaderMap,
    form: Result<Form<RoleForm>, FormRejection>,
) -> Response {
    let localized = |status: StatusCode| {
        (
            status,
            oteryn_web::negotiate(&headers).text(&format!("error-{}-description", status.as_u16())),
        )
            .into_response()
    };
    let invalid = || localized(StatusCode::BAD_REQUEST);
    let standalone = chrome.is_none();
    let fallback = oteryn_ui::Registry::default().chrome(
        language(&headers),
        "/admin/roles",
        oteryn_ui::Account::Anonymous,
    );
    let chrome = chrome
        .as_ref()
        .map_or(&fallback, |Extension(chrome)| chrome);
    let Form(form) = match form {
        Ok(form) => form,
        Err(error) if error.status() == StatusCode::UNPROCESSABLE_ENTITY => {
            if standalone {
                return localized(StatusCode::UNPROCESSABLE_ENTITY);
            }
            return staff_actions::Error::Invalid.response(chrome);
        }
        Err(error) => return error.into_response(),
    };
    let csrf = csrf.map(|Extension(proof)| proof);
    if let Err(error) = staff_actions::verify_session(context, csrf) {
        return error.response(chrome);
    }
    let reason = if form.reason.is_empty() {
        ""
    } else {
        match staff_actions::validated_code(&form.reason, &[staff_actions::ROLE_REASON]) {
            Ok(reason) => reason,
            Err(error) => return error.response(chrome),
        }
    };
    let guard = staff_actions::Guard {
        actor: context,
        authority: &authority,
        csrf,
    };
    let grant_action = staff_actions::Action {
        permission: Permission::GrantRoles,
        reason,
        reason_codes: &[staff_actions::ROLE_REASON],
        step_up: staff_actions::StepUp::RecentPassword,
    };
    let result = match form.action.as_str() {
        "create" | "create-grant-role" | "set-permissions" => {
            let permissions: Result<Vec<Permission>, _> = form
                .permissions
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|value| !value.is_empty())
                .map(str::parse)
                .collect();
            let Ok(permissions) = permissions else {
                return invalid();
            };
            if form.action == "set-permissions" {
                if form.role.is_nil() {
                    return invalid();
                }
                let result = s
                    .auth
                    .edit_permissions_from_panel(
                        guard,
                        grant_action,
                        s.grant_confirmation,
                        form.role,
                        permissions,
                    )
                    .await;
                return staff_action_response(&s.pool, context.account, result, chrome).await;
            }
            if form.action == "create-grant-role" {
                if !s.grant_confirmation {
                    return staff_actions::Error::Forbidden.response(chrome);
                }
                let auth = s.auth.clone();
                let result = staff_actions::run(
                    &s.pool,
                    guard,
                    grant_action,
                    || Ok((Vec::new(), (form.key, permissions))),
                    move |connection, (key, permissions)| {
                        Box::pin(async move {
                            Ok(Some(
                                auth.create_grant_role(connection, context, &key, &permissions)
                                    .await?,
                            ))
                        })
                    },
                )
                .await;
                return staff_action_response(&s.pool, context.account, result, chrome).await;
            }
            s.auth
                .change_from_panel_with_reason(
                    context,
                    Uuid::now_v7(),
                    Change::CreateRole {
                        key: &form.key,
                        permissions: &permissions,
                    },
                    reason,
                    false,
                )
                .await
                .map(|_| ())
        }
        "grant" => {
            if !s.grant_confirmation {
                return staff_actions::Error::Forbidden.response(chrome);
            }
            if form.account.is_nil() || form.role.is_nil() {
                return invalid();
            }
            use staff_actions::AuditChange;
            let auth = s.auth.clone();
            let result = staff_actions::run(
                &s.pool,
                guard,
                grant_action,
                || Ok((vec![form.account], (form.account, form.role))),
                move |connection, (account, role)| {
                    Box::pin(async move {
                        let granted = auth
                            .grant_from_staff_action(connection, context, account, role)
                            .await?;
                        Ok(granted.then_some(AuditChange::RoleGrant { account, role }))
                    })
                },
            )
            .await;
            return staff_action_response(&s.pool, context.account, result, chrome).await;
        }
        "revoke" => {
            if form.account.is_nil() || form.role.is_nil() {
                return invalid();
            }
            if reason.is_empty() {
                return staff_actions::Error::Reason.response(chrome);
            }
            revoke(&s, context, form.account, form.role, reason).await
        }
        _ => return invalid(),
    };
    match result {
        Ok(()) => Redirect::to("/admin/roles").into_response(),
        Err(Error::Denied) => match refusal(&s.pool, context.account, "/admin/roles", None).await {
            Ok(()) => StatusCode::FORBIDDEN.into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
        Err(Error::InvalidInput | Error::RoleNotFound) => invalid(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn staff_action_response(
    pool: &PgPool,
    actor: Uuid,
    result: Result<(), staff_actions::Error>,
    chrome: &oteryn_ui::Chrome,
) -> Response {
    match result {
        Ok(()) => Redirect::to("/admin/roles").into_response(),
        Err(error @ (staff_actions::Error::Forbidden | staff_actions::Error::StepUp)) => {
            match refusal(pool, actor, "/admin/roles", None).await {
                Ok(()) => error.response(chrome),
                Err(_) => staff_actions::Error::Unavailable.response(chrome),
            }
        }
        Err(error) => error.response(chrome),
    }
}

/// Serialize with every T10 role writer and recheck authorization under that lock.
/// T10 has no owner key: its full-access platform administrator is also protected.
async fn revoke<I: StaffSession, A: AccountStateReader>(
    s: &Panel<I, A>,
    context: CheckContext,
    account: Uuid,
    role: Uuid,
    reason: &'static str,
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
        audit::append_with_reason(
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
            (!reason.is_empty()).then_some(reason),
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
mod landing_tests {
    use super::*;

    #[test]
    fn staff_landing_uses_configured_admin_origin_only_when_enabled()
    -> Result<(), Box<dyn std::error::Error>> {
        for enabled in [false, true] {
            let config = config_from(&|name| match name {
                "OTERYN_ADMIN_PANEL" => Some(enabled.to_string()),
                "OTERYN_ADMIN_HOST" => Some("Staff.Example.test:8443".into()),
                _ => None,
            })?;
            assert_eq!(
                config.landing_url().as_deref(),
                enabled.then_some("https://staff.example.test:8443/admin")
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;
