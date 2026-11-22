//! Guarded SSR support routes composed with an authoritative support domain adapter.
use super::*;
use crate::web::{self, PanelConfig, StaffSession};
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::BrowserSession;
use std::sync::Arc;

struct Panel<A, B> {
    pool: PgPool,
    auth: Authorizer<A>,
    backend: B,
}
const MAX_PAGE: u32 = 10000;

#[derive(Deserialize, Default)]
struct Paging {
    page: Option<u32>,
}
#[derive(Deserialize, Default)]
struct MessagePaging {
    after: Option<Uuid>,
}
#[derive(Deserialize)]
struct Input {
    lock_version: i64,
    reason: Option<String>,
    #[serde(rename = "_csrf", default)]
    _csrf: Option<String>,
    #[serde(flatten)]
    command: Command,
}

impl Input {
    fn reason(&self) -> Result<&str, Error> {
        let (reason, max) = match &self.command {
            Command::Moderate {
                moderator_notes, ..
            } => (moderator_notes.as_str(), 8000),
            Command::ApplySanction { public_reason, .. }
            | Command::RevokeSanction { public_reason }
            | Command::Sanction { public_reason, .. } => (public_reason.as_str(), 4000),
            _ => (self.reason.as_deref().unwrap_or_default(), 500),
        };
        if super::text(reason, max, true) {
            Ok(reason)
        } else {
            Err(Error::Invalid)
        }
    }
}

pub fn configured_router<B: Backend>(
    pool: PgPool,
    backend: B,
) -> Result<Router, Box<dyn std::error::Error>> {
    configured_router_from(pool, backend, &|key| std::env::var(key).ok())
}
/// [`configured_router`] reading its configuration through `lookup`.
pub fn configured_router_from<B: Backend>(
    pool: PgPool,
    backend: B,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let mut routes = Router::new();
    for area in [Area::Tickets, Area::Reports, Area::Enforcement] {
        // Fence disabled surfaces before staff authorization and form extraction.
        let flag = match area {
            Area::Tickets => Some("OTERYN_SUPPORT_TICKETS"),
            Area::Reports => Some("OTERYN_REPORTS_ENABLED"),
            Area::Enforcement => None,
        };
        if flag.is_some_and(|flag| lookup(flag).as_deref() != Some("true")) {
            continue;
        }
        let state = Arc::new(Panel {
            pool: pool.clone(),
            auth: Authorizer::new(
                pool.clone(),
                web::PortalIdentity::configured_from(pool.clone(), lookup),
            ),
            backend: backend.clone(),
        });
        routes = routes.merge(web::configured_guard_from(
            area_routes(area, state),
            pool.clone(),
            area.permission(),
            lookup,
        )?);
    }
    Ok(routes)
}
/// All routes use the same mandatory host/session/CSRF/MFA and exact permission guard.
pub fn router<I: StaffSession, A: AccountStateReader + Clone + 'static, B: Backend>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    backend: B,
) -> Router {
    let mut routes = Router::new();
    for area in [Area::Tickets, Area::Reports, Area::Enforcement] {
        let state = Arc::new(Panel {
            pool: pool.clone(),
            auth: Authorizer::new(pool.clone(), accounts.clone()),
            backend: backend.clone(),
        });
        routes = routes.merge(web::guarded(
            area_routes(area, state),
            pool.clone(),
            config.clone(),
            identity.clone(),
            accounts.clone(),
            area.permission(),
        ));
    }
    routes
}
fn base(area: Area) -> &'static str {
    match area {
        Area::Tickets => "/admin/support/tickets",
        Area::Reports => "/admin/moderation/reports",
        Area::Enforcement => "/admin/moderation/enforcement",
    }
}
fn area_routes<A: AccountStateReader + 'static, B: Backend>(
    area: Area,
    state: Arc<Panel<A, B>>,
) -> Router {
    let routes = Router::new();
    let routes = if area == Area::Enforcement {
        routes.route("/admin/moderation/enforcement/create", get(create_page))
    } else {
        routes
    };
    let routes = if area == Area::Tickets {
        routes.route(base(area), get(ticket_queue::<A, B>))
    } else {
        routes.route(base(area), get(list::<A, B>))
    };
    routes
        .route(
            &format!("{}/{{id}}", base(area)),
            get(show::<A, B>).post(change::<A, B>),
        )
        .layer(Extension(area))
        .with_state(state)
}
fn valid_record(area: Area, record: &Record) -> bool {
    !record.id.is_nil()
        && record.version > 0
        && match area {
            Area::Tickets => ticket_status(&record.status),
            Area::Reports => matches!(
                record.status.as_str(),
                "submitted" | "reviewing" | "actioned" | "rejected" | "closed"
            ),
            Area::Enforcement => matches!(record.status.as_str(), "active" | "expired" | "revoked"),
        }
}
fn status(error: Error, lang: &str) -> Response {
    let code = match error {
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Conflict => StatusCode::CONFLICT,
    };
    let key = match error {
        Error::Unavailable => "error-unavailable",
        Error::Denied => "error-denied",
        Error::NotFound => "error-not-found",
        Error::Invalid => "error-invalid",
        Error::Conflict => "error-conflict",
    };
    (code, text(lang, key)).into_response()
}
fn language(headers: &HeaderMap) -> &'static str {
    if headers
        .get("accept-language")
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
        include_str!("support_en.ftl")
    } else {
        include_str!("support_pl.ftl")
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
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn shell(lang: &str, body: String) -> Response {
    axum::response::Html(format!("<!doctype html><html lang=\"{lang}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><title>{}</title><body class=\"op-theme\"><main class=\"op-stack\"><h1>{}</h1>{body}</main></body></html>",text(lang,"title"),text(lang,"title"))).into_response()
}
async fn list<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    Extension(area): Extension<Area>,
    Query(paging): Query<Paging>,
    headers: HeaderMap,
) -> Response {
    queue(
        &s.backend,
        area,
        paging,
        TicketFilter::default(),
        language(&headers),
    )
    .await
}
async fn ticket_queue<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    paging: Result<Query<Paging>, QueryRejection>,
    filter: Result<Query<TicketFilter>, QueryRejection>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    let (Ok(Query(paging)), Ok(Query(filter))) = (paging, filter) else {
        let mut response = shell(
            lang,
            format!(
                "<p role=\"alert\">{}</p>",
                escape(&text(lang, "error-queue-query"))
            ),
        );
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    };
    queue(&s.backend, Area::Tickets, paging, filter, lang).await
}

async fn queue<B: Backend>(
    backend: &B,
    area: Area,
    paging: Paging,
    filter: TicketFilter,
    lang: &str,
) -> Response {
    let page = paging.page.unwrap_or(1);
    if page == 0 || page > MAX_PAGE || !filter.valid() {
        return status(Error::Invalid, lang);
    }
    let records = match backend.list_filtered(area, page, &filter).await {
        Ok(v) => v,
        Err(e) => return status(e, lang),
    };
    if records.len() > 30 || records.iter().any(|record| !valid_record(area, record)) {
        return status(Error::Unavailable, lang);
    }
    let has_next = if records.len() == 30 && page < MAX_PAGE {
        match backend.list_filtered(area, page + 1, &filter).await {
            Ok(rows) => !rows.is_empty(),
            Err(e) => return status(e, lang),
        }
    } else {
        false
    };
    let mut body = format!("<h2>{}</h2>", text(lang, area_title(area)));
    if area == Area::Tickets {
        body.push_str(&format!("<form method=\"get\" action=\"{}\">", base(area)));
        body.push_str(&select(
            lang,
            "status",
            TICKET_STATUSES,
            &filter.status,
            true,
        ));
        body.push_str(&select(
            lang,
            "category",
            TICKET_CATEGORIES,
            &filter.category,
            true,
        ));
        body.push_str(&format!(
            "<button class=\"op-button\" type=\"submit\">{}</button></form>",
            text(lang, "filter")
        ));
    }
    let filter_query = if filter.is_empty() {
        String::new()
    } else {
        format!(
            "&amp;status={}&amp;category={}",
            filter.status, filter.category
        )
    };
    if !filter.is_empty() && page > 1 {
        body.push_str(&format!(
            "<a href=\"{}?page={}{filter_query}\">{}</a>",
            base(area),
            page - 1,
            text(lang, "previous")
        ));
    }
    body.push_str("<ul>");
    if area == Area::Enforcement {
        body.push_str(&format!(
            "<li><a href=\"/admin/moderation/enforcement/create\">{}</a></li>",
            text(lang, "create")
        ));
    }
    for record in records {
        body.push_str(&format!(
            "<li><a href=\"{}/{}\">{}</a> <span>{}</span></li>",
            base(area),
            record.id,
            escape(&record.title),
            text(lang, &record.status)
        ));
    }
    body.push_str("</ul>");
    if has_next {
        body.push_str(&format!(
            "<a href=\"{}?page={}{filter_query}\">{}</a>",
            base(area),
            page + 1,
            text(lang, "next")
        ));
    }
    shell(lang, body)
}
async fn show<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    Extension(area): Extension<Area>,
    Extension(session): Extension<BrowserSession>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Query(paging): Query<MessagePaging>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    if id.is_nil() {
        return status(Error::Invalid, lang);
    }
    let record = match s.backend.load(area, id, paging.after).await {
        Ok(v) => v,
        Err(e) => return status(e, lang),
    };
    if record.id != id || !valid_record(area, &record) {
        return status(Error::Unavailable, lang);
    }
    let details = match s.backend.details(area, &record).await {
        Ok(details) => details,
        Err(error) => return status(error, lang),
    };
    let can_link = area == Area::Reports
        && details.verified_target.is_some()
        && matches!(record.status.as_str(), "submitted" | "reviewing")
        && s.auth
            .check(context, Permission::ManageSupportEnforcement)
            .await
            .is_ok();
    render_record(
        area,
        record,
        details,
        session,
        lang,
        RecordForm::Detail { can_link },
    )
    .await
}
async fn create_page(
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    let record = Record {
        id: Uuid::now_v7(),
        version: 1,
        title: text(lang, "create"),
        status: "active".into(),
        messages: vec![],
        next_message: None,
    };
    render_record(
        Area::Enforcement,
        record,
        Details::default(),
        session,
        lang,
        RecordForm::CreateSanction,
    )
    .await
}

enum RecordForm {
    CreateSanction,
    Detail { can_link: bool },
}

async fn render_record(
    area: Area,
    record: Record,
    details: Details,
    session: BrowserSession,
    lang: &str,
    form: RecordForm,
) -> Response {
    let create = matches!(form, RecordForm::CreateSanction);
    let csrf = match session.csrf_token().await {
        Ok(v) => v,
        Err(_) => return status(Error::Unavailable, lang),
    };
    let mut body = format!(
        "<a href=\"{}\">{}</a><header class=\"op-card\"><h2>{}</h2>",
        base(area),
        text(lang, area_title(area)),
        escape(&record.title),
    );
    if !create {
        body.push_str(&format!("<p>{}</p>", text(lang, &record.status)));
    }
    body.push_str("</header>");
    if area == Area::Tickets {
        let assignment = details
            .assigned_to
            .map_or_else(|| text(lang, "unassigned"), |id| id.to_string());
        body.push_str(&format!(
            "<p>{} <span>{assignment}</span></p>",
            text(lang, "assigned-to")
        ));
    }
    if let Some(id) = details.linked_sanction {
        body.push_str(&format!(
            "<p>{} <span>{id}</span></p>",
            text(lang, "linked-sanction")
        ));
    }
    if let Some(id) = details.verified_target {
        body.push_str(&format!(
            "<p>{} <span>{id}</span></p>",
            text(lang, "verified-target")
        ));
    }
    for message in record.messages {
        body.push_str(&format!(
            "<article{}><h3>{}</h3><p>{}</p></article>",
            message
                .id
                .map_or_else(String::new, |id| format!(" id=\"message-{id}\"")),
            text(
                lang,
                match (area, message.internal) {
                    (_, true) => "internal",
                    (Area::Reports, false) => "report-evidence",
                    _ => "public",
                }
            ),
            escape(&message.body)
        ));
    }
    if let Some(after) = record.next_message {
        body.push_str(&format!(
            "<a href=\"{}/{}?after={}\">{}</a>",
            base(area),
            record.id,
            after,
            text(lang, "next")
        ));
    }
    let actions: &[(&str, &[&str])] = match area {
        Area::Tickets => &[
            ("assign_ticket", &["assignee", "reason"]),
            ("reply", &["body", "reason"]),
            ("ticket_status", &["status", "reason"]),
        ],
        Area::Reports => &[
            ("moderate", &["status", "public_outcome", "moderator_notes"]),
            ("verify_report_target", &["target", "reason"]),
            ("link_sanction", &["sanction", "public_outcome", "reason"]),
        ],
        Area::Enforcement if create => &[(
            "apply_sanction",
            &[
                "target",
                "category",
                "duration_hours",
                "public_reason",
                "moderator_notes",
            ],
        )],
        Area::Enforcement => &[
            ("revoke_sanction", &["public_reason"]),
            ("appeal", &["appeal_status", "appeal_outcome", "reason"]),
        ],
    };
    for (action, fields) in actions {
        let open_ticket = matches!(
            record.status.as_str(),
            "open" | "waiting_staff" | "waiting_user"
        );
        if (matches!(*action, "assign_ticket" | "reply") && !open_ticket)
            || (*action == "link_sanction"
                && !matches!(form, RecordForm::Detail { can_link: true }))
            || (*action == "revoke_sanction" && record.status != "active")
            || (*action == "moderate" && record.status == "closed")
            || (*action == "verify_report_target"
                && (!details.player_report
                    || !matches!(record.status.as_str(), "submitted" | "reviewing")))
        {
            continue;
        }
        body.push_str(&format!("<form method=\"post\" action=\"{}/{}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><input type=\"hidden\" name=\"lock_version\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"{action}\">",base(area),record.id,escape(&csrf),record.version));
        for field in *fields {
            let choices: &[&str] = match (*field, area) {
                ("status", Area::Tickets) => TICKET_STATUSES,
                ("status", Area::Reports) => match record.status.as_str() {
                    "submitted" | "reviewing" => &["reviewing", "rejected", "actioned"],
                    _ => &["closed"],
                },
                ("appeal_status", _) => &["reviewing", "accepted", "rejected"],
                ("category", _) => &["warning", "restriction", "suspension"],
                _ => &[],
            };
            if choices.is_empty() {
                body.push_str(&form_field(lang, action, field));
            } else {
                body.push_str(&select(lang, field, choices, &record.status, false));
            }
        }
        if *action == "reply" {
            body.push_str(&format!("<p><label><input type=\"checkbox\" name=\"internal\" value=\"true\">{}</label></p>",text(lang,"internal")));
        }
        body.push_str(&format!(
            "<button class=\"op-button\" type=\"submit\">{}</button></form>",
            text(lang, action),
        ));
    }
    shell(lang, body)
}
fn area_title(area: Area) -> &'static str {
    match area {
        Area::Tickets => "tickets-title",
        Area::Reports => "reports-title",
        Area::Enforcement => "sanctions-title",
    }
}

fn select(lang: &str, field: &str, choices: &[&str], selected: &str, all: bool) -> String {
    let mut html = format!(
        "<p><label>{}<br><select class=\"op-input\" name=\"{field}\">",
        text(lang, field),
    );
    if all {
        html.push_str(&format!(
            "<option value=\"\">{}</option>",
            text(lang, "all")
        ));
    }
    for choice in choices {
        let attribute = if *choice == selected { " selected" } else { "" };
        let label = match (field, *choice) {
            ("status", "reviewing") => "escalate-report",
            ("status", "rejected") => "dismiss-report",
            _ => choice,
        };
        html.push_str(&format!(
            "<option value=\"{choice}\"{attribute}>{}</option>",
            text(lang, label),
        ));
    }
    html.push_str("</select></label></p>");
    html
}

fn form_field(lang: &str, action: &str, field: &str) -> String {
    let label = match (action, field) {
        ("moderate", "moderator_notes") => "private-reason",
        ("revoke_sanction", "public_reason") => "revocation-reason",
        ("verify_report_target", "target") => "report-target-account",
        (_, "sanction") => "sanction-id",
        _ => field,
    };
    let control = match field {
        "duration_hours" => format!(
            "<input class=\"op-input\" type=\"number\" name=\"{field}\" min=\"1\" max=\"{MAX_SANCTION_DURATION_HOURS}\" required>"
        ),
        "assignee" => {
            "<input class=\"op-input\" name=\"assignee\" maxlength=\"36\" size=\"24\">".to_owned()
        }
        "sanction" | "target" => format!(
            "<input class=\"op-input\" name=\"{field}\" maxlength=\"36\" size=\"24\" required>"
        ),
        _ => {
            let max = match field {
                "reason" => 500,
                "body" | "moderator_notes" => 8000,
                _ => 4000,
            };
            let required = if matches!(field, "body" | "reason" | "public_reason")
                || (action == "moderate" && field == "moderator_notes")
                || (action == "link_sanction" && field == "public_outcome")
            {
                " required"
            } else {
                ""
            };
            format!(
                "<textarea class=\"op-input\" name=\"{field}\" rows=\"3\" cols=\"24\" maxlength=\"{max}\"{required}></textarea>"
            )
        }
    };
    format!("<p><label>{}<br>{control}</label></p>", text(lang, label))
}

async fn change<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    Extension(area): Extension<Area>,
    Extension(context): Extension<CheckContext>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    headers: HeaderMap,
    input: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Response {
    let lang = language(&headers);
    let Ok(Path(id)) = id else {
        return (StatusCode::BAD_REQUEST, text(lang, "error-invalid")).into_response();
    };
    let input = match input {
        Ok(Form(input)) => input,
        Err(error) => return (error.status(), text(lang, "error-invalid")).into_response(),
    };
    if input.command.area() != area {
        return status(Error::Invalid, lang);
    }
    let reason = match input.reason() {
        Ok(reason) => reason,
        Err(error) => return status(error, lang),
    };
    match execute_audited(
        &s.pool,
        &s.auth,
        &s.backend,
        context,
        (id, input.lock_version),
        &input.command,
        Some(reason),
    )
    .await
    {
        Ok(()) => Redirect::to(&format!("{}/{id}", base(area))).into_response(),
        Err(e) => status(e, lang),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn browser_form_decodes_and_rejects_unknown_fields() {
        let input: Input = serde_urlencoded::from_str(
            "_csrf=test&lock_version=1&action=reply&body=hello&internal=true",
        )
        .unwrap();
        assert!(matches!(
            input.command,
            Command::Reply { internal: true, .. }
        ));
        let input: Input = serde_urlencoded::from_str(&format!("lock_version=1&action=sanction&status=active&target={}&category=warning&public_reason=Reason&moderator_notes=&effective_at=100&expires_at=200", Uuid::now_v7())).unwrap();
        assert_eq!(input.command.validate(), Ok(()));

        assert!(
            serde_urlencoded::from_str::<Input>(
                "lock_version=1&action=reply&body=hello&unexpected=yes"
            )
            .is_err()
        );
    }
    #[test]
    fn staff_reasons_and_typed_assignments_are_checked() {
        let input: Input = serde_urlencoded::from_str(
            "lock_version=1&action=assign_ticket&assignee=&reason=Routing+request",
        )
        .unwrap();
        assert!(matches!(
            input.command,
            Command::AssignTicket { assignee: None }
        ));
        assert_eq!(input.reason(), Ok("Routing request"));
        let input: Input = serde_urlencoded::from_str(
            "lock_version=1&action=ticket_status&status=closed&reason=+",
        )
        .unwrap();
        assert_eq!(input.reason(), Err(Error::Invalid));
    }

    #[test]
    fn localized_labels_and_escaping() {
        assert_eq!(text("en", "internal"), "Internal staff note");
        assert_eq!(text("pl", "internal"), "Wewnętrzna notatka zespołu");
        assert!(!escape("<script>\"&").contains('<'));
    }
}

#[cfg(test)]
#[path = "support_pagination_tests.rs"]
mod pagination_tests;
