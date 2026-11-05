//! Reusable recursive golden-vector loader. Schemas, manifests and intentionally
//! semantically invalid examples are included; round-trip is not domain validity.
use crate::canonical::{self, Value};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub struct Fixture {
    pub path: PathBuf,
    pub original: Vec<u8>,
    pub canonical: Vec<u8>,
    pub value: Value,
    pub was_canonical: bool,
}
/// Pretty review artifacts compact in file member order. Already compact files
/// must round-trip exactly; no malformed JSON file is silently skipped.
pub fn load(root: impl AsRef<Path>) -> Result<Vec<Fixture>, Box<dyn Error>> {
    fn collect(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                collect(&entry.path(), files)?;
            } else if entry.path().extension().is_some_and(|e| e == "json") {
                files.push(entry.path());
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    collect(root.as_ref(), &mut paths)?;
    paths.sort();
    let mut fixtures = Vec::new();
    for path in paths {
        let original = fs::read(&path)?;
        let value = canonical::decode(&original, 16 * 1024 * 1024, 128)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let canonical = canonical::encode(&value)?;
        let was_canonical = canonical == original;
        let mut in_string = false;
        let mut escaped = false;
        let pretty = original.iter().any(|&byte| {
            if escaped {
                escaped = false;
                return false;
            }
            if in_string && byte == b'\\' {
                escaped = true;
                return false;
            }
            if byte == b'"' {
                in_string = !in_string;
                return false;
            }
            !in_string && byte.is_ascii_whitespace()
        });
        if !pretty && !was_canonical {
            return Err(format!("{}: compact fixture is not canonical", path.display()).into());
        }
        let round_trip = canonical::decode(&canonical, 16 * 1024 * 1024, 128)?;
        if canonical::encode(&round_trip)? != canonical {
            return Err(format!("{}: canonical round-trip differs", path.display()).into());
        }
        fixtures.push(Fixture {
            path,
            original,
            canonical,
            value,
            was_canonical,
        });
    }
    Ok(fixtures)
}
