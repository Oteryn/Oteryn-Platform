//! SSR event routes and staff adapters; request fields never supply staff identity.
mod calendar;
mod ical;

use super::{repository::ZonedEvent, *};
use crate::preview;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::{
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{PanelConfig, PortalIdentity, StaffSession},
};
use oteryn_web::{
    Language,
    ratelimit::{self, ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{self, BrowserSession, SessionLayer},
};
use sqlx::PgPool;
use std::sync::Arc;
use time::{
    Date, PrimitiveDateTime, UtcOffset, format_description, format_description::well_known::Rfc3339,
};

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let routes = staff_routes(pool.clone(), PortalIdentity::configured(pool.clone()));
    let admin =
        oteryn_admin::web::configured_guard(routes, pool.clone(), Permission::ManageEvents)?;
    Ok(public_router(pool).merge(admin))
}

pub fn public_router(pool: PgPool) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN").into_iter().collect();
    let calendar = Router::new()
        .route(
            "/{locale}/events/calendar/month",
            get(calendar::show).post(calendar::set_zone),
        )
        .route_layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .route("/{locale}/events/upcoming.ics", get(ical::upcoming))
        .route("/{locale}/events/{slug}/event.ics", get(ical::single))
        .route_layer(middleware::from_fn(limit_calendar))
        .route_layer(middleware::from_fn_with_state(
            RateLimiter::new(pool.clone()),
            ratelimit::supply,
        ));
    Router::new()
        .route("/events", get(|| async { Redirect::to("/pl/events") }))
        .route(
            "/events/{slug}",
            get(|Path(slug): Path<String>| async move {
                if valid_slug(&slug) {
                    Redirect::to(&format!("/pl/events/{slug}")).into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
        .route("/{locale}/events", get(index))
        .route("/{locale}/events/{slug}", get(show))
        .route("/events/calendar.css", get(calendar::stylesheet))
        .merge(calendar)
        .with_state(Events::new(pool))
        .layer(middleware::from_fn(headers))
}

async fn limit_calendar(
    Extension(limiter): Extension<RateLimiter>,
    source: Option<ClientSource>,
    request: Request,
    next: Next,
) -> Response {
    // Network listeners attach the transport peer. In-process callers have no peer.
    if let Some(ClientSource(ip)) = source
        && let Err(denial) = limiter
            .check(NamedPolicy::PublicGameData, &[Keys::source(ip)])
            .await
    {
        let lang = if request.uri().path().starts_with("/en/") {
            Language::En
        } else {
            Language::Pl
        };
        return denial.response(lang, false);
    }
    next.run(request).await
}
async fn headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "no-store"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
fn locale(v: &str) -> Result<&str, Error> {
    if matches!(v, "en" | "pl") {
        Ok(v)
    } else {
        Err(Error::NotFound)
    }
}
fn text(lang: &str, key: &str) -> String {
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "en" {
        include_str!("../../../../templates/events/en.ftl")
    } else {
        include_str!("../../../../templates/events/pl.ftl")
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
fn response(error: Error) -> Response {
    match error {
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Authorization(oteryn_admin::rbac::Error::Denied) => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
    .into_response()
}
struct TimeLabel {
    datetime: String,
    display: String,
}

fn display_date(lang: &str, date: Date) -> String {
    let (year, month, day) = date.to_calendar_date();
    let month = u8::from(month);
    if lang == "pl" {
        format!("{day:02}.{month:02}.{year:04}")
    } else {
        format!("{year:04}-{month:02}-{day:02}")
    }
}

impl TimeLabel {
    fn new(lang: &str, utc: OffsetDateTime, local: PrimitiveDateTime) -> Self {
        let date = display_date(lang, local.date());
        let offset = (local.assume_utc() - utc).whole_seconds();
        let offset_minutes = offset.unsigned_abs() / 60;
        let sign = if offset < 0 { '-' } else { '+' };
        let display = format!(
            "{date} {:02}:{:02} (UTC{sign}{:02}:{:02})",
            local.hour(),
            local.minute(),
            offset_minutes / 60,
            offset_minutes % 60,
        );
        Self {
            datetime: utc
                .to_offset(UtcOffset::UTC)
                .format(&Rfc3339)
                .unwrap_or_default(),
            display,
        }
    }
}

async fn viewer_zone(events: &Events, headers: &HeaderMap) -> Result<String, Error> {
    match calendar::cookie_zone(headers) {
        Some(name) => Ok(events
            .time_zones(Some(name))
            .await?
            .pop()
            .unwrap_or_else(|| "UTC".to_owned())),
        None => Ok("UTC".to_owned()),
    }
}

struct EventCard {
    event: PublicEvent,
    start: TimeLabel,
    end: TimeLabel,
}

impl EventCard {
    fn new(lang: &str, row: ZonedEvent) -> Self {
        Self {
            start: TimeLabel::new(lang, row.event.starts_at, row.local_start),
            end: TimeLabel::new(lang, row.event.ends_at, row.local_end),
            event: row.event,
        }
    }
}

#[derive(Template)]
#[template(path = "events/public.html", config = "src/events/askama.toml")]
struct PublicPage<'a> {
    lang: &'a str,
    title: String,
    zone: String,
    calendar_label: String,
    zone_label: String,
    export_label: String,
    empty: String,
    events: Vec<EventCard>,
    detail: bool,
    preview_notice: String,
    next: u64,
    next_label: String,
}

async fn index(
    State(events): State<Events>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(q): Query<crate::Pagination>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return response(e);
    }
    let zone = match viewer_zone(&events, &headers).await {
        Ok(zone) => zone,
        Err(e) => return response(e),
    };
    const PAGE_SIZE: usize = 200;
    match events
        .public_page(
            &lang,
            None,
            q.offset(PAGE_SIZE as i64),
            PAGE_SIZE as i64 + 1,
            &zone,
        )
        .await
    {
        Ok(mut rows) => {
            let next_label = if rows.len() > PAGE_SIZE {
                rows.truncate(PAGE_SIZE);
                text(&lang, "next")
            } else {
                String::new()
            };
            oteryn_web::render(&PublicPage {
                lang: &lang,
                title: text(&lang, "events"),
                calendar_label: text(&lang, "calendar"),
                zone_label: text(&lang, "time-zone"),
                export_label: text(&lang, "export-upcoming"),
                zone,
                empty: text(&lang, "empty"),
                next: q.next(),
                next_label,
                events: rows
                    .into_iter()
                    .map(|row| EventCard::new(&lang, row))
                    .collect(),
                detail: false,
                preview_notice: String::new(),
            })
        }
        Err(e) => response(e),
    }
}
async fn show(
    State(events): State<Events>,
    Path((lang, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if locale(&lang).is_err() || !valid_slug(&slug) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let zone = match viewer_zone(&events, &headers).await {
        Ok(zone) => zone,
        Err(e) => return response(e),
    };
    match events.public_page(&lang, Some(&slug), 0, 1, &zone).await {
        Ok(mut rows) => match rows.pop() {
            Some(row) => oteryn_web::render(&PublicPage {
                lang: &lang,
                title: row.event.title.clone(),
                calendar_label: text(&lang, "calendar"),
                zone_label: text(&lang, "time-zone"),
                export_label: text(&lang, "export-event"),
                zone,
                empty: String::new(),
                events: vec![EventCard::new(&lang, row)],
                detail: true,
                preview_notice: String::new(),
                next_label: String::new(),
                next: 0,
            }),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Err(e) => response(e),
    }
}
fn detail(lang: &str, event: PublicEvent, preview_notice: String) -> Response {
    let local_start = time::PrimitiveDateTime::new(event.starts_at.date(), event.starts_at.time());
    let local_end = time::PrimitiveDateTime::new(event.ends_at.date(), event.ends_at.time());
    oteryn_web::render(&PublicPage {
        lang,
        title: event.title.clone(),
        zone: String::new(),
        calendar_label: String::new(),
        zone_label: String::new(),
        export_label: String::new(),
        empty: String::new(),
        events: vec![EventCard::new(lang, ZonedEvent { event, local_start, local_end })],
        detail: true,
        preview_notice,
        next_label: String::new(),
        next: 0,
    })
}
struct Staff<A> {
    auth: Authorizer<A>,
    events: Events,
}
pub fn admin_router<I: StaffSession, A: AccountStateReader + Clone + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let routes = staff_routes(pool.clone(), accounts.clone());
    oteryn_admin::web::guarded(
        routes,
        pool,
        config,
        identity,
        accounts,
        Permission::ManageEvents,
    )
}
fn staff_routes<A: AccountStateReader + 'static>(pool: PgPool, accounts: A) -> Router {
    let state = Arc::new(Staff {
        auth: Authorizer::new(pool.clone(), accounts),
        events: Events::new(pool),
    });
    Router::new()
        .route("/admin/events", get(admin_index::<A>).post(save_new::<A>))
        .route("/admin/events/create", get(create::<A>))
        .route("/admin/events/{id}/edit", get(edit::<A>))
        .route(
            "/admin/events/{id}/preview/{locale}/{revision}",
            get(preview::<A>),
        )
        .route(
            "/admin/events/{id}",
            axum::routing::put(save_existing::<A>).post(save_existing::<A>),
        )
        .route(
            "/admin/events/{id}/status",
            axum::routing::put(status::<A>).post(status::<A>),
        )
        .with_state(state)
        .layer(middleware::from_fn(headers))
}
fn lang(h: &axum::http::HeaderMap) -> &'static str {
    if h.get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    }
}
#[derive(Template)]
#[template(path = "admin/events/index.html", config = "src/events/askama.toml")]
struct AdminList {
    next: u64,
    more: bool,
    lang: &'static str,
    title: String,
    create: String,
    rows: Vec<(Uuid, String, i64)>,
}
async fn admin_index<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    h: axum::http::HeaderMap,
    axum::extract::Query(q): axum::extract::Query<crate::Pagination>,
) -> Response {
    let lang = lang(&h);
    match s.events.list_page(q.offset(200)).await {
        Ok(rows) => oteryn_web::render(&AdminList {
            next: q.next(),
            more: rows.len() == 200,
            lang,
            title: text(lang, "events"),
            create: text(lang, "create"),
            rows: rows
                .into_iter()
                .map(|e| (e.id, text(lang, &e.status), e.lock_version))
                .collect(),
        }),
        Err(e) => response(e),
    }
}
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct EventForm {
    #[serde(default, rename = "_csrf")]
    _csrf: Option<String>,
    #[serde(default)]
    reason: String,
    starts_at: String,
    ends_at: String,
    #[serde(default)]
    featured: Option<String>,
    en_title: String,
    en_slug: String,
    en_summary: String,
    en_body: String,
    #[serde(default)]
    pl_title: String,
    #[serde(default)]
    pl_slug: String,
    #[serde(default)]
    pl_summary: String,
    #[serde(default)]
    pl_body: String,
    lock_version: Option<i64>,
}
fn date(v: &str) -> Result<OffsetDateTime, Error> {
    let format = format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]")
        .map_err(|_| Error::Invalid)?;
    PrimitiveDateTime::parse(v, &format)
        .map(|v| v.assume_utc())
        .map_err(|_| Error::Invalid)
}
fn formatted(v: OffsetDateTime) -> String {
    format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]")
        .ok()
        .and_then(|f| v.format(&f).ok())
        .unwrap_or_default()
}
impl EventForm {
    fn draft(&self) -> Result<Draft, Error> {
        if self.featured.as_deref().is_some_and(|v| v != "true") {
            return Err(Error::Invalid);
        }
        let pl = Translation {
            title: self.pl_title.clone(),
            slug: self.pl_slug.clone(),
            summary: self.pl_summary.clone(),
            body: self.pl_body.clone(),
        };
        let pl = if [&pl.title, &pl.slug, &pl.summary, &pl.body]
            .iter()
            .all(|v| v.trim().is_empty())
        {
            None
        } else {
            Some(pl)
        };
        Ok(Draft {
            reason: self.reason.clone(),
            starts_at: date(&self.starts_at)?,
            ends_at: date(&self.ends_at)?,
            featured: self.featured.is_some(),
            en: Translation {
                title: self.en_title.clone(),
                slug: self.en_slug.clone(),
                summary: self.en_summary.clone(),
                body: self.en_body.clone(),
            },
            pl,
        })
    }
}
#[derive(Template)]
#[template(path = "admin/events/form.html", config = "src/events/askama.toml")]
struct AdminForm {
    lang: &'static str,
    title: String,
    csrf: String,
    action: String,
    id: String,
    form: EventForm,
    can_publish: bool,
    status: String,
    labels: Vec<String>,
    links: Vec<(String, String, String)>,
}
async fn form<A: AccountStateReader>(
    s: &Staff<A>,
    context: CheckContext,
    session: BrowserSession,
    h: axum::http::HeaderMap,
    id: Option<Uuid>,
) -> Response {
    let lang = lang(&h);
    let (form, status) = if let Some(id) = id {
        let (e, en, pl) = match s.events.load(id).await {
            Ok(v) => v,
            Err(e) => return response(e),
        };
        if Status::parse(&e.status).is_none() {
            return response(Error::Conflict);
        }
        let pl = pl.unwrap_or(Translation {
            title: String::new(),
            slug: String::new(),
            summary: String::new(),
            body: String::new(),
        });
        (
            EventForm {
                _csrf: None,
                reason: String::new(),
                starts_at: formatted(e.starts_at),
                ends_at: formatted(e.ends_at),
                featured: e.featured.then(|| "true".to_owned()),
                en_title: en.title,
                en_slug: en.slug,
                en_summary: en.summary,
                en_body: en.body,
                pl_title: pl.title,
                pl_slug: pl.slug,
                pl_summary: pl.summary,
                pl_body: pl.body,
                lock_version: Some(e.lock_version),
            },
            text(lang, &e.status),
        )
    } else {
        (EventForm::default(), text(lang, "draft"))
    };
    let csrf = match session.csrf_token().await {
        Ok(v) => v,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let can_publish = match s.auth.check(context, Permission::PublishEvents).await {
        Ok(()) => true,
        Err(oteryn_admin::rbac::Error::Denied) => false,
        Err(e) => return response(e.into()),
    };
    let links = match (id, form.lock_version) {
        (Some(id), Some(revision)) => {
            let base = format!("/admin/events/{id}");
            [
                ("en", &form.en_title, &form.en_body),
                ("pl", &form.pl_title, &form.pl_body),
            ]
            .into_iter()
            .filter(|(_, title, body)| preview::complete(title, body))
            .map(|(locale, _, _)| preview::link(&base, locale, revision))
            .collect()
        }
        _ => vec![],
    };
    oteryn_web::render(&AdminForm {
        links,
        lang,
        title: text(lang, "events"),
        csrf,
        action: id.map_or_else(
            || "/admin/events".to_owned(),
            |id| format!("/admin/events/{id}"),
        ),
        id: id.map(|v| v.to_string()).unwrap_or_default(),
        form,
        can_publish,
        status,
        labels: [
            "starts",
            "ends",
            "featured",
            "title",
            "slug",
            "summary",
            "body",
            "save",
            "publication",
            "draft",
            "scheduled",
            "published",
            "archived",
            "apply",
            "reason",
        ]
        .iter()
        .map(|k| text(lang, k))
        .collect(),
    })
}
async fn create<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(c): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    h: axum::http::HeaderMap,
) -> Response {
    form(&s, c, session, h, None).await
}
async fn edit<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(c): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    h: axum::http::HeaderMap,
) -> Response {
    form(&s, c, session, h, Some(id)).await
}
async fn preview<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(_): Extension<CheckContext>,
    Path((id, lang, revision)): Path<(Uuid, String, i64)>,
) -> Result<Response, StatusCode> {
    let lang = preview::locale(&lang)?;
    Ok(match s.events.preview(id, lang, revision).await {
        Ok(event) => preview::response(detail(lang, event, preview::notice(lang)), lang),
        Err(error) => response(error),
    })
}

async fn save<A: AccountStateReader>(
    s: &Staff<A>,
    c: CheckContext,
    id: Option<Uuid>,
    f: EventForm,
) -> Response {
    let draft = match f.draft() {
        Ok(v) => v,
        Err(e) => return response(e),
    };
    let existing = match id {
        Some(id) => match f.lock_version {
            Some(v) => Some((id, v)),
            None => return response(Error::Invalid),
        },
        None => None,
    };
    match s.events.save(&s.auth, c, existing, draft).await {
        Ok(id) => Redirect::to(&format!("/admin/events/{id}/edit")).into_response(),
        Err(e) => response(e),
    }
}
async fn save_new<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(c): Extension<CheckContext>,
    Form(f): Form<EventForm>,
) -> Response {
    save(&s, c, None, f).await
}
async fn save_existing<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(c): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(f): Form<EventForm>,
) -> Response {
    save(&s, c, Some(id), f).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusForm {
    #[serde(default, rename = "_csrf")]
    _csrf: Option<String>,
    status: String,
    #[serde(default)]
    reason: String,
    lock_version: i64,
}
async fn status<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(c): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(f): Form<StatusForm>,
) -> Response {
    match s
        .events
        .transition(&s.auth, c, id, f.lock_version, &f.status, &f.reason)
        .await
    {
        Ok(()) => Redirect::to(&format!("/admin/events/{id}/edit")).into_response(),
        Err(e) => response(e),
    }
}
