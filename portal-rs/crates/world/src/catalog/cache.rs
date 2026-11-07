//! Capability-scoped read cache. These are inactive transport records, not a
//! capability payload contract or permission to publish native facts.
use super::{Candidate, Entity, Error, Result, json};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::{BTreeMap, BTreeSet};

/// Verified generic carrier read from the derived cache. Presentation requires
/// a separately accepted capability payload profile and publication authority.
#[derive(Debug)]
pub struct CachedEntity {
    pub entity_type: String,
    pub content_key: String,
    pub capability_id: String,
    pub data: Value,
}
struct Record {
    class: &'static str,
    key: String,
    capability: String,
    bytes: Vec<u8>,
}
fn record<T: Serialize>(
    class: &'static str,
    key: &str,
    capability: &str,
    value: &T,
) -> Result<Record> {
    Ok(Record {
        class,
        key: key.into(),
        capability: capability.into(),
        bytes: json::canonical(&serde_json::to_value(value).map_err(|_| Error::InvalidInput)?)?,
    })
}
impl Candidate {
    /// Eligibility of a requested capability set only. This does not authorize
    /// activation: payload/visibility semantics and publication authority must
    /// come from the selected producer profile.
    pub fn require_complete(&self, required: &[&str]) -> Result<()> {
        let mut seen = BTreeSet::new();
        if required.is_empty() {
            return Err(Error::UnsupportedCapability);
        }
        for capability in required {
            if !seen.insert(capability) {
                return Err(Error::InvalidInput);
            }
            let supported = self
                .snapshot
                .capability_manifest
                .iter()
                .any(|c| c.capability_id == *capability && c.support == "supported");
            let complete = self
                .snapshot
                .completeness_manifest
                .iter()
                .any(|c| c.capability_id == *capability && c.state == "complete");
            if !supported || !complete {
                return Err(Error::UnsupportedCapability);
            }
        }
        Ok(())
    }
    pub(super) async fn rebuild_cache(&self, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        let mut records = Vec::new();
        for e in &self.snapshot.entities {
            records.push(record("entity", &e.content_key, &e.capability_id, e)?);
        }
        for r in &self.snapshot.relations {
            records.push(record("relation", &r.relation_key, &r.capability_id, r)?);
        }
        for t in &self.snapshot.tombstones {
            records.push(record("tombstone", &t.content_key, &t.capability_id, t)?);
        }
        // Validation already rejects records assigned to unsupported capabilities.
        // Replace atomically so a damaged/incomplete derived cache is rebuildable.
        sqlx::query("DELETE FROM world_catalog_records WHERE snapshot_id=$1")
            .bind(self.snapshot_id())
            .execute(&mut **tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        for chunk in records.chunks(500) {
            let mut query = QueryBuilder::<Postgres>::new(
                "INSERT INTO world_catalog_records(snapshot_id,record_class,record_key,capability_id,record) ",
            );
            query.push_values(chunk, |mut row, r| {
                row.push_bind(self.snapshot_id())
                    .push_bind(r.class)
                    .push_bind(&r.key)
                    .push_bind(&r.capability)
                    .push_bind(&r.bytes);
            });
            query
                .build()
                .execute(&mut **tx)
                .await
                .map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }
    /// Complete-capability reads fence every cached record against the verified
    /// snapshot. Partial/unknown coverage cannot masquerade as an empty inventory.
    pub async fn cached_entities(
        &self,
        db: &PgPool,
        capability: &str,
    ) -> Result<Vec<CachedEntity>> {
        self.require_complete(&[capability])?;
        let mut tx = db.begin().await.map_err(|_| Error::Unavailable)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        let digest:Option<String>=sqlx::query_scalar("SELECT artifact_sha256 FROM world_catalog_candidates WHERE snapshot_id=$1 AND status='inactive'").bind(self.snapshot_id()).fetch_optional(&mut *tx).await.map_err(|_|Error::Unavailable)?;
        if digest.as_deref() != Some(self.artifact_sha256.as_str()) {
            return Err(Error::Unavailable);
        }
        let rows:Vec<(String,Vec<u8>)>=sqlx::query_as("SELECT record_key,record FROM world_catalog_records WHERE snapshot_id=$1 AND capability_id=$2 AND record_class='entity' ORDER BY record_key COLLATE \"C\" LIMIT 200001").bind(self.snapshot_id()).bind(capability).fetch_all(&mut *tx).await.map_err(|_|Error::Unavailable)?;
        if rows.len() > 200_000 {
            return Err(Error::CapacityExceeded);
        }
        let expected: BTreeMap<_, _> = self
            .snapshot
            .entities
            .iter()
            .filter(|e| e.capability_id == capability)
            .map(|e| (e.content_key.as_str(), e))
            .collect();
        if rows.len() != expected.len() {
            return Err(Error::IntegrityMismatch);
        }
        let mut entities = Vec::new();
        for (key, bytes) in rows {
            let e = expected.get(key.as_str()).ok_or(Error::IntegrityMismatch)?;
            if bytes != record("entity", &e.content_key, &e.capability_id, e)?.bytes {
                return Err(Error::IntegrityMismatch);
            }
            let cached: Entity =
                serde_json::from_slice(&bytes).map_err(|_| Error::IntegrityMismatch)?;
            entities.push(CachedEntity {
                entity_type: cached.r#type,
                content_key: cached.content_key,
                capability_id: cached.capability_id,
                data: cached.data,
            });
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(entities)
    }
}

impl Candidate {
    /// Relation carriers are fenced exactly like entities. This allows the wiki's
    /// generic loot record view without inventing a drop formula or graph renderer.
    pub(super) async fn cached_loot_relations(&self, db: &PgPool) -> Result<Vec<(String, Value)>> {
        self.require_complete(&["creature_loot"])?;
        let mut tx = db.begin().await.map_err(|_| Error::Unavailable)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
        let digest: Option<String> = sqlx::query_scalar(
            "SELECT artifact_sha256 FROM world_catalog_candidates WHERE snapshot_id=$1",
        )
        .bind(self.snapshot_id())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
        if digest.as_deref() != Some(self.artifact_sha256.as_str()) {
            return Err(Error::IntegrityMismatch);
        }
        let rows:Vec<(String,Vec<u8>)>=sqlx::query_as("SELECT record_key,record FROM world_catalog_records WHERE snapshot_id=$1 AND capability_id='creature_loot' AND record_class='relation' ORDER BY record_key COLLATE \"C\" LIMIT 1000001")
            .bind(self.snapshot_id()).fetch_all(&mut *tx).await.map_err(|_|Error::Unavailable)?;
        let expected: BTreeMap<_, _> = self
            .snapshot
            .relations
            .iter()
            .filter(|r| r.capability_id == "creature_loot")
            .map(|r| (r.relation_key.as_str(), r))
            .collect();
        if rows.len() != expected.len() {
            return Err(Error::IntegrityMismatch);
        }
        let mut result = Vec::new();
        for (key, bytes) in rows {
            let relation = expected.get(key.as_str()).ok_or(Error::IntegrityMismatch)?;
            if bytes
                != record(
                    "relation",
                    &relation.relation_key,
                    &relation.capability_id,
                    relation,
                )?
                .bytes
            {
                return Err(Error::IntegrityMismatch);
            }
            result.push((key, relation.data.clone()));
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }
}
