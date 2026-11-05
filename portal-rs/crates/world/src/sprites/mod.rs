//! Blocking offline worker function for a pinned release; format provenance is in SOURCE.md.
//! Items use phase zero, pattern (0,0,0): count one, empty fluid, no placement/hook.
//! Creatures use the first standing group, phase zero, south, no addons/mount or tint.
//! Colour masks are omitted, matching the game base-outfit reader. Activation is separate.
mod proto;
mod sheets;
use image::{ExtendedColorType, RgbaImage, codecs::webp::WebPEncoder, imageops};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sprite generation is disabled")]
    Disabled,
    #[error("invalid sprite input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
/// Off by default; callers explicitly enable the world sprite feature.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpriteFeature {
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Item,
    Creature,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Creature => "creature",
        }
    }
}
/// Appearance IDs, not server item IDs or entity keys: the importer supplies the mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Request {
    pub kind: Kind,
    pub appearance_id: u32,
    /// Explicit [x,y,z], or None for the canonical thumbnail described above.
    pub pattern: Option<[u32; 3]>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbnail {
    pub request: Request,
    pub pattern: [u32; 3],
    pub file: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub game_commit: String,
    /// Digest of each source actually read, relative to the assets directory.
    pub sources: BTreeMap<String, String>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug)]
pub struct Generated {
    /// Immutable output/game_commit/SHA256(manifest) directory.
    pub directory: PathBuf,
    pub manifest: Manifest,
    /// A complete, byte-verified artifact already existed; no files were rewritten.
    pub reused: bool,
}
#[derive(Debug, Deserialize)]
struct CatalogEntry {
    #[serde(rename = "type")]
    kind: String,
    file: Option<String>,
    firstspriteid: Option<u32>,
    lastspriteid: Option<u32>,
    spritetype: Option<u32>,
}
#[derive(Debug)]
struct Sheet {
    first: u32,
    last: u32,
    size: (u32, u32),
    file: String,
}
struct Assets<'a> {
    root: &'a Path,
    sources: BTreeMap<String, String>,
    sheets: Vec<Sheet>,
    // One decoded sheet bounds resident memory even for a complete release.
    resident: Option<(String, RgbaImage)>,
}
fn plain_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && name != "."
        && name != ".."
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("asset is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("asset exceeds size limit"));
    }
    Ok(bytes)
}
impl Assets<'_> {
    fn read(&mut self, file: &str, limit: u64) -> Result<Vec<u8>, Error> {
        if !plain_file(file) {
            return Err(invalid("unsafe asset file name"));
        }
        let root = self.root.canonicalize()?;
        let path = root.join(file).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(invalid("asset escapes release directory"));
        }
        let bytes = read_bounded(&path, limit)?;
        let hash = digest(&bytes);
        if self
            .sources
            .get(file)
            .is_some_and(|previous| previous != &hash)
        {
            return Err(invalid("release changed during generation"));
        }
        self.sources.insert(file.to_owned(), hash);
        Ok(bytes)
    }
    fn sprite(&mut self, id: u32) -> Result<RgbaImage, Error> {
        let sheet = self
            .sheets
            .iter()
            .find(|s| s.first <= id && id <= s.last)
            .ok_or_else(|| invalid(format!("missing sprite {id}")))?;
        let (first, (w, h), file) = (sheet.first, sheet.size, sheet.file.clone());
        if self.resident.as_ref().is_none_or(|(name, _)| name != &file) {
            let bytes = self.read(&file, 2 * 1024 * 1024)?;
            self.resident = Some((file, sheets::decode(&bytes)?));
        }
        let image = &self
            .resident
            .as_ref()
            .ok_or_else(|| invalid("missing decoded sheet"))?
            .1;
        let slot = id - first;
        Ok(imageops::crop_imm(image, slot % (384 / w) * w, slot / (384 / w) * h, w, h).to_image())
    }
}
fn catalog(bytes: &[u8]) -> Result<(String, Vec<Sheet>), Error> {
    let entries: Vec<CatalogEntry> = serde_json::from_slice(bytes)?;
    if entries.len() > 20_000 {
        return Err(invalid("catalog entry limit"));
    }
    let mut appearance = None;
    let mut sheets = Vec::new();
    for entry in entries {
        if entry.kind != "appearances" && entry.kind != "sprite" {
            continue;
        }
        let file = entry
            .file
            .filter(|s| plain_file(s))
            .ok_or_else(|| invalid("invalid catalog file"))?;
        if entry.kind == "appearances" {
            if !file.ends_with(".dat") || appearance.replace(file).is_some() {
                return Err(invalid("invalid or duplicate appearances entry"));
            }
            continue;
        }
        let (Some(first), Some(last), Some(kind)) =
            (entry.firstspriteid, entry.lastspriteid, entry.spritetype)
        else {
            return Err(invalid("incomplete sprite catalog entry"));
        };
        let (w, h) = sheets::geometry(kind)?;
        if !file.ends_with(".bmp.lzma")
            || last < first
            || u64::from(last) - u64::from(first) + 1 > u64::from((384 / w) * (384 / h))
        {
            return Err(invalid("invalid sprite range"));
        }
        sheets.push(Sheet {
            first,
            last,
            size: (w, h),
            file,
        });
    }
    sheets.sort_by_key(|s| s.first);
    if sheets.windows(2).any(|s| s[0].last >= s[1].first) || sheets.is_empty() {
        return Err(invalid("empty or overlapping sprite ranges"));
    }
    Ok((
        appearance.ok_or_else(|| invalid("missing appearances entry"))?,
        sheets,
    ))
}
fn index(entries: Vec<proto::Appearance>) -> Result<BTreeMap<u32, proto::Info>, Error> {
    if entries.len() > 100_000 {
        return Err(invalid("appearance count limit"));
    }
    let mut result = BTreeMap::new();
    for entry in entries {
        let id = entry
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| invalid("invalid appearance id"))?;
        if entry.groups.is_empty() || entry.groups.len() > 16 {
            return Err(invalid("invalid frame groups"));
        }
        for group in &entry.groups {
            group
                .info
                .as_ref()
                .ok_or_else(|| invalid("missing sprite info"))?
                .dimensions()?;
        }
        let info = entry
            .groups
            .into_iter()
            .next()
            .and_then(|g| g.info)
            .ok_or_else(|| invalid("missing first group"))?;
        if result.insert(id, info).is_some() {
            return Err(invalid("duplicate appearance id"));
        }
    }
    Ok(result)
}
fn render(
    assets: &mut Assets<'_>,
    request: Request,
    info: &proto::Info,
) -> Result<(RgbaImage, [u32; 3]), Error> {
    let [w, h, d, layers] = info.dimensions()?;
    let pattern = match request.pattern {
        Some(pattern) => pattern,
        None => match request.kind {
            Kind::Item => [0, 0, 0],
            Kind::Creature if w >= 4 => [2, 0, 0],
            Kind::Creature if w == 1 => [0, 0, 0],
            Kind::Creature => return Err(invalid("unsupported creature direction patterns")),
        },
    };
    let [x, y, z] = pattern;
    if x >= w || y >= h || z >= d {
        return Err(invalid("pattern outside appearance grid"));
    }
    let count = if request.kind == Kind::Creature {
        1
    } else {
        layers
    };
    let mut parts = Vec::new();
    for layer in 0..count {
        let at = (((z * h + y) * w + x) * layers + layer) as usize;
        let id = *info
            .sprites
            .get(at)
            .ok_or_else(|| invalid("missing sprite reference"))?;
        if id != 0 {
            parts.push(assets.sprite(id)?);
        }
    }
    let width = parts.iter().map(RgbaImage::width).max().unwrap_or(32);
    let height = parts.iter().map(RgbaImage::height).max().unwrap_or(32);
    let mut output = RgbaImage::new(width, height);
    for part in parts {
        composite(
            &mut output,
            &part,
            width - part.width(),
            height - part.height(),
        );
    }
    Ok((output, pattern))
}
// Integer source-over avoids floating point rounding an opaque alpha down to 254.
fn composite(target: &mut RgbaImage, source: &RgbaImage, x: u32, y: u32) {
    for (sx, sy, pixel) in source.enumerate_pixels() {
        let dst = target.get_pixel_mut(x + sx, y + sy);
        let sa = u32::from(pixel[3]);
        let da = u32::from(dst[3]);
        let alpha = sa * 255 + da * (255 - sa);
        if alpha == 0 {
            continue;
        }
        for channel in 0..3 {
            let value =
                u32::from(pixel[channel]) * sa * 255 + u32::from(dst[channel]) * da * (255 - sa);
            dst[channel] = ((value + alpha / 2) / alpha) as u8;
        }
        dst[3] = ((alpha + 127) / 255) as u8;
    }
}

/// Blocking worker entry point. `assets_dir` is the pinned release's `content/assets/files`.
/// It contains catalog-content.json, appearances and sheets. No fetching or database I/O.
/// Run on a blocking executor. The caller owns release authenticity and world activation.
/// A complete batch is published by directory rename; existing bytes are verified before
/// reuse. A failure leaves no newly published manifest or partially populated batch.
pub fn generate(
    feature: SpriteFeature,
    game_commit: &str,
    assets_dir: &Path,
    output_root: &Path,
    requests: &[Request],
) -> Result<Generated, Error> {
    if !feature.enabled {
        return Err(Error::Disabled);
    }
    if game_commit.len() != 40
        || !game_commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("game version must be a full lowercase commit SHA"));
    }
    if requests.is_empty() || requests.len() > 100_000 {
        return Err(invalid("invalid thumbnail request count"));
    }
    let ordered: BTreeSet<_> = requests.iter().copied().collect();
    if ordered.len() != requests.len() {
        return Err(invalid("duplicate thumbnail request"));
    }
    let mut assets = Assets {
        root: assets_dir,
        sources: BTreeMap::new(),
        sheets: Vec::new(),
        resident: None,
    };
    let bytes = assets.read("catalog-content.json", 8 * 1024 * 1024)?;
    let (appearance_file, sheets) = catalog(&bytes)?;
    assets.sheets = sheets;
    let bytes = assets.read(&appearance_file, 64 * 1024 * 1024)?;
    let appearances = proto::Appearances::decode(bytes.as_slice())
        .map_err(|e| invalid(format!("invalid appearances protobuf: {e}")))?;
    let items = index(appearances.objects)?;
    let creatures = index(appearances.outfits)?;
    let parent = output_root.join(game_commit);
    fs::create_dir_all(&parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".sprites-")
        .tempdir_in(&parent)?;
    let mut thumbnails = Vec::new();
    for request in ordered {
        let entries = match request.kind {
            Kind::Item => &items,
            Kind::Creature => &creatures,
        };
        let info = entries
            .get(&request.appearance_id)
            .ok_or_else(|| invalid("unknown appearance"))?;
        let (image, pattern) = render(&mut assets, request, info)?;
        let mut bytes = Vec::new();
        WebPEncoder::new_lossless(&mut bytes).encode(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )?;
        let hash = digest(&bytes);
        let [x, y, z] = pattern;
        let file = format!(
            "{}-{}-{x}-{y}-{z}-{hash}.webp",
            request.kind.label(),
            request.appearance_id
        );
        // Canonical and explicit requests can select identical pixels and file names.
        if !stage.path().join(&file).exists() {
            write_new(&stage.path().join(&file), &bytes)?;
        }
        thumbnails.push(Thumbnail {
            request,
            pattern,
            file,
            sha256: hash,
            width: image.width(),
            height: image.height(),
        });
    }
    let manifest = Manifest {
        format: 1,
        game_commit: game_commit.to_owned(),
        sources: assets.sources,
        thumbnails,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let directory = parent.join(digest(&bytes));
    write_new(&stage.path().join("manifest.json"), &bytes)?;
    let verify = || -> Result<(), Error> {
        if read_bounded(&directory.join("manifest.json"), 64 * 1024 * 1024)? != bytes {
            return Err(invalid("existing manifest mismatch"));
        }
        for thumbnail in &manifest.thumbnails {
            if digest(&read_bounded(
                &directory.join(&thumbnail.file),
                1024 * 1024,
            )?) != thumbnail.sha256
            {
                return Err(invalid("existing thumbnail hash mismatch"));
            }
        }
        Ok(())
    };
    let reused = if directory.exists() {
        verify()?;
        true
    } else {
        match fs::rename(stage.path(), &directory) {
            Ok(()) => false,
            Err(_) if directory.exists() => {
                verify()?;
                true
            }
            Err(error) => return Err(error.into()),
        }
    };
    Ok(Generated {
        directory,
        manifest,
        reused,
    })
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
#[cfg(test)]
mod tests;
