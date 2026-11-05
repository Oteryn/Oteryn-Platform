//! Public atlas viewer. Only active, successfully built world versions are served.
use std::{path::PathBuf, sync::Arc};

use crate::tiles::{Manifest, NpcMarker};
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_world::{WorldOptions, WorldStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct Viewer {
    pub enabled: bool,
    pub directory: PathBuf,
    pub world: WorldStore,
}

/// Requires explicit enablement and a private T40 output directory, outside static/.
pub fn configured_router(pool: sqlx::PgPool) -> Router {
    router(Viewer {
        enabled: std::env::var("OTERYN_ATLAS_ENABLED").is_ok_and(|v| v == "true"),
        directory: std::env::var_os("OTERYN_ATLAS_DIRECTORY")
            .map(PathBuf::from)
            .unwrap_or_default(),
        world: WorldStore::new(
            pool,
            WorldOptions {
                enabled: std::env::var("OTERYN_WORLD_ENABLED").is_ok_and(|v| v == "true"),
            },
        ),
    })
}

pub fn router(viewer: Viewer) -> Router {
    if !viewer.enabled || viewer.directory.as_os_str().is_empty() {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/atlas", get(page))
        .route("/atlas/tiles/{version}/{floor}/{zoom}/{x}/{y}", get(tile))
        .with_state(Arc::new(viewer))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coordinates {
    x: Option<u32>,
    y: Option<u32>,
    z: Option<u8>,
    zoom: Option<u32>,
}

impl Coordinates {
    fn resolve(&self, m: &Manifest) -> Result<(u32, u32, u32), StatusCode> {
        if self.x.is_some() != self.y.is_some() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let x = self
            .x
            .unwrap_or(m.bounds.min_x + (m.bounds.max_x - m.bounds.min_x) / 2);
        let y = self
            .y
            .unwrap_or(m.bounds.min_y + (m.bounds.max_y - m.bounds.min_y) / 2);
        let zoom = self
            .zoom
            .unwrap_or_else(|| m.levels.last().map_or(0, |l| l.z));
        if x < m.bounds.min_x
            || x >= m.bounds.max_x
            || y < m.bounds.min_y
            || y >= m.bounds.max_y
            || !m.levels.iter().any(|l| l.z == zoom)
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok((x, y, zoom))
    }
}

async fn version(v: &Viewer) -> Result<String, StatusCode> {
    let id = v
        .world
        .active_version_id()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let report = v
        .world
        .import_report(id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if report.status != oteryn_world::VersionStatus::Ready {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(report.source_commit)
}
async fn manifest(v: &Viewer, version: &str, floor: u8) -> Option<Manifest> {
    let bytes = tokio::fs::read(
        v.directory
            .join(version)
            .join(floor.to_string())
            .join("manifest.json"),
    )
    .await
    .ok()?;
    if bytes.len() > 8 * 1024 * 1024 {
        return None;
    }
    let m: Manifest = serde_json::from_slice(&bytes).ok()?;
    (m.format_version == 1
        && m.world_version == version
        && m.floor == floor
        && m.tile_size == 256
        && m.bounds.max_x > m.bounds.min_x
        && m.bounds.max_y > m.bounds.min_y
        && !m.levels.is_empty())
    .then_some(m)
}

#[derive(Serialize)]
struct MapData {
    manifests: Vec<Manifest>,
    markers: Vec<NpcMarker>,
    x: u32,
    y: u32,
    floor: u8,
    zoom: u32,
}

#[derive(Template)]
#[template(path = "atlas/page.html", config = "src/web-askama.toml")]
struct Page {
    lang: String,
    title: String,
    help: String,
    floor: String,
    zoom_in: String,
    zoom_out: String,
    north: String,
    south: String,
    west: String,
    east: String,
    map: String,
    markers: String,
    missing: String,
    data: String,
    nonce: String,
}

async fn page(
    State(v): State<Arc<Viewer>>,
    Path(lang): Path<String>,
    Query(coords): Query<Coordinates>,
) -> Result<Response, StatusCode> {
    if lang != "pl" && lang != "en" {
        return Err(StatusCode::NOT_FOUND);
    }
    let version = version(&v).await?;
    let mut manifests = Vec::new();
    for floor in 0..=u8::MAX {
        if let Some(m) = manifest(&v, &version, floor).await {
            manifests.push(m);
        }
    }
    let m = manifests
        .iter()
        .find(|m| m.floor == coords.z.unwrap_or(7))
        .or_else(|| {
            if coords.z.is_none() {
                manifests.first()
            } else {
                None
            }
        })
        .ok_or(StatusCode::NOT_FOUND)?;
    let (x, y, zoom) = coords.resolve(m)?;
    let floor = m.floor;
    let mut markers = Vec::new();
    // Both placement and target NPC must pass T14 visibility filtering.
    let mut after = None;
    loop {
        let placements = v
            .world
            .list_by_kind("npc-placements", after.as_deref(), 1000)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let count = placements.len();
        for placement in placements {
            after = Some(placement.key.clone());
            let Some(marker) = placement_marker(&placement.data) else {
                continue;
            };
            if v.world
                .get_entity("npcs", &marker.entity_key)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .is_some_and(|e| e.version_id == placement.version_id)
                && manifests.iter().any(|m| {
                    m.floor == marker.floor
                        && marker.x >= m.bounds.min_x
                        && marker.x < m.bounds.max_x
                        && marker.y >= m.bounds.min_y
                        && marker.y < m.bounds.max_y
                })
            {
                markers.push(marker);
            }
        }
        if count < 1000 {
            break;
        }
    }
    // Reject a version switch during reads; never combine world snapshots.
    if crate::web::version(&v).await? != version {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let data = serde_json::to_string(&MapData {
        manifests,
        markers,
        x,
        y,
        floor,
        zoom,
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let text = |key| translate(&lang, key);
    let template = Page {
        lang: lang.clone(),
        title: text("title"),
        help: text("help"),
        floor: text("floor"),
        zoom_in: text("zoom-in"),
        zoom_out: text("zoom-out"),
        north: text("north"),
        south: text("south"),
        west: text("west"),
        east: text("east"),
        map: text("map"),
        markers: text("markers"),
        missing: text("missing"),
        data,
        nonce: nonce.clone(),
    };
    let mut response = Html(
        template
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response();
    let csp = format!(
        "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        csp.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}

async fn tile(
    State(v): State<Arc<Viewer>>,
    Path((requested, floor, zoom, x, y)): Path<(String, u8, u32, u32, String)>,
) -> Result<Response, StatusCode> {
    let version = version(&v).await?;
    if requested != version {
        return Err(StatusCode::NOT_FOUND);
    }
    let y = y
        .strip_suffix(".webp")
        .and_then(|y| y.parse::<u32>().ok())
        .ok_or(StatusCode::NOT_FOUND)?;
    let m = manifest(&v, &version, floor)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let path = format!("{zoom}/{x}/{y}.webp");
    let t = m
        .tiles
        .iter()
        .find(|t| t.path == path)
        .ok_or(StatusCode::NOT_FOUND)?;
    let bytes = tokio::fs::read(
        v.directory
            .join(&version)
            .join(floor.to_string())
            .join(path),
    )
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;
    if format!("{:x}", Sha256::digest(&bytes)) != t.sha256 {
        return Err(StatusCode::NOT_FOUND);
    }
    if crate::web::version(&v).await? != version {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok((
        [
            (header::CONTENT_TYPE, "image/webp"),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}

// SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
// content/world/npc-placements/placements-00000-00032.json (declaration shape).
fn placement_marker(data: &serde_json::Value) -> Option<NpcMarker> {
    let declaration = data.get("declaration")?;
    let cell = declaration.get("cell")?;
    Some(NpcMarker {
        entity_key: declaration.get("npc")?.as_str()?.to_owned(),
        x: u32::try_from(cell.get("x")?.as_u64()?).ok()?,
        y: u32::try_from(cell.get("y")?.as_u64()?).ok()?,
        floor: u8::try_from(cell.get("floor")?.as_u64()?).ok()?,
    })
}

fn translate(lang: &str, key: &str) -> String {
    let source = if lang == "pl" {
        include_str!("../../../templates/atlas/pl.ftl")
    } else {
        include_str!("../../../templates/atlas/en.ftl")
    };
    let locale = if lang == "pl" {
        unic_langid::langid!("pl")
    } else {
        unic_langid::langid!("en")
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.into())
        && bundle.add_resource(resource).is_ok()
        && let Some(pattern) = bundle.get_message(key).and_then(|m| m.value())
    {
        return bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned();
    }
    String::new()
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
