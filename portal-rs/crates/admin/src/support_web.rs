//! Guarded SSR support routes composed with an authoritative support domain adapter.
use super::*;
use crate::web::{self, PanelConfig, StaffSession};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{FromRequest, Path, Query, Request, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionMutationDeadline, with_post_deadline};
use std::sync::Arc;

struct Panel<A, B> {
    pool: PgPool,
    auth: Authorizer<A>,
    backend: B,
    tickets_mounted: bool,
}
const MAX_PAGE: u32 = 10000;
pub(crate) const REPORTS_FLAG: &str = "OTERYN_REPORTS_ENABLED";
pub(crate) const REPORTS_PATH: &str = "/admin/moderation/reports";

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
            Command::Appeal { appeal_outcome, .. } => {
                (appeal_outcome.as_str(), MAX_APPEAL_OUTCOME_LENGTH)
            }
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
            Area::Reports => Some(REPORTS_FLAG),
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
            tickets_mounted: lookup("OTERYN_SUPPORT_TICKETS").as_deref() == Some("true"),
        });
        routes = routes.merge(web::configured_guard_any_from(
            area_routes(area, state),
            pool.clone(),
            area.permissions(),
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
            tickets_mounted: true,
        });
        routes = routes.merge(web::guarded_any(
            area_routes(area, state),
            pool.clone(),
            config.clone(),
            identity.clone(),
            accounts.clone(),
            area.permissions(),
        ));
    }
    routes
}
fn base(area: Area) -> &'static str {
    match area {
        Area::Tickets => "/admin/support/tickets",
        Area::Reports => REPORTS_PATH,
        Area::Enforcement => "/admin/moderation/enforcement",
    }
}

/// Apply outside shared chrome to cover enforcement submissions from their first session read.
pub fn with_browser_deadline(app: Router) -> Router {
    with_post_deadline(app, |path| {
        path.strip_suffix("/{id}") == Some(base(Area::Enforcement))
    })
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
        Error::Conflict | Error::StaleTicketVersion | Error::TicketStateConflict => {
            StatusCode::CONFLICT
        }
    };
    let key = match error {
        Error::Unavailable => "error-unavailable",
        Error::Denied => "error-denied",
        Error::NotFound => "error-not-found",
        Error::Invalid => "error-invalid",
        Error::Conflict | Error::StaleTicketVersion | Error::TicketStateConflict => {
            "error-conflict"
        }
    };
    (code, text(lang, key)).into_response()
}
fn invalid_mutation(headers: &HeaderMap) -> Response {
    let language = if language(headers) == "en" {
        oteryn_web::Language::En
    } else {
        oteryn_web::Language::Pl
    };
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        language.text("error-422-description"),
    )
        .into_response()
}

fn language(headers: &HeaderMap) -> &'static str {
    oteryn_web::negotiate(headers).code()
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
        return QueuePage {
            lang,
            title: text(lang, "title"),
            error: Some(text(lang, "error-queue-query")),
            queue: None,
        }
        .response(StatusCode::BAD_REQUEST);
    };
    queue(&s.backend, Area::Tickets, paging, filter, lang).await
}

#[derive(Template)]
#[template(path = "support_queue.html")]
struct QueuePage<'a> {
    lang: &'a str,
    title: String,
    error: Option<String>,
    queue: Option<Queue>,
}

impl QueuePage<'_> {
    fn response(&self, status: StatusCode) -> Response {
        match self.render() {
            Ok(html) => (status, axum::response::Html(html)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

struct Queue {
    heading: String,
    base: &'static str,
    filters: Option<QueueFilters>,
    all: String,
    filter: String,
    previous: Option<String>,
    previous_label: String,
    next: Option<String>,
    next_label: String,
    enforcement: bool,
    create: String,
    rows: Vec<QueueRow>,
}

struct QueueRow {
    id: Uuid,
    title: String,
    status: String,
    appeal: Option<String>,
}

struct QueueFilter {
    label: String,
    choices: Vec<QueueChoice>,
}

struct QueueFilters {
    status: QueueFilter,
    category: QueueFilter,
}

struct QueueChoice {
    value: &'static str,
    label: String,
    selected: bool,
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
    let filters = (area == Area::Tickets).then(|| {
        let options = |field, values: &[&'static str], current: &str| QueueFilter {
            label: text(lang, field),
            choices: values
                .iter()
                .map(|&value| QueueChoice {
                    value,
                    selected: value == current,
                    label: text(lang, value),
                })
                .collect(),
        };
        QueueFilters {
            status: options("status", TICKET_STATUSES, &filter.status),
            category: options("category", TICKET_CATEGORIES, &filter.category),
        }
    });
    let filter_query = if filter.is_empty() {
        String::new()
    } else {
        format!("&status={}&category={}", filter.status, filter.category)
    };
    let base = base(area);
    let page_url = |number| format!("{base}?page={number}{filter_query}");
    let queue = Queue {
        heading: text(lang, area_title(area)),
        base,
        filters,
        all: text(lang, "all"),
        filter: text(lang, "filter"),
        previous: (!filter.is_empty() && page > 1).then(|| page_url(page - 1)),
        previous_label: text(lang, "previous"),
        next: has_next.then(|| page_url(page + 1)),
        next_label: text(lang, "next"),
        enforcement: area == Area::Enforcement,
        create: text(lang, "create"),
        rows: records
            .into_iter()
            .map(|record| QueueRow {
                id: record.id,
                title: record.title,
                status: text(lang, &record.status),
                appeal: record
                    .appeal_status
                    .filter(|value| value != "none")
                    .map(|value| {
                        format!("{} · {}", text(lang, "appeal_status"), text(lang, &value))
                    }),
            })
            .collect(),
    };
    QueuePage {
        lang,
        title: text(lang, "title"),
        error: None,
        queue: Some(queue),
    }
    .response(StatusCode::OK)
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
    let can_manage_report = area == Area::Reports
        && matches!(record.status.as_str(), "submitted" | "reviewing")
        && s.auth
            .check(context, Permission::ManageSupportReports)
            .await
            .is_ok();
    let can_verify = can_manage_report && details.player_report;
    let can_link = can_manage_report
        && details.verified_target.is_some()
        && s.auth
            .check(context, Permission::ManageSupportEnforcement)
            .await
            .is_ok();
    let can_open_tickets = area == Area::Reports
        && s.tickets_mounted
        && s.auth
            .check(context, Permission::ManageSupportTickets)
            .await
            .is_ok();
    render_record(
        area,
        record,
        details,
        session,
        lang,
        RecordForm::Detail {
            can_verify,
            can_link,
            can_open_tickets,
        },
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
        appeal_status: None,
        next_message: None,
        character_enforcement: None,
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
    Detail {
        can_verify: bool,
        can_link: bool,
        can_open_tickets: bool,
    },
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
    let summary = if area == Area::Tickets {
        Some((
            text(lang, "assigned-to"),
            details
                .assigned_to
                .map_or_else(|| text(lang, "unassigned"), |id| id.to_string()),
        ))
    } else {
        None
    };
    let linked = details
        .linked_sanction
        .map(|id| (text(lang, "linked-sanction"), id));
    let verified = details
        .verified_target
        .map(|id| (text(lang, "verified-target"), id));
    let tickets = matches!(
        form,
        RecordForm::Detail {
            can_open_tickets: true,
            ..
        }
    )
    .then(|| text(lang, "tickets"));
    let game_state = record.character_enforcement.as_ref().map(|game| {
        use crate::console::enforcement::Language;
        let locale = if lang == "en" {
            Language::En
        } else {
            Language::Pl
        };
        GameState {
            profile_label: text(lang, "enforcement-profile"),
            profile: game.profile.name(),
            platform_label: text(lang, "platform-decision"),
            platform: text(lang, &record.status),
            request_label: text(lang, "game-request"),
            request: locale.state(&game.request),
            confirmed_label: text(lang, "game-confirmed"),
            confirmed: locale.state(&game.current),
        }
    });
    let messages = record
        .messages
        .into_iter()
        .map(|message| RecordMessage {
            id: message.id,
            kind: text(
                lang,
                if message.internal {
                    "internal"
                } else {
                    "public"
                },
            ),
            body: message.body,
        })
        .collect();
    let action_path = format!("{}/{}", base(area), record.id);
    let next = record
        .next_message
        .map(|after| format!("{action_path}?after={after}"));
    let sanction_fields: &[&str] = &[
        "target",
        "category",
        "status",
        "public_reason",
        "moderator_notes",
        "effective_at",
        "expires_at",
    ];
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
        Area::Enforcement if create => &[
            ("sanction", sanction_fields),
            (
                "apply_sanction",
                &[
                    "target",
                    "category",
                    "duration_hours",
                    "public_reason",
                    "moderator_notes",
                ],
            ),
        ],
        Area::Enforcement => &[
            ("sanction", sanction_fields),
            ("revoke_sanction", &["public_reason"]),
            ("appeal", &["appeal_status", "appeal_outcome"]),
        ],
    };
    let mut forms = Vec::new();
    for &(action, fields) in actions {
        let open_ticket = matches!(
            record.status.as_str(),
            "open" | "waiting_staff" | "waiting_user"
        );
        if (action == "appeal"
            && !matches!(
                record.appeal_status.as_deref(),
                Some("requested" | "reviewing")
            ))
            || (action == "sanction" && record.character_enforcement.is_some())
            || (matches!(action, "assign_ticket" | "reply") && !open_ticket)
            || (action == "link_sanction"
                && !matches!(form, RecordForm::Detail { can_link: true, .. }))
            || (action == "revoke_sanction"
                && (record.status != "active" || record.character_enforcement.is_some()))
            || (action == "moderate" && record.status == "closed")
            || (action == "verify_report_target"
                && !matches!(
                    form,
                    RecordForm::Detail {
                        can_verify: true,
                        ..
                    }
                ))
        {
            continue;
        }
        let fields = fields
            .iter()
            .map(|&field| {
                let choices: &[&str] = match (field, area) {
                    ("status", Area::Tickets) => TICKET_STATUSES,
                    ("status", Area::Reports) => match record.status.as_str() {
                        "submitted" => &["submitted", "reviewing", "actioned", "rejected"],
                        "reviewing" => &["reviewing", "actioned", "rejected"],
                        _ => &["closed"],
                    },
                    ("status", Area::Enforcement) => &["active", "expired", "revoked"],
                    ("appeal_status", _) => &["", "accepted", "rejected"],
                    ("category", _) => &["warning", "restriction", "suspension"],
                    _ => &[],
                };
                form_field(lang, action, field, choices, &record.status)
            })
            .collect();
        forms.push(SupportForm {
            action,
            fields,
            internal: (action == "reply").then(|| text(lang, "internal")),
            submit: text(
                lang,
                if create && action == "sanction" {
                    "create"
                } else {
                    action
                },
            ),
            sanction: action == "sanction",
            create,
        });
    }
    DetailPage {
        lang,
        title: text(lang, "title"),
        heading: text(lang, area_title(area)),
        base: base(area),
        record_title: record.title,
        status: Some(text(lang, &record.status)),
        appeal_status: record
            .appeal_status
            .filter(|value| value != "none")
            .map(|value| (text(lang, "appeal_status"), text(lang, &value))),
        summary,
        linked,
        verified,
        tickets,
        game_state,
        messages,
        next,
        next_label: text(lang, "next"),
        csrf,
        version: record.version,
        action_path,
        forms,
    }
    .render()
    .map_or_else(
        |_| StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        |html| axum::response::Html(html).into_response(),
    )
}

fn area_title(area: Area) -> &'static str {
    match area {
        Area::Tickets => "tickets-title",
        Area::Reports => "reports-title",
        Area::Enforcement => "sanctions-title",
    }
}

#[derive(Template)]
#[template(path = "support_record.html")]
struct DetailPage<'a> {
    lang: &'a str,
    title: String,
    heading: String,
    base: &'static str,
    record_title: String,
    status: Option<String>,
    appeal_status: Option<(String, String)>,
    summary: Option<(String, String)>,
    linked: Option<(String, Uuid)>,
    verified: Option<(String, Uuid)>,
    tickets: Option<String>,
    game_state: Option<GameState>,
    messages: Vec<RecordMessage>,
    next: Option<String>,
    next_label: String,
    csrf: String,
    version: i64,
    action_path: String,
    forms: Vec<SupportForm>,
}

struct GameState {
    profile_label: String,
    profile: &'static str,
    platform_label: String,
    platform: String,
    request_label: String,
    request: &'static str,
    confirmed_label: String,
    confirmed: &'static str,
}

struct RecordMessage {
    id: Option<Uuid>,
    kind: String,
    body: String,
}

struct SupportForm {
    action: &'static str,
    fields: Vec<SupportField>,
    internal: Option<String>,
    submit: String,
    sanction: bool,
    create: bool,
}

struct SupportField {
    name: &'static str,
    label: String,
    control: SupportControl,
}

enum SupportControl {
    Select(Vec<QueueChoice>),
    Duration(i64),
    Identifier(bool),
    Text { max: usize, required: bool },
}

fn form_field(
    lang: &str,
    action: &str,
    field: &'static str,
    choices: &[&'static str],
    selected: &str,
) -> SupportField {
    let label = match (action, field) {
        ("revoke_sanction", "public_reason") => "revocation-reason",
        ("verify_report_target", "target") => "report-target-account",
        (_, "sanction") => "sanction-id",
        _ => field,
    };
    let control = match field {
        _ if !choices.is_empty() => SupportControl::Select(
            choices
                .iter()
                .map(|&value| {
                    let label = match (field, value) {
                        ("appeal_status", "") => "choose_decision",
                        ("appeal_status", "accepted") => "overturn",
                        ("appeal_status", "rejected") => "uphold",
                        _ => value,
                    };
                    QueueChoice {
                        value,
                        label: text(lang, label),
                        selected: value == selected,
                    }
                })
                .collect(),
        ),
        "duration_hours" => SupportControl::Duration(MAX_SANCTION_DURATION_HOURS),
        "assignee" | "sanction" | "target" => SupportControl::Identifier(field != "assignee"),
        _ => SupportControl::Text {
            max: match field {
                "reason" => 500,
                "body" | "moderator_notes" => 8000,
                _ => 4000,
            },
            required: matches!(field, "body" | "reason" | "public_reason")
                || field == "appeal_outcome"
                || (action == "moderate" && field == "moderator_notes")
                || (action == "link_sanction" && field == "public_outcome"),
        },
    };
    SupportField {
        name: field,
        label: text(lang, label),
        control,
    }
}

async fn change<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    Extension(area): Extension<Area>,
    Extension(context): Extension<CheckContext>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    deadline: Option<Extension<SessionMutationDeadline>>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    let lang = language(&headers);
    let Ok(Path(id)) = id else {
        return (StatusCode::BAD_REQUEST, text(lang, "error-invalid")).into_response();
    };
    let deadline = deadline.as_ref().map(|value| &value.0);
    let mut input =
        match web::read_request(deadline, Form::<Input>::from_request(request, &s)).await {
            Ok(Ok(Form(input))) => input,
            Ok(Err(error)) if error.status() == StatusCode::UNPROCESSABLE_ENTITY => {
                return invalid_mutation(&headers);
            }
            Ok(Err(error)) => return (error.status(), text(lang, "error-invalid")).into_response(),
            Err(status) => return status.into_response(),
        };
    if let Command::Appeal { appeal_outcome, .. } = &mut input.command {
        *appeal_outcome = oteryn_web::normalize_textarea(appeal_outcome);
    }
    if input.command.area() != area {
        return status(Error::Invalid, lang);
    }
    let reason = match input.reason() {
        Ok(reason) => reason,
        Err(Error::Invalid) if matches!(input.command, Command::Appeal { .. }) => "",
        Err(error) => return status(error, lang),
    };
    match execute_audited(
        &s.pool,
        &s.auth,
        &s.backend,
        context,
        (id, input.lock_version),
        &input.command,
        ((!reason.is_empty()).then_some(reason), deadline),
    )
    .await
    {
        Ok(()) => Redirect::to(&format!("{}/{id}", base(area))).into_response(),
        Err(error) => {
            if matches!(input.command, Command::Appeal { .. }) {
                let content = match error {
                    Error::Conflict => Some((StatusCode::CONFLICT, "conflict", "reload")),
                    Error::Invalid => Some((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "appeal_invalid",
                        "appeal_return",
                    )),
                    _ => None,
                };
                if let Some((code, message, link)) = content {
                    let path = format!("{}/{id}", base(area));
                    let chrome = oteryn_ui::Registry::default().chrome(
                        lang,
                        &path,
                        oteryn_ui::Account::Anonymous,
                    );
                    return oteryn_ui::error_with_link(
                        &chrome,
                        code,
                        &text(lang, "title"),
                        &text(lang, message),
                        &path,
                        &text(lang, link),
                    );
                }
            }
            use oteryn_web::chrome::TicketConflict;
            let conflict = match error {
                Error::StaleTicketVersion => Some(TicketConflict::StaleVersion {
                    recovery_path: format!("{}/{id}", base(area)),
                }),
                Error::TicketStateConflict => Some(TicketConflict::State),
                _ => None,
            };
            let mut response = status(error, lang);
            if let Some(conflict) = conflict {
                response.extensions_mut().insert(conflict);
            }
            response
        }
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
        let html = QueuePage {
            lang: "en",
            title: "Support".into(),
            error: Some("<script>\"&".into()),
            queue: None,
        }
        .render()
        .unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&#60;script&#62;&#34;&#38;"));
    }

    #[tokio::test]
    async fn invalid_submissions_use_the_panel_language() {
        for (lang, message) in [
            (
                "en",
                "Some information could not be accepted. Check your input and try again.",
            ),
            (
                "pl",
                "Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.",
            ),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert("accept-language", lang.parse().unwrap());
            let response = invalid_mutation(&headers);
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = axum::body::to_bytes(response.into_body(), 256)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), message.as_bytes());
        }
    }
}

#[cfg(test)]
#[path = "support_pagination_tests.rs"]
mod pagination_tests;
