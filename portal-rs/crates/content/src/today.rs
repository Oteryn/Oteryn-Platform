//! Public daily digest. Each source retains its own failure state.
use crate::announcements::{Announcement, Announcements, safe_action};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Present,
    Empty,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Complete,
    Partial,
    Unavailable,
}
pub fn page_state(cards: &[Card]) -> PageState {
    if cards.iter().all(|c| c.state == SourceState::Unavailable) {
        PageState::Unavailable
    } else if cards.iter().any(|c| c.state == SourceState::Unavailable) {
        PageState::Partial
    } else {
        PageState::Complete
    }
}
#[derive(Debug, Clone)]
pub struct Item {
    pub title: String,
    pub body: String,
    pub url: Option<String>,
    pub action: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Card {
    pub kind: &'static str,
    pub state: SourceState,
    pub items: Vec<Item>,
}
impl Card {
    fn from_result(kind: &'static str, result: Result<Vec<Item>, impl std::fmt::Display>) -> Self {
        match result {
            Ok(items) => Self {
                kind,
                state: if items.is_empty() {
                    SourceState::Empty
                } else {
                    SourceState::Present
                },
                items,
            },
            Err(_) => Self {
                kind,
                state: SourceState::Unavailable,
                items: vec![],
            },
        }
    }
}
/// No game provider is enabled by this editorial feature.
pub async fn cards(pool: &PgPool, lang: &str, at: OffsetDateTime) -> Vec<Card> {
    let announcements = Announcements::new(pool.clone()).active(lang, at, 3).await;
    let events = crate::events::Events::new(pool.clone())
        .public(lang, None)
        .await;
    let news = latest_news(pool, lang, at).await;
    vec![
        Card {
            kind: "liveops",
            state: SourceState::Unavailable,
            items: vec![],
        },
        Card::from_result(
            "announcements",
            announcements.map(|v| v.into_iter().map(|a| item(a, lang)).collect()),
        ),
        Card::from_result(
            "events",
            events.map(|mut v| {
                v.retain(|e| e.ends_at > at);
                v.sort_by_key(|e| e.starts_at > at);
                v.into_iter()
                    .take(1)
                    .map(|e| Item {
                        title: e.title,
                        body: e.summary,
                        url: Some(format!("/{lang}/events/{}", e.slug)),
                        action: Some(text(lang, "view")),
                    })
                    .collect()
            }),
        ),
        Card::from_result("news", news),
    ]
}
// T11 exposes HTTP routes only; this bounded adapter preserves its publication predicate.
async fn latest_news(
    pool: &PgPool,
    lang: &str,
    at: OffsetDateTime,
) -> Result<Vec<Item>, sqlx::Error> {
    let rows:Vec<(String,String,String)>=sqlx::query_as("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END,CASE WHEN $1='pl' THEN body_pl ELSE body END FROM news WHERE published_at <= $2 AND ($1='en' OR (translation_published_at <= $2 AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT 3").bind(lang).bind(at).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(slug, title, body)| Item {
            title,
            body,
            url: if crate::events::valid_slug(&slug) {
                Some(format!("/{lang}/news/{slug}"))
            } else {
                None
            },
            action: Some(text(lang, "view")),
        })
        .collect())
}
fn item(a: Announcement, lang: &str) -> Item {
    Item {
        title: format!("{}: {}", text(lang, &a.severity), a.title),
        body: a.body,
        url: a.action_url,
        action: a.action_label,
    }
}
pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "pl" {
        "today = Dziś\nannouncements = Ogłoszenia\nevents = Wydarzenia\nnews = Aktualności\nliveops = Status świata\nempty = Brak aktualnych treści.\nunavailable = Dane są teraz niedostępne.\npartial = Część źródeł jest niedostępna.\nall-unavailable = Wszystkie źródła są teraz niedostępne.\nview = Zobacz\ninfo = Informacja\nmaintenance = Przerwa techniczna\nwarning = Ostrzeżenie"
    } else {
        "today = Today\nannouncements = Announcements\nevents = Events\nnews = News\nliveops = World status\nempty = No current content.\nunavailable = Data is currently unavailable.\npartial = Some sources are unavailable.\nall-unavailable = All sources are currently unavailable.\nview = View\ninfo = Information\nmaintenance = Maintenance\nwarning = Warning"
    };
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
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
fn render_items(items: &[Item]) -> String {
    let mut out = String::new();
    for i in items {
        out.push_str(&format!(
            "<article class=\"op-card\"><h3>{}</h3><p>{}</p>",
            escape(&i.title),
            escape(&i.body.chars().take(240).collect::<String>())
        ));
        if let (Some(url), Some(label)) = (&i.url, &i.action)
            && safe_action(url)
            && !label.trim().is_empty()
        {
            out.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape(url),
                escape(label)
            ));
        }
        out.push_str("</article>");
    }
    out
}
/// Shared SSR ticker: unavailable and empty sources never fabricate announcements.
pub async fn render_ticker(pool: &PgPool, lang: &str, at: OffsetDateTime) -> String {
    let card = Card::from_result(
        "announcements",
        Announcements::new(pool.clone())
            .active(lang, at, 5)
            .await
            .map(|v| v.into_iter().map(|a| item(a, lang)).collect()),
    );
    if card.state == SourceState::Empty {
        return String::new();
    }
    format!(
        "<aside aria-label=\"{}\" class=\"op-stack\">{}</aside>",
        text(lang, "announcements"),
        if card.state == SourceState::Unavailable {
            format!("<p>{}</p>", text(lang, "unavailable"))
        } else {
            render_items(&card.items)
        }
    )
}
#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\" class=\"op-theme\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"></head><body><main class=\"op-stack\"><h1>{{ title }}</h1>{{ body|safe }}</main></body></html>",
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    body: String,
}
#[derive(Default, serde::Deserialize)]
struct Options {
    lang: Option<String>,
}
fn language(lang: &str) -> Result<&str, StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(lang)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/today", get(index))
        .route("/{lang}/today", get(local))
        .route("/announcements/ticker", get(ticker))
        .route("/{lang}/announcements/ticker", get(local_ticker))
        .with_state(pool)
}
async fn index(
    State(pool): State<PgPool>,
    Query(q): Query<Options>,
) -> Result<Response, StatusCode> {
    digest(&pool, language(q.lang.as_deref().unwrap_or("pl"))?).await
}
async fn local(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
) -> Result<Response, StatusCode> {
    digest(&pool, language(&lang)?).await
}
async fn ticker(
    State(pool): State<PgPool>,
    Query(q): Query<Options>,
) -> Result<Response, StatusCode> {
    Ok(response(
        render_ticker(
            &pool,
            language(q.lang.as_deref().unwrap_or("pl"))?,
            OffsetDateTime::now_utc(),
        )
        .await,
    ))
}
async fn local_ticker(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
) -> Result<Response, StatusCode> {
    Ok(response(
        render_ticker(&pool, language(&lang)?, OffsetDateTime::now_utc()).await,
    ))
}
async fn digest(pool: &PgPool, lang: &str) -> Result<Response, StatusCode> {
    use askama::Template;
    let cards = cards(pool, lang, OffsetDateTime::now_utc()).await;
    let mut body = String::new();
    if page_state(&cards) != PageState::Complete {
        body.push_str(&format!(
            "<p role=\"status\">{}</p>",
            text(
                lang,
                if page_state(&cards) == PageState::Unavailable {
                    "all-unavailable"
                } else {
                    "partial"
                }
            )
        ));
    }
    for c in cards {
        body.push_str(&format!(
            "<section aria-labelledby=\"{}\" class=\"op-stack\"><h2 id=\"{}\">{}</h2>",
            c.kind,
            c.kind,
            text(lang, c.kind)
        ));
        body.push_str(&match c.state {
            SourceState::Empty => format!("<p>{}</p>", text(lang, "empty")),
            SourceState::Unavailable => format!("<p>{}</p>", text(lang, "unavailable")),
            SourceState::Present => render_items(&c.items),
        });
        body.push_str("</section>");
    }
    let html = Page {
        lang,
        title: text(lang, "today"),
        body,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response(html))
}
fn response(html: String) -> Response {
    let mut r = Html(html).into_response();
    for (key, value) in [
        ("cache-control", "no-store, max-age=0"),
        ("pragma", "no-cache"),
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
    ] {
        r.headers_mut().insert(key, HeaderValue::from_static(value));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digest_states_and_localization() {
        let mut cards = vec![Card {
            kind: "news",
            state: SourceState::Empty,
            items: vec![],
        }];
        assert_eq!(page_state(&cards), PageState::Complete);
        cards.push(Card {
            kind: "events",
            state: SourceState::Unavailable,
            items: vec![],
        });
        assert_eq!(page_state(&cards), PageState::Partial);
        cards[0].state = SourceState::Unavailable;
        assert_eq!(page_state(&cards), PageState::Unavailable);
        assert_eq!(text("pl", "today"), "Dziś");
        assert_eq!(text("en", "today"), "Today");
        assert!(language("unknown").is_err());
    }
}
