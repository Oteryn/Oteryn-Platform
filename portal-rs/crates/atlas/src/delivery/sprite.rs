//! Resolve a reviewed item mapping to a digest-addressed T42 public thumbnail.
use oteryn_world::sprites::{Kind, Manifest, Request};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::AsyncReadExt;

pub(super) const KIND: &str = "Item.Sprite";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mapping {
    appearance_id: u32,
    manifest_sha256: String,
}

fn hash(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

async fn read(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let file = tokio::fs::File::open(path).await.ok()?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).await.ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

pub(super) async fn url(static_dir: &Path, commit: &str, mapping: &Mapping) -> Option<String> {
    let manifest_hash = &mapping.manifest_sha256;
    if !hash(commit, 40) || !hash(manifest_hash, 64) || mapping.appearance_id == 0 {
        return None;
    }
    let relative = format!("world-sprites/{commit}/{manifest_hash}");
    let directory = static_dir.join(&relative);
    let bytes = read(&directory.join("manifest.json"), 8 * 1024 * 1024).await?;
    if format!("{:x}", Sha256::digest(&bytes)) != *manifest_hash {
        return None;
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    if manifest.format != 1 || manifest.game_commit != commit {
        return None;
    }
    let request = Request {
        kind: Kind::Item,
        appearance_id: mapping.appearance_id,
        pattern: None,
    };
    let mut matches = manifest
        .thumbnails
        .iter()
        .filter(|image| image.request == request);
    let image = matches.next()?;
    if matches.next().is_some() || !hash(&image.sha256, 64) {
        return None;
    }
    let [x, y, z] = image.pattern;
    let file = format!(
        "item-{}-{x}-{y}-{z}-{}.webp",
        request.appearance_id, image.sha256
    );
    if image.file != file {
        return None;
    }
    let bytes = read(&directory.join(&file), 1024 * 1024).await?;
    (format!("{:x}", Sha256::digest(bytes)) == image.sha256)
        .then(|| format!("/static/{relative}/{file}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sprite_requires_matching_world_manifest_and_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let commit = "a".repeat(40);
        let bytes = b"example thumbnail bytes";
        let sha = format!("{:x}", Sha256::digest(bytes));
        let file = format!("item-7-0-0-0-{sha}.webp");
        let manifest = serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "game_commit": commit,
            "sources": {},
            "thumbnails": [{
                "request": {"kind": "item", "appearance_id": 7, "pattern": null},
                "pattern": [0, 0, 0], "file": file, "sha256": sha, "width": 32, "height": 32
            }],
        }))?;
        let mapping = Mapping {
            appearance_id: 7,
            manifest_sha256: format!("{:x}", Sha256::digest(&manifest)),
        };
        let dir = root
            .path()
            .join("world-sprites")
            .join(&commit)
            .join(&mapping.manifest_sha256);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("manifest.json"), manifest)?;
        std::fs::write(dir.join(&file), bytes)?;
        assert!(
            url(root.path(), &commit, &mapping)
                .await
                .is_some_and(|s| s.ends_with(&file))
        );
        assert!(url(root.path(), &"b".repeat(40), &mapping).await.is_none());
        std::fs::write(dir.join(&file), b"changed")?;
        assert!(url(root.path(), &commit, &mapping).await.is_none());
        Ok(())
    }
}
