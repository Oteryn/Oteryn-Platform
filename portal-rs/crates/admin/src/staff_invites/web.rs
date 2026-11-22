use super::{Invites, Listing, Pending, delegable_roles_sql, pending_for, valid_token};
use crate::{
    rbac::{Authorizer, CheckContext, Error, Permission},
    web::{PortalIdentity, StaffSession, configured_guard, selected_language, staff_context, text},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, RawForm, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_identity::{reset::DeliveryKey, verify::token_hash};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Denial, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{net::IpAddr, sync::Arc};
use uuid::Uuid;

const MANAGE_PATH: &str = "/admin/staff-invites";

struct App {
    invites: Invites<PortalIdentity>,
    identity: PortalIdentity,
    key: Option<DeliveryKey>,
    limits: RateLimiter,
}

pub fn router(
    pool: PgPool,
    key: Option<DeliveryKey>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_ADMIN_PANEL").as_deref() == Ok("true");
    let identity = PortalIdentity::configured(pool.clone());
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .map(|origin| origin.trim_end_matches('/').to_owned())
        .into_iter()
        .collect();
    let state = Arc::new(App {
        invites: Invites {
            pool: pool.clone(),
            auth: Authorizer::new(pool.clone(), identity.clone()),
        },
        identity,
        key,
        limits: RateLimiter::new(pool.clone()),
    });
    let management = Router::new()
        .route("/admin/staff-invites", get(manage).post(create))
        .route("/admin/staff-invites/{id}/revoke", post(revoke))
        .with_state(state.clone());
    let public = Router::new()
        .route("/{locale}/staff-invite", get(open).post(accept))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .route_layer(middleware::from_fn(
            move |request: axum::extract::Request, next: middleware::Next| async move {
                if enabled {
                    next.run(request).await
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            },
        ))
        .route_layer(middleware::map_response(private_link))
        .with_state(state);
    Ok(configured_guard(management, pool, Permission::ManageRoles)?.merge(public))
}

async fn private_link(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("x-robots-tag", HeaderValue::from_static("noindex"));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
    );
    response
}

#[derive(Template)]
#[template(path = "admin/staff-invites.html", config = "src/web_askama.toml")]
struct Management {
    lang: &'static str,
    csrf: String,
    mail_configured: bool,
    roles: Vec<(Uuid, String)>,
    pending: Vec<Listing>,
    request_key: Uuid,
    approval: Option<Listing>,
    next: Option<Uuid>,
}

impl Management {
    fn tr(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

#[derive(Deserialize)]
struct PageQuery {
    before: Option<Uuid>,
    approval: Option<Uuid>,
    lang: Option<String>,
}

#[derive(Deserialize)]
struct LocaleQuery {
    lang: Option<String>,
}

fn manage_redirect(lang: &str) -> Response {
    Redirect::to(&format!("{MANAGE_PATH}?lang={lang}")).into_response()
}

async fn manage(
    State(app): State<Arc<App>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response {
    let lang = selected_language(&headers, query.lang.as_deref());
    let result = async {
        let csrf = session.csrf_token().await?;
        let roles = sqlx::query_as(delegable_roles_sql())
            .bind(actor.account)
            .fetch_all(&app.invites.pool)
            .await?;
        let mut pending = pending_for(&app.invites.pool, actor.account, query.before).await?;
        let approval = if let Some(id) = query.approval {
            let mut conn = app.invites.pool.acquire().await?;
            let invite = Pending::by_id(&mut conn, id).await?;
            app.invites
                .check_approver(&mut conn, actor.account, &invite)
                .await?;
            let roles = sqlx::query_scalar("SELECT key FROM roles WHERE id=ANY($1) ORDER BY key")
                .bind(invite.role_ids)
                .fetch_all(&mut *conn)
                .await?;
            Some(Listing {
                id,
                email: invite.email,
                roles,
                awaiting_approval: true,
            })
        } else {
            None
        };
        let next = if pending.len() > 50 {
            pending.pop();
            pending.last().map(|row| row.id)
        } else {
            None
        };
        Ok(Management {
            lang,
            csrf,
            mail_configured: app.key.is_some(),
            roles,
            pending,
            next,
            approval,
            request_key: Uuid::now_v7(),
        })
    }
    .await;
    match result {
        Ok(page) => oteryn_web::render(&page),
        Err(error) => failure(lang, error),
    }
}

async fn limit(
    app: &App,
    lang: &str,
    ip: IpAddr,
    actor: Option<Uuid>,
    policy: NamedPolicy,
) -> Result<(), Box<Response>> {
    let mut keys = Keys::source(ip);
    keys.account = actor.map(|id| id.to_string());
    let account_policy = match policy {
        NamedPolicy::StaffInviteManage => Some(NamedPolicy::StaffInviteManageAccount),
        NamedPolicy::StaffInviteAccept => Some(NamedPolicy::StaffInviteAcceptAccount),
        _ => None,
    };
    async {
        app.limits
            .check(policy, std::slice::from_ref(&keys))
            .await?;
        if let Some(account_policy) = account_policy {
            app.limits.check(account_policy, &[keys]).await?;
        }
        Ok(())
    }
    .await
    .map_err(|error: Denial| {
        Box::new(error.response(
            if lang == "en" {
                Language::En
            } else {
                Language::Pl
            },
            false,
        ))
    })
}

async fn create(
    State(app): State<Arc<App>>,
    Extension(actor): Extension<CheckContext>,
    ClientSource(ip): ClientSource,
    headers: HeaderMap,
    Query(query): Query<LocaleQuery>,
    RawForm(form): RawForm,
) -> Response {
    let lang = selected_language(&headers, query.lang.as_deref());
    if let Err(response) = limit(
        &app,
        lang,
        ip,
        Some(actor.account),
        NamedPolicy::StaffInviteManage,
    )
    .await
    {
        return *response;
    }
    let Ok(fields) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&form) else {
        return failure(lang, Error::InvalidInput);
    };
    if let Some((_, id)) = fields.iter().find(|(name, _)| name == "approve") {
        let Ok(id) = id.parse() else {
            return failure(lang, Error::InvalidInput);
        };
        return match app.invites.approve(actor, id).await {
            Ok(()) => manage_redirect(lang),
            Err(error) => failure(lang, error),
        };
    }
    let Some(key) = &app.key else {
        return invite_page(
            lang,
            StatusCode::SERVICE_UNAVAILABLE,
            "invite-mail-unavailable",
        );
    };
    let mut email = "";
    let mut roles = Vec::new();
    let mut request_key = Uuid::nil();
    for (name, value) in &fields {
        match name.as_str() {
            "email" => email = value,
            "request_key" => match value.parse() {
                Ok(id) => request_key = id,
                Err(_) => return failure(lang, Error::InvalidInput),
            },
            "role" => match value.parse() {
                Ok(id) => roles.push(id),
                Err(_) => return failure(lang, Error::InvalidInput),
            },
            _ => {}
        }
    }
    match app
        .invites
        .create(actor, key, email, roles, lang, request_key)
        .await
    {
        Ok(_) => manage_redirect(lang),
        Err(error) => failure(lang, error),
    }
}

async fn revoke(
    State(app): State<Arc<App>>,
    Extension(actor): Extension<CheckContext>,
    ClientSource(ip): ClientSource,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<LocaleQuery>,
) -> Response {
    let lang = selected_language(&headers, query.lang.as_deref());
    if let Err(response) = limit(
        &app,
        lang,
        ip,
        Some(actor.account),
        NamedPolicy::StaffInviteManage,
    )
    .await
    {
        return *response;
    }
    match app.invites.revoke(actor, id).await {
        Ok(()) => manage_redirect(lang),
        Err(error) => failure(lang, error),
    }
}

#[derive(Deserialize)]
struct Link {
    #[serde(default)]
    token: String,
}

#[derive(Template)]
#[template(path = "admin/staff-invite.html", config = "src/web_askama.toml")]
struct Invitation<'a> {
    lang: &'a str,
    message: &'static str,
    token: &'a str,
    csrf: &'a str,
}

impl Invitation<'_> {
    fn tr(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

fn invite_page(lang: &str, status: StatusCode, message: &'static str) -> Response {
    let mut response = oteryn_web::render(&Invitation {
        lang,
        message,
        token: "",
        csrf: "",
    });
    *response.status_mut() = status;
    response
}

fn failure(lang: &str, error: Error) -> Response {
    match error {
        Error::Denied => invite_page(lang, StatusCode::FORBIDDEN, "invite-refused"),
        Error::InvalidInput | Error::RoleNotFound => {
            invite_page(lang, StatusCode::BAD_REQUEST, "invite-invalid")
        }
        Error::Storage(_) => {
            invite_page(lang, StatusCode::SERVICE_UNAVAILABLE, "invite-unavailable")
        }
    }
}

async fn open(
    State(app): State<Arc<App>>,
    Extension(session): Extension<BrowserSession>,
    ClientSource(ip): ClientSource,
    Path(lang): Path<String>,
    Query(link): Query<Link>,
) -> Response {
    if !matches!(lang.as_str(), "en" | "pl") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(response) = limit(&app, &lang, ip, None, NamedPolicy::StaffInviteLink).await {
        return *response;
    }
    if !valid_token(&link.token) {
        return failure(&lang, Error::Denied);
    }
    let email: Option<String> = match sqlx::query_scalar(
        "SELECT email FROM staff_invites WHERE token_hash=$1
         AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at>clock_timestamp()",
    )
    .bind(token_hash(&link.token))
    .fetch_optional(&app.invites.pool)
    .await
    {
        Ok(value) => value,
        Err(error) => return failure(&lang, error.into()),
    };
    let Some(email) = email else {
        return failure(&lang, Error::Denied);
    };
    let Some(identity) = app.identity.resolve(&session).await else {
        return invite_page(&lang, StatusCode::OK, "invite-sign-in");
    };
    let matches: bool = match sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND lower(email)=$2 AND status='active' AND email_verified_at IS NOT NULL)",
    ).bind(identity.account).bind(email).fetch_one(&app.invites.pool).await {
        Ok(value) => value,
        Err(error) => return failure(&lang, error.into()),
    };
    if !matches {
        return invite_page(&lang, StatusCode::FORBIDDEN, "invite-wrong-account");
    }
    if !identity.mfa_satisfied {
        let csrf = match session.csrf_token().await {
            Ok(value) => value,
            Err(error) => return failure(&lang, error.into()),
        };
        return oteryn_web::render(&Invitation {
            lang: &lang,
            message: "invite-mfa",
            token: "",
            csrf: &csrf,
        });
    }
    let csrf = match session.csrf_token().await {
        Ok(value) => value,
        Err(error) => return failure(&lang, error.into()),
    };
    oteryn_web::render(&Invitation {
        lang: &lang,
        message: "invite-confirm",
        token: &link.token,
        csrf: &csrf,
    })
}

async fn accept(
    State(app): State<Arc<App>>,
    Extension(session): Extension<BrowserSession>,
    ClientSource(ip): ClientSource,
    Path(lang): Path<String>,
    Form(link): Form<Link>,
) -> Response {
    if !matches!(lang.as_str(), "en" | "pl") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(response) = limit(&app, &lang, ip, None, NamedPolicy::StaffInviteLink).await {
        return *response;
    }
    let Some(identity) = app.identity.resolve(&session).await else {
        return invite_page(&lang, StatusCode::UNAUTHORIZED, "invite-sign-in");
    };
    if let Err(response) = limit(
        &app,
        &lang,
        ip,
        Some(identity.account),
        NamedPolicy::StaffInviteAccept,
    )
    .await
    {
        return *response;
    }
    if !identity.mfa_satisfied {
        return invite_page(&lang, StatusCode::FORBIDDEN, "invite-mfa");
    }
    let actor = staff_context(&session, identity).await;
    match app.invites.accept(actor, &link.token).await {
        Ok(rotation) => {
            session::apply_rotation(&session, rotation).await;
            invite_page(&lang, StatusCode::OK, "invite-accepted")
        }
        Err(error) => failure(&lang, error),
    }
}
