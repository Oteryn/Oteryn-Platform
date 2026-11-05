//! Portal full-text search. Results are read at the current publication boundary.
//! Owning content crates depend on web, so the local query is a read-only seam.
use std::{future::Future, pin::Pin, sync::Arc};

use crate::ratelimit::{RateLimit, RateLimiter, WikiSearch};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};

/// Search providers must enforce current visibility, lifecycle and freshness on
/// every call. Unknown evidence is an error; names never establish identity.
pub type SearchFuture = Pin<Box<dyn Future<Output = Result<Vec<Hit>, StatusCode>> + Send>>;
pub type Provider = Arc<dyn Fn(String, String, i64) -> SearchFuture + Send + Sync>;

#[derive(Clone)]
pub struct SearchState {
    pub pool: PgPool,
    /// Public world API adapter, supplied by the composition root to avoid cycles.
    pub world: Option<Provider>,
    /// T24b adapter. Absent until its public privacy/freshness API is available.
    pub characters: Option<Provider>,
}
impl std::fmt::Debug for SearchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchState")
            .field("world", &self.world.is_some())
            .field("characters", &self.characters.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub title: String,
    /// Local, locale-prefixed destination. Providers must percent-encode segments.
    pub href: String,
    pub kind: String,
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    #[serde(default)]
    q: String,
    #[serde(default)]
    kind: String,
    page: Option<i64>,
}
impl Filters {
    fn validate(&mut self) -> Result<(), StatusCode> {
        if self.q.len() > 1024 || self.q.chars().any(char::is_control) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        self.q = self.q.split_whitespace().collect::<Vec<_>>().join(" ");
        if (!self.q.is_empty() && !(2..=80).contains(&self.q.chars().count()))
            || !(1..=1000).contains(&self.page.unwrap_or(1))
            || !["", "news", "pages", "events", "wiki", "world", "characters"]
                .contains(&self.kind.as_str())
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(())
    }
}
/// Standalone routes carry the shared language/security policy and search limiter.
pub fn routes(state: SearchState) -> Router {
    let limiter = RateLimiter::new(state.pool.clone());
    let app = Router::new()
        .route("/search", get(show))
        .route("/pl/search", get(show))
        .route("/en/search", get(show))
        .layer(axum::middleware::from_fn(no_store))
        .layer(Extension(limiter))
        .with_state(state);
    crate::surface::finish(app)
}

async fn no_store(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("x-robots-tag"),
        axum::http::HeaderValue::from_static("noindex, follow"),
    );
    response
}

#[derive(Template)]
#[template(path = "search/index.html")]
struct Page {
    lang: &'static str,
    filters: Filters,
    hits: Vec<Hit>,
    next: i64,
    more: bool,
    unavailable: bool,
}
impl Page {
    fn t(&self, id: &str) -> String {
        text(self.lang, id)
    }
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "pl" {
        include_str!("search/pl.ftl")
    } else {
        include_str!("search/en.ftl")
    };
    let locale = if lang == "pl" {
        unic_langid::langid!("pl")
    } else {
        unic_langid::langid!("en")
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.into())
        && bundle.add_resource(resource).is_ok()
        && let Some(value) = bundle.get_message(key).and_then(|m| m.value())
    {
        return bundle.format_pattern(value, None, &mut vec![]).into_owned();
    }
    String::new()
}
async fn editorial(pool: &PgPool, lang: &str, f: &Filters) -> Result<Vec<Hit>, sqlx::Error> {
    let rows = sqlx::query(include_str!("search.sql"))
        .bind(lang)
        .bind(&f.q)
        .bind(&f.kind)
        .bind((f.page.unwrap_or(1) - 1) * 20)
        .fetch_all(pool)
        .await?;
    rows.into_iter()
        .map(|r| {
            let kind: String = r.try_get("kind")?;
            let slug: String = r.try_get("slug")?;
            Ok(Hit {
                title: r.try_get("title")?,
                href: destination(lang, &kind, &slug),
                kind,
            })
        })
        .collect()
}
fn destination(lang: &str, kind: &str, slug: &str) -> String {
    let reserved = if kind == "pages" {
        match slug {
            "rules" => Some("rules"),
            "terms-of-service" => Some("legal/terms"),
            "privacy-policy" => Some("legal/privacy"),
            "cookie-policy" => Some("legal/cookies"),
            "server-information" => Some("server-information"),
            "report-a-bug" => Some("support/report-a-bug"),
            _ => None,
        }
    } else {
        None
    };
    if let Some(path) = reserved {
        return format!("/{lang}/{path}");
    }
    let path = if kind == "wiki" {
        "wiki/articles"
    } else {
        kind
    };
    format!("/{lang}/{path}/{}", segment(slug))
}

/// Encode a single path segment without allowing a source key to alter the route.
pub fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
async fn show(
    State(state): State<SearchState>,
    Extension(language): Extension<crate::Language>,
    _: RateLimit<WikiSearch>,
    query: Result<Query<Filters>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let lang = language.code();
    let Ok(Query(mut filters)) = query else {
        return (StatusCode::UNPROCESSABLE_ENTITY, text(lang, "invalid")).into_response();
    };
    if filters.validate().is_err() {
        return (StatusCode::UNPROCESSABLE_ENTITY, text(lang, "invalid")).into_response();
    }
    let mut hits = vec![];
    let mut unavailable = false;
    if !filters.q.is_empty() {
        let result = match filters.kind.as_str() {
            "world" | "characters" => {
                let provider = if filters.kind == "world" {
                    &state.world
                } else {
                    &state.characters
                };
                match provider {
                    Some(provider) => {
                        provider(lang.into(), filters.q.clone(), filters.page.unwrap_or(1)).await
                    }
                    None => Err(StatusCode::SERVICE_UNAVAILABLE),
                }
            }
            _ => editorial(&state.pool, lang, &filters)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE),
        };
        match result {
            Ok(rows) => hits = rows,
            Err(_) => unavailable = true,
        }
    }
    let more = hits.len() > 20;
    hits.truncate(20);
    let next = filters.page.unwrap_or(1) + 1;
    let mut response = crate::render(&Page {
        lang,
        filters,
        hits,
        next,
        more: more && next <= 1000,
        unavailable,
    });
    if unavailable {
        *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("x-robots-tag"),
        axum::http::HeaderValue::from_static("noindex, follow"),
    );
    response
}

#[cfg(test)]
mod tests;

pub type VisibilityFuture =
    Pin<Box<dyn Future<Output = Result<Option<String>, StatusCode>> + Send>>;
pub type WorldReader = Arc<dyn Fn(String, String) -> VisibilityFuture + Send + Sync>;

/// Candidate keys are never visibility authority. Re-read every candidate with
/// the owning public world API before returning a title or destination.
pub async fn world_search(
    pool: &PgPool,
    lang: &str,
    query: &str,
    page: i64,
    reader: WorldReader,
) -> Result<Vec<Hit>, StatusCode> {
    if !["pl", "en"].contains(&lang)
        || !(1..=1000).contains(&page)
        || !(2..=80).contains(&query.chars().count())
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let rows = sqlx::query(include_str!("search_world.sql"))
        .bind(query)
        .bind((page - 1) * 20)
        .bind(lang)
        .fetch_all(pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut hits = vec![];
    for row in rows {
        let kind: String = row
            .try_get("kind")
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let key: String = row
            .try_get("key")
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(title) = reader(kind.clone(), key.clone()).await? {
            let matches: bool = sqlx::query_scalar(
                "SELECT to_tsvector('simple',$1) @@ plainto_tsquery('simple',$2)",
            )
            .bind(format!("{key} {title}"))
            .bind(query)
            .fetch_one(pool)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            if !matches {
                continue;
            }
            let route = match kind.as_str() {
                "Item" => "items",
                "Creature" => "creatures",
                "NPC" => "npcs",
                "Spell" => "spells",
                "Quest" => "quests",
                "Loot" => "loot",
                "Achievement" => "achievements",
                "Area" => "areas",
                _ => continue,
            };
            hits.push(Hit {
                title,
                href: format!("/{lang}/wiki/{route}/{}", segment(&key)),
                kind: "world".into(),
            });
        }
    }
    Ok(hits)
}

/// Select only the public presentation name from an approved world entity.
/// Unknown or malformed presentation falls back to the stable public key.
pub fn world_title(data: &serde_json::Value, key: &str, lang: &str) -> String {
    let body = data
        .get("definition")
        .or_else(|| data.get("declaration"))
        .or_else(|| data.pointer("/bundle/spell"))
        .unwrap_or(data);
    [
        data.pointer("/authoring/profile/details/display_name"),
        body.get("names").and_then(|v| v.get(lang)),
        body.get("display_name"),
        body.get("name"),
    ]
    .into_iter()
    .flatten()
    .filter_map(|v| v.as_str())
    .find(|v| !v.is_empty() && v.len() <= 256 && !v.chars().any(char::is_control))
    .unwrap_or(key)
    .to_owned()
}
