//! Public wiki projection of verified complete capabilities. This does not activate
//! maps or grant the independent Platform world_visibility approval.
use super::{Candidate, Error, Result, Source};
use crate::Entity;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

const KINDS: [(&str, &str); 7] = [
    ("item", "Item"),
    ("creature", "Creature"),
    ("npc", "NPC"),
    ("spell", "Spell"),
    ("quest", "Quest"),
    ("achievement", "Achievement"),
    ("creature_loot", "Loot"),
];
fn label(value: Option<&serde_json::Value>, max: usize) -> bool {
    value
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
}
/// Minimum current wiki presentation, not a replacement native payload schema.
/// No unknown field, guessed display name, unit or relation formula is promoted.
fn presentation(capability: &str, data: &serde_json::Value) -> bool {
    let body = data
        .get("definition")
        .or_else(|| data.get("declaration"))
        .or_else(|| data.pointer("/bundle/spell"))
        .unwrap_or(data);
    let authoring = label(data.pointer("/authoring/profile/details/display_name"), 256);
    let plain = ["display_name", "name"]
        .iter()
        .any(|field| label(body.get(field), 256));
    let known = body
        .pointer("/semantics/presentation/state")
        .and_then(|v| v.as_str())
        == Some("KNOWN")
        && body
            .pointer("/semantics/presentation/value/name/state")
            .and_then(|v| v.as_str())
            == Some("KNOWN")
        && label(
            body.pointer("/semantics/presentation/value/name/value"),
            256,
        );
    let named = ["pl", "en"].iter().all(|lang| {
        let localized = body.get("names").and_then(|names| names.get(lang));
        authoring
            || known
            || match localized.and_then(|v| v.as_str()) {
                Some(_) => label(localized, 256),
                None => plain,
            }
    });
    named && (capability != "quest" || label(body.get("classification"), 80))
}
impl Candidate {
    /// Admin-selected source provenance and the publication switch are checked by
    /// the caller. This projects only complete cache categories, never approvals.
    pub async fn publish(&self, db: &PgPool) -> Result<()> {
        let mut categories = Vec::new();
        let mut entities = Vec::new();
        for (capability, kind) in KINDS {
            if self.require_complete(&[capability]).is_err() {
                continue;
            }
            let mut records = Vec::new();
            for entity in self.cached_entities(db, capability).await? {
                records.push((kind, entity.content_key, entity.data));
            }
            if capability == "creature_loot" {
                records.extend(
                    self.cached_loot_relations(db)
                        .await?
                        .into_iter()
                        .map(|(key, data)| ("Loot", key, data)),
                );
            }
            // Complete transport inventory does not supply a missing wiki field.
            // Refuse that category rather than inventing a name from its key.
            if records
                .iter()
                .any(|(_, key, data)| key.len() > 512 || !presentation(capability, data))
            {
                continue;
            }
            categories.push(capability.to_owned());
            entities.extend(records);
        }
        let mut tx = db.begin().await.map_err(|_| Error::Unavailable)?;
        // One publication lock orders switches, including unsupported replacements.
        sqlx::query("SELECT singleton FROM world_catalog_publication WHERE singleton FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        if categories.is_empty() {
            sqlx::query("UPDATE world_catalog_publication SET snapshot_id=NULL WHERE singleton")
                .execute(&mut *tx)
                .await
                .map_err(|_| Error::Unavailable)?;
        } else {
            let existing: Option<Uuid> = sqlx::query_scalar(
                "SELECT version_id FROM world_catalog_versions WHERE snapshot_id=$1",
            )
            .bind(self.snapshot_id())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
            let version = existing.unwrap_or_else(Uuid::now_v7);
            if existing.is_none() {
                sqlx::query("INSERT INTO world_versions(id,source_commit,status,document_count) VALUES($1,$2,'ready',$3)")
                    .bind(version).bind(&self.snapshot.source_revision).bind(entities.len() as i64)
                    .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
                sqlx::query("INSERT INTO world_catalog_versions(snapshot_id,version_id,capabilities) VALUES($1,$2,$3)")
                    .bind(self.snapshot_id()).bind(version).bind(&categories)
                    .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
            }
            // Rebuild the derived presentation cache idempotently; no hand edits survive refresh.
            sqlx::query("DELETE FROM world_entities WHERE version_id=$1")
                .bind(version)
                .execute(&mut *tx)
                .await
                .map_err(|_| Error::Unavailable)?;
            for chunk in entities.chunks(500) {
                let mut query = QueryBuilder::<Postgres>::new(
                    "INSERT INTO world_entities(version_id,kind,key,data,publishable) ",
                );
                query.push_values(chunk, |mut row, (kind, key, data)| {
                    row.push_bind(version)
                        .push_bind(kind)
                        .push_bind(key)
                        .push_bind(data)
                        .push_bind(true);
                });
                query
                    .build()
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| Error::Unavailable)?;
            }
            sqlx::query("UPDATE world_catalog_publication SET snapshot_id=$1 WHERE singleton")
                .bind(self.snapshot_id())
                .execute(&mut *tx)
                .await
                .map_err(|_| Error::Unavailable)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}

/// Source-bound public reader. A missing/invalid configuration never falls back
/// to another revision or to the legacy world pointer.
#[derive(Debug, Clone)]
pub struct CatalogStore {
    pool: PgPool,
    source: Option<Source>,
    ready: Arc<AtomicBool>,
}
impl CatalogStore {
    pub fn new(pool: PgPool, source: Option<Source>) -> Self {
        Self {
            pool,
            source,
            ready: Arc::new(AtomicBool::new(true)),
        }
    }
    pub fn with_readiness(mut self, ready: Arc<AtomicBool>) -> Self {
        self.ready = ready;
        self
    }
    pub fn configured(pool: PgPool, ready: Arc<AtomicBool>) -> Self {
        Self {
            pool,
            source: Source::configured().ok().flatten(),
            ready,
        }
    }
    async fn selection(&self) -> Result<Option<(Uuid, Vec<String>)>> {
        if !self.ready.load(Ordering::Acquire) {
            return Err(Error::Unavailable);
        }
        let source = self.source.as_ref().ok_or(Error::Unavailable)?;
        sqlx::query_as("SELECT v.version_id,v.capabilities FROM world_catalog_publication p JOIN world_catalog_versions v USING(snapshot_id) JOIN world_catalog_candidates c USING(snapshot_id) WHERE p.singleton AND c.snapshot_id=$1 AND c.artifact_sha256=$2 AND c.source_revision=$3 AND c.authority_epoch=$4 AND c.ruleset_id=$5 AND c.content_profile_id=$6")
            .bind(&source.fence.snapshot_id).bind(&source.expected_sha256).bind(&source.fence.source_revision)
            .bind(&source.fence.authority_epoch).bind(&source.fence.ruleset_id).bind(&source.fence.content_profile_id)
            .fetch_optional(&self.pool).await.map_err(|_| Error::Unavailable)
    }
    pub async fn active_version_id(&self) -> Result<Option<Uuid>> {
        Ok(self.selection().await?.map(|s| s.0))
    }
    pub async fn available(&self, kind: &str) -> Result<bool> {
        let (_, caps) = self.selection().await?.ok_or(Error::Unavailable)?;
        Ok(KINDS
            .iter()
            .any(|(cap, k)| *k == kind && caps.iter().any(|c| c == cap)))
    }
    async fn selected_kind(&self, kind: &str) -> Result<Uuid> {
        let (version, caps) = self.selection().await?.ok_or(Error::Unavailable)?;
        if !KINDS
            .iter()
            .any(|(cap, k)| *k == kind && caps.iter().any(|c| c == cap))
        {
            return Err(Error::UnsupportedCapability);
        }
        Ok(version)
    }
    pub async fn list_by_kind(
        &self,
        kind: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Entity>> {
        if !(1..=1000).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let version = self.selected_kind(kind).await?;
        let rows = sqlx::query_as("SELECT e.version_id,e.kind,e.key,e.data FROM world_entities e JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed JOIN world_catalog_versions cv ON cv.version_id=e.version_id JOIN world_catalog_publication p ON p.snapshot_id=cv.snapshot_id WHERE p.singleton AND e.version_id=$1 AND e.kind=$2 AND e.publishable AND ($3::text IS NULL OR e.key COLLATE \"C\">$3) ORDER BY e.key COLLATE \"C\" LIMIT $4")
            .bind(version).bind(kind).bind(after).bind(i64::from(limit)).fetch_all(&self.pool).await.map_err(|_|Error::Unavailable)?;
        if self.active_version_id().await? != Some(version) {
            return Err(Error::Unavailable);
        }
        Ok(rows)
    }
    pub async fn get_entity(&self, kind: &str, key: &str) -> Result<Option<Entity>> {
        let version = self.selected_kind(kind).await?;
        let entity = sqlx::query_as("SELECT e.version_id,e.kind,e.key,e.data FROM world_entities e JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed JOIN world_catalog_versions cv ON cv.version_id=e.version_id JOIN world_catalog_publication p ON p.snapshot_id=cv.snapshot_id WHERE p.singleton AND e.version_id=$1 AND e.kind=$2 AND e.key=$3 AND e.publishable")
            .bind(version).bind(kind).bind(key).fetch_optional(&self.pool).await.map_err(|_|Error::Unavailable)?;
        if self.active_version_id().await? != Some(version) {
            return Err(Error::Unavailable);
        }
        Ok(entity)
    }
}
