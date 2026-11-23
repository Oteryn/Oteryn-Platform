//! Guarded SSR support routes composed with an authoritative support domain adapter.
use super::*;
use crate::web::{self, PanelConfig, StaffSession};
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
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
    #[serde(rename = "_csrf", default)]
    _csrf: Option<String>,
    #[serde(flatten)]
    command: Command,
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
    routes
        .route(base(area), get(list::<A, B>))
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
        && record.appeal_status.as_deref().is_none_or(|status| {
            area == Area::Enforcement
                && matches!(status, "requested" | "reviewing" | "accepted" | "rejected")
        })
        && match area {
            Area::Tickets => ticket_status(&record.status),
            Area::Reports => matches!(
                record.status.as_str(),
                "submitted" | "reviewing" | "actioned" | "rejected" | "closed"
            ),
            Area::Enforcement => matches!(record.status.as_str(), "active" | "expired" | "revoked"),
        }
}
fn status(error: Error) -> Response {
    match error {
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Conflict => StatusCode::CONFLICT,
    }
    .into_response()
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
    let page = paging.page.unwrap_or(1);
    if page == 0 || page > MAX_PAGE {
        return status(Error::Invalid);
    }
    let records = match s.backend.list(area, page).await {
        Ok(v) => v,
        Err(e) => return status(e),
    };
    if records.len() > 30 || records.iter().any(|record| !valid_record(area, record)) {
        return status(Error::Unavailable);
    }
    let lang = language(&headers);
    let has_next = if records.len() == 30 && page < MAX_PAGE {
        match s.backend.list(area, page + 1).await {
            Ok(rows) => !rows.is_empty(),
            Err(e) => return status(e),
        }
    } else {
        false
    };
    let mut body = String::from("<ul>");
    if area == Area::Enforcement {
        body.push_str(&format!(
            "<li><a href=\"/admin/moderation/enforcement/create\">{}</a></li>",
            text(lang, "create")
        ));
    }
    for record in records {
        body.push_str(&format!(
            "<li><a href=\"{}/{}\">{}</a>",
            base(area),
            record.id,
            escape(&record.title)
        ));
        if let Some(status) = record.appeal_status {
            body.push_str(&format!(" · {}", text(lang, &format!("appeal-{status}"))));
        }
        body.push_str("</li>");
    }
    body.push_str("</ul>");
    if has_next {
        body.push_str(&format!(
            "<a href=\"{}?page={}\">{}</a>",
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
    Path(id): Path<Uuid>,
    Query(paging): Query<MessagePaging>,
    headers: HeaderMap,
) -> Response {
    if id.is_nil() {
        return status(Error::Invalid);
    }
    let record = match s.backend.load(area, id, paging.after).await {
        Ok(v) => v,
        Err(e) => return status(e),
    };
    if record.id != id || !valid_record(area, &record) {
        return status(Error::Unavailable);
    }
    render_record(area, record, session, headers, false).await
}
async fn create_page(
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let record = Record {
        id: Uuid::now_v7(),
        version: 1,
        title: text(language(&headers), "create"),
        status: "active".into(),
        appeal_status: None,
        messages: vec![],
        next_message: None,
    };
    render_record(Area::Enforcement, record, session, headers, true).await
}
async fn render_record(
    area: Area,
    record: Record,
    session: BrowserSession,
    headers: HeaderMap,
    create: bool,
) -> Response {
    let csrf = match session.csrf_token().await {
        Ok(v) => v,
        Err(_) => return status(Error::Unavailable),
    };
    let lang = language(&headers);
    let mut body = format!(
        "<h2>{}</h2><p>{}</p>",
        escape(&record.title),
        escape(&text(lang, &record.status))
    );
    if area == Area::Tickets {
        body.push_str(&format!(
            "<p><a href=\"/{lang}/admin/support/tickets/{}#screenshots\">{}</a></p>",
            record.id,
            escape(&text(lang, "screenshots")),
        ));
    }
    if let Some(status) = &record.appeal_status {
        body.push_str(&format!(
            "<p>{}</p>",
            text(lang, &format!("appeal-{status}"))
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
                if message.internal {
                    "internal"
                } else {
                    "public"
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
        Area::Tickets => &[("reply", &["body"]), ("ticket_status", &["status"])],
        Area::Reports if report_status_choices(&record.status).is_empty() => &[],
        Area::Reports => &[("moderate", &["status", "public_outcome", "moderator_notes"])],
        Area::Enforcement => &[
            (
                "sanction",
                &[
                    "target",
                    "category",
                    "status",
                    "public_reason",
                    "moderator_notes",
                    "effective_at",
                    "expires_at",
                ],
            ),
            ("appeal", &["appeal_status", "appeal_outcome"]),
        ],
    };
    for (action, fields) in actions {
        if (create && *action != "sanction")
            || (*action == "appeal"
                && !matches!(
                    record.appeal_status.as_deref(),
                    Some("requested" | "reviewing")
                ))
        {
            continue;
        }
        body.push_str(&format!("<form method=\"post\" action=\"{}/{}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><input type=\"hidden\" name=\"lock_version\" value=\"{}\"><input type=\"hidden\" name=\"action\" value=\"{action}\">",base(area),record.id,escape(&csrf),record.version));
        if *action == "sanction" {
            body.push_str(&format!(
                "<input type=\"hidden\" name=\"create\" value=\"{create}\">"
            ));
        }
        for field in *fields {
            let choices: &[&str] = match (*field, area) {
                ("status", Area::Tickets) => &[
                    "open",
                    "waiting_staff",
                    "waiting_user",
                    "resolved",
                    "closed",
                ],
                ("status", Area::Reports) => report_status_choices(&record.status),
                ("status", Area::Enforcement) => &["active", "expired", "revoked"],
                ("appeal_status", _) => &["reviewing", "accepted", "rejected"],
                ("category", _) => &["warning", "restriction", "suspension"],
                _ => &[],
            };
            if choices.is_empty() {
                body.push_str(&format!("<p><label>{}<br><textarea class=\"op-input\" name=\"{field}\" rows=\"3\" cols=\"24\" maxlength=\"8000\"></textarea></label></p>",text(lang,field)));
            } else {
                body.push_str(&format!(
                    "<p><label>{}<select class=\"op-input\" name=\"{field}\">",
                    text(lang, field)
                ));
                for choice in choices {
                    body.push_str(&format!(
                        "<option value=\"{choice}\">{}</option>",
                        text(lang, choice)
                    ));
                }
                body.push_str("</select></label></p>");
            }
        }
        if *action == "reply" {
            body.push_str(&format!("<p><label><input type=\"checkbox\" name=\"internal\" value=\"true\">{}</label></p>",text(lang,"internal")));
        }
        body.push_str(&format!(
            "<button class=\"op-button\" type=\"submit\">{}</button></form>",
            text(lang, action)
        ));
    }
    shell(lang, body)
}
async fn change<A: AccountStateReader, B: Backend>(
    State(s): State<Arc<Panel<A, B>>>,
    Extension(area): Extension<Area>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(input): Form<Input>,
) -> Response {
    if input.command.area() != area {
        return status(Error::Invalid);
    }
    match execute(
        &s.pool,
        &s.auth,
        &s.backend,
        context,
        id,
        input.lock_version,
        &input.command,
    )
    .await
    {
        Ok(()) => Redirect::to(&format!("{}/{id}", base(area))).into_response(),
        Err(e) => status(e),
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
    fn localized_labels_and_escaping() {
        assert_eq!(text("en", "internal"), "Internal staff note");
        assert_eq!(text("pl", "internal"), "Wewnętrzna notatka zespołu");
        assert!(!escape("<script>\"&").contains('<'));
    }
}

#[cfg(test)]
#[path = "support_pagination_tests.rs"]
mod pagination_tests;
