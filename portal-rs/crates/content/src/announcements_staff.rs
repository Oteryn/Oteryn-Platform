//! Staff announcement editing through the shared panel guard and announcement service.
use super::*;
use crate::preview;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::web::{PanelConfig, PortalIdentity, StaffSession};
use oteryn_web::session::BrowserSession;
use sqlx::Row;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    oteryn_admin::web::configured_guard(
        admin_router(pool.clone()),
        pool,
        Permission::ManagePortalAnnouncements,
    )
}
pub fn admin_router(pool: PgPool) -> Router {
    routes(pool.clone(), PortalIdentity::configured(pool))
}
pub fn staff_router<I: StaffSession, A: AccountStateReader + Clone + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    oteryn_admin::web::guarded(
        routes(pool.clone(), accounts.clone()),
        pool,
        config,
        identity,
        accounts,
        Permission::ManagePortalAnnouncements,
    )
}
fn routes<A: AccountStateReader + Clone + 'static>(pool: PgPool, accounts: A) -> Router {
    Router::new()
        .route("/admin/announcements", get(index).post(create::<A>))
        .route("/admin/announcements/create", get(new))
        .route(
            "/admin/announcements/{id}/edit",
            get(edit).post(update::<A>),
        )
        .route(
            "/admin/announcements/{id}/preview/{locale}/{revision}",
            get(preview),
        )
        .with_state(pool)
        .layer(Extension(accounts))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
}
#[derive(askama::Template)]
#[template(path = "content/announcements.html", config = "src/pages/askama.toml")]
struct Page {
    next: u64,
    more: bool,
    csrf: String,
    action: String,
    links: Vec<(String, String, String)>,
    version: i64,
    fields: Vec<(String, String, String)>,
    rows: Vec<(Uuid, String, String)>,
}
async fn index(
    State(pool): State<PgPool>,
    axum::extract::Query(q): axum::extract::Query<crate::Pagination>,
) -> Result<Response, StatusCode> {
    let rows = sqlx::query_as("SELECT id,title,publication_state FROM announcements ORDER BY updated_at DESC,id DESC LIMIT 500 OFFSET $1").bind(q.offset(500)).fetch_all(&pool).await.map_err(unavailable)?;
    Ok(oteryn_web::render(&Page {
        next: q.next(),
        more: rows.len() == 500,
        csrf: String::new(),
        action: String::new(),
        version: 0,
        fields: vec![],
        links: vec![],
        rows,
    }))
}
const FIELDS: [&str; 12] = [
    "publication_state",
    "severity",
    "starts_at",
    "ends_at",
    "title",
    "body",
    "action_label",
    "action_url",
    "title_pl",
    "body_pl",
    "action_label_pl",
    "translation_published_at",
];
fn field_label(name: &str) -> &str {
    match name {
        "publication_state" => "Publication / Publikacja",
        "severity" => "Severity / Ważność",
        "starts_at" => "Starts at (UTC) / Początek",
        "ends_at" => "Ends at (UTC, optional) / Koniec (opcjonalny)",
        "title" => "Title (English)",
        "body" => "Body (English)",
        "action_label" => "Action label (English)",
        "action_url" => "Action path / Ścieżka akcji",
        "title_pl" => "Tytuł (polski)",
        "body_pl" => "Treść (polski)",
        "action_label_pl" => "Etykieta akcji (polski)",
        "translation_published_at" => "Translation approved at (UTC) / Zatwierdzenie tłumaczenia",
        _ => name,
    }
}
fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
async fn new(Extension(session): Extension<BrowserSession>) -> Result<Response, StatusCode> {
    let fields = FIELDS
        .iter()
        .map(|name| {
            (
                (*name).into(),
                field_label(name).into(),
                match *name {
                    "publication_state" => "draft".into(),
                    "severity" => "info".into(),
                    "starts_at" => OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    _ => String::new(),
                },
            )
        })
        .collect();
    Ok(oteryn_web::render(&Page {
        next: 0,
        more: false,
        csrf: session.csrf_token().await.map_err(unavailable)?,
        action: "/admin/announcements".into(),
        version: 0,
        fields,
        links: vec![],
        rows: vec![],
    }))
}
async fn edit(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let row =
        sqlx::query("SELECT to_jsonb(a) AS data,lock_version FROM announcements a WHERE id=$1")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::NOT_FOUND)?;
    let data: serde_json::Value = row.get("data");
    let fields = FIELDS
        .iter()
        .map(|name| {
            (
                (*name).into(),
                field_label(name).into(),
                data[*name].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let version: i64 = row.get("lock_version");
    let base = format!("/admin/announcements/{id}");
    let links = [("en", "title", "body"), ("pl", "title_pl", "body_pl")]
        .into_iter()
        .filter(|(_, title, body)| {
            preview::complete(
                data[title].as_str().unwrap_or_default(),
                data[body].as_str().unwrap_or_default(),
            )
        })
        .map(|(locale, _, _)| preview::link(&base, locale, version))
        .collect();
    Ok(oteryn_web::render(&Page {
        next: 0,
        more: false,
        csrf: session.csrf_token().await.map_err(unavailable)?,
        action: format!("/admin/announcements/{id}/edit"),
        version,
        links,
        fields,
        rows: vec![],
    }))
}
async fn preview(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Path((id, lang, revision)): Path<(Uuid, String, i64)>,
) -> Result<Response, StatusCode> {
    let lang = preview::locale(&lang)?;
    let row: Announcement = sqlx::query_as(
        "SELECT id,CASE WHEN $2='pl' THEN title_pl ELSE title END AS title,
                CASE WHEN $2='pl' THEN body_pl ELSE body END AS body,severity,
                CASE WHEN $2='pl' THEN action_label_pl ELSE action_label END AS action_label,
                action_url
         FROM announcements WHERE id=$1 AND lock_version=$3
         AND ($2='en' OR (title_pl IS NOT NULL AND body_pl IS NOT NULL))",
    )
    .bind(id)
    .bind(lang)
    .bind(revision)
    .fetch_optional(&pool)
    .await
    .map_err(unavailable)?
    .filter(|row: &Announcement| preview::complete(&row.title, &row.body))
    .ok_or(StatusCode::NOT_FOUND)?;
    Ok(preview::response(
        crate::today::preview_announcement(lang, row),
        lang,
    ))
}

#[derive(Deserialize)]
struct Submission {
    lock_version: i64,
    #[serde(flatten)]
    fields: std::collections::HashMap<String, String>,
}
impl Submission {
    fn input(&self) -> Result<Input, StatusCode> {
        let field = |name: &str| self.fields.get(name).cloned().unwrap_or_default();
        let optional = |name: &str| {
            let value = field(name);
            (!value.is_empty()).then_some(value)
        };
        let stamp = |name: &str| -> Result<Option<OffsetDateTime>, StatusCode> {
            optional(name)
                .map(|value| {
                    OffsetDateTime::parse(&value, &time::format_description::well_known::Rfc3339)
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
                })
                .transpose()
        };
        Ok(Input {
            publication_state: field("publication_state"),
            severity: field("severity"),
            starts_at: stamp("starts_at")?.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            ends_at: stamp("ends_at")?,
            title: field("title"),
            body: field("body"),
            action_label: optional("action_label"),
            action_url: optional("action_url"),
            title_pl: optional("title_pl"),
            body_pl: optional("body_pl"),
            action_label_pl: optional("action_label_pl"),
            translation_published_at: stamp("translation_published_at")?,
        })
    }
}
async fn create<A: AccountStateReader + Clone + 'static>(
    State(pool): State<PgPool>,
    Extension(accounts): Extension<A>,
    Extension(context): Extension<CheckContext>,
    Form(form): Form<Submission>,
) -> Result<Response, StatusCode> {
    persist(pool, accounts, context, None, form).await
}
async fn update<A: AccountStateReader + Clone + 'static>(
    State(pool): State<PgPool>,
    Extension(accounts): Extension<A>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(form): Form<Submission>,
) -> Result<Response, StatusCode> {
    persist(pool, accounts, context, Some(id), form).await
}
async fn persist<A: AccountStateReader>(
    pool: PgPool,
    accounts: A,
    context: CheckContext,
    id: Option<Uuid>,
    form: Submission,
) -> Result<Response, StatusCode> {
    let input = form.input()?;
    let auth = Authorizer::new(pool.clone(), accounts);
    let id = Announcements::new(pool)
        .save(&auth, context, id.map(|id| (id, form.lock_version)), input)
        .await
        .map_err(|e| match e {
            Error::Conflict => StatusCode::CONFLICT,
            Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Error::Authorization(oteryn_admin::rbac::Error::Denied) => StatusCode::FORBIDDEN,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    Ok(Redirect::to(&format!("/admin/announcements/{id}/edit")).into_response())
}
