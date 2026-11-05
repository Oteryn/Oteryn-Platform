use super::*;
use oteryn_world::sprites::{Kind, Manifest};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Component, Path},
};
/// Verified, bounded thumbnail bytes from T42; never a browser URL or filesystem path.
#[derive(Debug)]
pub struct Sprite {
    pub(super) bytes: Vec<u8>,
    pub(super) commit: String,
    pub(super) appearance: u32,
}
impl Sprite {
    pub fn load(root: &Path, manifest: &Manifest, appearance: u32) -> Result<Self> {
        if manifest.format != 1 || appearance == 0 {
            return Err(Error::BadInput);
        }
        let entries: Vec<_> = manifest
            .thumbnails
            .iter()
            .filter(|t| {
                t.request.kind == Kind::Creature
                    && t.request.appearance_id == appearance
                    && t.request.pattern.is_none()
            })
            .collect();
        if entries.len() != 1 {
            return Err(Error::Unavailable);
        }
        let item = entries[0];
        let file = Path::new(&item.file);
        if file.as_os_str().is_empty()
            || file
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || item.width == 0
            || item.height == 0
            || item.width > 512
            || item.height > 512
        {
            return Err(Error::BadInput);
        }
        let root = root.canonicalize().map_err(|_| Error::Unavailable)?;
        let path = root
            .join(file)
            .canonicalize()
            .map_err(|_| Error::Unavailable)?;
        if !path.starts_with(&root) {
            return Err(Error::Denied);
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| Error::Unavailable)?
            .take(1048577)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Unavailable)?;
        if bytes.len() > 1048576 || format!("{:x}", Sha256::digest(&bytes)) != item.sha256 {
            return Err(Error::Unavailable);
        }
        // Read dimensions before allocating an image. T42 emits WebP thumbnails.
        let reader =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::WebP);
        let dimensions = reader.into_dimensions().map_err(|_| Error::BadInput)?;
        if dimensions != (item.width, item.height) {
            return Err(Error::BadInput);
        }
        Ok(Self {
            bytes,
            commit: manifest.game_commit.clone(),
            appearance,
        })
    }
}
