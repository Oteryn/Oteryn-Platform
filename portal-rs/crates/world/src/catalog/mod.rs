//! Verified CATALOG_EXPORT_V1 read cache with independently gated public projection.
mod cache;
mod json;
mod publication;
mod refresh;
mod transport;
pub use cache::CachedEntity;
pub use publication::CatalogStore;
pub use refresh::{RetryPolicy, refresh_source_in_background};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

const CAPABILITIES: [&str; 8] = [
    "achievement",
    "creature",
    "creature_loot",
    "item",
    "npc",
    "npc_shop",
    "quest",
    "spell",
];
const MAX_FILE_BYTES: usize = 268_435_456;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("INVALID_INPUT")]
    InvalidInput,
    #[error("CAPACITY_EXCEEDED")]
    CapacityExceeded,
    #[error("UNSUPPORTED_CAPABILITY")]
    UnsupportedCapability,
    #[error("INTEGRITY_MISMATCH")]
    IntegrityMismatch,
    #[error("PROVENANCE_CONFLICT")]
    ProvenanceConflict,
    #[error("INVALID_INPUT: loot entity/relation publication key collision")]
    PublicationKeyCollision,
    #[error("catalog source unavailable")]
    Unavailable,
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    capability_id: String,
    support: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Completeness {
    capability_id: String,
    state: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entity {
    r#type: String,
    content_key: String,
    capability_id: String,
    data: Value,
    #[serde(
        default,
        deserialize_with = "public_visibility",
        skip_serializing_if = "Option::is_none"
    )]
    visibility: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Relation {
    r#type: String,
    relation_key: String,
    capability_id: String,
    source: String,
    #[serde(deserialize_with = "nullable_target")]
    target: Option<String>,
    data: Value,
    #[serde(
        default,
        deserialize_with = "public_visibility",
        skip_serializing_if = "Option::is_none"
    )]
    visibility: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Tombstone {
    content_key: String,
    capability_id: String,
    reason: String,
    #[serde(
        default,
        deserialize_with = "public_visibility",
        skip_serializing_if = "Option::is_none"
    )]
    visibility: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    contract_id: String,
    schema_version: String,
    snapshot_id: String,
    content_authority_id: String,
    authority_epoch: String,
    source_revision: String,
    generated_at: String,
    ruleset_id: String,
    content_profile_id: String,
    required_capabilities: Vec<String>,
    capability_manifest: Vec<Capability>,
    completeness_manifest: Vec<Completeness>,
    entities: Vec<Entity>,
    relations: Vec<Relation>,
    tombstones: Vec<Tombstone>,
    payload_digest: String,
}
fn public_visibility<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    let value = String::deserialize(d)?;
    if value != "public" {
        return Err(serde::de::Error::custom("non-public catalogue record"));
    }
    Ok(Some(value))
}
fn nullable_target<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
/// Exact administrator-selected provenance; artifact bytes cannot choose their own fence.
#[derive(Debug, Clone)]
pub struct Fence {
    pub source_revision: String,
    pub authority_epoch: String,
    pub ruleset_id: String,
    pub content_profile_id: String,
    pub snapshot_id: String,
}
/// Verified immutable source; only complete categories are eligible for publication.
#[derive(Debug)]
pub struct Candidate {
    snapshot: Snapshot,
    artifact: Vec<u8>,
    artifact_sha256: String,
}
#[derive(Clone)]
pub struct Source {
    pub file: PathBuf,
    pub fence: Fence,
    pub expected_sha256: String,
    pub url: Option<url::Url>,
}
impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("fence", &self.fence)
            .finish_non_exhaustive()
    }
}
impl Source {
    pub fn local(file: PathBuf, fence: Fence, expected_sha256: String) -> Self {
        Self {
            file,
            fence,
            expected_sha256,
            url: None,
        }
    }
    pub fn configured() -> Result<Option<Self>> {
        let names = [
            "OTERYN_WORLD_CATALOG_FILE",
            "OTERYN_WORLD_CATALOG_SOURCE_REVISION",
            "OTERYN_WORLD_CATALOG_AUTHORITY_EPOCH",
            "OTERYN_WORLD_CATALOG_RULESET_ID",
            "OTERYN_WORLD_CATALOG_PROFILE_ID",
            "OTERYN_WORLD_CATALOG_SNAPSHOT_ID",
            "OTERYN_WORLD_CATALOG_SHA256",
            "OTERYN_WORLD_CATALOG_URL",
        ];
        Self::from_values(names.iter().map(std::env::var).collect())
    }
    fn from_values(
        values: Vec<std::result::Result<String, std::env::VarError>>,
    ) -> Result<Option<Self>> {
        if values
            .iter()
            .all(|v| matches!(v, Err(std::env::VarError::NotPresent)))
        {
            return Ok(None);
        }
        if values.len() != 8 {
            return Err(Error::ProvenanceConflict);
        }
        let value = |index: usize| {
            values[index]
                .as_ref()
                .map(String::as_str)
                .map_err(|_| Error::ProvenanceConflict)
        };
        let file = values[0].as_ref().ok();
        let url = values[7].as_ref().ok();
        if file.is_some() == url.is_some() {
            return Err(Error::ProvenanceConflict);
        }
        let expected_sha256 = value(6)?.to_owned();
        if !sha(&expected_sha256, 64) {
            return Err(Error::InvalidInput);
        }
        let url = url
            .map(|s| url::Url::parse(s).map_err(|_| Error::InvalidInput))
            .transpose()?;
        if let Some(url) = &url {
            transport::validate(url)?;
        }
        Ok(Some(Self {
            file: file.map_or_else(PathBuf::new, PathBuf::from),
            url,
            expected_sha256,
            fence: Fence {
                source_revision: value(1)?.into(),
                authority_epoch: value(2)?.into(),
                ruleset_id: value(3)?.into(),
                content_profile_id: value(4)?.into(),
                snapshot_id: value(5)?.into(),
            },
        }))
    }
    fn verify_bytes(&self, raw: &[u8]) -> Result<Candidate> {
        if !sha(&self.expected_sha256, 64) {
            return Err(Error::InvalidInput);
        }
        verify(
            raw,
            format!("{}\n", self.expected_sha256).as_bytes(),
            &self.fence,
        )
    }
    pub fn read(&self) -> Result<Candidate> {
        if self.url.is_some() || self.file.as_os_str().is_empty() {
            return Err(Error::InvalidInput);
        }
        self.verify_bytes(&read_regular(&self.file, MAX_FILE_BYTES)?)
    }
    pub async fn fetch(&self) -> Result<Candidate> {
        if !sha(&self.expected_sha256, 64) {
            return Err(Error::InvalidInput);
        }
        if let Some(url) = &self.url {
            if !self.file.as_os_str().is_empty() {
                return Err(Error::InvalidInput);
            }
            let raw = transport::fetch(url).await?;
            let source = self.clone();
            tokio::task::spawn_blocking(move || source.verify_bytes(&raw))
                .await
                .map_err(|_| Error::Unavailable)?
        } else {
            let source = self.clone();
            tokio::task::spawn_blocking(move || source.read())
                .await
                .map_err(|_| Error::Unavailable)?
        }
    }
}
fn read_regular(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Error::Unavailable)?;
    if !metadata.file_type().is_file() {
        return Err(Error::InvalidInput);
    }
    if metadata.len() > limit as u64 {
        return Err(Error::CapacityExceeded);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| Error::Unavailable)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > limit {
        return Err(Error::CapacityExceeded);
    }
    Ok(bytes)
}
fn token(s: &str, max: usize, first_letter: bool) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes().enumerate().all(|(i, b)| {
            if i == 0 {
                b.is_ascii_lowercase() || (!first_letter && b.is_ascii_digit())
            } else {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)
            }
        })
}
fn key(s: &str) -> bool {
    s.len() <= 2048
        && s.split_once(':')
            .is_some_and(|(a, b)| token(a, 2048, true) && token(b, 2048, true))
}
fn sha(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn data(value: &Value, depth: usize) -> Result<()> {
    if depth > 16 {
        return Err(Error::CapacityExceeded);
    }
    match value {
        Value::Object(map) => {
            if map.len() > 4096 {
                return Err(Error::CapacityExceeded);
            }
            for (k, v) in map {
                if k.is_empty() || k.len() > 2048 {
                    return Err(Error::InvalidInput);
                }
                if k == "visibility" && v.as_str() != Some("public") {
                    return Err(Error::InvalidInput);
                }
                data(v, depth + 1)?;
            }
        }
        Value::Array(items) => {
            if items.len() > 200_000 {
                return Err(Error::CapacityExceeded);
            }
            for v in items {
                data(v, depth + 1)?;
            }
        }
        Value::Number(n) if n.as_i64().is_none() => return Err(Error::InvalidInput),
        Value::String(s) if s.len() > 2048 => return Err(Error::CapacityExceeded),
        _ => {}
    }
    Ok(())
}
fn sorted_unique<T: Ord>(values: impl IntoIterator<Item = T>) -> bool {
    let mut previous = None;
    for value in values {
        if previous.as_ref().is_some_and(|p| p >= &value) {
            return false;
        }
        previous = Some(value);
    }
    true
}
/// Verify both byte integrity and canonical semantic identity before any persistence.
pub fn verify(raw: &[u8], sidecar: &[u8], fence: &Fence) -> Result<Candidate> {
    if raw.len() > MAX_FILE_BYTES {
        return Err(Error::CapacityExceeded);
    }
    let artifact_sha256 = format!("{:x}", Sha256::digest(raw));
    if sidecar != format!("{artifact_sha256}\n").as_bytes() {
        return Err(Error::IntegrityMismatch);
    }
    let value: Value = serde_json::from_slice::<json::Unique>(raw)
        .map_err(|error| {
            if error.to_string().starts_with("CATALOG_CAPACITY") {
                Error::CapacityExceeded
            } else {
                Error::InvalidInput
            }
        })?
        .0;
    let mut canonical = json::canonical(&value)?;
    canonical.push(b'\n');
    if canonical != raw {
        return Err(Error::InvalidInput);
    }
    let snapshot: Snapshot =
        serde_json::from_value(value.clone()).map_err(|_| Error::InvalidInput)?;
    if snapshot.contract_id != "oteryn.game-platform-catalog"
        || snapshot.schema_version != "1.0.0"
        || snapshot.content_authority_id != "oteryn-native"
    {
        return Err(Error::UnsupportedCapability);
    }
    if !sha(&snapshot.source_revision, 40)
        || !token(&snapshot.authority_epoch, 128, false)
        || !key(&snapshot.ruleset_id)
        || !key(&snapshot.content_profile_id)
    {
        return Err(Error::InvalidInput);
    }
    validate_timestamp(&snapshot.generated_at)?;
    if snapshot.source_revision != fence.source_revision
        || snapshot.authority_epoch != fence.authority_epoch
        || snapshot.ruleset_id != fence.ruleset_id
        || snapshot.content_profile_id != fence.content_profile_id
        || snapshot.snapshot_id != fence.snapshot_id
    {
        return Err(Error::ProvenanceConflict);
    }
    if !snapshot
        .payload_digest
        .strip_prefix("sha256:")
        .is_some_and(|s| sha(s, 64))
        || snapshot.snapshot_id != snapshot.payload_digest
    {
        return Err(Error::IntegrityMismatch);
    }
    let mut semantic = value.clone();
    let object = semantic.as_object_mut().ok_or(Error::InvalidInput)?;
    object.remove("snapshot_id");
    object.remove("payload_digest");
    if snapshot.payload_digest
        != format!("sha256:{:x}", Sha256::digest(json::canonical(&semantic)?))
    {
        return Err(Error::IntegrityMismatch);
    }
    validate(&snapshot)?;
    Ok(Candidate {
        snapshot,
        artifact: raw.to_vec(),
        artifact_sha256,
    })
}
fn validate_timestamp(s: &str) -> Result<()> {
    if s.len() != 20
        || !s.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'T',
            13 | 16 => b == b':',
            19 => b == b'Z',
            _ => b.is_ascii_digit(),
        })
    {
        return Err(Error::InvalidInput);
    }
    let number = |start: usize, end: usize| {
        s[start..end]
            .parse::<u32>()
            .map_err(|_| Error::InvalidInput)
    };
    let (y, m, d, h, min, sec) = (
        number(0, 4)?,
        number(5, 7)?,
        number(8, 10)?,
        number(11, 13)?,
        number(14, 16)?,
        number(17, 19)?,
    );
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    if y == 0 || d == 0 || d > days || h > 23 || min > 59 || sec > 59 {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
fn validate(s: &Snapshot) -> Result<()> {
    if s.capability_manifest.len() > 256
        || s.completeness_manifest.len() > 256
        || s.required_capabilities.len() > 256
        || s.entities.len() > 200_000
        || s.relations.len() > 1_000_000
        || s.tombstones.len() > 200_000
    {
        return Err(Error::CapacityExceeded);
    }
    if !sorted_unique(s.capability_manifest.iter().map(|c| &c.capability_id))
        || !sorted_unique(s.completeness_manifest.iter().map(|c| &c.capability_id))
        || !sorted_unique(&s.required_capabilities)
    {
        return Err(Error::InvalidInput);
    }
    let support: BTreeMap<_, _> = s
        .capability_manifest
        .iter()
        .map(|c| (c.capability_id.as_str(), c.support.as_str()))
        .collect();
    let complete: BTreeMap<_, _> = s
        .completeness_manifest
        .iter()
        .map(|c| (c.capability_id.as_str(), c.state.as_str()))
        .collect();
    if support.keys().ne(complete.keys()) {
        return Err(Error::InvalidInput);
    }
    for (cap, state) in &support {
        if !CAPABILITIES.contains(cap) {
            return Err(Error::UnsupportedCapability);
        }
        if !matches!(*state, "supported" | "unsupported")
            || !matches!(complete[cap], "complete" | "partial" | "unknown")
            || (*state == "unsupported" && complete[cap] != "unknown")
        {
            return Err(Error::InvalidInput);
        }
    }
    let supported = |c: &str| {
        if support.get(c) == Some(&"supported") {
            Ok(())
        } else {
            Err(Error::UnsupportedCapability)
        }
    };
    for c in &s.required_capabilities {
        supported(c)?;
    }
    if !sorted_unique(s.entities.iter().map(|e| (&e.r#type, &e.content_key)))
        || !sorted_unique(s.relations.iter().map(|r| (&r.r#type, &r.relation_key)))
        || !sorted_unique(s.tombstones.iter().map(|t| &t.content_key))
    {
        return Err(Error::InvalidInput);
    }
    let mut entities = BTreeSet::new();
    for e in &s.entities {
        supported(&e.capability_id)?;
        if !token(&e.r#type, 64, true)
            || !key(&e.content_key)
            || !entities.insert(e.content_key.as_str())
            || !e.data.is_object()
        {
            return Err(Error::InvalidInput);
        }
        data(&e.data, 0)?;
    }
    let loot_keys: BTreeSet<_> = s
        .entities
        .iter()
        .filter(|e| e.capability_id == "creature_loot")
        .map(|e| e.content_key.as_str())
        .collect();
    let mut relations = BTreeSet::new();
    for r in &s.relations {
        supported(&r.capability_id)?;
        if r.capability_id == "creature_loot" && loot_keys.contains(r.relation_key.as_str()) {
            return Err(Error::PublicationKeyCollision);
        }
        if !token(&r.r#type, 64, true)
            || !key(&r.relation_key)
            || !relations.insert(r.relation_key.as_str())
            || !key(&r.source)
            || !entities.contains(r.source.as_str())
            || r.target
                .as_ref()
                .is_some_and(|t| !key(t) || !entities.contains(t.as_str()))
            || !r.data.is_object()
        {
            return Err(Error::InvalidInput);
        }
        data(&r.data, 0)?;
    }
    let mut tombstones = BTreeSet::new();
    for t in &s.tombstones {
        supported(&t.capability_id)?;
        if !key(&t.content_key)
            || !tombstones.insert(&t.content_key)
            || entities.contains(t.content_key.as_str())
            || complete.get(t.capability_id.as_str()) != Some(&"complete")
            || t.reason.is_empty()
            || t.reason.len() > 2048
        {
            return Err(Error::InvalidInput);
        }
    }
    Ok(())
}
impl Candidate {
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot.snapshot_id
    }
    /// Immutable, transactional, idempotent intake. Never changes the active pointer,
    /// public world_entities, independent visibility approvals or any non-world table.
    /// Supported entity/relation/tombstone carriers are rebuilt in the same transaction.
    pub async fn persist_inactive(&self, db: &PgPool) -> Result<()> {
        let s = &self.snapshot;
        let mut tx = db.begin().await.map_err(|_| Error::Unavailable)?;
        sqlx::query("INSERT INTO world_catalog_candidates(snapshot_id,artifact_sha256,source_revision,authority_epoch,ruleset_id,content_profile_id,artifact) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(snapshot_id) DO NOTHING")
            .bind(&s.snapshot_id).bind(&self.artifact_sha256).bind(&s.source_revision).bind(&s.authority_epoch).bind(&s.ruleset_id).bind(&s.content_profile_id).bind(&self.artifact).execute(&mut *tx).await.map_err(|_|Error::Unavailable)?;
        let stored: (String, Vec<u8>) = sqlx::query_as(
            "SELECT artifact_sha256,artifact FROM world_catalog_candidates WHERE snapshot_id=$1 FOR UPDATE",
        )
        .bind(&s.snapshot_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
        if stored != (self.artifact_sha256.clone(), self.artifact.clone()) {
            return Err(Error::IntegrityMismatch);
        }
        self.rebuild_cache(&mut tx).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}
/// Refresh is invoked in a background task by serve. Only the explicit publication
/// switch enables complete-category projection; intake alone never grants visibility.
pub async fn read_configured(db: &PgPool) -> Result<Option<String>> {
    let Some(source) = Source::configured()? else {
        return Ok(None);
    };
    refresh::read_source(db, &source, publication_enabled())
        .await
        .map(Some)
}

pub fn publication_enabled() -> bool {
    std::env::var("OTERYN_WORLD_CATALOG_PUBLICATION_ENABLED").as_deref() == Ok("true")
}
/// Never awaited by server startup/readiness. The task owns bounded source I/O.
pub fn refresh_in_background(
    db: PgPool,
    ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> tokio::task::JoinHandle<Result<Option<String>>> {
    ready.store(false, std::sync::atomic::Ordering::Release);
    tokio::spawn(async move {
        let Some(source) = Source::configured()? else {
            return Ok(None);
        };
        refresh::run(
            &db,
            &source,
            publication_enabled(),
            RetryPolicy::configured()?,
            &ready,
        )
        .await
        .map(Some)
    })
}

#[cfg(test)]
mod tests;
