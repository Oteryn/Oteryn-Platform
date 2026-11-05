//! SSR surface. A T72 adapter is required before private data can be served.
use super::*;
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
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
/// Default-off registration. T72 has no configured trusted browser authority yet;
/// enablement alone cannot manufacture an ownership or freshness proof.
pub fn configured_router(pool: sqlx::PgPool) -> Router {
    router(
        pool,
        std::env::var("OTERYN_ATLAS_BESTIARY_ENABLED").as_deref() == Ok("true"),
        Arc::new(Unavailable),
    )
}
pub fn router(pool: sqlx::PgPool, enabled: bool, source: Arc<dyn Source>) -> Router {
    if !enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/atlas/bestiary/{character}", get(page))
        .with_state(source)
        .layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(pool),
            oteryn_web::session::middleware,
        ))
        .layer(axum::middleware::from_fn(no_cache))
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
        None => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    match show(source.as_ref(), account, character, &lang, &filters, now).await {
        Ok(body) => Html(body).into_response(),
        Err(e) => status(e).into_response(),
    }
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
    if !matches!(lang, "pl" | "en") || character.get_version_num() != 7 {
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
    if !s.baseline_complete || s.observed_at_ms > now || now - s.observed_at_ms > 10_000 {
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
    render(lang, map, s.world_version)
}
#[derive(Template)]
#[template(path = "bestiary/page.html", config = "src/bestiary/askama.toml")]
struct Page {
    lang: String,
    version: String,
    version_label: String,
    title: String,
    notice: String,
    class_label: String,
    difficulty_label: String,
    stars_label: String,
    filter_label: String,
    recommendations_label: String,
    locations_label: String,
    points_label: String,
    recommendations: Vec<String>,
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
fn wrap(s: &str) -> String {
    s.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 16 == 0 {
                vec!['\u{200b}', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
fn render(lang: &str, map: Map, version: String) -> Result<String> {
    let locale = lang.parse().map_err(|_| Error::BadInput)?;
    let resource = FluentResource::try_new(
        if lang == "pl" {
            include_str!("pl.ftl")
        } else {
            include_str!("en.ftl")
        }
        .into(),
    )
    .map_err(|_| Error::Unavailable)?;
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle
        .add_resource(resource)
        .map_err(|_| Error::Unavailable)?;
    let text = |key: &str| -> Result<String> {
        let pattern = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or(Error::Unavailable)?;
        let mut errors = Vec::new();
        let s = bundle
            .format_pattern(pattern, None, &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(s)
        } else {
            Err(Error::Unavailable)
        }
    };
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
                Some(c) => format!("; {}: {}%", text("chance")?, c * 100.0),
                None => String::new(),
            };
            locations.push(Location {
                x: p.cell.x,
                y: p.cell.y,
                floor: p.cell.floor,
                label: format!(
                    "{} ({}, {}, {}){}",
                    wrap(&p.spawn),
                    p.cell.x,
                    p.cell.y,
                    p.cell.floor,
                    chance
                ),
            });
        }
        rows.push(Row {
            key: wrap(&e.creature),
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
        version: wrap(&version),
        version_label: text("version")?,
        title: text("title")?,
        notice: text("notice")?,
        class_label: text("class")?,
        difficulty_label: text("difficulty")?,
        stars_label: text("stars")?,
        filter_label: text("filter")?,
        recommendations_label: text("recommendations")?,
        locations_label: text("locations")?,
        points_label: text("points")?,
        recommendations: map.recommendations.iter().map(|k| wrap(k)).collect(),
        rows,
    }
    .render()
    .map_err(|_| Error::Unavailable)
}
