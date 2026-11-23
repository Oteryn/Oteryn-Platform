use super::{Panel, StaffSession, language, text};
use crate::rbac::{
    AccountStateReader, CheckContext, Error, MutationAuthorization, Permission,
    staff::{self, InvitationKey, InviteError, NewInvitation, PendingInvitation},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub(super) const PATH: &str = "/admin/roles/invitations";

pub(super) async fn allowed<A: AccountStateReader>(
    panel: &Panel<impl StaffSession, A>,
    actor: CheckContext,
    permission: Permission,
) -> Result<bool, Error> {
    match panel.auth.check(actor, permission).await {
        Ok(()) => Ok(true),
        Err(Error::Denied) => Ok(false),
        Err(error) => Err(error),
    }
}

#[derive(Template)]
#[template(path = "admin/staff_invitations.html", config = "src/web_askama.toml")]
struct ListPage {
    lang: &'static str,
    csrf: String,
    roles: Vec<(Uuid, String)>,
    pending: Vec<PendingInvitation>,
    can_create: bool,
    can_list: bool,
    can_revoke: bool,
    notice: Option<&'static str>,
}

impl ListPage {
    fn label(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

#[derive(Deserialize)]
pub(super) struct ListQuery {
    result: Option<String>,
}

pub(super) async fn list<I: StaffSession, A: AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Response, StatusCode> {
    let lang = language(&headers);
    let unavailable = |_| StatusCode::SERVICE_UNAVAILABLE;
    let can_list = allowed(&panel, actor, Permission::ListStaffInvites)
        .await
        .map_err(unavailable)?;
    let can_create = allowed(&panel, actor, Permission::CreateStaffInvite)
        .await
        .map_err(unavailable)?;
    if !can_list && !can_create {
        return Err(StatusCode::FORBIDDEN);
    }
    let can_revoke = can_list
        && allowed(&panel, actor, Permission::RevokeStaffInvite)
            .await
            .map_err(unavailable)?;
    let roles = if can_create {
        staff::delegable_roles(&panel.pool, actor.account)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    } else {
        Vec::new()
    };
    let pending = if can_list {
        staff::pending(&panel.pool, actor.account)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    } else {
        Vec::new()
    };
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let notice = match query.result.as_deref() {
        Some("created") => Some("invite-created"),
        Some("revoked") => Some("invite-revoked"),
        _ => None,
    };
    Ok(oteryn_web::render(&ListPage {
        lang,
        csrf,
        roles,
        pending,
        can_create,
        can_list,
        can_revoke,
        notice,
    }))
}

#[derive(Deserialize)]
pub(super) struct CreateForm {
    email: String,
    role: Uuid,
    days: i32,
}

pub(super) async fn create<I: StaffSession, A: AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Form(form): Form<CreateForm>,
) -> Response {
    let lang = language(&headers);
    let mail_configured = [
        "OTERYN_SMTP_HOST",
        "OTERYN_SMTP_USERNAME",
        "OTERYN_SMTP_PASSWORD",
        "OTERYN_MAIL_FROM",
        "OTERYN_PUBLIC_ORIGIN",
    ]
    .into_iter()
    .all(|setting| std::env::var_os(setting).is_some_and(|value| !value.is_empty()));
    let Some(key) = InvitationKey::configured().filter(|_| mail_configured) else {
        return admin_notice(
            lang,
            StatusCode::SERVICE_UNAVAILABLE,
            "invite-service-unavailable",
        );
    };
    let input = NewInvitation {
        email: &form.email,
        role: form.role,
        days: form.days,
        locale: lang,
    };
    match staff::create(&panel.pool, &auth, actor, &key, input).await {
        Ok(_) => Redirect::to("/admin/roles/invitations?result=created").into_response(),
        Err(error) => admin_error(lang, error),
    }
}

pub(super) async fn revoke<I: StaffSession, A: AccountStateReader>(
    State(panel): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    let lang = language(&headers);
    match staff::revoke(&panel.pool, &auth, actor, id).await {
        Ok(()) => Redirect::to("/admin/roles/invitations?result=revoked").into_response(),
        Err(error) => admin_error(lang, error),
    }
}

fn admin_error(lang: &'static str, error: InviteError) -> Response {
    let (status, key) = match error {
        InviteError::Rbac(Error::InvalidInput | Error::RoleNotFound) => {
            (StatusCode::BAD_REQUEST, "invite-invalid-input")
        }
        InviteError::Duplicate => (StatusCode::CONFLICT, "invite-duplicate"),
        InviteError::Limit => (StatusCode::TOO_MANY_REQUESTS, "invite-limit"),
        InviteError::Rbac(Error::Denied) => (StatusCode::FORBIDDEN, "invite-forbidden"),
        InviteError::Unavailable => (StatusCode::NOT_FOUND, "invite-unavailable"),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "invite-service-unavailable",
        ),
    };
    admin_notice(lang, status, key)
}

fn admin_notice(lang: &'static str, status: StatusCode, key: &str) -> Response {
    (
        status,
        oteryn_web::render(&Notice {
            lang,
            title: text(lang, "invite-title"),
            message: text(lang, key),
        }),
    )
        .into_response()
}

#[derive(Template)]
#[template(
    path = "admin/staff_invitation_notice.html",
    config = "src/web_askama.toml"
)]
struct Notice {
    lang: &'static str,
    title: String,
    message: String,
}
impl Notice {
    fn label(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

pub(super) fn acceptance_router(pool: PgPool) -> Router {
    let mut layer = SessionLayer::new(pool.clone());
    layer.csrf.trusted_origins = vec![
        std::env::var("OTERYN_PUBLIC_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:3000".into())
            .trim_end_matches('/')
            .to_owned(),
    ];
    Router::new()
        .route("/{lang}/staff-invitation", get(confirm).post(accept))
        .route_layer(middleware::from_fn_with_state(
            layer,
            oteryn_web::session::middleware,
        ))
        .with_state(pool)
}

#[derive(Template)]
#[template(path = "admin/staff_invitation.html", config = "src/web_askama.toml")]
struct Confirmation {
    lang: &'static str,
    message: &'static str,
    token: String,
    csrf: String,
    role: String,
    staff_login: String,
}
impl Confirmation {
    fn label(&self, key: &str) -> String {
        text(self.lang, key)
    }
    fn response(&self, status: StatusCode) -> Response {
        (
            status,
            [
                (header::CACHE_CONTROL, "no-store, private"),
                (header::REFERRER_POLICY, "strict-origin"),
            ],
            oteryn_web::render(self),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
struct Link {
    #[serde(default)]
    token: String,
}

fn confirmation(lang: &str) -> Result<Confirmation, StatusCode> {
    let lang = match lang {
        "pl" => "pl",
        "en" => "en",
        _ => return Err(StatusCode::NOT_FOUND),
    };
    Ok(Confirmation {
        lang,
        message: "invite-unavailable",
        token: String::new(),
        csrf: String::new(),
        role: String::new(),
        staff_login: String::new(),
    })
}

async fn confirm(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    Query(link): Query<Link>,
) -> Result<Response, StatusCode> {
    let mut page = confirmation(&lang)?;
    if !staff::valid_token(&link.token) {
        return Ok(page.response(StatusCode::NOT_FOUND));
    }
    let Some(account) = session.account_id().await else {
        page.message = "invite-sign-in";
        return Ok(page.response(StatusCode::OK));
    };
    let (role, enrolled) = match staff::preview(&pool, account, &link.token).await {
        Ok(preview) => preview,
        Err(error) => return Ok(accept_error(page, error)),
    };
    if !enrolled {
        return Ok(accept_error(page, InviteError::Mfa));
    }
    page.token = link.token;
    page.role = role;
    page.message = "invite-confirm";
    page.csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(page.response(StatusCode::OK))
}

async fn accept(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    Form(link): Form<Link>,
) -> Result<Response, StatusCode> {
    let mut page = confirmation(&lang)?;
    let (Some(account), Some(session_hash)) =
        (session.account_id().await, session.identity_hash().await)
    else {
        page.message = "invite-sign-in";
        return Ok(page.response(StatusCode::UNAUTHORIZED));
    };
    match staff::accept(&pool, account, &session_hash, &link.token).await {
        Ok(()) => {
            page.message = "invite-accepted";
            let origin = super::config_from_env()
                .ok()
                .filter(|config| config.admin_panel)
                .map(|config| config.csrf_origin())
                .unwrap_or_default();
            page.staff_login = format!("{origin}/{lang}/login");
            Ok(page.response(StatusCode::OK))
        }
        Err(error) => Ok(accept_error(page, error)),
    }
}

fn accept_error(mut page: Confirmation, error: InviteError) -> Response {
    let status = match error {
        InviteError::Mfa => {
            page.message = "invite-mfa";
            StatusCode::FORBIDDEN
        }
        InviteError::Unavailable => StatusCode::NOT_FOUND,
        _ => {
            page.message = "invite-service-unavailable";
            StatusCode::SERVICE_UNAVAILABLE
        }
    };
    page.response(status)
}
