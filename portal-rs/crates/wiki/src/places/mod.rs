//! Place cards consume qualified public evidence, never private journal rows.
//!
//! T81 currently exposes authenticated solo cohorts only. Public cohorts, party
//! sizes and kill rates need a qualified adapter; G-9 v1 forbids session sharing.
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
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
#[derive(Debug)]
struct SamplePlaces(oteryn_world::WorldStore);

impl PublicPlaces for SamplePlaces {
    fn cards<'a>(&'a self, locale: &'a str) -> CardsFuture<'a> {
        Box::pin(async move {
            let entities = self
                .0
                .list_by_kind("HuntPlace", None, 100)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            entities
                .into_iter()
                .map(|entity| {
                    let data = entity.data;
                    let unavailable = StatusCode::SERVICE_UNAVAILABLE;
                    let purpose = match data["purpose"].as_str() {
                        Some("hunt") => Purpose::Hunt,
                        Some("boss") => Purpose::Boss,
                        Some("bestiary") => Purpose::Bestiary,
                        _ => return Err(unavailable),
                    };
                    Ok(Card {
                        key: entity.key,
                        title: data["title"][locale].as_str().ok_or(unavailable)?.into(),
                        purpose,
                        level: data["level"]
                            .as_u64()
                            .and_then(|n| n.try_into().ok())
                            .ok_or(unavailable)?,
                        party_size: data["party_size"]
                            .as_u64()
                            .and_then(|n| n.try_into().ok())
                            .ok_or(unavailable)?,
                        experience_per_hour: None,
                        profit_per_hour: None,
                        points: None,
                        completion_seconds: None,
                        editorial_summary: data["summary"][locale]
                            .as_str()
                            .ok_or(unavailable)?
                            .into(),
                    })
                })
                .collect()
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlacesState {
    pub enabled: bool,
    pub source: Arc<dyn PublicPlaces>,
}
fn enabled() -> bool {
    std::env::var("PORTAL_WIKI_PLACES_ENABLED").as_deref() == Ok("true")
}

pub fn configured_router() -> Router {
    router(PlacesState {
        enabled: enabled(),
        source: Arc::new(Unavailable),
    })
}
pub fn configured_with_catalog(db: sqlx::PgPool) -> Router {
    if std::env::var("OTERYN_ENV").as_deref() != Ok("testing")
        || std::env::var("PORTAL_STAGING_SAMPLE_DATA").as_deref() != Ok("true")
    {
        return configured_router();
    }
    router(PlacesState {
        enabled: std::env::var("PORTAL_WIKI_PLACES_ENABLED").as_deref() == Ok("true"),
        source: Arc::new(SamplePlaces(oteryn_world::WorldStore::new(
            db,
            oteryn_world::WorldOptions {
                enabled: oteryn_world::configured_world_enabled(),
            },
        ))),
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
    #[serde(default, deserialize_with = "optional_party")]
    party: Option<u8>,
}

fn optional_party<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<u8>, D::Error> {
    let raw = String::deserialize(value)?;
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

impl Filters {
    fn purpose(&self) -> Result<Purpose, StatusCode> {
        match self.purpose.as_deref() {
            None | Some("hunt") => Ok(Purpose::Hunt),
            Some("bestiary") => Ok(Purpose::Bestiary),
            Some("boss") => Ok(Purpose::Boss),
            _ => Err(StatusCode::BAD_REQUEST),
        }
    }

    fn validate(&self) -> Result<(), StatusCode> {
        self.purpose()?;
        if self.party.is_some_and(|n| !matches!(n, 1 | 2 | 4))
            || !matches!(self.sort.as_deref(), None | Some("experience" | "profit"))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(())
    }
}

fn select(mut cards: Vec<Card>, filters: &Filters) -> Result<Vec<Card>, StatusCode> {
    filters.validate()?;
    let purpose = filters.purpose()?;
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
    text: Text,
    state: Option<&'static str>,
    filters: Filters,
    cards: Vec<Card>,
}
struct Text(fluent_bundle::FluentBundle<fluent_bundle::FluentResource>);

impl Text {
    fn new(locale: &str) -> Result<Self, StatusCode> {
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
        Ok(Self(bundle))
    }

    fn get(&self, key: &str) -> Result<String, askama::Error> {
        self.0
            .get_message(key)
            .and_then(|m| m.value())
            .map(|value| self.0.format_pattern(value, None, &mut vec![]).into_owned())
            .ok_or(askama::Error::Fmt)
    }
}

impl Page {
    fn party_selected(&self, party: &u8) -> bool {
        self.filters.party == Some(*party)
    }

    fn t(&self, key: &str) -> Result<String, askama::Error> {
        self.text.get(key)
    }
}

async fn page(
    State(state): State<PlacesState>,
    Path(locale): Path<String>,
    filters: Result<Query<Filters>, axum::extract::rejection::QueryRejection>,
) -> Response {
    if !matches!(locale.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (status, filters, cards) = match filters {
        Ok(Query(filters)) if filters.validate().is_ok() => {
            let cards = if state.enabled {
                state
                    .source
                    .cards(&locale)
                    .await
                    .and_then(|cards| select(cards, &filters))
            } else {
                Err(StatusCode::SERVICE_UNAVAILABLE)
            };
            match cards {
                Ok(cards) => (StatusCode::OK, filters, cards),
                Err(StatusCode::FORBIDDEN) => return StatusCode::FORBIDDEN.into_response(),
                Err(_) => (StatusCode::SERVICE_UNAVAILABLE, filters, Vec::new()),
            }
        }
        _ => (StatusCode::BAD_REQUEST, Filters::default(), Vec::new()),
    };
    let state = match status {
        StatusCode::BAD_REQUEST => Some("invalid"),
        StatusCode::SERVICE_UNAVAILABLE => Some("unavailable"),
        _ if cards.is_empty() => Some("empty"),
        _ => None,
    };
    let text = match Text::new(&locale) {
        Ok(text) => text,
        Err(status) => return status.into_response(),
    };
    render(
        status,
        Page {
            locale,
            text,
            state,
            filters,
            cards,
        },
    )
}

fn render(status: StatusCode, page: impl Template) -> Response {
    match page.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Template)]
#[template(path = "admin/wiki/game_evidence.html", config = "wiki_askama.toml")]
struct Operator {
    lang: &'static str,
    text: Text,
    bestiary_enabled: bool,
    places_enabled: bool,
}

impl Operator {
    fn t(&self, key: &str) -> Result<String, askama::Error> {
        self.text.get(key)
    }

    fn state(&self, enabled: &bool) -> Result<String, askama::Error> {
        self.t(if *enabled {
            "awaiting-qualification"
        } else {
            "flag-off"
        })
    }
}

pub(crate) async fn operator(headers: HeaderMap) -> Response {
    let lang = crate::web::language(&headers);
    match Text::new(lang) {
        Ok(text) => render(
            StatusCode::OK,
            Operator {
                lang,
                text,
                bestiary_enabled: oteryn_atlas::bestiary::web::enabled(),
                places_enabled: enabled(),
            },
        ),
        Err(status) => status.into_response(),
    }
}

#[cfg(test)]
mod tests;
