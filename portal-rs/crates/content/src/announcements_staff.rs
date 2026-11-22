//! Staff announcement editing through the shared panel guard and announcement service.
use super::*;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_admin::web::{PanelConfig, PortalIdentity, StaffSession};
use oteryn_ui::Chrome;
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
        .with_state(pool)
        .layer(Extension(accounts))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
}
#[derive(askama::Template)]
#[template(path = "content/announcements.html", config = "src/pages/askama.toml")]
struct Page {
    lang: String,
    labels: FluentBundle<FluentResource>,
    next: u64,
    more: bool,
    csrf: String,
    action: String,
    version: i64,
    fields: Vec<(String, String)>,
    rows: Vec<(Uuid, String, String)>,
}
impl Page {
    fn labels(lang: &str) -> FluentBundle<FluentResource> {
        let source = if lang == "en" {
            include_str!("announcements_staff.en.ftl")
        } else {
            include_str!("announcements_staff.pl.ftl")
        };
        let mut bundle = FluentBundle::new(vec![lang.parse().unwrap_or_default()]);
        if let Ok(resource) = FluentResource::try_new(source.to_owned()) {
            let _ = bundle.add_resource(resource);
        }
        bundle
    }

    fn text(&self, key: &str) -> String {
        self.labels
            .get_message(&format!("announcements-{key}"))
            .and_then(|message| message.value())
            .map(|value| {
                self.labels
                    .format_pattern(value, None, &mut Vec::new())
                    .into_owned()
            })
            .unwrap_or_default()
    }
}

async fn index(
    State(pool): State<PgPool>,
    chrome: Option<Extension<Chrome>>,
    axum::extract::Query(q): axum::extract::Query<crate::Pagination>,
) -> Result<Response, StatusCode> {
    let rows = sqlx::query_as("SELECT id,title,publication_state FROM announcements ORDER BY updated_at DESC,id DESC LIMIT 500 OFFSET $1").bind(q.offset(500)).fetch_all(&pool).await.map_err(unavailable)?;
    let lang = chrome.map_or_else(|| "pl".to_owned(), |Extension(chrome)| chrome.lang);
    Ok(oteryn_web::render(&Page {
        labels: Page::labels(&lang),
        lang,
        next: q.next(),
        more: rows.len() == 500,
        csrf: String::new(),
        action: String::new(),
        version: 0,
        fields: vec![],
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
fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
async fn new(
    chrome: Option<Extension<Chrome>>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let fields = FIELDS
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
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
    let lang = chrome.map_or_else(|| "pl".to_owned(), |Extension(chrome)| chrome.lang);
    let csrf = session.csrf_token().await.map_err(unavailable)?;
    Ok(oteryn_web::render(&Page {
        labels: Page::labels(&lang),
        lang,
        next: 0,
        more: false,
        csrf,
        action: "/admin/announcements".into(),
        version: 0,
        fields,
        rows: vec![],
    }))
}
async fn edit(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    chrome: Option<Extension<Chrome>>,
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
                (*name).to_owned(),
                data[*name].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let lang = chrome.map_or_else(|| "pl".to_owned(), |Extension(chrome)| chrome.lang);
    let csrf = session.csrf_token().await.map_err(unavailable)?;
    Ok(oteryn_web::render(&Page {
        labels: Page::labels(&lang),
        lang,
        next: 0,
        more: false,
        csrf,
        action: format!("/admin/announcements/{id}/edit"),
        version: row.get("lock_version"),
        fields,
        rows: vec![],
    }))
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
    chrome: Option<Extension<Chrome>>,
    Form(form): Form<Submission>,
) -> Result<Response, StatusCode> {
    submission_response(persist(pool, accounts, context, None, form).await, chrome)
}
async fn update<A: AccountStateReader + Clone + 'static>(
    State(pool): State<PgPool>,
    Extension(accounts): Extension<A>,
    Extension(context): Extension<CheckContext>,
    chrome: Option<Extension<Chrome>>,
    Path(id): Path<Uuid>,
    Form(form): Form<Submission>,
) -> Result<Response, StatusCode> {
    submission_response(
        persist(pool, accounts, context, Some(id), form).await,
        chrome,
    )
}
fn submission_response(
    result: Result<Response, StatusCode>,
    chrome: Option<Extension<Chrome>>,
) -> Result<Response, StatusCode> {
    match result {
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            let chrome = chrome.map(|Extension(chrome)| chrome).unwrap_or_default();
            Ok(oteryn_ui::error(&chrome, StatusCode::UNPROCESSABLE_ENTITY))
        }
        other => other,
    }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use askama::Template;

    #[test]
    fn editor_uses_the_requested_locale_for_labels_and_options() {
        for (lang, heading, other, save, severity) in [
            (
                "pl",
                "Ogłoszenia",
                "Announcements",
                "Zapisz ogłoszenie",
                "Prace techniczne",
            ),
            (
                "en",
                "Announcements",
                "Ogłoszenia",
                "Save announcement",
                "Maintenance",
            ),
        ] {
            let page = Page {
                lang: lang.to_owned(),
                labels: Page::labels(lang),
                next: 0,
                more: true,
                csrf: String::new(),
                action: "/admin/announcements".into(),
                version: 0,
                fields: FIELDS
                    .iter()
                    .map(|name| ((*name).to_owned(), String::new()))
                    .collect(),
                rows: vec![],
            };
            let html = page.render().unwrap();
            assert!(html.contains(&format!("<html lang=\"{lang}\">")));
            assert!(html.contains(&format!("<title>{heading}</title>")));
            assert!(!html.contains(other));
            assert!(html.contains(save));
            assert!(html.contains(&format!(
                "<option value=\"maintenance\">{severity}</option>"
            )));
            for name in FIELDS {
                assert!(!page.text(name).is_empty(), "{lang}: missing {name}");
            }
        }
    }
}
