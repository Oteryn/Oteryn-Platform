//! Staff surface. Authentication adapters are trusted server configuration, never request data.
use crate::staff_actions;
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
    extract::{Query, Request, State, rejection::FormRejection},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_ui::panels::{self, Panel as PanelKind};
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

struct Panel<I, A> {
    config: PanelConfig,
    /// Set for feature routes wrapped by [`guarded`]; the panel itself picks per path.
    required: Option<Permission>,
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
    let host = lookup("OTERYN_ADMIN_HOST").unwrap_or_default();
    if enabled && (host.is_empty() || host.parse::<axum::http::uri::Authority>().is_err()) {
        return Err("invalid admin host".into());
    }
    Ok(PanelConfig {
        admin_panel: enabled,
        host,
    })
}

pub fn configured_router(
    pool: PgPool,
    grant_confirmation: bool,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(roles_router(
        pool.clone(),
        config_from_env()?,
        PortalIdentity::configured(pool.clone()),
        PortalIdentity::configured(pool),
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
        grant_confirmation: false,
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
    roles_router(pool, config, identity, accounts, true)
}

fn roles_router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    grant_confirmation: bool,
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![config.csrf_origin()];
    let admin_csrf = session.csrf.clone();
    let state = Arc::new(Panel {
        config,
        required: None,
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
        let locale = request.extensions().get::<oteryn_ui::Chrome>().map_or_else(
            || language(request.headers()),
            |chrome| chrome.lang.as_str(),
        );
        return (StatusCode::FORBIDDEN, axum::response::Html(format!("<html lang=\"{locale}\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><main><h1>{}</h1></main></html>", text(locale, "mfa")))).into_response();
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
    let permission = s
        .required
        .unwrap_or(if request.uri().path() == "/admin/roles" {
            Permission::ManageRoles
        } else {
            Permission::Access
        });
    for required in [Permission::Access, permission] {
        if let Err(error) = s.auth.check(context, required).await {
            if matches!(error, Error::Storage(_)) {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            return match refusal(&s.pool, context.account, request.uri().path()).await {
                Ok(()) => {
                    let fallback = oteryn_ui::Registry::default().chrome(
                        language(request.headers()),
                        request.uri().path(),
                        oteryn_ui::Account::Anonymous,
                    );
                    oteryn_ui::error(
                        request
                            .extensions()
                            .get::<oteryn_ui::Chrome>()
                            .unwrap_or(&fallback),
                        StatusCode::FORBIDDEN,
                    )
                }
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
/// Presentation check for the shared account menu.
pub fn configured_dashboard_link(
    pool: PgPool,
) -> Result<Option<oteryn_web::chrome::StaffDashboard>, Box<dyn std::error::Error>> {
    let config = config_from_env()?;
    if !config.admin_panel {
        return Ok(None);
    }
    let auth = Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured(pool),
    ));
    Ok(Some(oteryn_web::chrome::StaffDashboard {
        url: format!("{}/admin", config.csrf_origin()),
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
pub(crate) fn page_language(
    headers: &axum::http::HeaderMap,
    chrome: Option<&oteryn_ui::Chrome>,
) -> &'static str {
    match chrome.map(|chrome| chrome.lang.as_str()) {
        Some("en") => "en",
        Some(_) => "pl",
        None => language(headers),
    }
}

fn language(headers: &axum::http::HeaderMap) -> &'static str {
    oteryn_web::negotiate(headers).code()
}
fn text(lang: &str, key: &str) -> String {
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("../../../templates/admin/en.ftl")
    } else {
        include_str!("../../../templates/admin/pl.ftl")
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
#[derive(Template)]
#[template(path = "admin/roles.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    roles_label: String,
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
    let lang = page_language(&headers, chrome.as_deref());
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
            return staff_actions::Error::Invalid.response(chrome);
        }
        Err(error) => return error.into_response(),
    };
    let guard = staff_actions::Guard {
        actor: context,
        authority: &authority,
        csrf: csrf.map(|Extension(proof)| proof),
    };
    let grant_action = staff_actions::Action {
        permission: Permission::GrantRoles,
        reason: &form.reason,
        reason_codes: &["role_assignment"],
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
                return StatusCode::BAD_REQUEST.into_response();
            };
            if form.action == "set-permissions" {
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
                .change_from_panel(
                    context,
                    Uuid::now_v7(),
                    Change::CreateRole {
                        key: &form.key,
                        permissions: &permissions,
                    },
                )
                .await
                .map(|_| ())
        }
        "grant" => {
            if !s.grant_confirmation {
                return staff_actions::Error::Forbidden.response(chrome);
            }
            use staff_actions::AuditChange;
            let auth = s.auth.clone();
            let result = staff_actions::run(
                &s.pool,
                guard,
                grant_action,
                || {
                    if form.account.is_nil() || form.role.is_nil() {
                        return Err(staff_actions::Error::Invalid);
                    }
                    Ok((vec![form.account], (form.account, form.role)))
                },
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
async fn staff_action_response(
    pool: &PgPool,
    actor: Uuid,
    result: Result<(), staff_actions::Error>,
    chrome: &oteryn_ui::Chrome,
) -> Response {
    match result {
        Ok(()) => Redirect::to("/admin/roles").into_response(),
        Err(error @ (staff_actions::Error::Forbidden | staff_actions::Error::StepUp)) => {
            match refusal(pool, actor, "/admin/roles").await {
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
