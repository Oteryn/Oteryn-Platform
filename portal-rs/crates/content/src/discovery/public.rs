use askama::Template;
use axum::{
    Router,
    body::Body,
    extract::{Path, Query, Request, State},
    http::{Method, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_ui::{EditorialLink, EditorialMenu};
use oteryn_web::{
    Language,
    chrome::{Shell, with_chrome},
    seo::{EditorialSitemap, Seo},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{borrow::Cow, future::Future, sync::LazyLock, time::Duration};
use tokio::{sync::Semaphore, time::timeout};
use uuid::Uuid;

static ORIGIN: LazyLock<Option<Seo>> = LazyLock::new(|| {
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".to_owned());
    Seo::new(&origin, false).ok()
});

#[derive(sqlx::FromRow)]
struct MenuRow {
    placement: String,
    pl: String,
    en: String,
    menu_label_pl: String,
    menu_label_en: String,
}
async fn menu(pool: &PgPool) -> Result<Vec<MenuRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT d.placement, pl.path AS pl, en.path AS en, d.menu_label_pl, d.menu_label_en
         FROM editorial_discovery d
         JOIN editorial_publication pl USING (kind, document_id)
         JOIN editorial_publication en USING (kind, document_id)
         WHERE d.placement IS NOT NULL AND pl.locale = 'pl' AND en.locale = 'en'
         ORDER BY d.menu_position",
    )
    .fetch_all(pool)
    .await
}
fn localized_menu(rows: Vec<MenuRow>, lang: &str, current: &str) -> EditorialMenu {
    let mut menu = EditorialMenu::default();
    for row in rows {
        let (url, label) = if lang == "pl" {
            (row.pl, row.menu_label_pl)
        } else {
            (row.en, row.menu_label_en)
        };
        let link = EditorialLink {
            current: url == current,
            url,
            label,
        };
        if row.placement == "world" {
            menu.world.push(link);
        } else {
            menu.community.push(link);
        }
    }
    menu
}

const SITEMAP_PAGE_SIZE: i64 = 1_000;
const MAX_SITEMAP_PAGES: u32 = 50_000;
static SITEMAP_READS: Semaphore = Semaphore::const_new(2);
static SHELL_READS: Semaphore = Semaphore::const_new(8);

#[derive(Deserialize)]
struct SitemapQuery {
    page: Option<u32>,
}

pub(super) async fn sitemap(
    pool: &PgPool,
    page: Option<u32>,
) -> Result<EditorialSitemap, StatusCode> {
    const UNAVAILABLE: StatusCode = StatusCode::SERVICE_UNAVAILABLE;
    let mut tx = pool.begin().await.map_err(|_| UNAVAILABLE)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| UNAVAILABLE)?;
    sqlx::query("SET LOCAL statement_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| UNAVAILABLE)?;
    let offset = i64::from(page.unwrap_or(1) - 1) * SITEMAP_PAGE_SIZE;
    let limit = SITEMAP_PAGE_SIZE + i64::from(page.is_none());
    let rows = sqlx::query_as(
        "SELECT max(path) FILTER (WHERE locale='pl') AS pl, max(path) FILTER (WHERE locale='en') AS en,
                to_char(greatest(max(p.modified_at), d.updated_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS lastmod
         FROM editorial_publication p LEFT JOIN editorial_discovery d USING (kind, document_id)
         WHERE coalesce(d.indexing, true)
         GROUP BY p.kind, p.document_id, d.updated_at
         ORDER BY p.kind, p.document_id
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| UNAVAILABLE)?;
    let result = if page.is_none() && rows.len() as i64 > SITEMAP_PAGE_SIZE {
        let documents: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (
                SELECT DISTINCT p.kind, p.document_id
                FROM editorial_publication p LEFT JOIN editorial_discovery d USING (kind, document_id)
                WHERE coalesce(d.indexing, true)
             ) indexed",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| UNAVAILABLE)?;
        let pages = documents.saturating_add(SITEMAP_PAGE_SIZE - 1) / SITEMAP_PAGE_SIZE;
        EditorialSitemap::Index(pages.clamp(1, i64::from(MAX_SITEMAP_PAGES)) as usize)
    } else if page.is_some_and(|page| page > 1) && rows.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    } else {
        EditorialSitemap::Pages {
            rows,
            include_static: offset == 0,
        }
    };
    tx.commit().await.map_err(|_| UNAVAILABLE)?;
    Ok(result)
}

#[derive(sqlx::FromRow)]
struct Metadata {
    path: String,
    title: String,
    description: String,
    indexing: bool,
    alternate: Option<String>,
}
async fn metadata(pool: &PgPool, target: &EditorialPath) -> Result<Option<Metadata>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.path,
                coalesce(CASE WHEN p.locale='pl' THEN d.title_pl ELSE d.title_en END, '') AS title,
                coalesce(CASE WHEN p.locale='pl' THEN d.description_pl ELSE d.description_en END, '') AS description,
                coalesce(d.indexing, true) AS indexing, alt.path AS alternate
         FROM editorial_publication p
         LEFT JOIN editorial_discovery d USING (kind, document_id)
         LEFT JOIN editorial_publication alt
           ON alt.kind=p.kind AND alt.document_id=p.document_id AND alt.locale<>p.locale
         WHERE p.kind=$1 AND p.locale=$2 AND p.slug=$3
           AND ($4::uuid IS NULL OR p.document_id=$4)",
    )
    .bind(target.kind)
    .bind(target.lang)
    .bind(&target.slug)
    .bind(target.id)
    .fetch_optional(pool)
    .await
}

#[derive(Deserialize)]
struct RequestedLanguage {
    lang: Option<String>,
}
struct EditorialPath {
    kind: &'static str,
    lang: &'static str,
    slug: String,
    id: Option<Uuid>,
}

fn decode_segment(segment: &str) -> Option<Cow<'_, str>> {
    if !segment.contains('%') {
        return Some(Cow::Borrowed(segment));
    }
    let mut decoded = Vec::with_capacity(segment.len());
    let mut bytes = segment.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            decoded.push((high * 16 + low) as u8);
        } else {
            decoded.push(byte);
        }
    }
    String::from_utf8(decoded).ok().map(Cow::Owned)
}

fn editorial_path(uri: &Uri) -> Option<EditorialPath> {
    let path = uri.path();
    let detail = |kind, lang, segment| {
        let slug = decode_segment(segment)?;
        (crate::pages::valid_slug(&slug) && (kind != "pages" || !crate::pages::reserved(&slug)))
            .then(|| EditorialPath {
                kind,
                lang,
                slug: slug.into_owned(),
                id: None,
            })
    };
    if let Some(segment) = path.strip_prefix("/news/") {
        let lang = if Query::<RequestedLanguage>::try_from_uri(uri)
            .is_ok_and(|q| q.lang.as_deref() == Some("en"))
        {
            "en"
        } else {
            "pl"
        };
        return detail("news", lang, segment);
    }

    let (prefix, suffix) = path.strip_prefix('/')?.split_once('/')?;
    let lang = match decode_segment(prefix)?.as_ref() {
        "pl" => "pl",
        "en" => "en",
        _ => return None,
    };
    if let Some((section, segment)) = suffix.split_once('/') {
        match section {
            "pages" => return detail("pages", lang, segment),
            "news" => return detail("news", lang, segment),
            "events" => return detail("events", lang, segment),
            "announcements" => {
                return Some(EditorialPath {
                    kind: "announcements",
                    lang,
                    slug: String::new(),
                    id: Some(decode_segment(segment)?.parse().ok()?),
                });
            }
            _ => {}
        }
    }
    let slug = match suffix {
        "rules" => "rules",
        "help" | "support" => "support",
        "support/report-a-bug" => "report-a-bug",
        "server-information" => "server-information",
        "legal/terms" => "terms-of-service",
        "legal/privacy" => "privacy-policy",
        "legal/cookies" => "cookie-policy",
        _ => return None,
    };
    Some(EditorialPath {
        kind: "pages",
        lang,
        slug: slug.to_owned(),
        id: None,
    })
}

#[derive(Template)]
#[template(path = "content/discovery-head.html", config = "src/pages/askama.toml")]
struct Head<'a> {
    lang: &'a str,
    canonical: String,
    alternate: Option<String>,
    alternate_lang: &'a str,
    title: &'a str,
    description: &'a str,
    indexing: bool,
}

fn decorate_html(html: &str, head: &Head<'_>) -> Option<String> {
    let end = html.find("</head>")?;
    let mut output = html.to_owned();
    output.insert_str(end, &head.render().ok()?);
    if !head.title.is_empty()
        && let Some(start) = html[..end].find("<title>")
        && let Some(close) = html[start..end].find("</title>")
    {
        output.replace_range(start..start + close + "</title>".len(), "");
    }
    Some(output)
}

async fn decorate(response: Response, metadata: &Metadata) -> Response {
    if !response.status().is_success()
        || response.headers().contains_key(header::CONTENT_ENCODING)
        || !response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|value| value.as_bytes().starts_with(b"text/html"))
    {
        return response;
    }
    let Some(seo) = ORIGIN.as_ref() else {
        return response;
    };
    let Ok(canonical) = seo.absolute(&metadata.path) else {
        return response;
    };
    let lang = if metadata.path.starts_with("/pl/") {
        "pl"
    } else {
        "en"
    };
    let head = Head {
        lang,
        canonical,
        alternate: metadata
            .alternate
            .as_deref()
            .and_then(|path| seo.absolute(path).ok()),
        alternate_lang: if lang == "pl" { "en" } else { "pl" },
        title: &metadata.title,
        description: &metadata.description,
        indexing: metadata.indexing,
    };
    let (mut parts, body) = response.into_parts();
    let bytes = match oteryn_web::decoration::buffer(body, 4 * 1024 * 1024, None).await {
        Ok(bytes) => bytes,
        Err(body) => return Response::from_parts(parts, body),
    };
    let html = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|html| decorate_html(html, &head));
    if let Some(html) = html {
        parts.headers.remove(header::CONTENT_LENGTH);
        Response::from_parts(parts, Body::from(html))
    } else {
        Response::from_parts(parts, Body::from(bytes))
    }
}

async fn best_effort<T>(read: impl Future<Output = Result<T, sqlx::Error>>) -> Option<T> {
    let _permit = SHELL_READS.try_acquire().ok()?;
    timeout(Duration::from_millis(500), read).await.ok()?.ok()
}

/// Read discovery projections for the public shell and sitemap before rendering.
pub async fn shell(State(shell): State<Shell>, mut request: Request, next: Next) -> Response {
    let pool = &shell.sessions.pool;
    let readable = matches!(*request.method(), Method::GET | Method::HEAD);
    if readable && request.uri().path() == "/sitemap.xml" {
        let Ok(Query(query)) = Query::<SitemapQuery>::try_from_uri(request.uri()) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if query
            .page
            .is_some_and(|page| !(1..=MAX_SITEMAP_PAGES).contains(&page))
        {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let Ok(_permit) = SITEMAP_READS.try_acquire() else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::RETRY_AFTER, "1")],
            )
                .into_response();
        };
        return match timeout(Duration::from_secs(5), sitemap(pool, query.page)).await {
            Ok(Ok(entries)) => {
                request.extensions_mut().insert(entries);
                next.run(request).await
            }
            Ok(Err(status)) => status.into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
    }

    let target = readable.then(|| editorial_path(request.uri())).flatten();
    let metadata = if let Some(target) = &target {
        best_effort(metadata(pool, target)).await.flatten()
    } else {
        None
    };
    let rows = if oteryn_web::chrome::accepts_html(&request) {
        best_effort(menu(pool)).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let current = metadata
        .as_ref()
        .map_or(request.uri().path(), |document| &document.path)
        .to_owned();
    let editor_path = request
        .uri()
        .path()
        .starts_with("/admin/discovery/")
        .then(|| request.uri().path().to_owned());
    let editor_language = editor_path.as_ref().and_then(|_| {
        Query::<RequestedLanguage>::try_from_uri(request.uri())
            .ok()?
            .0
            .lang
    });
    let language = match target
        .as_ref()
        .map(|target| target.lang)
        .or(editor_language.as_deref())
    {
        Some("pl") => Some(Language::Pl),
        Some("en") => Some(Language::En),
        _ => None,
    };
    let response = with_chrome(shell, request, next, language, |chrome| {
        chrome.editorial_menu = localized_menu(rows, &chrome.lang, &current);
        if let Some(path) = editor_path {
            chrome.alternate_url = format!("{path}?lang={}", chrome.alternate_lang);
        }
        if let Some(m) = &metadata {
            if let Some(alternate) = &m.alternate {
                chrome.alternate_url.clone_from(alternate);
            } else {
                chrome.alternate_url = format!("/language/{}", chrome.alternate_lang);
            }
            if chrome
                .editorial_menu
                .world
                .iter()
                .chain(&chrome.editorial_menu.community)
                .any(|link| link.current)
            {
                chrome.current_section.clear();
                for link in &mut chrome.entries {
                    link.current = false;
                }
            }
        }
    })
    .await;
    let mut response = response;
    if let Some(target) = target {
        let success = response.status().is_success();
        if success {
            response.headers_mut().insert(
                header::CONTENT_LANGUAGE,
                header::HeaderValue::from_static(target.lang),
            );
        }
        if !success || metadata.as_ref().is_none_or(|m| !m.indexing) {
            response.headers_mut().insert(
                header::HeaderName::from_static("x-robots-tag"),
                header::HeaderValue::from_static("noindex"),
            );
        }
    }
    if let Some(metadata) = &metadata {
        decorate(response, metadata).await
    } else {
        response
    }
}

pub fn public_router(pool: PgPool) -> Router {
    Router::new()
        .route("/{lang}/announcements/{id}", get(announcement))
        .with_state(pool)
        .layer(axum::middleware::from_fn(crate::pages::public_headers))
}
#[derive(Template)]
#[template(
    path = "content/announcement-detail.html",
    config = "src/pages/askama.toml"
)]
struct AnnouncementPage {
    lang: String,
    title: String,
    body: String,
    action_label: Option<String>,
    action_url: Option<String>,
}
async fn announcement(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
) -> Result<Response, StatusCode> {
    if !matches!(lang.as_str(), "pl" | "en") {
        return Err(StatusCode::NOT_FOUND);
    }
    let row: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT CASE WHEN $1='pl' THEN a.title_pl ELSE a.title END,
                CASE WHEN $1='pl' THEN a.body_pl ELSE a.body END,
                CASE WHEN $1='pl' THEN a.action_label_pl ELSE a.action_label END, a.action_url
         FROM announcements a JOIN editorial_publication p ON p.kind='announcements' AND p.document_id=a.id
         WHERE a.id=$2 AND p.locale=$1",
    )
    .bind(&lang)
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (title, body, action_label, action_url) = row.ok_or(StatusCode::NOT_FOUND)?;
    Ok(oteryn_web::render(&AnnouncementPage {
        lang,
        title,
        body,
        action_label,
        action_url: action_url.filter(|url| crate::announcements::safe_action(url)),
    }))
}
