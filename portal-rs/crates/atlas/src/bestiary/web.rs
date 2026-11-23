//! SSR surface. A T72 adapter is required before private data can be served.
use super::*;
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_playerdata::authority::CollectionEvidence;
use oteryn_web::session::{BrowserSession, SessionLayer};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// Implementations revalidate current account/character ownership and transfer,
/// erasure, restore fences before AND after reading. They call `journal` for
/// private progress, never infer ownership from public character projections.
/// Catalog/spawns must belong to one visible ready active world version; private
/// pace observations must belong to the same ownership cut. Unknown denies.
pub trait Source: Send + Sync {
    fn load_owned(
        &self,
        account: Uuid,
        character: Uuid,
        now_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Snapshot>> + Send + '_>>;
}
#[derive(Debug)]
pub struct Snapshot {
    pub world_version: String,
    pub account: Uuid,
    pub character: Uuid,
    /// Oldest source observation across both proven complete family baselines.
    pub observed_at_ms: u64,
    /// Oldest active world, spawn and pace observation used by the snapshot.
    pub world_observed_at_ms: u64,
    pub baseline_complete: bool,
    pub level: u32,
    pub definitions: Vec<Definition>,
    pub spawns: Vec<Spawn>,
    pub progress: Vec<Payload>,
    pub pace: Vec<Pace>,
}
struct Unavailable;
impl Source for Unavailable {
    fn load_owned(
        &self,
        _: Uuid,
        _: Uuid,
        _: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Snapshot>> + Send + '_>> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
/// Flag for Game-backed Atlas reads. The browsing entry point is always mounted.
pub fn enabled() -> bool {
    std::env::var("OTERYN_ATLAS_BESTIARY_ENABLED").as_deref() == Ok("true")
}

pub fn configured_router(pool: sqlx::PgPool) -> Router {
    router(pool, enabled(), Arc::new(Unavailable))
}

#[derive(Clone)]
struct LandingState {
    pool: sqlx::PgPool,
    enabled: bool,
}

pub fn router(pool: sqlx::PgPool, enabled: bool, source: Arc<dyn Source>) -> Router {
    let state = LandingState {
        pool: pool.clone(),
        enabled,
    };
    let mut routes = Router::new()
        .route("/{lang}/atlas/bestiary", get(landing))
        .with_state(state);
    if enabled {
        routes = routes.merge(
            Router::new()
                .route("/{lang}/atlas/bestiary/{character}", get(page))
                .with_state(source),
        );
    }
    routes
        .layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(pool),
            oteryn_web::session::middleware,
        ))
        .layer(axum::middleware::from_fn(no_cache))
}

enum LandingContent {
    SignIn,
    Characters(Vec<(Uuid, String)>),
    Unavailable,
    JournalUnavailable,
}

#[derive(Template)]
#[template(path = "bestiary/landing.html", config = "src/bestiary/askama.toml")]
struct Landing<'a> {
    lang: &'a str,
    text: Text,
    view: LandingContent,
}

fn landing_view(lang: &str, view: LandingContent) -> Response {
    let code = match view {
        LandingContent::SignIn | LandingContent::Characters(_) => StatusCode::OK,
        LandingContent::Unavailable | LandingContent::JournalUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    };
    let result = Text::new(lang).and_then(|text| {
        Landing { lang, text, view }
            .render()
            .map_err(|_| Error::Unavailable)
    });
    match result {
        Ok(body) => (code, Html(body)).into_response(),
        Err(error) => status(error).into_response(),
    }
}

async fn landing(
    State(state): State<LandingState>,
    Path(lang): Path<String>,
    RawQuery(query): RawQuery,
    Extension(session): Extension<BrowserSession>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if query.is_some_and(|value| !value.is_empty()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(account) = session.account_id().await else {
        let view = if chrome.is_some_and(|Extension(chrome)| chrome.login_mounted) {
            LandingContent::SignIn
        } else {
            LandingContent::Unavailable
        };
        return landing_view(&lang, view);
    };
    if !state.enabled {
        return landing_view(&lang, LandingContent::Unavailable);
    }
    let Ok(tx) = state.pool.begin().await else {
        return landing_view(&lang, LandingContent::Unavailable);
    };
    match CollectionEvidence::characters(tx, account).await {
        Ok(characters) => landing_view(&lang, LandingContent::Characters(characters)),
        Err(oteryn_playerdata::store::Error::Denied) => StatusCode::FORBIDDEN.into_response(),
        Err(_) => landing_view(&lang, LandingContent::Unavailable),
    }
}

async fn no_cache(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
        .headers_mut()
        .insert(header::VARY, axum::http::HeaderValue::from_static("Cookie"));
    response
}
fn status(e: Error) -> StatusCode {
    match e {
        Error::Disabled => StatusCode::NOT_FOUND,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
async fn page(
    State(source): State<Arc<dyn Source>>,
    Path((lang, character)): Path<(String, Uuid)>,
    Query(filters): Query<Filters>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let now = match SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|v| u64::try_from(v.as_millis()).ok())
    {
        Some(now) => now,
        None => return unavailable(&lang),
    };
    match show(source.as_ref(), account, character, &lang, &filters, now).await {
        Ok(body) => Html(body).into_response(),
        Err(Error::Unavailable) => unavailable(&lang),
        Err(e) => status(e).into_response(),
    }
}

fn unavailable(lang: &str) -> Response {
    landing_view(lang, LandingContent::JournalUnavailable)
}

/// Reusable authenticated actor-to-result path; never accepts an account from a
/// query string. HTTP obtains it exclusively from BrowserSession.
pub async fn show(
    source: &dyn Source,
    account: Uuid,
    character: Uuid,
    lang: &str,
    filters: &Filters,
    now: u64,
) -> Result<String> {
    if !matches!(lang, "pl" | "en")
        || character.get_version_num() != 7
        || character.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Error::BadInput);
    }
    filters.validate()?;
    let s = source.load_owned(account, character, now).await?;
    if s.account != account || s.character != character {
        return Err(Error::Denied);
    }
    // Cooldowns and recommendations require fresh evidence. Do not silently
    // present stale progress as a current map, or unknown availability as ready.
    if s.world_version.len() != 40
        || !s
            .world_version
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Unavailable);
    }
    if !s.baseline_complete || !fresh(s.observed_at_ms, now) || !fresh(s.world_observed_at_ms, now)
    {
        return Err(Error::Unavailable);
    }
    let map = build(
        &s.definitions,
        &s.spawns,
        &s.progress,
        &s.pace,
        s.level,
        filters,
        now,
    )?;
    render(lang, map, s.world_version, filters, &s.definitions)
}
fn fresh(observed: u64, now: u64) -> bool {
    now.checked_sub(observed).is_some_and(|age| age <= 10_000)
}

#[derive(Template)]
#[template(path = "bestiary/page.html", config = "src/bestiary/askama.toml")]
struct Page {
    lang: String,
    version: String,
    text: Text,
    classes: Vec<(String, bool)>,
    difficulties: Vec<(String, bool)>,
    stars: Option<u8>,
    recommendations: Vec<(usize, String)>,
    rows: Vec<Row>,
}
struct Row {
    key: String,
    state: String,
    kills: u64,
    threshold: u64,
    points: u32,
    possible: u32,
    cooldown: String,
    stage: String,
    thresholds: String,
    colour: &'static str,
    points_on_map: Vec<Location>,
}
struct Location {
    x: u16,
    y: u16,
    floor: u8,
    label: String,
}
struct Text(FluentBundle<FluentResource>);

impl Text {
    fn new(lang: &str) -> Result<Self> {
        let source = match lang {
            "pl" => include_str!("pl.ftl"),
            "en" => include_str!("en.ftl"),
            _ => return Err(Error::BadInput),
        };
        let locale = lang.parse().map_err(|_| Error::BadInput)?;
        let resource = FluentResource::try_new(source.into()).map_err(|_| Error::Unavailable)?;
        let mut bundle = FluentBundle::new(vec![locale]);
        bundle
            .add_resource(resource)
            .map_err(|_| Error::Unavailable)?;
        Ok(Self(bundle))
    }

    fn get(&self, key: &str) -> std::result::Result<String, askama::Error> {
        self.0
            .get_message(key)
            .and_then(|m| m.value())
            .map(|value| self.0.format_pattern(value, None, &mut vec![]).into_owned())
            .ok_or(askama::Error::Fmt)
    }
}

fn options<'a>(
    values: impl Iterator<Item = &'a str>,
    selected: Option<&str>,
) -> Vec<(String, bool)> {
    values
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|value| (value.to_owned(), selected == Some(value)))
        .collect()
}

fn render(
    lang: &str,
    map: Map,
    version: String,
    filters: &Filters,
    definitions: &[Definition],
) -> Result<String> {
    let strings = Text::new(lang)?;
    let text = |key| strings.get(key).map_err(|_| Error::Unavailable);
    let row_index: BTreeMap<_, _> = map
        .entries
        .iter()
        .enumerate()
        .map(|(index, row)| (row.creature.as_str(), index))
        .collect();
    let recommendations = map
        .recommendations
        .iter()
        .filter_map(|key| Some((*row_index.get(key.as_str())?, key.clone())))
        .collect();
    let mut rows = Vec::new();
    for e in map.entries {
        let state = text(match e.state {
            ProgressState::NotStarted => "not-started",
            ProgressState::InProgress => "in-progress",
            ProgressState::Completed => "completed",
        })?;
        let cooldown = if !e.boss {
            String::new()
        } else {
            match e.cooldown_remaining_ms {
                Some(ms) => format!("{}: {}", text("cooldown")?, ms.div_ceil(60_000)),
                None => text("unknown-cooldown")?,
            }
        };
        let mut locations = Vec::new();
        for p in e.spawns {
            let chance = match p.reset_chance {
                Some(c) => format!("; {}: {}/{}", text("chance")?, c.numerator, c.denominator),
                None => String::new(),
            };
            locations.push(Location {
                x: p.cell.x,
                y: p.cell.y,
                floor: p.cell.floor,
                label: format!(
                    "{} ({}, {}, {}){}",
                    p.spawn, p.cell.x, p.cell.y, p.cell.floor, chance
                ),
            });
        }
        rows.push(Row {
            key: e.creature,
            state,
            kills: e.kills,
            threshold: e.thresholds[2],
            points: e.points,
            possible: e.possible_points,
            stage: if e.boss {
                text(match e.stage {
                    1 => "prowess",
                    2 => "expertise",
                    3 => "mastery",
                    _ => "not-started",
                })?
            } else {
                String::new()
            },
            thresholds: format!(
                "{}: {}, {}, {}",
                text("thresholds")?,
                e.thresholds[0],
                e.thresholds[1],
                e.thresholds[2]
            ),
            cooldown,
            colour: match e.state {
                ProgressState::NotStarted => "#b7c4d1",
                ProgressState::InProgress => "#ffdc78",
                ProgressState::Completed => "#93e5ab",
            },
            points_on_map: locations,
        });
    }
    Page {
        lang: lang.into(),
        version,
        text: strings,
        classes: options(
            definitions.iter().map(|d| d.class.as_str()),
            filters.class.as_deref(),
        ),
        difficulties: options(
            definitions.iter().map(|d| d.difficulty.as_str()),
            filters.difficulty.as_deref(),
        ),
        stars: filters.stars,
        recommendations,
        rows,
    }
    .render()
    .map_err(|_| Error::Unavailable)
}
