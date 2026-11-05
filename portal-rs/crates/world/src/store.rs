use std::path::Path;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    Entity, ImportConfig, ImportIssue, ImportMapping, ImportReport, VersionStatus, WorldError,
    WorldOptions,
    import::{self, Document},
    types::validate_label,
};

/// Trusted service API for world data. Callers authorize import, rollback and
/// visibility management; public readers use only the allow-listed methods here.
#[derive(Debug, Clone)]
pub struct WorldStore {
    pool: PgPool,
    options: WorldOptions,
}

impl WorldStore {
    pub fn new(pool: PgPool, options: WorldOptions) -> Self {
        Self { pool, options }
    }

    fn check_enabled(&self) -> Result<(), WorldError> {
        if self.options.enabled {
            Ok(())
        } else {
            Err(WorldError::Disabled)
        }
    }

    /// Imports a caller-provided immutable local checkout. The caller is responsible
    /// for verifying that its bytes belong to the full supplied commit hash.
    /// Every JSON file under content must have exactly one configured mapping.
    pub async fn import_directory(
        &self,
        root: impl AsRef<Path>,
        source_commit: &str,
        mappings: Vec<ImportMapping>,
    ) -> Result<ImportReport, WorldError> {
        self.import_configured_directory(
            root,
            source_commit,
            ImportConfig {
                mappings,
                ..ImportConfig::default()
            },
        )
        .await
    }

    /// Imports using explicit schema URI aliases and exact file exclusions.
    /// Excluded files are outside the configured data surface and never stored.
    pub async fn import_configured_directory(
        &self,
        root: impl AsRef<Path>,
        source_commit: &str,
        config: ImportConfig,
    ) -> Result<ImportReport, WorldError> {
        self.check_enabled()?;
        if !matches!(source_commit.len(), 40 | 64)
            || !source_commit
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(WorldError::InvalidInput(
                "a full lowercase hexadecimal commit hash is required".into(),
            ));
        }
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO world_versions (id, source_commit) VALUES ($1, $2)")
            .bind(id)
            .bind(source_commit)
            .execute(&self.pool)
            .await?;
        let root = root.as_ref().to_owned();
        let documents = match tokio::task::spawn_blocking(move || {
            import::validate_config(root, config)
        })
        .await
        {
            Ok(Ok(documents)) => documents,
            Ok(Err(issues)) => {
                self.record_failure(id, &issues).await?;
                return Err(WorldError::ImportFailed {
                    version_id: id,
                    issues,
                });
            }
            Err(error) => {
                self.record_failure(
                    id,
                    &[ImportIssue {
                        file: String::new(),
                        pointer: String::new(),
                        message: "validation task failed".into(),
                    }],
                )
                .await?;
                return Err(error.into());
            }
        };
        let count = documents.len() as i64;
        if let Err(error) = self.publish(id, documents).await {
            self.record_failure(
                id,
                &[ImportIssue {
                    file: String::new(),
                    pointer: String::new(),
                    message: "database publication failed; transaction rolled back".into(),
                }],
            )
            .await?;
            return Err(error);
        }
        Ok(ImportReport {
            version_id: id,
            source_commit: source_commit.into(),
            status: VersionStatus::Ready,
            document_count: count,
            errors: Vec::new(),
        })
    }

    async fn record_failure(&self, id: Uuid, issues: &[ImportIssue]) -> Result<(), WorldError> {
        sqlx::query("UPDATE world_versions SET status = 'failed', errors = $2, updated_at = now() WHERE id = $1 AND status = 'importing'")
            .bind(id).bind(sqlx::types::Json(issues)).execute(&self.pool).await?;
        Ok(())
    }

    async fn publish(&self, id: Uuid, documents: Vec<Document>) -> Result<(), WorldError> {
        let mut tx = self.pool.begin().await?;
        // Acquire the singleton lock before reading its predecessor. Import and
        // rollback share this lock, including when no active version exists yet.
        let (previous, _) = lock_active(&mut tx).await?;
        for document in &documents {
            sqlx::query(
                "INSERT INTO world_entities (version_id, kind, key, data, publishable) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(&document.kind)
            .bind(&document.key)
            .bind(&document.data)
            .bind(document.publishable)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE world_versions SET status = 'ready', document_count = $2, updated_at = now() WHERE id = $1")
            .bind(id).bind(documents.len() as i64).execute(&mut *tx).await?;
        sqlx::query("UPDATE world_active_version SET version_id = $1, previous_version_id = $2, updated_at = now() WHERE singleton")
            .bind(id).bind(previous).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Swaps active and previous versions under the publication lock. Documents
    /// are never deleted by a switch, a failed import or a rollback.
    pub async fn rollback(&self) -> Result<Uuid, WorldError> {
        self.check_enabled()?;
        let mut tx = self.pool.begin().await?;
        let (active, previous) = lock_active(&mut tx).await?;
        let previous = previous.ok_or(WorldError::NoPreviousVersion)?;
        sqlx::query("UPDATE world_active_version SET version_id = $1, previous_version_id = $2, updated_at = now() WHERE singleton")
            .bind(previous).bind(active).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(previous)
    }

    pub async fn active_version_id(&self) -> Result<Option<Uuid>, WorldError> {
        self.check_enabled()?;
        Ok(
            sqlx::query_scalar("SELECT version_id FROM world_active_version WHERE singleton")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Diagnostic API for trusted import/admin callers; no entity data is returned.
    pub async fn import_report(&self, id: Uuid) -> Result<Option<ImportReport>, WorldError> {
        self.check_enabled()?;
        let row: Option<(String, String, i64, sqlx::types::Json<Vec<ImportIssue>>)> = sqlx::query_as(
            "SELECT source_commit, status, document_count, errors FROM world_versions WHERE id = $1"
        ).bind(id).fetch_optional(&self.pool).await?;
        row.map(|(source_commit, status, document_count, errors)| {
            let status = match status.as_str() {
                "importing" => VersionStatus::Importing,
                "ready" => VersionStatus::Ready,
                "failed" => VersionStatus::Failed,
                _ => {
                    return Err(WorldError::InvalidInput(
                        "unknown stored import status".into(),
                    ));
                }
            };
            Ok(ImportReport {
                version_id: id,
                source_commit,
                status,
                document_count,
                errors: errors.0,
            })
        })
        .transpose()
    }

    pub async fn allow(&self, kind: &str, key: &str) -> Result<(), WorldError> {
        self.set_visibility(kind, key, true).await
    }

    pub async fn deny(&self, kind: &str, key: &str) -> Result<(), WorldError> {
        self.set_visibility(kind, key, false).await
    }

    async fn set_visibility(&self, kind: &str, key: &str, allowed: bool) -> Result<(), WorldError> {
        self.check_enabled()?;
        validate_label(kind, "kind")?;
        validate_label(key, "key")?;
        sqlx::query("INSERT INTO world_visibility (kind, key, allowed) VALUES ($1, $2, $3) ON CONFLICT (kind, key) DO UPDATE SET allowed = EXCLUDED.allowed, updated_at = now()")
            .bind(kind).bind(key).bind(allowed).execute(&self.pool).await?;
        Ok(())
    }

    /// Hidden, missing and inactive entities all return None. Each read joins
    /// the active pointer and allow-list in the same database statement.
    pub async fn get_entity(&self, kind: &str, key: &str) -> Result<Option<Entity>, WorldError> {
        self.check_enabled()?;
        validate_label(kind, "kind")?;
        validate_label(key, "key")?;
        Ok(sqlx::query_as(
            "SELECT e.version_id, e.kind, e.key, e.data FROM world_entities e
             JOIN world_active_version a ON a.version_id = e.version_id
             JOIN world_visibility v ON v.kind = e.kind AND v.key = e.key AND v.allowed
             WHERE e.kind = $1 AND e.key = $2 AND e.publishable",
        )
        .bind(kind)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Keyset pagination, ordered by bytewise key. Limits must be 1..=1000.
    pub async fn list_by_kind(
        &self,
        kind: &str,
        after_key: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Entity>, WorldError> {
        self.check_enabled()?;
        validate_label(kind, "kind")?;
        if let Some(key) = after_key {
            validate_label(key, "after_key")?;
        }
        if !(1..=1000).contains(&limit) {
            return Err(WorldError::InvalidInput(
                "limit must be between 1 and 1000".into(),
            ));
        }
        Ok(sqlx::query_as(
            "SELECT e.version_id, e.kind, e.key, e.data FROM world_entities e
             JOIN world_active_version a ON a.version_id = e.version_id
             JOIN world_visibility v ON v.kind = e.kind AND v.key = e.key AND v.allowed
             WHERE e.kind = $1 AND e.publishable AND ($2::text IS NULL OR e.key COLLATE \"C\" > $2)
             ORDER BY e.key COLLATE \"C\" LIMIT $3",
        )
        .bind(kind)
        .bind(after_key)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }
}

async fn lock_active(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(Option<Uuid>, Option<Uuid>), sqlx::Error> {
    sqlx::query_as("SELECT version_id, previous_version_id FROM world_active_version WHERE singleton FOR UPDATE")
        .fetch_one(&mut **tx).await
}
