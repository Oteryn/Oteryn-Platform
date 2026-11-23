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
    chrome::{Shell, with_chrome},
    seo::{EditorialSitemap, Seo},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::LazyLock;
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

async fn sitemap(pool: &PgPool) -> Result<EditorialSitemap, sqlx::Error> {
    let rows = sqlx::query_as(
        "SELECT max(path) FILTER (WHERE locale='pl') AS pl, max(path) FILTER (WHERE locale='en') AS en,
                to_char(greatest(max(p.modified_at), d.updated_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS lastmod
         FROM editorial_publication p LEFT JOIN editorial_discovery d USING (kind, document_id)
         WHERE coalesce(d.indexing, true)
         GROUP BY p.kind, p.document_id, d.updated_at
         ORDER BY p.kind, p.document_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(EditorialSitemap(rows))
}

#[derive(sqlx::FromRow)]
struct Metadata {
    path: String,
    title: String,
    description: String,
    indexing: bool,
    alternate: Option<String>,
}
async fn metadata(pool: &PgPool, path: &str) -> Result<Option<Metadata>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.path,
                coalesce(CASE WHEN p.locale='pl' THEN d.title_pl ELSE d.title_en END, '') AS title,
                coalesce(CASE WHEN p.locale='pl' THEN d.description_pl ELSE d.description_en END, '') AS description,
                coalesce(d.indexing, true) AS indexing, alt.path AS alternate
         FROM editorial_publication p
         LEFT JOIN editorial_discovery d USING (kind, document_id)
         LEFT JOIN editorial_publication alt
           ON alt.kind=p.kind AND alt.document_id=p.document_id AND alt.locale<>p.locale
         WHERE p.path=$1",
    )
    .bind(path)
    .fetch_optional(pool)
    .await
}

#[derive(Deserialize)]
struct LegacyNews {
    lang: Option<String>,
}
fn editorial_path(uri: &Uri) -> Option<String> {
    let path = uri.path();
    if let Some(slug) = path.strip_prefix("/news/")
        && crate::pages::valid_slug(slug)
    {
        let lang = if Query::<LegacyNews>::try_from_uri(uri)
            .is_ok_and(|q| q.lang.as_deref() == Some("en"))
        {
            "en"
        } else {
            "pl"
        };
        return Some(format!("/{lang}{path}"));
    }
    let (lang, suffix) = path.strip_prefix('/')?.split_once('/')?;
    if !matches!(lang, "pl" | "en") {
        return None;
    }
    if suffix == "support" {
        return Some(format!("/{lang}/help"));
    }
    let detail = suffix
        .split_once('/')
        .is_some_and(|(section, slug)| match section {
            "pages" | "news" | "events" => crate::pages::valid_slug(slug),
            "announcements" => slug.parse::<Uuid>().is_ok_and(|id| id.to_string() == slug),
            _ => false,
        });
    (detail
        || matches!(
            suffix,
            "rules"
                | "help"
                | "support/report-a-bug"
                | "server-information"
                | "legal/terms"
                | "legal/privacy"
                | "legal/cookies"
        ))
    .then(|| path.to_owned())
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

/// Read discovery projections for the public shell and sitemap before rendering.
pub async fn shell(State(shell): State<Shell>, mut request: Request, next: Next) -> Response {
    let pool = &shell.sessions.pool;
    if request.uri().path() == "/sitemap.xml"
        && matches!(*request.method(), Method::GET | Method::HEAD)
    {
        return match sitemap(pool).await {
            Ok(entries) => {
                request.extensions_mut().insert(entries);
                next.run(request).await
            }
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
    }

    let path = editorial_path(request.uri());
    let metadata = if let Some(path) = &path {
        metadata(pool, path).await.ok().flatten()
    } else {
        None
    };
    let rows = if request.method() == Method::GET
        && request
            .headers()
            .get(header::ACCEPT)
            .is_some_and(|value| value.to_str().is_ok_and(|s| s.contains("text/html")))
    {
        menu(pool).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let current = path.as_deref().unwrap_or(request.uri().path()).to_owned();
    let response = with_chrome(shell, request, next, |chrome| {
        chrome.editorial_menu = localized_menu(rows, &chrome.lang, &current);
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
    if path.is_some() && (metadata.as_ref().is_none_or(|m| !m.indexing)) {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-robots-tag"),
            header::HeaderValue::from_static("noindex"),
        );
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
