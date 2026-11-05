use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{ConnectInfo, Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::ratelimit::{Keys, NamedPolicy, RateLimiter};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug)]
pub(super) struct NotifyAccounts(pub(super) PgPool);
impl oteryn_notify::AccountState for NotifyAccounts {
    fn active_verified(
        &self,
        id: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async move {
            let account = oteryn_identity::accounts::Accounts {
                pool: self.0.clone(),
            }
            .find_by_id(id)
            .await?;
            Ok(account
                .filter(|a| a.status == "active" && a.email_verified_at.is_some())
                .and_then(|a| {
                    Some(oteryn_notify::Account {
                        email: a.email,
                        locale: match a.locale.as_str() {
                            "pl" => oteryn_notify::Locale::Pl,
                            "en" => oteryn_notify::Locale::En,
                            _ => return None,
                        },
                    })
                }))
        })
    }
}
pub fn configured_router(pool: PgPool) -> std::result::Result<Router, Box<dyn std::error::Error>> {
    if std::env::var("OTERYN_SUPPORT_TICKETS").as_deref() != Ok("true") {
        return Ok(Router::new());
    }
    let key = std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_KEY")?;
    let base = std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_URL")?;
    let unsubscribe = oteryn_notify::Unsubscribe::new(key.into_bytes(), base)
        .map_err(|_| "invalid notification configuration")?;
    let tickets = Tickets {
        pool: pool.clone(),
        notifications: Arc::new(Notifications {
            pool: pool.clone(),
            accounts: Arc::new(NotifyAccounts(pool.clone())),
            unsubscribe,
        }),
        enabled: true,
        open_limit: 10,
    };
    let staff = oteryn_admin::web::configured_guard(
        staff_routes(tickets.clone()),
        pool,
        oteryn_admin::rbac::Permission::ManageSupportTickets,
    )?;
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")?;
    Ok(router(tickets, &origin).merge(staff))
}
/// Player routes always apply session and CSRF middleware.
pub fn router(tickets: Tickets, origin: &str) -> Router {
    if !tickets.enabled {
        return Router::new();
    }
    let mut session = SessionLayer::new(tickets.pool.clone());
    session.csrf.trusted_origins = vec![origin.to_owned()];
    Router::new()
        .route("/{lang}/support/tickets", get(index).post(create))
        .route("/{lang}/support/tickets/create", get(new_ticket))
        .route("/{lang}/support/tickets/{id}", get(show))
        .route(
            "/{lang}/support/tickets/{id}/reply",
            axum::routing::post(reply),
        )
        .route(
            "/{lang}/support/tickets/{id}/status",
            axum::routing::put(status).post(status),
        )
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(private))
        .with_state(tickets)
}
/// The caller must wrap these routes in `oteryn_admin::web::guarded`.
fn staff_routes(tickets: Tickets) -> Router {
    if !tickets.enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/admin/support/tickets", get(index))
        .route("/{lang}/admin/support/tickets/{id}", get(show))
        .route(
            "/{lang}/admin/support/tickets/{id}/reply",
            axum::routing::post(reply),
        )
        .route(
            "/{lang}/admin/support/tickets/{id}/status",
            axum::routing::put(status).post(status),
        )
        .layer(middleware::from_fn(private))
        .with_state(tickets)
}
/// Staff routes are always protected by the shared host, MFA, RBAC and CSRF guard.
pub fn staff_router<
    I: oteryn_admin::web::StaffSession,
    A: oteryn_admin::rbac::AccountStateReader + 'static,
>(
    tickets: Tickets,
    config: oteryn_admin::web::PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let pool = tickets.pool.clone();
    oteryn_admin::web::guarded(
        staff_routes(tickets),
        pool,
        config,
        identity,
        accounts,
        oteryn_admin::rbac::Permission::ManageSupportTickets,
    )
}
async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
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
    ] {
        response
            .headers_mut()
            .insert(name, header::HeaderValue::from_static(value));
    }
    response.headers_mut().insert(
        "x-robots-tag",
        header::HeaderValue::from_static("noindex, nofollow"),
    );
    response
}
async fn account(
    t: &Tickets,
    session: &BrowserSession,
    staff: Option<Extension<CheckContext>>,
) -> std::result::Result<(Uuid, Option<CheckContext>), StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    if staff
        .as_ref()
        .is_some_and(|c| c.account != id || !c.mfa_verified || !c.require_staff_mfa)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let state = oteryn_identity::accounts::Accounts {
        pool: t.pool.clone(),
    }
    .find_by_id(id)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !state.is_some_and(|a| a.status == "active") {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((id, staff.map(|c| c.0)))
}
fn lang(value: &str) -> std::result::Result<&str, StatusCode> {
    if matches!(value, "pl" | "en") {
        Ok(value)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
fn fail(error: Error, language: &str) -> Response {
    let code = match error {
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (code, text::text(language, "error")).into_response()
}
async fn limited(
    t: &Tickets,
    policy: NamedPolicy,
    account: Uuid,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> std::result::Result<(), StatusCode> {
    let peer = peer.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let mut keys = Keys::source(peer.0.0.ip());
    keys.account = Some(account.to_string());
    RateLimiter::new(t.pool.clone())
        .check(policy, &[keys])
        .await
        .map_err(|e| e.status())
}
#[derive(Deserialize, Default)]
struct Paging {
    #[serde(default)]
    page: u32,
}
#[derive(Debug)]
struct OptionText {
    value: String,
    label: String,
}
#[derive(Debug)]
struct Row {
    id: Uuid,
    subject: String,
    status: String,
    unread: bool,
}
#[derive(Debug)]
struct DisplayMessage {
    body: String,
    author: String,
    internal: bool,
}
#[derive(Template)]
#[template(path = "page.html", config = "src/tickets/askama.toml")]
struct Page {
    lang: String,
    title: String,
    base: String,
    csrf: String,
    creating: bool,
    showing: bool,
    listing: bool,
    staff: bool,
    error: bool,
    error_text: String,
    create_text: String,
    category_text: String,
    subject_text: String,
    body_text: String,
    send_text: String,
    status_text: String,
    version_text: String,
    reply_text: String,
    internal_text: String,
    unread_text: String,
    previous_text: String,
    next_text: String,
    rows: Vec<Row>,
    categories: Vec<OptionText>,
    statuses: Vec<OptionText>,
    messages: Vec<DisplayMessage>,
    request_key: Uuid,
    subject: String,
    draft_body: String,
    selected_category: String,
    status: String,
    version: i64,
    id: Uuid,
    can_reply: bool,
    previous: u32,
    next: u32,
}
impl Page {
    fn new(language: &str, staff: bool) -> Self {
        let tr = |key: &str| text::text(language, key);
        let options = |values: &[&str]| {
            values
                .iter()
                .map(|v| OptionText {
                    value: (*v).into(),
                    label: tr(v),
                })
                .collect()
        };
        Self {
            lang: language.into(),
            title: tr("title"),
            base: format!(
                "/{language}/{}support/tickets",
                if staff { "admin/" } else { "" }
            ),
            csrf: String::new(),
            creating: false,
            showing: false,
            listing: false,
            staff,
            error: false,
            error_text: tr("error"),
            create_text: tr("create"),
            category_text: tr("category"),
            subject_text: tr("subject"),
            body_text: tr("body"),
            send_text: tr("send"),
            status_text: tr("status"),
            version_text: tr("version"),
            reply_text: tr("reply"),
            internal_text: tr("internal"),
            unread_text: tr("new-reply"),
            previous_text: tr("previous"),
            next_text: tr("next"),
            rows: vec![],
            categories: options(CATEGORIES),
            statuses: options(if staff { STATUSES } else { &["open", "closed"] }),
            messages: vec![],
            request_key: Uuid::now_v7(),
            subject: String::new(),
            draft_body: String::new(),
            selected_category: String::new(),
            status: String::new(),
            version: 0,
            id: Uuid::nil(),
            can_reply: false,
            previous: 0,
            next: 1,
        }
    }
    fn response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
async fn index(
    State(t): State<Tickets>,
    Path(language): Path<String>,
    Query(p): Query<Paging>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    let (account, staff) = match account(&t, &session, staff).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let mut page = Page::new(&language, staff.is_some());
    page.listing = true;
    page.previous = p.page.saturating_sub(1);
    page.next = p.page.saturating_add(1);
    match t.list(account, staff.is_some(), p.page).await {
        Ok(rows) => {
            page.rows = rows
                .into_iter()
                .map(|r| Row {
                    id: r.id,
                    subject: r.subject,
                    status: text::text(&language, &r.status),
                    unread: r.unread_reply,
                })
                .collect()
        }
        Err(e) => return fail(e, &language),
    }
    page.response()
}
async fn new_ticket(
    State(t): State<Tickets>,
    Path(language): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    if let Err(e) = account(&t, &session, None).await {
        return e.into_response();
    }
    let mut page = Page::new(&language, false);
    page.creating = true;
    match session.get(DRAFT).await {
        Ok(Some(Some(draft))) => {
            page.error = true;
            page.request_key = draft.request_key;
            page.subject = draft.subject;
            page.draft_body = draft.body;
            page.selected_category = draft.category;
            if session.set(DRAFT, &None::<Create>).await.is_err() {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        }
        Ok(_) => {}
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    match session.csrf_token().await {
        Ok(token) => page.csrf = token,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    page.response()
}
async fn show(
    State(t): State<Tickets>,
    Path((language, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    let (account, staff) = match account(&t, &session, staff).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let (ticket, messages) = match t.show(account, staff.is_some(), id).await {
        Ok(v) => v,
        Err(e) => return fail(e, &language),
    };
    let mut page = Page::new(&language, staff.is_some());
    page.showing = true;
    page.subject = ticket.subject;
    page.status = text::text(&language, &ticket.status);
    page.version = ticket.lock_version;
    page.id = id;
    page.can_reply = if staff.is_some() {
        active(&ticket.status)
    } else {
        matches!(ticket.status.as_str(), "open" | "waiting_user")
    };
    page.messages = messages
        .into_iter()
        .map(|m| DisplayMessage {
            body: m.body,
            author: text::text(&language, &m.author_kind),
            internal: m.internal,
        })
        .collect();
    match session.csrf_token().await {
        Ok(token) => page.csrf = token,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    page.response()
}
const DRAFT: oteryn_web::session::SessionKey<Option<Create>> =
    oteryn_web::session::SessionKey::new("support.ticket.draft");
#[derive(Deserialize, Serialize)]
struct Create {
    request_key: Uuid,
    category: String,
    subject: String,
    body: String,
}
async fn create(
    State(t): State<Tickets>,
    Path(language): Path<String>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: std::result::Result<Form<Create>, axum::extract::rejection::FormRejection>,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    let (account, _) = match account(&t, &session, None).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let form = match form {
        Ok(Form(form)) => form,
        Err(_) => return fail(Error::Invalid, &language),
    };
    if let Err(e) = limited(&t, NamedPolicy::SupportTicketCreate, account, peer).await {
        return e.into_response();
    }
    match t
        .create(
            account,
            form.request_key,
            &form.category,
            &form.subject,
            &form.body,
        )
        .await
    {
        Ok(ticket) => redirect(&language, false, ticket.id),
        Err(Error::Conflict) => {
            if session.set(DRAFT, &Some(form)).await.is_err() {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            (
                StatusCode::FOUND,
                [(
                    header::LOCATION,
                    format!("/{language}/support/tickets/create"),
                )],
            )
                .into_response()
        }
        Err(e) => fail(e, &language),
    }
}
#[derive(Deserialize)]
struct Mutation {
    lock_version: i64,
    #[serde(default)]
    body: String,
    #[serde(default)]
    status: String,
    #[serde(default, deserialize_with = "form_bool")]
    internal: bool,
}
fn form_bool<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<bool, D::Error> {
    match String::deserialize(deserializer)?.as_str() {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(serde::de::Error::custom("invalid boolean")),
    }
}
fn redirect(language: &str, staff: bool, id: Uuid) -> Response {
    (
        StatusCode::FOUND,
        [(
            header::LOCATION,
            format!(
                "/{language}/{}support/tickets/{id}",
                if staff { "admin/" } else { "" }
            ),
        )],
    )
        .into_response()
}
async fn reply(
    State(t): State<Tickets>,
    Path((language, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: std::result::Result<Form<Mutation>, axum::extract::rejection::FormRejection>,
) -> Response {
    let form = match form {
        Ok(Form(form)) => form,
        Err(_) => return fail(Error::Invalid, &language),
    };
    mutate(t, (language, id), session, staff, peer, form, true).await
}
async fn status(
    State(t): State<Tickets>,
    Path((language, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    staff: Option<Extension<CheckContext>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: std::result::Result<Form<Mutation>, axum::extract::rejection::FormRejection>,
) -> Response {
    let form = match form {
        Ok(Form(form)) => form,
        Err(_) => return fail(Error::Invalid, &language),
    };
    mutate(t, (language, id), session, staff, peer, form, false).await
}
async fn mutate(
    t: Tickets,
    (language, id): (String, Uuid),
    session: BrowserSession,
    staff: Option<Extension<CheckContext>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    form: Mutation,
    reply: bool,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    let (account, staff) = match account(&t, &session, staff).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = limited(
        &t,
        if reply {
            NamedPolicy::SupportTicketReply
        } else {
            NamedPolicy::SupportTicketStatus
        },
        account,
        peer,
    )
    .await
    {
        return e.into_response();
    }
    let change = if reply {
        Change::Reply {
            body: &form.body,
            internal: form.internal && staff.is_some(),
        }
    } else {
        Change::Status(&form.status)
    };
    match t
        .change(account, staff, id, form.lock_version, change)
        .await
    {
        Ok(_) => redirect(&language, staff.is_some(), id),
        Err(e) => fail(e, &language),
    }
}

#[cfg(test)]
pub(super) fn test_render(language: &str) -> std::result::Result<String, askama::Error> {
    let mut page = Page::new(language, false);
    page.showing = true;
    page.subject = "<script>alert(1)</script>".into();
    page.render()
}
