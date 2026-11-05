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
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
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
#[derive(Debug, Clone, Default)]
pub struct PanelConfig {
    pub admin_panel: bool,
    pub host: String,
}
struct Panel<I, A> {
    config: PanelConfig,
    identity: I,
    auth: Authorizer<A>,
    pool: PgPool,
}

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_ADMIN_PANEL").is_ok_and(|v| v == "true");
    let host = std::env::var("OTERYN_ADMIN_HOST").unwrap_or_default();
    if enabled && (host.is_empty() || host.parse::<axum::http::uri::Authority>().is_err()) {
        return Err("invalid admin host".into());
    }
    Ok(router(
        pool,
        PanelConfig {
            admin_panel: enabled,
            host,
        },
        PendingAuthentication,
        PendingAuthentication,
    ))
}
/// Mount under `/admin`; host checks run before session or CSRF processing.
pub fn router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![format!("https://{}", config.host)];
    let state = Arc::new(Panel {
        config,
        identity,
        auth: Authorizer::new(pool.clone(), accounts),
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
        .route_layer(middleware::from_fn_with_state(state.clone(), host::<I, A>))
        .with_state(state)
}
async fn host<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    request: Request,
    next: Next,
) -> Response {
    let hosts = request.headers().get_all(header::HOST);
    if !s.config.admin_panel
        || s.config.host.is_empty()
        || hosts.iter().count() != 1
        || !hosts
            .iter()
            .next()
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case(&s.config.host))
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
        ("cache-control", "no-store"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
async fn authenticate<I: StaffSession, A: AccountStateReader>(
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
    let context = CheckContext {
        account: identity.account,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let permission = if request.uri().path() == "/admin/roles" {
        Permission::ManageRoles
    } else {
        Permission::Access
    };
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
fn language(headers: &axum::http::HeaderMap) -> &'static str {
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
#[template(path = "admin/panel.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    roles_label: String,
    account_label: String,
    role_label: String,
    grant: String,
    revoke: String,
    csrf: String,
    roles: Vec<(Uuid, String)>,
    manage: bool,
}
async fn page<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: axum::http::HeaderMap,
) -> Response {
    let manage = match s.auth.check(context, Permission::ManageRoles).await {
        Ok(()) => true,
        Err(Error::Denied) => false,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let roles = if manage {
        match sqlx::query_as("SELECT id,key FROM roles ORDER BY key")
            .fetch_all(&s.pool)
            .await
        {
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
    oteryn_web::render(&Page {
        lang,
        title: text(lang, "title"),
        roles_label: text(lang, "roles"),
        account_label: text(lang, "account"),
        role_label: text(lang, "role"),
        grant: text(lang, "grant"),
        revoke: text(lang, "revoke"),
        csrf,
        roles,
        manage,
    })
}
#[derive(Deserialize)]
struct RoleForm {
    account: Uuid,
    role: Uuid,
    action: String,
}
async fn change<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Panel<I, A>>>,
    Extension(context): Extension<CheckContext>,
    Form(form): Form<RoleForm>,
) -> Response {
    if form.account.is_nil()
        || form.role.is_nil()
        || !matches!(form.action.as_str(), "grant" | "revoke")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    // Privileged grants require a future four-eyes workflow; this shell fails closed.
    let privileged: Result<bool,_>=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM role_permissions WHERE role_id=$1 AND permission='admin.roles.manage')").bind(form.role).fetch_one(&s.pool).await;
    match privileged {
        Ok(true) if form.action == "grant" => {
            return match refusal(&s.pool, context.account, "/admin/roles").await {
                Ok(()) => StatusCode::FORBIDDEN.into_response(),
                Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            };
        }
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        _ => {}
    }
    let result = if form.action == "grant" {
        s.auth
            .change(
                context,
                Uuid::now_v7(),
                Change::Grant {
                    account: form.account,
                    role: form.role,
                },
            )
            .await
            .map(|_| ())
    } else {
        revoke(&s, context, form.account, form.role).await
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
    let mut tx = s.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *tx)
        .await?;
    s.auth.check(context, Permission::ManageRoles).await?;
    let key: Option<String> = sqlx::query_scalar("SELECT key FROM roles WHERE id=$1")
        .bind(role)
        .fetch_optional(&mut *tx)
        .await?;
    let key = key.ok_or(Error::RoleNotFound)?;
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2)",
    )
    .bind(account)
    .bind(role)
    .fetch_one(&mut *tx)
    .await?;
    if assigned && matches!(key.as_str(), "owner" | "platform_admin") {
        let others: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM account_roles WHERE role_id=$1 AND account_id<>$2)",
        )
        .bind(role)
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
        if !others {
            return Err(Error::Denied);
        }
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
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod web_tests;
