//! Delivery map presentation over the viewer's publication-gated tile route.
use std::{
    collections::{BTreeMap, BTreeSet},
    io::ErrorKind,
    path::Path,
};

use askama::Template;
use oteryn_world::{VersionStatus, WorldStore};
use tokio::{fs::File, io::AsyncReadExt};

use super::Delivery;
use crate::tiles::Manifest;

const MANIFEST_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Template)]
#[template(path = "overlay.html", config = "src/delivery/askama.toml")]
struct Overlay<'a> {
    title: &'a str,
    floor_label: &'a str,
    floors: Vec<Floor>,
}

struct Floor {
    number: u8,
    tile: String,
    width: u32,
    height: u32,
    markers: Vec<(f64, f64)>,
}

pub(super) async fn render(
    store: &WorldStore,
    directory: &Path,
    delivery: &Delivery,
    title: &str,
    floor_label: &str,
) -> Option<String> {
    let report = store.import_report(delivery.version).await.ok()??;
    let commit = &report.source_commit;
    if report.status != VersionStatus::Ready
        || !matches!(commit.len(), 40 | 64)
        || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    let mut points: BTreeMap<u8, BTreeSet<(u32, u32)>> = BTreeMap::new();
    for spawn in delivery.sources.iter().flat_map(|source| &source.spawns) {
        let (Ok(floor), Ok(x), Ok(y)) = (
            u8::try_from(spawn.floor),
            u32::try_from(spawn.x),
            u32::try_from(spawn.y),
        ) else {
            continue;
        };
        points.entry(floor).or_default().insert((x, y));
    }

    let mut floors = Vec::new();
    for (floor, points) in points {
        let path = directory
            .join(commit)
            .join(floor.to_string())
            .join("manifest.json");
        let file = match File::open(path).await {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => return None,
        };
        let mut bytes = Vec::new();
        file.take(MANIFEST_LIMIT + 1)
            .read_to_end(&mut bytes)
            .await
            .ok()?;
        if bytes.len() as u64 > MANIFEST_LIMIT {
            return None;
        }
        let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
        let overview = manifest.levels.first()?;
        let bounds = manifest.bounds;
        // The tile builder always fits its coarsest level in one padded tile.
        if manifest.format_version != 1
            || manifest.world_version != *commit
            || manifest.floor != floor
            || manifest.tile_size != 256
            || bounds.min_x >= bounds.max_x
            || bounds.min_y >= bounds.max_y
            || overview.z != 0
            || !(1..=256).contains(&overview.width)
            || !(1..=256).contains(&overview.height)
            || overview.columns != 1
            || overview.rows != 1
            || !manifest.tiles.iter().any(|tile| tile.path == "0/0/0.webp")
        {
            return None;
        }
        let x_scale = f64::from(overview.width) / f64::from(bounds.max_x - bounds.min_x);
        let y_scale = f64::from(overview.height) / f64::from(bounds.max_y - bounds.min_y);
        let markers: Vec<_> = points
            .into_iter()
            .filter(|(x, y)| {
                (bounds.min_x..bounds.max_x).contains(x) && (bounds.min_y..bounds.max_y).contains(y)
            })
            .map(|(x, y)| {
                (
                    f64::from(x - bounds.min_x) * x_scale,
                    f64::from(y - bounds.min_y) * y_scale,
                )
            })
            .collect();
        if !markers.is_empty() {
            floors.push(Floor {
                number: floor,
                tile: format!("/atlas/tiles/{commit}/{floor}/0/0/0.webp"),
                width: overview.width,
                height: overview.height,
                markers,
            });
        }
    }
    // An import can republish the same commit under a different world version.
    if floors.is_empty() || store.active_version_id().await.ok()? != Some(delivery.version) {
        return None;
    }
    Overlay {
        title,
        floor_label,
        floors,
    }
    .render()
    .ok()
}
