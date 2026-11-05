//! SSR event routes and staff adapters; request fields never supply staff identity.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::{
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{PanelConfig, PendingAuthentication, StaffSession},
};
use oteryn_web::session::BrowserSession;
use sqlx::PgPool;
use std::sync::Arc;
use time::{PrimitiveDateTime, format_description};

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let routes = staff_routes(pool.clone(), PendingAuthentication);
    let admin =
        oteryn_admin::web::configured_guard(routes, pool.clone(), Permission::ManageEvents)?;
    Ok(public_router(pool).merge(admin))
}

pub fn public_router(pool: PgPool) -> Router {
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
        .with_state(Events::new(pool))
        .layer(middleware::from_fn(headers))
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
#[derive(Template)]
#[template(path = "events/public.html", config = "src/events/askama.toml")]
struct PublicPage<'a> {
    lang: &'a str,
    title: String,
    empty: String,
    events: Vec<PublicEvent>,
    detail: bool,
}
async fn index(State(events): State<Events>, Path(lang): Path<String>) -> Response {
    if let Err(e) = locale(&lang) {
        return response(e);
    }
    match events.public(&lang, None).await {
        Ok(rows) => oteryn_web::render(&PublicPage {
            lang: &lang,
            title: text(&lang, "events"),
            empty: text(&lang, "empty"),
            events: rows,
            detail: false,
        }),
        Err(e) => response(e),
    }
}
async fn show(
    State(events): State<Events>,
    Path((lang, slug)): Path<(String, String)>,
) -> Response {
    if locale(&lang).is_err() || !valid_slug(&slug) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match events.public(&lang, Some(&slug)).await {
        Ok(rows) if rows.len() == 1 => oteryn_web::render(&PublicPage {
            lang: &lang,
            title: rows[0].title.clone(),
            empty: String::new(),
            events: rows,
            detail: true,
        }),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => response(e),
    }
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
    lang: &'static str,
    title: String,
    create: String,
    rows: Vec<(Uuid, String, i64)>,
}
async fn admin_index<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    h: axum::http::HeaderMap,
) -> Response {
    let lang = lang(&h);
    match s.events.list().await {
        Ok(rows) => oteryn_web::render(&AdminList {
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
    oteryn_web::render(&AdminForm {
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
        .transition(&s.auth, c, id, f.lock_version, &f.status)
        .await
    {
        Ok(()) => Redirect::to(&format!("/admin/events/{id}/edit")).into_response(),
        Err(e) => response(e),
    }
}
