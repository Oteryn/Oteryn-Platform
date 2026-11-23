use super::{
    MAX_PAGE, Options, display_date, escape, excerpt, label, language, tag_valid, unavailable,
};
use askama::Template;
use axum::{
    Extension,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const PAGE_SIZE: i64 = 10;
const FEED_SIZE: i64 = 30;

#[derive(Clone)]
pub(super) struct FeedOrigin(pub(super) String);

#[derive(Clone, Copy)]
enum Section {
    News,
    Changelog,
}

impl Section {
    fn key(self) -> &'static str {
        match self {
            Self::News => "news",
            Self::Changelog => "changelog",
        }
    }
}

#[derive(Default)]
struct Filters<'a> {
    year: Option<i32>,
    month: Option<i32>,
    tag: &'a str,
    search: &'a str,
}

impl Options {
    fn filters(&self, section: Section) -> Result<Filters<'_>, StatusCode> {
        fn number(value: &str, max: i32) -> Result<Option<i32>, StatusCode> {
            if value.is_empty() {
                return Ok(None);
            }
            if value.len() <= 4
                && value.bytes().all(|byte| byte.is_ascii_digit())
                && let Ok(n) = value.parse::<i32>()
                && (1..=max).contains(&n)
            {
                return Ok(Some(n));
            }
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        }

        match section {
            Section::News => {
                if !self.tag.is_empty() && !tag_valid(&self.tag) {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                Ok(Filters {
                    year: number(&self.year, 9999)?,
                    month: number(&self.month, 12)?,
                    tag: &self.tag,
                    ..Filters::default()
                })
            }
            Section::Changelog => {
                let search = self.q.trim();
                if search.chars().count() > 120 || search.chars().any(char::is_control) {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                Ok(Filters {
                    search,
                    ..Filters::default()
                })
            }
        }
    }
}

struct Entry {
    id: Uuid,
    slug: String,
    title: String,
    excerpt: String,
    day: Option<String>,
    display_date: String,
    rss_date: Option<String>,
    tags: Vec<String>,
}

async fn entries(
    pool: &PgPool,
    lang: &str,
    section: Section,
    filters: &Filters<'_>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Entry>, sqlx::Error> {
    sqlx::query(
        "SELECT id, slug, tags,
                CASE WHEN $1='pl' THEN title_pl ELSE title END AS local_title,
                CASE WHEN $1='pl' THEN body_pl ELSE body END AS local_body,
                CASE WHEN published_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
                     THEN to_char(published_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') END AS day,
                CASE WHEN published_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
                     THEN to_char(published_at AT TIME ZONE 'UTC', 'Dy, DD Mon YYYY HH24:MI:SS') END AS rss_date
           FROM news
          WHERE is_changelog=$2 AND published_at<=now()
            AND ($1='en' OR (translation_published_at<=now()
                 AND translation_source_updated_at>=updated_at
                 AND length(trim(title_pl))>0 AND length(trim(body_pl))>0))
            AND ($3::int IS NULL OR EXTRACT(YEAR FROM published_at AT TIME ZONE 'UTC')=$3)
            AND ($4::int IS NULL OR EXTRACT(MONTH FROM published_at AT TIME ZONE 'UTC')=$4)
            AND ($5='' OR tags @> ARRAY[$5]::text[])
            AND ($6='' OR strpos(lower(CASE WHEN $1='pl' THEN title_pl ELSE title END), lower($6))>0
                       OR strpos(lower(CASE WHEN $1='pl' THEN body_pl ELSE body END), lower($6))>0)
          ORDER BY published_at DESC, id DESC LIMIT $7 OFFSET $8",
    )
    .bind(lang)
    .bind(matches!(section, Section::Changelog))
    .bind(filters.year)
    .bind(filters.month)
    .bind(filters.tag)
    .bind(filters.search)
    .bind(limit)
    .bind(offset)
    .try_map(|row| {
        let day: Option<String> = row.try_get("day")?;
        Ok(Entry {
            id: row.try_get("id")?,
            slug: row.try_get("slug")?,
            title: row.try_get("local_title")?,
            excerpt: excerpt(row.try_get("local_body")?, 200),
            display_date: day.as_deref().map(|date| display_date(lang, date)).unwrap_or_default(),
            day,
            rss_date: row.try_get("rss_date")?,
            tags: row.try_get("tags")?,
        })
    })
    .fetch_all(pool)
    .await
}

#[derive(Template)]
#[template(path = "content/news_archive.html", config = "src/pages/askama.toml")]
struct Archive<'a> {
    lang: &'a str,
    section: &'static str,
    title: String,
    filters: Filters<'a>,
    entries: Vec<Entry>,
    previous: Option<String>,
    next: Option<String>,
}

impl Archive<'_> {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
}

async fn list(
    pool: &PgPool,
    lang: &str,
    options: &Options,
    section: Section,
) -> Result<Html<String>, StatusCode> {
    let filters = options.filters(section)?;
    let entries = entries(
        pool,
        lang,
        section,
        &filters,
        PAGE_SIZE + 1,
        options.offset(PAGE_SIZE)?,
    )
    .await
    .map_err(unavailable)?;
    let current = options.page.unwrap_or(1);
    let next = (entries.len() as i64 > PAGE_SIZE && current < MAX_PAGE)
        .then(|| page_query(lang, &filters, current + 1));
    let previous = (current > 1).then(|| page_query(lang, &filters, current - 1));
    let archive = Archive {
        lang,
        section: section.key(),
        title: label(lang, section.key()),
        filters,
        entries: entries.into_iter().take(PAGE_SIZE as usize).collect(),
        previous,
        next,
    };
    archive.render().map(Html).map_err(unavailable)
}

fn page_query(lang: &str, filters: &Filters<'_>, page: i64) -> String {
    let mut query = format!("?page={page}&lang={lang}");
    for (key, value) in [
        (
            "year",
            filters
                .year
                .map(|year| year.to_string())
                .unwrap_or_default(),
        ),
        (
            "month",
            filters
                .month
                .map(|month| month.to_string())
                .unwrap_or_default(),
        ),
        ("tag", filters.tag.to_owned()),
        ("q", filters.search.to_owned()),
    ] {
        if !value.is_empty() {
            query.push('&');
            query.push_str(key);
            query.push('=');
            for byte in value.bytes() {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                    query.push(char::from(byte));
                } else {
                    use std::fmt::Write;
                    let _ = write!(query, "%{byte:02X}");
                }
            }
        }
    }
    query
}

pub(super) async fn index(
    State(pool): State<PgPool>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    listing(&pool, q.lang(), &q).await
}

pub(super) async fn local_index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    language(&lang)?;
    listing(&pool, &lang, &q).await
}

pub(super) async fn listing(
    pool: &PgPool,
    lang: &str,
    q: &Options,
) -> Result<Html<String>, StatusCode> {
    list(pool, lang, q, Section::News).await
}

pub(super) async fn changelog(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    language(&lang)?;
    list(&pool, &lang, &q, Section::Changelog).await
}

fn xml(value: &str) -> String {
    let valid: String = value
        .chars()
        .filter(|&c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
        .collect();
    escape(&valid)
}

pub(super) async fn rss(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(origin): Extension<FeedOrigin>,
) -> Result<Response, StatusCode> {
    use std::fmt::Write;

    language(&lang)?;
    let items = entries(
        &pool,
        &lang,
        Section::News,
        &Filters::default(),
        FEED_SIZE,
        0,
    )
    .await
    .map_err(unavailable)?;
    let channel = xml(&format!("{}/{lang}/news", origin.0));
    let mut feed = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel><title>{}</title><link>{channel}</link><description>{}</description><language>{lang}</language>",
        xml(&label(&lang, "feed-title")),
        xml(&label(&lang, "feed-description")),
    );
    for item in items {
        let _ = write!(
            feed,
            "<item><title>{}</title><link>{channel}/{}</link><guid isPermaLink=\"false\">urn:uuid:{}</guid><description>{}</description>",
            xml(&item.title),
            xml(&item.slug),
            item.id,
            xml(&escape(&item.excerpt)),
        );
        if let Some(date) = item.rss_date {
            let _ = write!(feed, "<pubDate>{date} +0000</pubDate>");
        }
        for tag in item.tags {
            let _ = write!(feed, "<category>{}</category>", xml(&tag));
        }
        feed.push_str("</item>");
    }
    feed.push_str("</channel></rss>\n");
    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        feed,
    )
        .into_response())
}
