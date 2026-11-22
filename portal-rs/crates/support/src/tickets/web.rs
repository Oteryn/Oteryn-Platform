use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{OriginalUri, Path, Query, Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::Language;
use oteryn_web::ratelimit::ClientSource;
use oteryn_web::ratelimit::{Denial, Keys, NamedPolicy, RateLimiter};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::{Deserialize, Serialize};

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
    fn active_verified_in<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> oteryn_notify::NotifyFuture<'a, Option<oteryn_notify::Account>> {
        Box::pin(async move {
            let account = oteryn_identity::accounts::Accounts::find_by_id_in(tx, id).await?;
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
            "/{lang}/support/tickets/{id}/read",
            axum::routing::post(mark_read),
        )
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
        Error::Conflict | Error::RequestKeyConflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (code, text::text(language, "error")).into_response()
}
async fn limited(
    t: &Tickets,
    policy: NamedPolicy,
    account: Uuid,
    peer: Option<ClientSource>,
) -> std::result::Result<(), Denial> {
    let peer = peer.ok_or(Denial::Unavailable)?;
    let mut keys = Keys::source(peer.0);
    keys.account = Some(account.to_string());
    RateLimiter::new(t.pool.clone())
        .check(policy, &[keys])
        .await
}

fn limiter_response(error: Denial, language: &str) -> Response {
    if matches!(error, Denial::Limited { .. }) {
        let locale = if language == "pl" {
            Language::Pl
        } else {
            Language::En
        };
        return error.response(locale, false);
    }
    error.status().into_response()
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
    reason_text: String,
    unread_text: String,
    previous_text: String,
    next_text: String,
    rows: Vec<Row>,
    categories: Vec<OptionText>,
    statuses: Vec<OptionText>,
    messages: Vec<DisplayMessage>,
    message_after: String,
    read_reply: Option<Uuid>,
    read_after: Option<Uuid>,
    mark_read_text: String,
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
    has_next: bool,
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
            reason_text: tr("reason"),
            unread_text: tr("new-reply"),
            previous_text: tr("previous"),
            next_text: tr("next"),
            rows: vec![],
            categories: options(CATEGORIES),
            statuses: options(if staff { STATUSES } else { &["open", "closed"] }),
            messages: vec![],
            message_after: String::new(),
            read_reply: None,
            read_after: None,
            mark_read_text: tr("mark-read"),
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
            has_next: false,
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
            page.has_next = if rows.len() == 20 {
                if let Some(next) = p.page.checked_add(1) {
                    match t.list(account, staff.is_some(), next).await {
                        Ok(rows) => !rows.is_empty(),
                        Err(e) => return fail(e, &language),
                    }
                } else {
                    false
                }
            } else {
                false
            };
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
#[derive(Deserialize)]
struct MessagePage {
    after: Option<Uuid>,
}
#[derive(Deserialize)]
struct ReadForm {
    reply: Uuid,
    after: Option<Uuid>,
}
async fn mark_read(
    State(t): State<Tickets>,
    Path((language, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    form: std::result::Result<Form<ReadForm>, axum::extract::rejection::FormRejection>,
) -> Response {
    if let Err(e) = lang(&language) {
        return e.into_response();
    }
    let (owner, _) = match account(&t, &session, None).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let Ok(Form(form)) = form else {
        return fail(Error::Invalid, &language);
    };
    match t
        .load_page(owner, false, id, form.after, Acknowledge::Reply(form.reply))
        .await
    {
        Ok(_) => redirect(&language, false, id),
        Err(e) => fail(e, &language),
    }
}
async fn show(
    State(t): State<Tickets>,
    OriginalUri(uri): OriginalUri,
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
    let cursor = match Query::<MessagePage>::try_from_uri(&uri) {
        Ok(Query(value)) => value,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (ticket, messages, read_reply) = match t
        .load_page(
            account,
            staff.is_some(),
            id,
            cursor.after,
            Acknowledge::Never,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return fail(e, &language),
    };
    let mut page = Page::new(&language, staff.is_some());
    page.message_after = messages
        .last()
        .filter(|_| messages.len() == 50)
        .map(|m| m.id.to_string())
        .unwrap_or_default();
    page.read_reply = read_reply;
    page.read_after = cursor.after;
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
    peer: Option<ClientSource>,
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
        return limiter_response(e, &language);
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
        Ok(ticket) => {
            if sqlx::query("UPDATE web_sessions SET data=data-'support.ticket.draft' WHERE id_hash=$1 AND data #>> '{support.ticket.draft,request_key}'=$2")
                .bind(session.identity_hash().await).bind(form.request_key.to_string()).execute(&t.pool).await.is_err() {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            redirect(&language, false, ticket.id)
        }
        Err(error @ (Error::Conflict | Error::RequestKeyConflict)) => {
            let mut form = form;
            if matches!(error, Error::RequestKeyConflict) {
                // The edited draft is a new submission. Keep its text, but do
                // not replay the key already bound to the original ticket.
                form.request_key = Uuid::now_v7();
            }
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
    reason: Option<String>,
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
    peer: Option<ClientSource>,
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
    peer: Option<ClientSource>,
    method: Method,
    form: std::result::Result<Form<Mutation>, axum::extract::rejection::FormRejection>,
) -> Response {
    let form = match form {
        Ok(Form(form)) => form,
        Err(_) => return fail(Error::Invalid, &language),
    };
    let mut response = mutate(t, (language, id), session, staff, peer, form, false).await;
    if method == Method::PUT && response.status() == StatusCode::FOUND {
        // Browsers can preserve PUT across a 302. The ticket detail accepts GET.
        *response.status_mut() = StatusCode::SEE_OTHER;
    }
    response
}
async fn mutate(
    t: Tickets,
    (language, id): (String, Uuid),
    session: BrowserSession,
    staff: Option<Extension<CheckContext>>,
    peer: Option<ClientSource>,
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
        return limiter_response(e, &language);
    }
    let reason = form.reason.as_deref();
    if staff.is_some() && !reason.is_some_and(|reason| valid_text(reason, 500)) {
        return fail(Error::Invalid, &language);
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
        .change_direct(account, staff, (id, form.lock_version), change, reason)
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

#[cfg(test)]
#[tokio::test]
async fn limiter_response_preserves_retry_after_and_locale() -> std::result::Result<(), axum::Error>
{
    for (lang, title) in [("en", "Too many requests"), ("pl", "Zbyt wiele żądań")] {
        let response = limiter_response(Denial::Limited { retry_after: 23 }, lang);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "23");
        assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024).await?;
        assert!(String::from_utf8_lossy(&body).contains(title));
    }
    Ok(())
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        extract::ConnectInfo,
    };
    use oteryn_admin::web::{PanelConfig, PortalIdentity};
    use sha2::{Digest, Sha256};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tower::ServiceExt;

    type TestError = Box<dyn std::error::Error + Send + Sync>;
    type TestResult = std::result::Result<(), TestError>;

    async fn frozen_clock_pool(
        url: &str,
        admin: &PgPool,
        schema: &str,
        second: u32,
    ) -> std::result::Result<PgPool, TestError> {
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(admin)
            .await?;
        // Use the real limiter SQL and resolve its statement clock in this test
        // schema. Other PostgreSQL sessions and clocks retain their normal behavior.
        let options = url.parse::<PgConnectOptions>()?.options([
            ("search_path", format!("{schema},pg_catalog")),
            ("oteryn.test_clock_second", second.to_string()),
        ]);
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        oteryn_db::migrate(&pool).await?;
        sqlx::query(
            "CREATE FUNCTION statement_timestamp() RETURNS timestamptz LANGUAGE sql STABLE
             AS $$ SELECT TIMESTAMPTZ '2026-11-22 00:00:00+00' +
                 make_interval(secs => current_setting('oteryn.test_clock_second')::integer) $$",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    async fn account_session(
        pool: &PgPool,
        locale: &str,
    ) -> std::result::Result<(Uuid, String, String), sqlx::Error> {
        let mut conn = pool.acquire().await?;
        let account = oteryn_identity::accounts::Accounts::create(
            &mut conn,
            &format!("{}@example.test", Uuid::now_v7()),
            "unused-test-hash",
            locale,
        )
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
        oteryn_identity::accounts::Accounts::mark_verified(&mut conn, account.id).await?;

        let token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
        let csrf = Uuid::now_v7().simple().to_string().repeat(2);
        sqlx::query(
            "INSERT INTO web_sessions(id_hash, account_id, csrf_token, user_agent_summary,
                idle_expires_at, absolute_expires_at)
             VALUES ($1, $2, $3, 'test', now() + interval '1 hour', now() + interval '1 day')",
        )
        .bind(Sha256::digest(token.as_bytes()).to_vec())
        .bind(account.id)
        .bind(&csrf)
        .execute(&mut *conn)
        .await?;
        drop(conn);
        let cookie = format!("{}={token}", oteryn_web::session::COOKIE_NAME);
        crate::test_staff::browser(pool, account.id, &cookie).await?;
        Ok((account.id, cookie, csrf))
    }

    async fn ticket_snapshot(pool: &PgPool) -> Result<serde_json::Value> {
        Ok(sqlx::query_scalar(
            "SELECT jsonb_build_array(
                (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tickets t),
                (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM ticket_messages m)
            )",
        )
        .fetch_one(pool)
        .await?)
    }

    async fn assert_ticket_route_quota(
        pool: &PgPool,
        app: &Router,
        policy: NamedPolicy,
        budget: i64,
        locale: (&str, &str),
        retry_after: &str,
        request: impl Fn(i64) -> std::result::Result<Request, axum::http::Error>,
    ) -> TestResult {
        let (lang, title) = locale;
        for attempt in 1..=budget {
            let response = app.clone().oneshot(request(attempt)?).await?;
            assert_eq!(
                response.status(),
                StatusCode::FOUND,
                "{policy:?} {lang} request {attempt}"
            );
        }

        let before = ticket_snapshot(pool).await?;
        let response = app.clone().oneshot(request(budget + 1)?).await?;
        assert_eq!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "{policy:?} {lang}"
        );
        let (charged, total): (i64, i64) = sqlx::query_as(
            "SELECT max(count), sum(count)::bigint FROM web_rate_limits WHERE limiter_name=$1",
        )
        .bind(policy.name())
        .fetch_one(pool)
        .await?;
        assert_eq!(
            (charged, total),
            (budget + 1, budget + 1),
            "{policy:?} {lang}"
        );

        let headers = response.headers();
        assert_eq!(
            headers[header::RETRY_AFTER],
            retry_after,
            "{policy:?} {lang}"
        );
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(headers[header::CONTENT_LANGUAGE], lang);
        assert!(
            headers[header::CONTENT_TYPE]
                .to_str()?
                .starts_with("text/html")
        );
        let html = String::from_utf8(to_bytes(response.into_body(), 64 * 1024).await?.to_vec())?;
        assert!(
            html.contains(&format!("<html lang=\"{lang}\"")),
            "{policy:?} {lang}"
        );
        assert!(html.contains(title), "{policy:?} {lang}");
        assert_eq!(
            ticket_snapshot(pool).await?,
            before,
            "{policy:?} {lang} mutated ticket state"
        );
        Ok(())
    }

    #[tokio::test]
    async fn postgres_ticket_write_routes_preserve_localized_throttle_responses() -> TestResult {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL support test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
        let admin = oteryn_db::lazy_pool(&url)?;
        for (locale, clock_second, retry_after) in [
            (("pl", "Zbyt wiele żądań"), 37, "23"),
            (("en", "Too many requests"), 11, "49"),
        ] {
            let schema = format!("support_route_{}", Uuid::now_v7().simple());
            let pool = frozen_clock_pool(&url, &admin, &schema, clock_second).await?;
            let (lang, _) = locale;
            let (owner, owner_cookie, owner_csrf) = account_session(&pool, lang).await?;
            let (staff, staff_cookie, staff_csrf) = account_session(&pool, lang).await?;
            let assigned = sqlx::query(
                "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
            )
            .bind(staff)
            .execute(&pool)
            .await?;
            assert_eq!(assigned.rows_affected(), 1);

            let tickets = Tickets {
                pool: pool.clone(),
                notifications: Arc::new(Notifications {
                    pool: pool.clone(),
                    accounts: Arc::new(NotifyAccounts(pool.clone())),
                    unsubscribe: oteryn_notify::Unsubscribe::new(
                        Uuid::now_v7().simple().to_string().into_bytes(),
                        "https://portal.example.test/unsubscribe".into(),
                    )?,
                }),
                enabled: true,
                open_limit: 10,
            };
            let player_origin = "http://localhost:3000";
            let staff_host = "staff.test";
            let staff_origin = "https://staff.test";
            let identity = PortalIdentity::configured_from(pool.clone(), &|key| {
                (key == "OTERYN_MFA_KEY").then(|| Uuid::now_v7().simple().to_string().repeat(2))
            });
            let app = router(tickets.clone(), player_origin).merge(staff_router(
                tickets.clone(),
                PanelConfig {
                    admin_panel: true,
                    host: staff_host.to_owned(),
                },
                identity.clone(),
                identity,
            ));
            let peer = ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 3210)));
            let form = |path: &str, origin: &str, host: &str, cookie: &str| {
                Request::post(path)
                    .header(header::HOST, host)
                    .header(header::ORIGIN, origin)
                    .header(header::COOKIE, cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .extension(peer)
            };

            let create = format!("/{lang}/support/tickets");
            assert_ticket_route_quota(
                &pool, &app, NamedPolicy::SupportTicketCreate, 6, locale, retry_after,
                |attempt| form(&create, player_origin, "localhost:3000", &owner_cookie)
                    .body(Body::from(format!(
                        "_csrf={owner_csrf}&request_key={}&category=other&subject=Ticket+{attempt}&body=Help",
                        Uuid::now_v7()
                    ))),
            ).await?;

            let ticket = tickets
                .create(owner, Uuid::now_v7(), "other", "Staff test", "Help")
                .await?;
            let reply = format!("/{lang}/admin/support/tickets/{}/reply", ticket.id);
            assert_ticket_route_quota(
                &pool,
                &app,
                NamedPolicy::SupportTicketReply,
                12,
                locale,
                retry_after,
                |version| {
                    form(&reply, staff_origin, staff_host, &staff_cookie).body(Body::from(format!(
                        "_csrf={staff_csrf}&lock_version={version}&body=Reply+{version}"
                    )))
                },
            )
            .await?;

            let (ticket, _) = tickets.show(owner, false, ticket.id).await?;
            let status = format!("/{lang}/admin/support/tickets/{}/status", ticket.id);
            assert_ticket_route_quota(
                &pool,
                &app,
                NamedPolicy::SupportTicketStatus,
                12,
                locale,
                retry_after,
                |attempt| {
                    let version = ticket.lock_version + attempt - 1;
                    let next = if attempt % 2 == 1 { "closed" } else { "open" };
                    form(&status, staff_origin, staff_host, &staff_cookie).body(Body::from(
                        format!("_csrf={staff_csrf}&lock_version={version}&status={next}"),
                    ))
                },
            )
            .await?;
            pool.close().await;
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
                .execute(&admin)
                .await?;
        }
        Ok(())
    }
}
