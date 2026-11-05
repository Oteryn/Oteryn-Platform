use std::{collections::BTreeMap, sync::Arc};

use sqlx::{PgPool, Row, types::Json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    AccountState, Channel, Error, FeatureFlags, PublishAccess, RollbackAuthorization,
    UpdaterPolicy, Upload, Versions, domain::safe_path, verification,
};

type Metadata = BTreeMap<String, Vec<u8>>;
type Artifacts = BTreeMap<String, serde_json::Value>;

/// Public roots are deployment configuration, never accepted from the upload as trust anchors.
#[derive(Debug, Clone)]
pub struct Downloads {
    pool: PgPool,
    flags: FeatureFlags,
    trusted_roots: Arc<BTreeMap<Channel, Vec<u8>>>,
}

impl Downloads {
    pub fn new(
        pool: PgPool,
        flags: FeatureFlags,
        trusted_roots: BTreeMap<Channel, Vec<u8>>,
    ) -> Self {
        Self {
            pool,
            flags,
            trusted_roots: Arc::new(trusted_roots),
        }
    }

    fn enabled(&self) -> Result<(), Error> {
        if self.flags.downloads {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }

    async fn authorize(
        &self,
        access: &dyn PublishAccess,
        actor: Uuid,
        channel: Channel,
    ) -> Result<(), Error> {
        self.enabled()?;
        if actor.get_version_num() != 7
            || access.account_state(actor).await != AccountState::Active
            || access.may_publish(actor, channel).await != Some(true)
        {
            return Err(Error::Denied);
        }
        Ok(())
    }

    /// Atomically verifies and publishes an offline-signed generation. There is no public POST route.
    /// Publication records also retain actor, previous head and exact before/after documents for audit.
    pub async fn publish(
        &self,
        access: &dyn PublishAccess,
        actor: Uuid,
        upload: Upload,
    ) -> Result<Uuid, Error> {
        self.authorize(access, actor, upload.channel).await?;
        let root = self
            .trusted_roots
            .get(&upload.channel)
            .ok_or(Error::Verification)?;
        let verified = verification::verify(upload, root).await?;
        let channel = verified.upload.channel;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO client_channels (id, name) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(channel.as_str())
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(
            "SELECT id, active_publication_id FROM client_channels WHERE name = $1 FOR UPDATE",
        )
        .bind(channel.as_str())
        .fetch_one(&mut *tx)
        .await?;
        let channel_id: Uuid = row.try_get("id")?;
        let previous: Option<Uuid> = row.try_get("active_publication_id")?;
        self.authorize(access, actor, channel).await?;
        if verified.expires_at <= OffsetDateTime::now_utc() {
            return Err(Error::Verification);
        }
        let rows = sqlx::query("SELECT id, policy, metadata, versions, artifacts FROM client_releases WHERE channel_id = $1 ORDER BY policy_revision")
            .bind(channel_id).fetch_all(&mut *tx).await?;
        let mut history = Vec::with_capacity(rows.len());
        for row in rows {
            let policy_bytes: Vec<u8> = row.try_get("policy")?;
            history.push(History {
                policy: serde_json::from_slice(&policy_bytes).map_err(|_| Error::Verification)?,
                metadata: row.try_get::<Json<Metadata>, _>("metadata")?.0,
                versions: row.try_get::<Json<Versions>, _>("versions")?.0,
                artifacts: row.try_get::<Json<Artifacts>, _>("artifacts")?.0,
            });
        }
        check_history(&history, &verified)?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO client_releases (id, channel_id, policy_revision, release_id, release_sequence, version_display, policy, policy_file, metadata, versions, artifacts, expires_at, published_by, previous_publication_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(id).bind(channel_id).bind(verified.policy.policy_revision)
            .bind(&verified.policy.current_release_id).bind(verified.policy.current_release_sequence)
            .bind(&verified.policy.current_version_display).bind(&verified.upload.policy)
            .bind(&verified.policy_file).bind(Json(&verified.upload.metadata))
            .bind(Json(&verified.versions)).bind(Json(&verified.artifacts))
            .bind(verified.expires_at).bind(actor).bind(previous)
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE client_channels SET active_publication_id = $1, updated_at = now() WHERE id = $2")
            .bind(id).bind(channel_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Returns exact bytes; immutable versioned files remain available across head changes.
    pub async fn metadata(&self, channel: Channel, filename: &str) -> Result<Vec<u8>, Error> {
        self.enabled()?;
        if filename.contains('/') || !safe_path(filename) || !filename.ends_with(".json") {
            return Err(Error::Invalid("metadata filename"));
        }
        let role = filename
            .strip_suffix(".json")
            .ok_or(Error::Invalid("metadata filename"))?;
        let mutable = matches!(role, "root" | "targets" | "snapshot" | "timestamp")
            || role == channel.as_str();
        if mutable {
            let row = sqlx::query("SELECT r.metadata, r.versions FROM client_channels c JOIN client_releases r ON r.id = c.active_publication_id WHERE c.name = $1 AND r.expires_at > now()")
                .bind(channel.as_str()).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
            let versions = row.try_get::<Json<Versions>, _>("versions")?.0;
            let key = match role {
                "timestamp" => "timestamp.json".to_owned(),
                "root" => format!("{}.root.json", versions.root),
                "targets" => format!("{}.targets.json", versions.targets),
                "snapshot" => format!("{}.snapshot.json", versions.snapshot),
                _ => format!("{}.{}.json", versions.delegated, channel.as_str()),
            };
            return row
                .try_get::<Json<Metadata>, _>("metadata")?
                .0
                .remove(&key)
                .ok_or(Error::NotFound);
        }
        let bytes: Option<Json<Vec<u8>>> = sqlx::query_scalar("SELECT r.metadata -> $2 FROM client_releases r JOIN client_channels c ON c.id = r.channel_id WHERE c.name = $1 AND r.metadata ? $2 ORDER BY r.policy_revision DESC LIMIT 1")
            .bind(channel.as_str()).bind(filename).fetch_optional(&self.pool).await?;
        bytes.map(|b| b.0).ok_or(Error::NotFound)
    }

    /// `None` reads the current policy; a hash-prefixed target path reads immutable historical bytes.
    pub async fn policy(&self, channel: Channel, target: Option<&str>) -> Result<Vec<u8>, Error> {
        self.enabled()?;
        let bytes = if let Some(target) = target {
            if !safe_path(target) {
                return Err(Error::Invalid("policy target"));
            }
            sqlx::query_scalar("SELECT r.policy FROM client_releases r JOIN client_channels c ON c.id = r.channel_id WHERE c.name = $1 AND r.policy_file = $2 ORDER BY r.policy_revision DESC LIMIT 1")
                .bind(channel.as_str()).bind(target).fetch_optional(&self.pool).await?
        } else {
            sqlx::query_scalar("SELECT r.policy FROM client_channels c JOIN client_releases r ON r.id = c.active_publication_id WHERE c.name = $1 AND r.expires_at > now()")
                .bind(channel.as_str()).fetch_optional(&self.pool).await?
        };
        bytes.ok_or(Error::NotFound)
    }
}

struct History {
    policy: UpdaterPolicy,
    metadata: Metadata,
    versions: Versions,
    artifacts: Artifacts,
}

fn check_history(history: &[History], new: &verification::Verified) -> Result<(), Error> {
    let policy = &new.policy;
    if let Some(old) = history.last() {
        let v = &new.versions;
        let p = &old.versions;
        let rollback = policy.current_release_sequence < old.policy.current_release_sequence;
        if policy.policy_revision <= old.policy.policy_revision
            || v.root < p.root
            || v.targets < p.targets
            || v.delegated <= p.delegated
            || v.snapshot <= p.snapshot
            || v.timestamp <= p.timestamp
            || rollback != (policy.rollback_authorization == RollbackAuthorization::Explicit)
            || policy.minimum_supported_release_sequence
                < old.policy.minimum_supported_release_sequence
            || !old
                .policy
                .revoked_release_ids
                .iter()
                .all(|id| policy.revoked_release_ids.contains(id))
            || !old
                .policy
                .revoked_artifact_targets
                .iter()
                .all(|path| policy.revoked_artifact_targets.contains(path))
        {
            return Err(Error::Conflict);
        }
    } else if policy.rollback_authorization != RollbackAuthorization::None {
        return Err(Error::Conflict);
    }
    let mut known = false;
    let mut highest_sequence = 0;
    for old in history {
        highest_sequence = highest_sequence.max(old.policy.current_release_sequence);
        // Immutable filenames cannot be repointed even if they are absent from the current head.
        for (name, bytes) in &new.upload.metadata {
            if name != "timestamp.json" && old.metadata.get(name).is_some_and(|old| old != bytes) {
                return Err(Error::Conflict);
            }
        }
        for (path, target) in &new.artifacts {
            if old.artifacts.get(path).is_some_and(|old| old != target) {
                return Err(Error::Conflict);
            }
        }
        if policy.current_release_id == old.policy.current_release_id {
            known = true;
            if policy.current_release_sequence != old.policy.current_release_sequence
                || policy.current_version_display != old.policy.current_version_display
                || policy.artifacts != old.policy.artifacts
                || new.artifacts != old.artifacts
            {
                return Err(Error::Conflict);
            }
        } else if policy.current_release_sequence == old.policy.current_release_sequence {
            return Err(Error::Conflict);
        }
    }
    if !known && policy.current_release_sequence <= highest_sequence {
        return Err(Error::Conflict);
    }
    Ok(())
}
