//! News storage and public and staff routes.
use crate::revisions::{self, Edit, Target};
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
    rbac::{CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use oteryn_web::ratelimit::{self, PublicGameData, RateLimit, RateLimiter};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

mod public;

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
        include_str!("../../../templates/content/pl.ftl")
    } else {
        include_str!("../../../templates/content/en.ftl")
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
    #[serde(default)]
    year: String,
    #[serde(default)]
    month: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    q: String,
    media_id: Option<Uuid>,
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
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    #[default]
    News,
    Changelog,
}

#[derive(Debug, Default, Deserialize)]
struct Input {
    #[serde(default)]
    kind: Option<Kind>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    expected_revision: String,
    media_id: Option<String>,
    lang: Option<String>,
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
const MAX_TAGS: usize = 10;
const MAX_TAG_LENGTH: usize = 48;

fn tag_valid(tag: &str) -> bool {
    tag.len() <= MAX_TAG_LENGTH && slug_valid(tag)
}

fn parse_tags(value: &str) -> Result<Vec<&str>, StatusCode> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut tags: Vec<_> = value.split(',').take(MAX_TAGS + 1).map(str::trim).collect();
    if tags.len() > MAX_TAGS || tags.iter().any(|tag| !tag_valid(tag)) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    tags.sort_unstable();
    tags.dedup();
    Ok(tags)
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
    public_router_at(pool, "http://localhost:3000")
}

fn public_router_at(pool: PgPool, origin: &str) -> Router {
    let archive = Router::new()
        .route("/news", get(public::index))
        .route("/{lang}/news", get(public::local_index))
        .route("/{lang}/news/rss.xml", get(public::rss))
        .route("/{lang}/changelog", get(public::changelog))
        .route_layer(axum::middleware::from_fn(
            |_: RateLimit<PublicGameData>, request, next: axum::middleware::Next| async move {
                next.run(request).await
            },
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            RateLimiter::new(pool.clone()),
            ratelimit::supply,
        ));
    archive
        .route("/news/{slug}", get(show))
        .route("/news/media/{id}", get(crate::media::published_content))
        .route("/{lang}/news/{slug}", get(local_show))
        .with_state(pool)
        .layer(Extension(public::FeedOrigin(
            origin.trim_end_matches('/').to_owned(),
        )))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn admin_router(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/news", get(admin_index).post(create))
        .route("/admin/news/create", get(new))
        .route("/admin/news/{id}/edit", get(edit))
        .route("/admin/news/{id}/revisions", get(history))
        .route("/admin/news/{id}/revisions/{source}/restore", post(restore))
        .route("/admin/news/{id}", post(update).put(update).delete(delete))
        .route("/admin/news/{id}/delete", post(delete))
        .with_state(pool)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let origin =
        std::env::var("OTERYN_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());
    oteryn_web::seo::Seo::new(&origin, false)?;
    Ok(
        public_router_at(pool.clone(), &origin).merge(oteryn_admin::web::configured_guard(
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
/// Newest published entries for the home page, under the same language rule
/// as the public listing. The excerpt is plain text; templates escape it.
pub async fn headlines(
    pool: &PgPool,
    lang: oteryn_web::Language,
    limit: i64,
) -> Result<Vec<oteryn_ui::HomeNews>, sqlx::Error> {
    let lang = lang.code();
    let rows = sqlx::query("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body,to_char(published_at AT TIME ZONE 'UTC','YYYY-MM-DD') AS day FROM news WHERE NOT is_changelog AND published_at <= now() AND (expires_at IS NULL OR now() < expires_at) AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT $2").bind(lang).bind(limit.clamp(0, 10)).fetch_all(pool).await?;
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
    use askama::Template;
    let row = sqlx::query(
        "SELECT is_changelog, CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,
                CASE WHEN $1='pl' THEN body_pl ELSE body END AS body,
                m.id AS media_id, m.alt_text,
                least(m.width, greatest(1, round(m.width::numeric * 320 / greatest(m.width, m.height))::int)) AS image_width
         FROM news LEFT JOIN media m ON m.id=news.media_id
         WHERE published_at<=now() AND slug=$2
           AND (expires_at IS NULL OR now() < expires_at) AND ($1='en' OR
             (translation_published_at<=now() AND translation_source_updated_at>=news.updated_at
              AND length(trim(title_pl))>0 AND length(trim(body_pl))>0))",
    )
    .bind(lang)
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let section = if row.get("is_changelog") {
        "changelog"
    } else {
        "news"
    };
    let image = row
        .get::<Option<Uuid>, _>("media_id")
        .map(|id| ArticleImage {
            id,
            alt: row.get("alt_text"),
            width: row.get("image_width"),
        });
    let article = Article {
        image,
        body: escape(row.get("body")).replace('\n', "<br>"),
    };
    Ok(page(
        lang,
        row.get("title"),
        format!(
            "<p><a href=\"/{lang}/{section}\">{}</a>{}",
            label(lang, section),
            article.render().map_err(unavailable)?
        ),
    ))
}

struct ArticleImage {
    id: Uuid,
    alt: String,
    width: i32,
}

#[derive(askama::Template)]
#[template(path = "content/news_article.html")]
struct Article {
    image: Option<ArticleImage>,
    body: String,
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
    media_id: Option<Uuid>,
    media_library: Option<String>,
    rows: Vec<NewsQueueRow>,
}

struct NewsQueueRow {
    id: Uuid,
    title: String,
    kind: String,
    en_status: String,
    pl_status: String,
    start: String,
    pl_start: String,
    end: String,
}

async fn admin_index(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Response, StatusCode> {
    let can_manage_media =
        crate::media::can_manage(&pool, &auth, context, Permission::ManageMedia).await?;
    let lang = q.staff_lang(chrome.as_deref(), &headers);
    admin_listing(&pool, q, lang, can_manage_media).await
}

async fn admin_listing(
    pool: &PgPool,
    q: Options,
    lang: &str,
    can_manage_media: bool,
) -> Result<Response, StatusCode> {
    let rows = sqlx::query(
        "SELECT id, CASE WHEN $3='pl' THEN coalesce(title_pl,title) ELSE title END AS title, is_changelog,to_jsonb(published_at)#>>'{}' AS starts_at,
                 to_jsonb(pl_start)#>>'{}' AS pl_starts_at,to_jsonb(expires_at)#>>'{}' AS ends_at,
                 COALESCE(published_at>now(),false) AS pending,
                 COALESCE(pl_start>now(),false) AS pending_pl,
                 COALESCE(expires_at<=now(),false) AS expired,
                 COALESCE(translation_source_updated_at>=updated_at
                    AND length(trim(title_pl))>0 AND length(trim(body_pl))>0,false) AS current_pl
         FROM (
             SELECT news.*, CASE WHEN published_at IS NOT NULL AND translation_published_at IS NOT NULL
                 THEN greatest(published_at,translation_published_at) END AS pl_start
             FROM news WHERE ($2::uuid IS NULL OR media_id=$2)
         ) editions
         ORDER BY CASE WHEN published_at>now() THEN published_at END NULLS LAST,
                  updated_at DESC,id DESC LIMIT 26 OFFSET $1",
    )
    .bind(q.offset(25)?)
    .bind(q.media_id)
    .bind(lang)
    .fetch_all(pool)
    .await
    .map_err(unavailable)?;
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
                kind: label(lang, if row.get("is_changelog") { "changelog" } else { "news" }),
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
        media_id: q.media_id,
        media_library: if can_manage_media {
            Some(crate::media::label(&crate::media::messages(lang)?, "media-news-library"))
        } else {
            None
        },
        rows,
    }))
}

async fn new(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Extension(session): Extension<BrowserSession>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Html<String>, StatusCode> {
    let can_manage_media =
        crate::media::can_manage(&pool, &auth, context, Permission::ManageMedia).await?;
    form(
        q.staff_lang(chrome.as_deref(), &headers),
        editorial::staff_language(chrome.as_deref(), &headers),
        None,
        Input {
            media_id: q.media_id.map(|id| id.to_string()),
            ..Input::default()
        },
        session.csrf_token().await.map_err(unavailable)?,
        can_manage_media,
    )
}
async fn edit(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    Query(q): Query<Options>,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Html<String>, StatusCode> {
    let can_manage_media =
        crate::media::can_manage(&pool, &auth, context, Permission::ManageMedia).await?;
    let row = sqlx::query(
        "SELECT updated_at::text AS expected_revision,media_id::text AS media_id,slug,title,body,is_changelog,tags,
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
    form(
        q.staff_lang(chrome.as_deref(), &headers),
        editorial::staff_language(chrome.as_deref(), &headers),
        Some(id),
        Input {
            media_id: row.get("media_id"),
            kind: Some(if row.get("is_changelog") {
                Kind::Changelog
            } else {
                Kind::News
            }),
            tags: Some(row.get::<Vec<String>, _>("tags").join(", ")),
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
        can_manage_media,
    )
}
fn form(
    lang: &str,
    history_lang: &str,
    id: Option<Uuid>,
    input: Input,
    csrf: String,
    can_manage_media: bool,
) -> Result<Html<String>, StatusCode> {
    let action = id.map_or_else(
        || "/admin/news".to_owned(),
        |id| format!("/admin/news/{id}"),
    );
    let mut html = id.map_or_else(String::new, |id| {
        format!(
            "<a href=\"{}\">{}</a>",
            Target::News(id).history_path(),
            escape(&revisions::history_label(history_lang)),
        )
    });
    html.push_str(&format!(
        "<a href=\"/admin/news?lang={lang}\">{}</a><form class=\"op-stack\" method=\"post\" action=\"{action}?lang={lang}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\">",
        label(lang, "news"),
        escape(&csrf)
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"expected_revision\" value=\"{}\">",
        escape(&input.expected_revision)
    ));
    let (news_selected, changelog_selected) = match input.kind.unwrap_or_default() {
        Kind::News => (" selected", ""),
        Kind::Changelog => ("", " selected"),
    };
    html.push_str(&format!(
        "<label for=\"kind\">{}</label><select class=\"op-input\" id=\"kind\" name=\"kind\">\
         <option value=\"news\"{news_selected}>{}</option>\
         <option value=\"changelog\"{changelog_selected}>{}</option></select>\
         <label for=\"tags\">{}</label><input class=\"op-input\" id=\"tags\" name=\"tags\" value=\"{}\" aria-describedby=\"tags-help\">\
         <small id=\"tags-help\">{}</small>",
        label(lang, "kind"),
        label(lang, "news"),
        label(lang, "changelog"),
        label(lang, "tags"),
        escape(input.tags.as_deref().unwrap_or_default()),
        label(lang, "tags-help"),
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
    if can_manage_media {
        let messages = crate::media::messages(lang)?;
        let library = crate::media::label(&messages, "media-news-library");
        let field = crate::media::label(&messages, "media-news-field");
        html.push_str(&format!(
            "<label for=\"media_id\">{}</label><input class=\"op-input\" id=\"media_id\" name=\"media_id\" maxlength=\"36\" value=\"{}\"><a href=\"/admin/media?lang={lang}\" target=\"_blank\" rel=\"noopener\">{}</a>",
            escape(&field), escape(input.media_id.as_deref().unwrap_or_default()), escape(&library)
        ));
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
    Ok(page(lang, &label(lang, "news"), html))
}
async fn history(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    Query(query): Query<revisions::Query>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Response, StatusCode> {
    revisions::history(
        &pool,
        Target::News(id),
        revisions::language(chrome.as_deref()),
        query,
        &session,
    )
    .await
}

async fn restore(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path((id, source)): Path<(Uuid, i64)>,
    Form(form): Form<revisions::Restore<String>>,
) -> Result<Response, StatusCode> {
    save_change(
        &pool,
        &auth,
        context,
        Some(id),
        Edit::Restore { source, form },
    )
    .await
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
        && !location.contains('?')
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
    input: Input,
) -> Result<Response, StatusCode> {
    save_change(pool, auth, context, id, Edit::Save(input)).await
}

async fn save_change(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Option<Uuid>,
    change: Edit<Input, String>,
) -> Result<Response, StatusCode> {
    let media_id = match &change {
        Edit::Save(input) => input.media_id.as_deref(),
        Edit::Restore { .. } => None,
    }
        .filter(|field| !field.is_empty())
        .map(str::parse::<Uuid>)
        .transpose()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM media WHERE id=$1 FOR KEY SHARE")
                    .bind(media_id)
                    .execute(&mut *connection)
                    .await?;
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
    let (mut input, restored_from) = match change {
        Edit::Save(input) => {
            if let Some(id) = id {
                verify_revision(&mut tx, id, &input.expected_revision).await?;
            }
            (input, None)
        }
        Edit::Restore { source, form } => {
            if form.expected_revision.is_empty() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let id = id.ok_or(StatusCode::BAD_REQUEST)?;
            verify_revision(&mut tx, id, &form.expected_revision).await?;
            let snapshot = Target::News(id)
                .snapshot(&mut tx, source)
                .await
                .map_err(revisions::Error::status)?;
            let value = |field| revisions::value(&snapshot, field);
            (
                Input {
                    slug: value("slug"),
                    title: value("title"),
                    body: value("body"),
                    title_pl: value("title_pl"),
                    body_pl: value("body_pl"),
                    kind: snapshot["is_changelog"].as_bool().map(|is_changelog| {
                        if is_changelog {
                            Kind::Changelog
                        } else {
                            Kind::News
                        }
                    }),
                    tags: snapshot["tags"].as_array().map(|tags| {
                        tags.iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }),
                    ..Input::default()
                },
                Some(source),
            )
        }
    };
    let paired = match input.action.as_str() {
        "" | "save" => None,
        "schedule" if !input.title_pl.trim().is_empty() && !input.body_pl.trim().is_empty() => {
            let window = Window::parse(&input.schedule_at, &input.schedule_until)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            input.published_at = editorial::format_timestamp(Some(window.start));
            input.translation_published_at.clone_from(&input.published_at);
            Some(window)
        }
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    };
    if !input.valid() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut permissions = vec![Permission::Access, Permission::ManageNews];
    if input.media_id.is_some() && before["media_id"] != json!(media_id) {
        auth.check(&mut tx, context, Permission::ManageMedia)
            .await
            .map_err(crate::media::authorization_status)?;
        permissions.push(Permission::ManageMedia);
    }
    let tags = input.tags.as_deref().map(parse_tags).transpose()?;
    let is_changelog = input.kind.map(|kind| matches!(kind, Kind::Changelog));
    let action = if restored_from.is_some() {
        "cms.news_restored"
    } else if id.is_some() {
        "cms.news_updated"
    } else {
        "cms.news_created"
    };
    let end = paired.as_ref().and_then(|window| window.end);
    let (id, is_changelog, tags, expiry): (Uuid, bool, Vec<String>, Option<String>) = if let Some(id) = id {
        sqlx::query_as(
            "WITH revision AS MATERIALIZED (
                SELECT greatest(clock_timestamp(), updated_at + interval '1 microsecond') AS stamp
                  FROM news WHERE id=$1
             )
             UPDATE news SET slug=$2, title=$3, body=$4, published_at=NULLIF($5,'')::timestamptz,
                   title_pl=NULLIF($6,''), body_pl=NULLIF($7,''),
                   translation_published_at=NULLIF($8,'')::timestamptz,
                   expires_at=CASE WHEN $11 THEN $12 WHEN $5='' THEN NULL ELSE expires_at END,
                   translation_source_updated_at=CASE
                       WHEN $11 THEN (SELECT stamp FROM revision)
                       WHEN translation_published_at IS DISTINCT FROM NULLIF($8,'')::timestamptz
                           AND NULLIF($8,'')::timestamptz IS NOT NULL THEN (SELECT stamp FROM revision)
                       WHEN (title,body) IS DISTINCT FROM ($3,$4)
                           AND (title_pl,body_pl) IS NOT DISTINCT FROM (NULLIF($6,''),NULLIF($7,'')) THEN NULL
                       WHEN (title_pl,body_pl,translation_published_at) IS DISTINCT FROM
                           (NULLIF($6,''),NULLIF($7,''),NULLIF($8,'')::timestamptz) THEN (SELECT stamp FROM revision)
                       WHEN translation_source_updated_at>=updated_at THEN (SELECT stamp FROM revision)
                       ELSE NULL END,
                   is_changelog=coalesce($9,is_changelog), tags=coalesce($10,tags),
                   updated_at=(SELECT stamp FROM revision)
             WHERE id=$1 RETURNING id,is_changelog,tags,to_jsonb(expires_at)#>>'{}'",
        )
        .bind(id)
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.published_at)
        .bind(&input.title_pl)
        .bind(&input.body_pl)
        .bind(&input.translation_published_at)
        .bind(is_changelog)
        .bind(&tags)
        .bind(paired.is_some())
        .bind(end)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            if restored_from.is_some()
                && error.as_database_error().is_some_and(|db| db.is_unique_violation())
            {
                StatusCode::CONFLICT
            } else {
                storage(error)
            }
        })?
        .ok_or(StatusCode::NOT_FOUND)?
    } else {
        sqlx::query_as(
            "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,
                              translation_source_updated_at,is_changelog,tags,expires_at)
             VALUES($1,$2,$3,NULLIF($4,'')::timestamptz,NULLIF($5,''),NULLIF($6,''),
                    NULLIF($7,'')::timestamptz,now(),$8,$9,$10) RETURNING id,is_changelog,tags,to_jsonb(expires_at)#>>'{}'",
        )
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.published_at)
        .bind(&input.title_pl)
        .bind(&input.body_pl)
        .bind(&input.translation_published_at)
        .bind(is_changelog.unwrap_or_default())
        .bind(tags.unwrap_or_default())
        .bind(end)
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?
    };
    let media_id: Option<Uuid> = if input.media_id.is_some() {
        sqlx::query_scalar("UPDATE news SET media_id=$2 WHERE id=$1 RETURNING media_id")
            .bind(id)
            .bind(media_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?
    } else {
        sqlx::query_scalar("SELECT media_id FROM news WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?
    };
    Target::News(id)
        .append(&mut tx, context.account, restored_from)
        .await
        .map_err(unavailable)?;
    let mut after = json!({"slug":input.slug,"title":input.title,"body":input.body,"published_at":input.published_at,"title_pl":input.title_pl,"body_pl":input.body_pl,"translation_published_at":input.translation_published_at,"is_changelog":is_changelog,"tags":tags,"expires_at":expiry,"media_id":media_id});
    if let Some(source) = restored_from {
        after["restored_from"] = json!(source);
    }
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
            after,
        },
    )
    .await
    .map_err(unavailable)?;
    crate::revalidate(auth, &mut tx, context, &permissions)
        .await
        .map_err(crate::media::authorization_status)?;
    tx.commit().await.map_err(unavailable)?;
    let locale = match input.lang.as_deref() {
        Some("en") => "?lang=en",
        Some("pl") => "?lang=pl",
        _ => "",
    };
    Ok((
        StatusCode::FOUND,
        [(
            axum::http::header::LOCATION,
            format!("/admin/news/{id}/edit{locale}"),
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
