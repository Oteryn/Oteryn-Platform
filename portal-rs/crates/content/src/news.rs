//! News storage and public and staff routes.
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, Permission},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;
fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn label(lang: &str, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = if lang == "pl" {
        "news = Aktualności\nnew = Nowy wpis\nsave = Zapisz\ndelete = Usuń\nslug = Adres\ntitle = Tytuł\nbody = Treść\npublication = Data publikacji (UTC)\nnext = Następna strona"
    } else {
        "news = News\nnew = New post\nsave = Save\ndelete = Delete\nslug = Slug\ntitle = Title\nbody = Body\npublication = Publication date (UTC)\nnext = Next page"
    };
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
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
#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html class=\"op-theme\" lang=\"{{ lang }}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"><title>{{ title }}</title><main class=\"op-stack\"><h1>{{ title }}</h1>{{ body|safe }}</main></html>",
    ext = "html"
)]
struct NewsPage<'a> {
    lang: &'a str,
    title: &'a str,
    body: String,
}
fn page(lang: &str, title: &str, body: String) -> Html<String> {
    use askama::Template;
    Html(NewsPage { lang, title, body }.render().unwrap_or_default())
}
#[derive(Debug, Default, Deserialize)]
struct Options {
    lang: Option<String>,
    page: Option<i64>,
}
impl Options {
    fn lang(&self) -> &str {
        if self.lang.as_deref() == Some("en") {
            "en"
        } else {
            "pl"
        }
    }
    fn offset(&self, size: i64) -> Result<i64, StatusCode> {
        let n = self.page.unwrap_or(1);
        if !(1..=100000).contains(&n) {
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        } else {
            Ok((n - 1) * size)
        }
    }
}
#[derive(Debug, Default, Deserialize)]
struct Input {
    slug: String,
    title: String,
    body: String,
    published_at: String,
    #[serde(default)]
    title_pl: String,
    #[serde(default)]
    body_pl: String,
    #[serde(default)]
    translation_published_at: String,
}
fn slug_valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
impl Input {
    fn valid(&self) -> bool {
        slug_valid(&self.slug)
            && !self.title.trim().is_empty()
            && self.title.chars().count() <= 200
            && !self.body.trim().is_empty()
            && self.body.chars().count() <= 100000
            && self.title_pl.chars().count() <= 200
            && self.body_pl.chars().count() <= 100000
            && (self.translation_published_at.is_empty()
                || (!self.title_pl.trim().is_empty() && !self.body_pl.trim().is_empty()))
    }
}
pub fn public_router(pool: PgPool) -> Router {
    Router::new()
        .route("/news", get(index))
        .route("/news/{slug}", get(show))
        .route("/{lang}/news", get(local_index))
        .route("/{lang}/news/{slug}", get(local_show))
        .with_state(pool)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn admin_router(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/news", get(admin_index).post(create))
        .route("/admin/news/create", get(new))
        .route("/admin/news/{id}/edit", get(edit))
        .route("/admin/news/{id}", post(update).put(update).delete(delete))
        .route("/admin/news/{id}/delete", post(delete))
        .with_state(pool)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(
        public_router(pool.clone()).merge(oteryn_admin::web::configured_guard(
            admin_router(pool.clone()),
            pool,
            Permission::ManageNews,
        )?),
    )
}
fn language(lang: &str) -> Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
async fn index(
    State(pool): State<PgPool>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    listing(&pool, q.lang(), &q).await
}
async fn local_index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    language(&lang)?;
    listing(&pool, &lang, &q).await
}
async fn listing(pool: &PgPool, lang: &str, q: &Options) -> Result<Html<String>, StatusCode> {
    let rows=sqlx::query("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title FROM news WHERE published_at <= now() AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT 10 OFFSET $2").bind(lang).bind(q.offset(10)?).fetch_all(pool).await.map_err(unavailable)?;
    let mut html = String::new();
    for row in rows {
        html.push_str(&format!(
            "<p><a href=\"/{lang}/news/{}\">{}</a></p>",
            escape(row.get("slug")),
            escape(row.get("title"))
        ));
    }
    html.push_str(&format!(
        "<a href=\"?page={}\">{}</a>",
        q.page.unwrap_or(1) + 1,
        label(lang, "next")
    ));
    Ok(page(lang, &label(lang, "news"), html))
}
async fn show(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    article(&pool, q.lang(), &slug).await
}
async fn local_show(
    State(pool): State<PgPool>,
    Path((lang, slug)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    language(&lang)?;
    article(&pool, &lang, &slug).await
}
async fn article(pool: &PgPool, lang: &str, slug: &str) -> Result<Html<String>, StatusCode> {
    if !slug_valid(slug) {
        return Err(StatusCode::NOT_FOUND);
    }
    let row=sqlx::query("SELECT CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body FROM news WHERE published_at <= now() AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) AND slug=$2").bind(lang).bind(slug).fetch_optional(pool).await.map_err(unavailable)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(page(
        lang,
        row.get("title"),
        format!(
            "<article><p>{}</p></article>",
            escape(row.get("body")).replace('\n', "<br>")
        ),
    ))
}
async fn admin_index(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    let rows = sqlx::query(
        "SELECT id,title FROM news ORDER BY updated_at DESC,id DESC LIMIT 25 OFFSET $1",
    )
    .bind(q.offset(25)?)
    .fetch_all(&pool)
    .await
    .map_err(unavailable)?;
    let lang = q.lang();
    let mut html = format!(
        "<a href=\"/admin/news/create?lang={lang}\">{}</a>",
        label(lang, "new")
    );
    for row in rows {
        let id: Uuid = row.get("id");
        html.push_str(&format!(
            "<p><a href=\"/admin/news/{id}/edit?lang={lang}\">{}</a></p>",
            escape(row.get("title"))
        ));
    }
    html.push_str(&format!(
        "<a href=\"?page={}&amp;lang={lang}\">{}</a>",
        q.page.unwrap_or(1) + 1,
        label(lang, "next")
    ));
    Ok(page(lang, &label(lang, "news"), html))
}
async fn new(
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    Ok(form(
        q.lang(),
        None,
        Input::default(),
        session.csrf_token().await.map_err(unavailable)?,
    ))
}
async fn edit(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    let row=sqlx::query("SELECT slug,title,body,coalesce(published_at::text,'') AS published_at,coalesce(title_pl,'') AS title_pl,coalesce(body_pl,'') AS body_pl,coalesce(translation_published_at::text,'') AS translation_published_at FROM news WHERE id=$1").bind(id).fetch_optional(&pool).await.map_err(unavailable)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(form(
        q.lang(),
        Some(id),
        Input {
            slug: row.get("slug"),
            title: row.get("title"),
            body: row.get("body"),
            published_at: row.get("published_at"),
            title_pl: row.get("title_pl"),
            body_pl: row.get("body_pl"),
            translation_published_at: row.get("translation_published_at"),
        },
        session.csrf_token().await.map_err(unavailable)?,
    ))
}
fn form(lang: &str, id: Option<Uuid>, input: Input, csrf: String) -> Html<String> {
    let action = id.map_or_else(
        || "/admin/news".to_owned(),
        |id| format!("/admin/news/{id}"),
    );
    let mut html = format!(
        "<form class=\"op-stack\" method=\"post\" action=\"{action}?lang={lang}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\">",
        escape(&csrf)
    );
    for (name, key, value, multi) in [
        ("slug", "slug", input.slug, false),
        ("title", "title", input.title, false),
        ("body", "body", input.body, true),
        ("published_at", "publication", input.published_at, false),
        ("title_pl", "title", input.title_pl, false),
        ("body_pl", "body", input.body_pl, true),
        (
            "translation_published_at",
            "publication",
            input.translation_published_at,
            false,
        ),
    ] {
        let locale = if name.ends_with("_pl") || name.starts_with("translation") {
            "pl"
        } else {
            "en"
        };
        html.push_str(&format!(
            "<label for=\"{name}\">{} ({locale})</label>",
            label(lang, key)
        ));
        html.push_str(&if multi {
            format!(
                "<textarea class=\"op-input\" id=\"{name}\" name=\"{name}\" rows=\"10\">{}</textarea>",
                escape(&value)
            )
        } else {
            format!(
                "<input class=\"op-input\" id=\"{name}\" name=\"{name}\" value=\"{}\">",
                escape(&value)
            )
        });
    }
    html.push_str(&format!(
        "<button class=\"op-button\">{}</button></form>",
        label(lang, "save")
    ));
    if let Some(id) = id {
        html.push_str(&format!("<form class=\"op-stack\" method=\"post\" action=\"/admin/news/{id}/delete\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><button class=\"op-button\">{}</button></form>",escape(&csrf),label(lang,"delete")));
    }
    page(lang, &label(lang, "news"), html)
}
async fn create(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    save(&pool, context, None, input).await
}
async fn update(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    save(&pool, context, Some(id), input).await
}
fn storage(e: sqlx::Error) -> StatusCode {
    if e.as_database_error().is_some_and(|e| {
        e.is_unique_violation()
            || e.code()
                .is_some_and(|c| c.starts_with("22") || c.starts_with("23"))
    }) {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        unavailable(e)
    }
}
async fn save(
    pool: &PgPool,
    context: CheckContext,
    id: Option<Uuid>,
    input: Input,
) -> Result<Response, StatusCode> {
    if !input.valid() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let before: serde_json::Value = if let Some(id) = id {
        sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        json!({})
    };
    let action = if id.is_some() {
        "cms.news_updated"
    } else {
        "cms.news_created"
    };
    let id = if let Some(id) = id {
        sqlx::query_scalar::<_,Uuid>("UPDATE news SET slug=$2,title=$3,body=$4,published_at=NULLIF($5,'')::timestamptz,title_pl=NULLIF($6,''),body_pl=NULLIF($7,''),translation_published_at=NULLIF($8,'')::timestamptz,translation_source_updated_at=CASE WHEN (title,body) IS DISTINCT FROM ($3,$4) AND (title_pl,body_pl) IS NOT DISTINCT FROM (NULLIF($6,''),NULLIF($7,'')) THEN NULL ELSE now() END,updated_at=now() WHERE id=$1 RETURNING id").bind(id).bind(&input.slug).bind(&input.title).bind(&input.body).bind(&input.published_at).bind(&input.title_pl).bind(&input.body_pl).bind(&input.translation_published_at).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StatusCode::NOT_FOUND)?
    } else {
        sqlx::query_scalar("INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at) VALUES($1,$2,$3,NULLIF($4,'')::timestamptz,NULLIF($5,''),NULLIF($6,''),NULLIF($7,'')::timestamptz,now()) RETURNING id").bind(&input.slug).bind(&input.title).bind(&input.body).bind(&input.published_at).bind(&input.title_pl).bind(&input.body_pl).bind(&input.translation_published_at).fetch_one(&mut *tx).await.map_err(storage)?
    };
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before,
            after: json!({"slug":input.slug,"title":input.title,"body":input.body,"published_at":input.published_at,"title_pl":input.title_pl,"body_pl":input.body_pl,"translation_published_at":input.translation_published_at}),
        },
    )
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok((
        StatusCode::FOUND,
        [(
            axum::http::header::LOCATION,
            format!("/admin/news/{id}/edit"),
        )],
    )
        .into_response())
}
async fn delete(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let before: Option<serde_json::Value> =
        sqlx::query_scalar("DELETE FROM news WHERE id=$1 RETURNING to_jsonb(news)")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let before = before.ok_or(StatusCode::NOT_FOUND)?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "cms.news_deleted",
            target: &id.to_string(),
            before,
            after: json!({}),
        },
    )
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok((
        StatusCode::FOUND,
        [(axum::http::header::LOCATION, "/admin/news")],
    )
        .into_response())
}
#[cfg(test)]
#[path = "news_tests.rs"]
mod tests;

async fn headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
