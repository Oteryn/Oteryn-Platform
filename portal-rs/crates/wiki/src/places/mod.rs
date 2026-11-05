//! Place cards consume qualified public evidence, never private journal rows.
//!
//! T81 currently exposes authenticated solo cohorts only. Public cohorts, party
//! sizes and kill rates need a qualified adapter; G-9 v1 forbids session sharing.
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
};
use serde::Deserialize;
use std::{future::Future, pin::Pin, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    Hunt,
    Bestiary,
    Boss,
}
#[derive(Debug, Clone)]
pub struct Card {
    pub key: String,
    pub title: String,
    pub purpose: Purpose,
    pub level: u32,
    pub party_size: u8,
    pub experience_per_hour: Option<u64>,
    pub profit_per_hour: Option<i64>,
    pub points: Option<u32>,
    pub completion_seconds: Option<u64>,
    pub editorial_summary: String,
}
/// Implementations must recheck current visibility and published editorial status.
/// Rates must come from delayed, privacy-qualified T81 cohorts. No identities,
/// session links, private progress or today's boss presence may be returned.
/// Missing party or kill-rate evidence must remain absent, never inferred.
pub type CardsFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<Card>, StatusCode>> + Send + 'a>>;
pub trait PublicPlaces: Send + Sync + std::fmt::Debug {
    fn cards<'a>(&'a self, locale: &'a str) -> CardsFuture<'a>;
}
#[derive(Debug)]
struct Unavailable;
impl PublicPlaces for Unavailable {
    fn cards<'a>(&'a self, _: &'a str) -> CardsFuture<'a> {
        Box::pin(async { Err(StatusCode::SERVICE_UNAVAILABLE) })
    }
}
#[derive(Debug, Clone)]
pub struct PlacesState {
    pub enabled: bool,
    pub source: Arc<dyn PublicPlaces>,
}
pub fn configured_router() -> Router {
    router(PlacesState {
        enabled: std::env::var("PORTAL_WIKI_PLACES_ENABLED").as_deref() == Ok("true"),
        source: Arc::new(Unavailable),
    })
}
pub fn router(state: PlacesState) -> Router {
    Router::new()
        .route("/{locale}/wiki/places", get(page))
        .with_state(state)
        .layer(axum::middleware::from_fn(no_store))
}
async fn no_store(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    sort: Option<String>,
    purpose: Option<String>,
    party: Option<u8>,
}
fn select(mut cards: Vec<Card>, filters: &Filters) -> Result<Vec<Card>, StatusCode> {
    let purpose = match filters.purpose.as_deref() {
        None | Some("hunt") => Purpose::Hunt,
        Some("bestiary") => Purpose::Bestiary,
        Some("boss") => Purpose::Boss,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if filters.party.is_some_and(|n| !matches!(n, 1 | 2 | 4)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !matches!(
        filters.sort.as_deref(),
        None | Some("experience" | "profit")
    ) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if cards.iter().any(|c| {
        c.key.is_empty() || c.title.is_empty() || c.level == 0 || !matches!(c.party_size, 1 | 2 | 4)
    }) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    cards.retain(|c| c.purpose == purpose && filters.party.is_none_or(|n| c.party_size == n));
    cards.sort_by(|a, b| {
        let order = if filters.sort.as_deref() == Some("profit") {
            b.profit_per_hour.cmp(&a.profit_per_hour)
        } else {
            b.experience_per_hour.cmp(&a.experience_per_hour)
        };
        order.then_with(|| a.key.cmp(&b.key))
    });
    cards.truncate(100);
    Ok(cards)
}
#[derive(Template)]
#[template(path = "places/page.html", config = "wiki_askama.toml")]
struct Page {
    locale: String,
    title: String,
    unavailable: String,
    reports: String,
    cards: Vec<Card>,
    labels: Vec<String>,
}
fn text(locale: &str, key: &str) -> Result<String, StatusCode> {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = match locale {
        "pl" => include_str!("pl.ftl"),
        "en" => include_str!("en.ftl"),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let resource = FluentResource::try_new(source.to_owned())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let language = locale
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut bundle = FluentBundle::new(vec![language]);
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let value = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(bundle.format_pattern(value, None, &mut vec![]).into_owned())
}
async fn page(
    State(state): State<PlacesState>,
    Path(locale): Path<String>,
    Query(filters): Query<Filters>,
) -> Result<(StatusCode, Html<String>), StatusCode> {
    let title = text(&locale, "title")?;
    select(vec![], &filters)?;
    let (status, cards) = if state.enabled {
        match state.source.cards(&locale).await {
            Ok(cards) => (StatusCode::OK, select(cards, &filters)?),
            Err(StatusCode::SERVICE_UNAVAILABLE) => (StatusCode::SERVICE_UNAVAILABLE, vec![]),
            Err(StatusCode::FORBIDDEN) => return Err(StatusCode::FORBIDDEN),
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, vec![])
    };
    let page = Page {
        title,
        unavailable: text(&locale, "unavailable")?,
        reports: text(&locale, "reports")?,
        locale: locale.clone(),
        cards,
        labels: [
            "level",
            "party",
            "experience",
            "profit",
            "points",
            "completion",
        ]
        .iter()
        .map(|key| text(&locale, key))
        .collect::<Result<Vec<_>, _>>()?,
    };
    page.render()
        .map(|html| (status, Html(html)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
#[cfg(test)]
mod tests;
