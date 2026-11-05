use super::{Error, Kind};
use image::{ExtendedColorType, ImageEncoder, ImageReader, codecs::webp::WebPEncoder};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::{Component, Path},
};

/// Byte-verified output of the public sprite/Atlas pipelines. The editorial
/// caller binds a creature appearance or map location to the answer's entity.
/// Filenames and manifest metadata are never sent to the player.
pub struct Media {
    pub(super) kind: Kind,
    pub(super) version: String,
    pub(super) bytes: Vec<u8>,
}
impl std::fmt::Debug for Media {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Media")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl Media {
    /// The manifest must belong to a successfully generated immutable sprite
    /// directory. Only creature thumbnails are accepted, converted to silhouettes
    /// before persistence so the original sprite cannot leak through CSS or URLs.
    pub fn monster(
        directory: &Path,
        manifest: &oteryn_world::sprites::Manifest,
        request: oteryn_world::sprites::Request,
    ) -> Result<Self, Error> {
        if manifest.format != 1 || request.kind != oteryn_world::sprites::Kind::Creature {
            return Err(Error::Invalid);
        }
        let matches: Vec<_> = manifest
            .thumbnails
            .iter()
            .filter(|item| item.request == request)
            .collect();
        if matches.len() != 1 {
            return Err(Error::Unavailable);
        }
        let item = matches[0];
        let mut image = decode(&read(directory, &item.file, &item.sha256)?)?;
        if image.width() != item.width || image.height() != item.height {
            return Err(Error::Unavailable);
        }
        for pixel in image.pixels_mut() {
            // A light silhouette is visible on the portal's dark surface.
            pixel.0[0..3].copy_from_slice(&[240, 240, 240]);
        }
        Ok(Self {
            kind: Kind::Monster,
            version: manifest.game_commit.clone(),
            bytes: encode(&image)?,
        })
    }

    /// Narrow seam for T40: only a tile present in a successful T40 manifest is
    /// accepted. No map locations or private markers are inferred here.
    pub fn atlas(
        directory: &Path,
        manifest: &oteryn_atlas::tiles::Manifest,
        tile_path: &str,
    ) -> Result<Self, Error> {
        if manifest.format_version != 1 || manifest.tile_size != 256 {
            return Err(Error::Invalid);
        }
        let matches: Vec<_> = manifest
            .tiles
            .iter()
            .filter(|tile| tile.path == tile_path)
            .collect();
        if matches.len() != 1 {
            return Err(Error::Unavailable);
        }
        let image = decode(&read(directory, tile_path, &matches[0].sha256)?)?;
        if image.width() != 256 || image.height() != 256 {
            return Err(Error::Unavailable);
        }
        Ok(Self {
            kind: Kind::Atlas,
            version: manifest.world_version.clone(),
            bytes: encode(&image)?,
        })
    }
}
fn read(root: &Path, file: &str, hash: &str) -> Result<Vec<u8>, Error> {
    let path = Path::new(file);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(Error::Invalid);
    }
    let root = root.canonicalize().map_err(|_| Error::Unavailable)?;
    let path = root
        .join(path)
        .canonicalize()
        .map_err(|_| Error::Unavailable)?;
    if !path.starts_with(root) {
        return Err(Error::Invalid);
    }
    let file = std::fs::File::open(path).map_err(|_| Error::Unavailable)?;
    if !file.metadata().map_err(|_| Error::Unavailable)?.is_file() {
        return Err(Error::Unavailable);
    }
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > 1_048_576 || format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
fn decode(bytes: &[u8]) -> Result<image::RgbaImage, Error> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::WebP);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(256);
    limits.max_image_height = Some(256);
    limits.max_alloc = Some(1_048_576);
    reader.limits(limits);
    reader
        .decode()
        .map(|image| image.to_rgba8())
        .map_err(|_| Error::Unavailable)
}
fn encode(image: &image::RgbaImage) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    WebPEncoder::new_lossless(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|_| Error::Unavailable)?;
    Ok(bytes)
}
