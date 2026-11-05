use super::*;
use axum::{Extension, Form, response::Redirect};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, CheckContext, Error, Permission},
    web::{PanelConfig, StaffSession},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(
        router(pool.clone()).merge(oteryn_admin::web::configured_guard(
            admin_routes(pool.clone()),
            pool,
            Permission::ManagePages,
        )?),
    )
}
/// Test and authentication-adapter composition through the shared T10B staff guard.
pub fn staff_router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    oteryn_admin::web::guarded(
        admin_routes(pool.clone()),
        pool,
        config,
        identity,
        accounts,
        Permission::ManagePages,
    )
}
fn admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/pages", get(index))
        .route("/admin/pages/{slug}/{locale}", get(edit).post(save))
        .with_state(pool)
}
#[derive(Template)]
#[template(path = "admin/pages/edit.html", config = "src/pages/askama.toml")]
struct Editor {
    lang: String,
    title: String,
    csrf: String,
    slug: String,
    locale: String,
    form: PageForm,
    labels: Vec<String>,
}
#[derive(Debug, Deserialize, Default)]
pub(super) struct PageForm {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub legal_version: String,
    #[serde(default)]
    pub legal_effective_date: String,
    #[serde(default)]
    pub action: String,
}
fn valid(form: &PageForm, slug: &str, locale: &str) -> bool {
    (form.published_at.is_empty()
        || time::OffsetDateTime::parse(
            &form.published_at,
            &time::format_description::well_known::Rfc3339,
        )
        .is_ok())
        && (form.legal_effective_date.is_empty()
            || time::Date::parse(
                &form.legal_effective_date,
                time::macros::format_description!("[year]-[month]-[day]"),
            )
            .is_ok())
        && valid_slug(slug)
        && matches!(locale, "en" | "pl")
        && !form.title.trim().is_empty()
        && form.title.chars().count() <= 200
        && !form.body.trim().is_empty()
        && form.body.chars().count() <= 100000
        && form.legal_version.len() <= 80
        && matches!(form.action.as_str(), "save" | "delete")
        && (!legal(slug)
            || form.published_at.is_empty()
            || (!form.legal_version.trim().is_empty() && !form.legal_effective_date.is_empty()))
}
async fn index(
    State(pool): State<PgPool>,
    axum::extract::Query(query): axum::extract::Query<CreateQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(slug) = query.slug {
        if !valid_slug(&slug) || !matches!(query.locale.as_deref(), Some("en" | "pl")) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        return Redirect::to(&format!(
            "/admin/pages/{}/{}",
            slug,
            query.locale.unwrap_or_default()
        ))
        .into_response();
    }
    let lang = if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    };
    let rows:Result<Vec<(String,String)>,_>=sqlx::query_as("SELECT p.slug,e.locale FROM pages p JOIN editorial_pages e ON e.page_id=p.id ORDER BY p.slug,e.locale LIMIT 500").fetch_all(&pool).await;
    match rows {
        Ok(rows) => oteryn_web::render(&Listing {
            lang: lang.to_owned(),
            title: text(lang, "pages"),
            slug_label: text(lang, "slug"),
            locale_label: text(lang, "locale"),
            create_label: text(lang, "create"),
            rows,
        }),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[derive(Deserialize)]
struct CreateQuery {
    slug: Option<String>,
    locale: Option<String>,
}
#[derive(Template)]
#[template(path = "admin/pages/index.html", config = "src/pages/askama.toml")]
struct Listing {
    lang: String,
    title: String,
    slug_label: String,
    locale_label: String,
    create_label: String,
    rows: Vec<(String, String)>,
}
async fn edit(
    State(pool): State<PgPool>,
    Path((slug, locale)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if !valid_slug(&slug) || !matches!(locale.as_str(), "pl" | "en") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let row=sqlx::query("SELECT e.* FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale=$2").bind(&slug).bind(&locale).fetch_optional(&pool).await;
    let form = match row {
        Ok(Some(row)) => PageForm {
            title: row.get("title"),
            body: row.get("body"),
            published_at: row
                .get::<Option<sqlx::types::time::OffsetDateTime>, _>("published_at")
                .map(|t| {
                    t.format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            legal_version: row
                .get::<Option<String>, _>("legal_version")
                .unwrap_or_default(),
            legal_effective_date: row
                .get::<Option<sqlx::types::time::Date>, _>("legal_effective_date")
                .map(|v| v.to_string())
                .unwrap_or_default(),
            action: String::new(),
        },
        Ok(None) => PageForm::default(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let Ok(csrf) = session.csrf_token().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    oteryn_web::render(&Editor {
        title: text(&locale, "pages"),
        labels: [
            "title",
            "body",
            "published",
            "version",
            "effective",
            "save",
            "delete",
        ]
        .map(|k| text(&locale, k))
        .to_vec(),
        lang: locale.clone(),
        locale,
        slug,
        csrf,
        form,
    })
}
async fn save(
    State(pool): State<PgPool>,
    Path((slug, locale)): Path<(String, String)>,
    Extension(context): Extension<CheckContext>,
    Form(form): Form<PageForm>,
) -> Response {
    if !valid(&form, &slug, &locale) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match persist(&pool, context, &slug, &locale, &form).await {
        Ok(()) => Redirect::to("/admin/pages").into_response(),
        Err(Error::InvalidInput) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub(super) async fn persist(
    pool: &PgPool,
    context: CheckContext,
    slug: &str,
    locale: &str,
    form: &PageForm,
) -> Result<(), Error> {
    if !valid(form, slug, locale) {
        return Err(Error::InvalidInput);
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,71102))")
        .bind(slug)
        .execute(&mut *tx)
        .await?;
    let id:Uuid=sqlx::query_scalar("INSERT INTO pages(id,slug) VALUES($1,$2) ON CONFLICT(slug) DO UPDATE SET updated_at=now() RETURNING id").bind(Uuid::now_v7()).bind(slug).fetch_one(&mut *tx).await?;
    let before: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(e) FROM editorial_pages e WHERE page_id=$1 AND locale=$2",
    )
    .bind(id)
    .bind(locale)
    .fetch_optional(&mut *tx)
    .await?;
    if form.action == "delete" {
        if legal(slug) {
            return Err(Error::InvalidInput);
        }
        sqlx::query("DELETE FROM editorial_pages WHERE page_id=$1 AND locale=$2")
            .bind(id)
            .bind(locale)
            .execute(&mut *tx)
            .await?;
    } else {
        let content =
            json!({"title":form.title,"body":form.body,"effective":form.legal_effective_date});
        if legal(slug) && !form.published_at.is_empty() {
            let old:Option<serde_json::Value>=sqlx::query_scalar("SELECT content FROM page_legal_versions WHERE page_id=$1 AND locale=$2 AND version=$3").bind(id).bind(locale).bind(&form.legal_version).fetch_optional(&mut *tx).await?;
            if old.is_some_and(|v| v != content) {
                return Err(Error::InvalidInput);
            }
            sqlx::query("INSERT INTO page_legal_versions(page_id,locale,version,content) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(id).bind(locale).bind(&form.legal_version).bind(content).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO editorial_pages(page_id,locale,title,body,published_at,legal_version,legal_effective_date,source_revision) VALUES($1,$2,$3,$4,NULLIF($5,'')::timestamptz,NULLIF($6,''),NULLIF($7,'')::date,COALESCE((SELECT revision FROM editorial_pages WHERE page_id=$1 AND locale='en'),1)) ON CONFLICT(page_id,locale) DO UPDATE SET title=$3,body=$4,published_at=NULLIF($5,'')::timestamptz,legal_version=NULLIF($6,''),legal_effective_date=NULLIF($7,'')::date,source_revision=EXCLUDED.source_revision,revision=editorial_pages.revision+1,updated_at=now()")
            .bind(id).bind(locale).bind(&form.title).bind(&form.body).bind(&form.published_at).bind(if legal(slug){form.legal_version.as_str()}else{""}).bind(if legal(slug){form.legal_effective_date.as_str()}else{""}).execute(&mut *tx).await?;
    }
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: if form.action == "delete" {
                "cms.page.deleted"
            } else {
                "cms.page.saved"
            },
            target: slug,
            before: before.unwrap_or(json!(null)),
            after: json!({"locale":locale,"title":form.title,"body":form.body,"action":form.action,"published_at":form.published_at,"legal_version":form.legal_version,"legal_effective_date":form.legal_effective_date}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
