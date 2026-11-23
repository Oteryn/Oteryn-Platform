//! News storage and public and staff routes.
use crate::editorial::{self, Window};
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, MutationAuthorization, Permission},
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
        "news = Aktualności\nempty = Brak opublikowanych aktualności.\nnew = Nowy wpis\nsave = Zapisz\ndelete = Usuń\nslug = Adres\ntitle = Tytuł\nbody = Treść\npublication = Data publikacji (UTC)\nnext = Następna strona"
    } else {
        "news = News\nempty = No news has been published yet.\nnew = New post\nsave = Save\ndelete = Delete\nslug = Slug\ntitle = Title\nbody = Body\npublication = Publication date (UTC)\nnext = Next page"
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
    source = "<!doctype html><html class=\"op-theme\" lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"><title>{{ title }}</title></head><body class=\"op-theme\"><main class=\"op-stack\"><h1>{{ title }}</h1>{{ body|safe }}</main></body></html>",
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
const MAX_PAGE: i64 = 100000;

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
    fn staff_lang(&self, chrome: Option<&oteryn_ui::Chrome>, headers: &HeaderMap) -> &'static str {
        match self.lang.as_deref() {
            Some("pl") => "pl",
            Some("en") => "en",
            _ => editorial::staff_language(chrome, headers),
        }
    }
    fn offset(&self, size: i64) -> Result<i64, StatusCode> {
        let n = self.page.unwrap_or(1);
        if !(1..=MAX_PAGE).contains(&n) {
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        } else {
            Ok((n - 1) * size)
        }
    }
}
#[derive(Debug, Default, Deserialize)]
struct Input {
    #[serde(default)]
    expected_revision: String,
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
    #[serde(default)]
    action: String,
    #[serde(default)]
    schedule_at: String,
    #[serde(default)]
    schedule_until: String,
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
/// Newest published entries for the home page, under the same language rule
/// as the public listing. The excerpt is plain text; templates escape it.
pub async fn headlines(
    pool: &PgPool,
    lang: oteryn_web::Language,
    limit: i64,
) -> Result<Vec<oteryn_ui::HomeNews>, sqlx::Error> {
    let lang = lang.code();
    let rows = sqlx::query("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body,to_char(published_at AT TIME ZONE 'UTC','YYYY-MM-DD') AS day FROM news WHERE published_at <= now() AND (expires_at IS NULL OR now() < expires_at) AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT $2").bind(lang).bind(limit.clamp(0, 10)).fetch_all(pool).await?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let slug: String = row.try_get("slug")?;
        if !slug_valid(&slug) {
            continue;
        }
        let day: String = row.try_get("day")?;
        let body: String = row.try_get("body")?;
        entries.push(oteryn_ui::HomeNews {
            url: format!("/{lang}/news/{slug}"),
            title: row.try_get("title")?,
            display_date: display_date(lang, &day),
            date: day,
            excerpt: excerpt(&body, 160),
        });
    }
    Ok(entries)
}
fn display_date(lang: &str, day: &str) -> String {
    match (lang, day.split('-').collect::<Vec<_>>().as_slice()) {
        ("pl", [year, month, date]) => format!("{date}.{month}.{year}"),
        _ => day.to_owned(),
    }
}
/// First `limit` characters of the body's words, ending with an ellipsis when cut.
fn excerpt(body: &str, limit: usize) -> String {
    let text = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= limit {
        return text;
    }
    let cut: String = text.chars().take(limit).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':', '-', '–']))
}
async fn listing(pool: &PgPool, lang: &str, q: &Options) -> Result<Html<String>, StatusCode> {
    let rows=sqlx::query("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title FROM news WHERE published_at <= now() AND (expires_at IS NULL OR now() < expires_at) AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT 11 OFFSET $2").bind(lang).bind(q.offset(10)?).fetch_all(pool).await.map_err(unavailable)?;
    let mut html = String::new();
    let has_next = rows.len() > 10 && q.page.unwrap_or(1) < MAX_PAGE;
    if rows.is_empty() {
        html.push_str(&format!("<p>{}</p>", label(lang, "empty")));
    }
    for row in rows.into_iter().take(10) {
        html.push_str(&format!(
            "<p><a href=\"/{lang}/news/{}\">{}</a></p>",
            escape(row.get("slug")),
            escape(row.get("title"))
        ));
    }
    if has_next {
        html.push_str(&format!(
            "<a href=\"?page={}&amp;lang={lang}\">{}</a>",
            q.page.unwrap_or(1) + 1,
            label(lang, "next")
        ));
    }
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
    let row=sqlx::query("SELECT CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body FROM news WHERE published_at <= now() AND (expires_at IS NULL OR now() < expires_at) AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) AND slug=$2").bind(lang).bind(slug).fetch_optional(pool).await.map_err(unavailable)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(page(
        lang,
        row.get("title"),
        format!(
            "<article><p>{}</p></article>",
            escape(row.get("body")).replace('\n', "<br>")
        ),
    ))
}
#[derive(askama::Template)]
#[template(path = "admin/news/index.html", config = "src/pages/askama.toml")]
struct NewsQueue<'a> {
    lang: &'a str,
    title: String,
    create: String,
    next: Option<i64>,
    next_label: String,
    end_label: String,
    rows: Vec<NewsQueueRow>,
}

struct NewsQueueRow {
    id: Uuid,
    title: String,
    en_status: String,
    pl_status: String,
    start: String,
    pl_start: String,
    end: String,
}

async fn admin_index(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Response, StatusCode> {
    let rows = sqlx::query(
        "SELECT id,title,to_jsonb(published_at)#>>'{}' AS starts_at,
                 to_jsonb(pl_start)#>>'{}' AS pl_starts_at,to_jsonb(expires_at)#>>'{}' AS ends_at,
                 COALESCE(published_at>now(),false) AS pending,
                 COALESCE(pl_start>now(),false) AS pending_pl,
                 COALESCE(expires_at<=now(),false) AS expired,
                 COALESCE(translation_source_updated_at>=updated_at
                    AND length(trim(title_pl))>0 AND length(trim(body_pl))>0,false) AS current_pl
         FROM (
             SELECT news.*, CASE WHEN published_at IS NOT NULL AND translation_published_at IS NOT NULL
                 THEN greatest(published_at,translation_published_at) END AS pl_start
             FROM news
         ) editions
         ORDER BY CASE WHEN published_at>now() THEN published_at END NULLS LAST,
                  updated_at DESC,id DESC LIMIT 26 OFFSET $1",
    )
    .bind(q.offset(25)?)
    .fetch_all(&pool)
    .await
    .map_err(unavailable)?;
    let lang = q.staff_lang(chrome.as_deref(), &headers);
    let next = q.page.unwrap_or(1) + 1;
    let next = (rows.len() > 25 && next <= MAX_PAGE).then_some(next);
    let rows = rows
        .into_iter()
        .take(25)
        .map(|row| {
            let start: Option<String> = row.get("starts_at");
            let pl_start: Option<String> = row.get("pl_starts_at");
            let expired = row.get("expired");
            NewsQueueRow {
                id: row.get("id"),
                title: row.get("title"),
                en_status: crate::pages::text(
                    lang,
                    editorial::status(start.is_some(), row.get("pending"), expired, true),
                ),
                pl_status: crate::pages::text(
                    lang,
                    editorial::status(
                        pl_start.is_some(),
                        row.get("pending_pl"),
                        expired,
                        row.get("current_pl"),
                    ),
                ),
                start: start.unwrap_or_default(),
                pl_start: pl_start.unwrap_or_default(),
                end: row.get::<Option<String>, _>("ends_at").unwrap_or_default(),
            }
        })
        .collect();
    Ok(oteryn_web::render(&NewsQueue {
        lang,
        title: label(lang, "news"),
        create: label(lang, "new"),
        next,
        next_label: label(lang, "next"),
        end_label: crate::pages::text(lang, "publication-end-label"),
        rows,
    }))
}

async fn new(
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Html<String>, StatusCode> {
    Ok(form(
        q.staff_lang(chrome.as_deref(), &headers),
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
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Html<String>, StatusCode> {
    let row = sqlx::query(
        "SELECT updated_at::text AS expected_revision,slug,title,body,
                COALESCE(to_jsonb(published_at)#>>'{}','') AS published_at,
                COALESCE(to_jsonb(expires_at)#>>'{}','') AS expires_at,
                coalesce(title_pl,'') AS title_pl,coalesce(body_pl,'') AS body_pl,
                COALESCE(to_jsonb(translation_published_at)#>>'{}','') AS translation_published_at
         FROM news WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let published_at: String = row.get("published_at");
    let translation_published_at: String = row.get("translation_published_at");
    let schedule_at = if published_at == translation_published_at {
        published_at.clone()
    } else {
        String::new()
    };
    Ok(form(
        q.staff_lang(chrome.as_deref(), &headers),
        Some(id),
        Input {
            expected_revision: row.get("expected_revision"),
            slug: row.get("slug"),
            title: row.get("title"),
            body: row.get("body"),
            published_at,
            title_pl: row.get("title_pl"),
            body_pl: row.get("body_pl"),
            translation_published_at,
            schedule_at,
            schedule_until: row.get("expires_at"),
            ..Input::default()
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
        "<a href=\"/admin/news?lang={lang}\">{}</a><form class=\"op-stack\" method=\"post\" action=\"{action}?lang={lang}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\">",
        label(lang, "news"),
        escape(&csrf)
    );
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"expected_revision\" value=\"{}\">",
        escape(&input.expected_revision)
    ));
    let expected_revision = input.expected_revision.clone();
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
    let text = |key| crate::pages::text(lang, key);
    html.push_str(&format!(
        "<button class=\"op-button\" name=\"action\" value=\"save\">{}</button>
        <fieldset class=\"op-stack\"><legend>{}</legend><p id=\"window-format\">{}</p>
        <label for=\"schedule_at\">{}</label>
        <input class=\"op-input\" id=\"schedule_at\" name=\"schedule_at\" value=\"{}\" maxlength=\"35\" aria-describedby=\"window-format\">
        <label for=\"schedule_until\">{}</label>
        <input class=\"op-input\" id=\"schedule_until\" name=\"schedule_until\" value=\"{}\" maxlength=\"35\" aria-describedby=\"window-format\">
        <button class=\"op-button\" name=\"action\" value=\"schedule\">{}</button></fieldset></form>",
        label(lang, "save"),
        text("publication-queue"),
        text("publication-format"),
        text("publication-start"),
        escape(&input.schedule_at),
        text("publication-end"),
        escape(&input.schedule_until),
        text("publication-schedule"),
    ));
    if let Some(id) = id {
        html.push_str(&format!("<form class=\"op-stack\" method=\"post\" action=\"/admin/news/{id}/delete\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><input type=\"hidden\" name=\"expected_revision\" value=\"{}\"><button class=\"op-button\">{}</button></form>",escape(&csrf),escape(&expected_revision),label(lang,"delete")));
    }
    page(lang, &label(lang, "news"), html)
}
async fn create(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let scheduled = input.action == "schedule";
    let lang = q.staff_lang(chrome.as_deref(), &headers);
    match save(&pool, &auth, context, None, input).await {
        Err(StatusCode::UNPROCESSABLE_ENTITY) if scheduled => Ok(editorial::form_error(
            lang,
            StatusCode::UNPROCESSABLE_ENTITY,
            "publication-news-invalid",
            &format!("/admin/news/create?lang={lang}"),
        )),
        Ok(response) => Ok(preserve_staff_language(response, &q, lang)),
        Err(status) => Err(status),
    }
}
async fn update(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Query(q): Query<Options>,
    (headers, chrome): (HeaderMap, Option<Extension<oteryn_ui::Chrome>>),
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let scheduled = input.action == "schedule";
    let lang = q.staff_lang(chrome.as_deref(), &headers);
    match save(&pool, &auth, context, Some(id), input).await {
        Err(StatusCode::UNPROCESSABLE_ENTITY) if scheduled => Ok(editorial::form_error(
            lang,
            StatusCode::UNPROCESSABLE_ENTITY,
            "publication-news-invalid",
            &format!("/admin/news/{id}/edit?lang={lang}"),
        )),
        Ok(response) => Ok(preserve_staff_language(response, &q, lang)),
        Err(status) => Err(status),
    }
}
fn preserve_staff_language(mut response: Response, q: &Options, lang: &str) -> Response {
    use axum::http::header::LOCATION;
    if q.lang.is_some()
        && let Some(location) = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
        && let Ok(location) = format!("{location}?lang={lang}").parse()
    {
        response.headers_mut().insert(LOCATION, location);
    }
    response
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
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Option<Uuid>,
    mut input: Input,
) -> Result<Response, StatusCode> {
    let paired = match input.action.as_str() {
        "" | "save" => None,
        "schedule" if !input.title_pl.trim().is_empty() && !input.body_pl.trim().is_empty() => {
            let window = Window::parse(&input.schedule_at, &input.schedule_until)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            input.published_at = editorial::format_timestamp(Some(window.start));
            input
                .translation_published_at
                .clone_from(&input.published_at);
            Some(window)
        }
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    };
    if !input.valid() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        auth,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
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
    auth.check_in(&mut tx, context, Permission::ManageNews)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
    if let Some(id) = id {
        verify_revision(&mut tx, id, &input.expected_revision).await?;
    }
    let action = if id.is_some() {
        "cms.news_updated"
    } else {
        "cms.news_created"
    };
    let end = paired.as_ref().and_then(|window| window.end);
    let (id, expiry) = if let Some(id) = id {
        sqlx::query_as::<_, (Uuid, Option<String>)>(
            "WITH revision AS MATERIALIZED (
                SELECT greatest(clock_timestamp(),updated_at+interval '1 microsecond') AS stamp
                FROM news WHERE id=$1
            )
            UPDATE news SET slug=$2,title=$3,body=$4,published_at=NULLIF($5,'')::timestamptz,
                title_pl=NULLIF($6,''),body_pl=NULLIF($7,''),
                translation_published_at=NULLIF($8,'')::timestamptz,
                expires_at=CASE
                    WHEN $9 THEN $10
                    WHEN $5='' THEN NULL
                    ELSE expires_at END,
                translation_source_updated_at=CASE
                    WHEN $9 THEN (SELECT stamp FROM revision)
                    WHEN translation_published_at IS DISTINCT FROM NULLIF($8,'')::timestamptz
                        AND NULLIF($8,'')::timestamptz IS NOT NULL THEN (SELECT stamp FROM revision)
                    WHEN (title,body) IS DISTINCT FROM ($3,$4)
                        AND (title_pl,body_pl) IS NOT DISTINCT FROM (NULLIF($6,''),NULLIF($7,'')) THEN NULL
                    WHEN (title_pl,body_pl,translation_published_at) IS DISTINCT FROM
                        (NULLIF($6,''),NULLIF($7,''),NULLIF($8,'')::timestamptz) THEN (SELECT stamp FROM revision)
                    WHEN translation_source_updated_at>=updated_at THEN (SELECT stamp FROM revision)
                    ELSE NULL END,
                updated_at=(SELECT stamp FROM revision) WHERE id=$1 RETURNING id,to_jsonb(expires_at)#>>'{}'",
        )
        .bind(id).bind(&input.slug).bind(&input.title).bind(&input.body)
        .bind(&input.published_at).bind(&input.title_pl).bind(&input.body_pl)
        .bind(&input.translation_published_at).bind(paired.is_some()).bind(end)
        .fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StatusCode::NOT_FOUND)?
    } else {
        sqlx::query_as(
            "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,
                              translation_published_at,translation_source_updated_at,expires_at)
             VALUES($1,$2,$3,NULLIF($4,'')::timestamptz,NULLIF($5,''),NULLIF($6,''),
                    NULLIF($7,'')::timestamptz,now(),$8) RETURNING id,to_jsonb(expires_at)#>>'{}'",
        )
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.published_at)
        .bind(&input.title_pl)
        .bind(&input.body_pl)
        .bind(&input.translation_published_at)
        .bind(end)
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?
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
            after: json!({"slug":input.slug,"title":input.title,"body":input.body,"published_at":input.published_at,"title_pl":input.title_pl,"body_pl":input.body_pl,"translation_published_at":input.translation_published_at,"expires_at":expiry}),
        },
    )
    .await
    .map_err(unavailable)?;
    crate::revalidate(
        auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
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
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Form(revision): Form<Revision>,
) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let before: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let before = before.ok_or(StatusCode::NOT_FOUND)?;
    auth.check_in(&mut tx, context, Permission::ManageNews)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
    verify_revision(&mut tx, id, &revision.expected_revision).await?;
    sqlx::query("DELETE FROM news WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
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
    crate::revalidate(
        &auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
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
        ("cache-control", "no-store"),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}

#[derive(Deserialize)]
struct Revision {
    #[serde(default)]
    expected_revision: String,
}
async fn verify_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    expected: &str,
) -> Result<(), StatusCode> {
    // Compare the exact opaque value emitted by the edit form; no client date
    // parsing or timestamp precision loss participates in concurrency control.
    let current: String = sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
    if current != expected {
        return Err(StatusCode::CONFLICT);
    }
    Ok(())
}
