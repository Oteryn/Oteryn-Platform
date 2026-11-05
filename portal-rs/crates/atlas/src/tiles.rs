//! Offline minimap pipeline. Call [`build`] from a worker with a pinned world version.
//!
//! Input is a directory of `minimap-32-<x>-<y>-<floor>-<hash>.bmp.lzma`
//! chunks. Coordinates are native map pixels. Output is
//! `<version>/<floor>/<z>/<x>/<y>.webp`, with local tile coordinates and z=0
//! the smallest level. Edge tiles are transparently padded to 256 square pixels.
//! Only manifests published by a successful build should be activated by the caller.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
};

use image::{ImageEncoder, ImageFormat, ImageReader, RgbaImage, imageops};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const TILE_SIZE: u32 = 256;
const MAX_BYTES: usize = 256 * 1024 * 1024;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Explicit runtime feature flag; no input or output is accessed when disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct TilePipeline {
    pub enabled: bool,
}

/// A placement adapter can be implemented by the world import coordinator later.
/// This pipeline does not infer or publish placements without that adapter.
pub trait MarkerSource {
    fn npc_placements(&self, world_version: &str) -> Result<Vec<NpcMarker>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcMarker {
    pub entity_key: String,
    pub x: u32,
    pub y: u32,
    pub floor: u8,
}

/// Native pixel bounds; maxima are exclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub z: u32,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub world_version: String,
    pub floor: u8,
    pub tile_size: u32,
    pub bounds: Bounds,
    pub levels: Vec<Level>,
    pub tiles: Vec<Tile>,
}

#[derive(Debug, Default)]
pub struct BuildReport {
    pub manifests: Vec<Manifest>,
    pub files_written: usize,
    pub files_unchanged: usize,
}

#[derive(Debug)]
struct Chunk {
    x: u32,
    y: u32,
    image: RgbaImage,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Decode and render all floors in deterministic order. The caller owns exclusive
/// access to this version's output directory until the import is activated.
/// No database, network access or world activation occurs here.
pub fn build(
    config: TilePipeline,
    input: &Path,
    output: &Path,
    world_version: &str,
) -> Result<BuildReport> {
    if !config.enabled {
        return Err(
            io::Error::new(io::ErrorKind::PermissionDenied, "atlas tiles are disabled").into(),
        );
    }
    if world_version.is_empty()
        || world_version.len() > 128
        || !world_version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("invalid world version path component").into());
    }
    let mut paths = fs::read_dir(input)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort();
    let mut floors: BTreeMap<u8, Vec<Chunk>> = BTreeMap::new();
    let mut total_pixels = 0_u64;
    let mut chunk_count = 0_usize;
    for path in paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("minimap-") {
            continue;
        }
        let (x, y, floor) = parse_name(name)?;
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(invalid("minimap input must be a regular file").into());
        }
        chunk_count += 1;
        if chunk_count > 4096 {
            return Err(invalid("input exceeds chunk limit").into());
        }
        let image = decode(&path)?;
        total_pixels += u64::from(image.width()) * u64::from(image.height());
        if total_pixels > MAX_PIXELS {
            return Err(invalid("input exceeds pixel limit").into());
        }
        floors.entry(floor).or_default().push(Chunk { x, y, image });
    }
    if floors.is_empty() {
        return Err(invalid("no minimap chunks found").into());
    }
    // Validate every floor before writing any output.
    let bounds = floors
        .iter()
        .map(|(&floor, chunks)| Ok((floor, floor_bounds(chunks)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut report = BuildReport::default();
    for (floor, chunks) in floors {
        let bounds = bounds
            .get(&floor)
            .ok_or_else(|| invalid("missing floor bounds"))?
            .clone();
        let mut map = RgbaImage::new(bounds.max_x - bounds.min_x, bounds.max_y - bounds.min_y);
        for chunk in chunks {
            imageops::replace(
                &mut map,
                &chunk.image,
                i64::from(chunk.x - bounds.min_x),
                i64::from(chunk.y - bounds.min_y),
            );
        }
        let directory = output.join(world_version).join(floor.to_string());
        let mut pyramid = vec![map];
        while let Some(previous) = pyramid.last() {
            let (w, h) = previous.dimensions();
            if w <= TILE_SIZE && h <= TILE_SIZE {
                break;
            }
            pyramid.push(imageops::resize(
                previous,
                w.div_ceil(2),
                h.div_ceil(2),
                imageops::FilterType::Triangle,
            ));
        }
        let mut manifest = Manifest {
            format_version: 1,
            world_version: world_version.to_owned(),
            floor,
            tile_size: TILE_SIZE,
            bounds,
            levels: Vec::new(),
            tiles: Vec::new(),
        };
        for (z, map) in pyramid.iter().rev().enumerate() {
            let level = Level {
                z: z as u32,
                width: map.width(),
                height: map.height(),
                columns: map.width().div_ceil(TILE_SIZE),
                rows: map.height().div_ceil(TILE_SIZE),
            };
            for x in 0..level.columns {
                for y in 0..level.rows {
                    let crop = imageops::crop_imm(
                        map,
                        x * TILE_SIZE,
                        y * TILE_SIZE,
                        TILE_SIZE.min(map.width() - x * TILE_SIZE),
                        TILE_SIZE.min(map.height() - y * TILE_SIZE),
                    )
                    .to_image();
                    let mut tile = RgbaImage::new(TILE_SIZE, TILE_SIZE);
                    imageops::replace(&mut tile, &crop, 0, 0);
                    let mut bytes = Vec::new();
                    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes).write_image(
                        tile.as_raw(),
                        TILE_SIZE,
                        TILE_SIZE,
                        image::ExtendedColorType::Rgba8,
                    )?;
                    let path = format!("{z}/{x}/{y}.webp");
                    write_changed(&directory.join(&path), &bytes, &mut report)?;
                    manifest.tiles.push(Tile {
                        path,
                        sha256: hash(&bytes),
                    });
                }
            }
            manifest.levels.push(level);
        }
        write_changed(
            &directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)?,
            &mut report,
        )?;
        report.manifests.push(manifest);
    }
    Ok(report)
}

fn parse_name(name: &str) -> Result<(u32, u32, u8)> {
    let stem = name
        .strip_prefix("minimap-32-")
        .and_then(|name| name.strip_suffix(".bmp.lzma"))
        .ok_or_else(|| invalid("invalid minimap filename"))?;
    let parts: Vec<_> = stem.split('-').collect();
    if parts.len() != 4
        || ![40, 64].contains(&parts[3].len())
        || !parts[3].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(invalid("invalid minimap coordinates or hash").into());
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

fn floor_bounds(chunks: &[Chunk]) -> Result<Bounds> {
    let mut bounds = Bounds {
        min_x: u32::MAX,
        min_y: u32::MAX,
        max_x: 0,
        max_y: 0,
    };
    for (i, chunk) in chunks.iter().enumerate() {
        let max_x = chunk
            .x
            .checked_add(chunk.image.width())
            .ok_or_else(|| invalid("map coordinate overflow"))?;
        let max_y = chunk
            .y
            .checked_add(chunk.image.height())
            .ok_or_else(|| invalid("map coordinate overflow"))?;
        for other in &chunks[..i] {
            if u64::from(chunk.x) < u64::from(other.x) + u64::from(other.image.width())
                && u64::from(other.x) < u64::from(max_x)
                && u64::from(chunk.y) < u64::from(other.y) + u64::from(other.image.height())
                && u64::from(other.y) < u64::from(max_y)
            {
                return Err(invalid("overlapping minimap chunks").into());
            }
        }
        bounds.min_x = bounds.min_x.min(chunk.x);
        bounds.min_y = bounds.min_y.min(chunk.y);
        bounds.max_x = bounds.max_x.max(max_x);
        bounds.max_y = bounds.max_y.max(max_y);
    }
    if u64::from(bounds.max_x - bounds.min_x) * u64::from(bounds.max_y - bounds.min_y) > MAX_PIXELS
    {
        return Err(invalid("floor exceeds pixel limit").into());
    }
    Ok(bounds)
}

struct BoundedBuffer(Vec<u8>);
impl Write for BoundedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(invalid("decoded BMP exceeds byte limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn decode(path: &Path) -> Result<RgbaImage> {
    let mut compressed = Vec::new();
    fs::File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut compressed)?;
    if compressed.len() > MAX_BYTES {
        return Err(invalid("compressed minimap exceeds byte limit").into());
    }
    let mut bmp = BoundedBuffer(Vec::new());
    lzma_rs::lzma_decompress_with_options(
        &mut Cursor::new(compressed),
        &mut bmp,
        &lzma_rs::decompress::Options {
            memlimit: Some(MAX_BYTES),
            ..Default::default()
        },
    )?;
    let mut reader = ImageReader::with_format(Cursor::new(bmp.0), ImageFormat::Bmp);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_PIXELS * 4);
    limits.max_image_width = Some(MAX_PIXELS as u32);
    limits.max_image_height = Some(MAX_PIXELS as u32);
    reader.limits(limits);
    let image = reader.decode()?.to_rgba8();
    if image.width() == 0
        || image.height() == 0
        || u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS
    {
        return Err(invalid("invalid minimap dimensions").into());
    }
    Ok(image)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_changed(path: &Path, bytes: &[u8], report: &mut BuildReport) -> Result<()> {
    match fs::read(path) {
        Ok(existing) if hash(&existing) == hash(bytes) => {
            report.files_unchanged += 1;
            return Ok(());
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("output path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary: PathBuf = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    report.files_written += 1;
    Ok(())
}

#[cfg(test)]
#[path = "tiles/tests.rs"]
mod tests;
