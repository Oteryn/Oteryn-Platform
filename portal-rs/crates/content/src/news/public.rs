use super::{
    MAX_PAGE, Options, display_date, escape, excerpt, label, language, tag_valid, translation,
    unavailable,
};
use askama::Template;
use axum::{
    Extension,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use oteryn_web::Language;
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
    fn filters(&self, section: Section) -> Result<Filters<'_>, &'static str> {
        fn number(value: &str, max: i32) -> Option<Option<i32>> {
            if value.is_empty() {
                return Some(None);
            }
            if value.len() <= 4
                && value.bytes().all(|byte| byte.is_ascii_digit())
                && let Ok(n) = value.parse::<i32>()
                && (1..=max).contains(&n)
            {
                return Some(Some(n));
            }
            None
        }

        match section {
            Section::News => {
                if !self.tag.is_empty() && !tag_valid(&self.tag) {
                    return Err("invalid-tag");
                }
                Ok(Filters {
                    year: number(&self.year, 9999).ok_or("invalid-year")?,
                    month: number(&self.month, 12).ok_or("invalid-month")?,
                    tag: &self.tag,
                    ..Filters::default()
                })
            }
            Section::Changelog => {
                let search = self.q.trim();
                if search.chars().count() > 120 || search.chars().any(char::is_control) {
                    return Err("invalid-search");
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
            AND (expires_at IS NULL OR now() < expires_at)
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
    options: &'a Options,
    error: Option<String>,
    count: String,
    entries: Vec<Entry>,
    previous: Option<String>,
    next: Option<String>,
}

impl Archive<'_> {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
}

async fn list<'a>(
    pool: &PgPool,
    lang: &'a str,
    options: &'a Options,
    section: Section,
) -> Result<Archive<'a>, ArchiveError> {
    let filters = options.filters(section).map_err(ArchiveError::input)?;
    let entries = entries(
        pool,
        lang,
        section,
        &filters,
        PAGE_SIZE + 1,
        options
            .offset(PAGE_SIZE)
            .map_err(|_| ArchiveError::input("invalid-page"))?,
    )
    .await
    .map_err(|err| ArchiveError::new(unavailable(err), "archive-unavailable"))?;
    let current = options.page.unwrap_or(1);
    let next = (entries.len() as i64 > PAGE_SIZE && current < MAX_PAGE)
        .then(|| page_query(lang, &filters, current + 1));
    let previous = (current > 1).then(|| page_query(lang, &filters, current - 1));
    let entries: Vec<_> = entries.into_iter().take(PAGE_SIZE as usize).collect();
    Ok(Archive {
        lang,
        section: section.key(),
        title: label(lang, section.key()),
        options,
        error: None,
        count: count(lang, entries.len()).ok_or_else(|| {
            ArchiveError::new(StatusCode::SERVICE_UNAVAILABLE, "archive-unavailable")
        })?,
        entries,
        previous,
        next,
    })
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

fn count(lang: &str, count: usize) -> Option<String> {
    let mut args = fluent_bundle::FluentArgs::new();
    args.set("count", count);
    translation(lang, "archive-count", Some(&args))
}

#[derive(Debug)]
struct ArchiveError {
    status: StatusCode,
    key: &'static str,
}

impl ArchiveError {
    fn new(status: StatusCode, key: &'static str) -> Self {
        Self { status, key }
    }

    fn input(key: &'static str) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, key)
    }
}

async fn archive_response(
    lang: &str,
    section: Section,
    query: Result<Query<Options>, QueryRejection>,
    pool: &PgPool,
) -> Response {
    let options = query.as_ref().map(|Query(options)| options);
    let default = Options::default();
    let result = match options {
        Ok(options) if options.lang.as_deref().is_none_or(|v| language(v).is_ok()) => {
            list(pool, lang, options, section).await
        }
        _ => Err(ArchiveError::new(
            StatusCode::BAD_REQUEST,
            "invalid-archive-query",
        )),
    };
    let (status, archive) = match result {
        Ok(archive) => (StatusCode::OK, archive),
        Err(error) => {
            let archive = Archive {
                lang,
                section: section.key(),
                title: label(lang, section.key()),
                options: options.unwrap_or(&default),
                error: Some(label(lang, error.key)),
                count: String::new(),
                entries: Vec::new(),
                previous: None,
                next: None,
            };
            (error.status, archive)
        }
    };
    let Ok(html) = archive.render() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut response = (status, Html(html)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_LANGUAGE,
        HeaderValue::from_static(if lang == "en" { "en" } else { "pl" }),
    );
    headers.append(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response
}

pub(super) async fn index(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    query: Result<Query<Options>, QueryRejection>,
) -> Response {
    let lang = match query.as_ref().ok().and_then(|q| q.lang.as_deref()) {
        Some("en") => "en",
        Some("pl") => "pl",
        _ => Language::negotiate(&headers).code(),
    };
    archive_response(lang, Section::News, query, &pool).await
}

pub(super) async fn local_index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    query: Result<Query<Options>, QueryRejection>,
) -> Result<Response, StatusCode> {
    language(&lang)?;
    Ok(archive_response(&lang, Section::News, query, &pool).await)
}

pub(super) async fn changelog(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    query: Result<Query<Options>, QueryRejection>,
) -> Result<Response, StatusCode> {
    language(&lang)?;
    Ok(archive_response(&lang, Section::Changelog, query, &pool).await)
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn archive_count_and_filter_messages_are_localized() {
        assert_eq!(count("en", 1).as_deref(), Some("One entry on this page"));
        assert_eq!(count("en", 2).as_deref(), Some("2 entries on this page"));
        assert_eq!(count("pl", 2).as_deref(), Some("2 wpisy na tej stronie"));
        assert_eq!(count("pl", 5).as_deref(), Some("5 wpisów na tej stronie"));
        assert_eq!(translation("pl", "missing-key", None), None);

        let input = Options {
            month: "13".into(),
            ..Options::default()
        };
        assert_eq!(input.filters(Section::News).err(), Some("invalid-month"));
        let input = Options {
            q: "a\u{0000}b".into(),
            ..Options::default()
        };
        assert_eq!(
            input.filters(Section::Changelog).err(),
            Some("invalid-search")
        );
    }

    #[tokio::test]
    async fn invalid_locale_and_result_page_do_not_query_storage() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://localhost/unused")
            .unwrap();
        for (input, status, message) in [
            (
                Options {
                    lang: Some("fr".into()),
                    ..Options::default()
                },
                StatusCode::BAD_REQUEST,
                "Check the values in the form.",
            ),
            (
                Options {
                    page: Some(MAX_PAGE + 1),
                    ..Options::default()
                },
                StatusCode::UNPROCESSABLE_ENTITY,
                "Choose a valid result page.",
            ),
        ] {
            let response = archive_response("en", Section::News, Ok(Query(input)), &pool).await;
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CONTENT_LANGUAGE], "en");
            let body = axum::body::to_bytes(response.into_body(), 100_000)
                .await
                .unwrap();
            assert!(String::from_utf8_lossy(&body).contains(message));
        }
    }
}
