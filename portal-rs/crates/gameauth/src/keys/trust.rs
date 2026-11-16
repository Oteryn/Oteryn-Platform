//! Public-only registry shared with the existing native-evidence producer. State
//! witness advance precedes SQL commit; a witness-ahead ambiguity fails closed.
use super::{
    CauseKind, KeyError, PublicKey, Scope, Settings, SigningTrust, TrustedKey, check_settings, sql,
    unavailable, valid_id,
};
use crate::evidence::{AsyncWitness, WitnessFault, trust_state_namespace};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sqlx::{PgConnection, PgPool, Row, postgres::PgRow};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PublicTrustRegistry {
    deadline: Option<tokio::time::Instant>,
    pool: PgPool,
    settings: Settings,
    scope: Scope,
    witness_directory: PathBuf,
}
/// A bounded self-check cache. Refresh before every signing operation (no later
/// than 5 seconds); a stale snapshot cannot authorize signing or key destruction.
#[derive(Debug, Clone)]
pub struct TrustSnapshot {
    scope: Scope,
    purpose: String,
    observed_at: u64,
    keys: BTreeMap<String, TrustedKey>,
}
#[cfg(test)]
impl TrustSnapshot {
    pub(super) fn observed_at(&self) -> u64 {
        self.observed_at
    }
}
impl SigningTrust for TrustSnapshot {
    fn trusted_key(
        &self,
        issuer: &str,
        profile: &str,
        purpose: &str,
        kid: &str,
        now: u64,
    ) -> Result<Option<TrustedKey>, KeyError> {
        if issuer != self.scope.issuer()
            || profile != self.scope.profile()
            || purpose != self.purpose
        {
            return Err(unavailable(CauseKind::Mismatch));
        }
        if now < self.observed_at || now - self.observed_at >= 5 {
            return Err(unavailable(CauseKind::Stale));
        }
        Ok(self.keys.get(kid).copied())
    }
}
impl PublicTrustRegistry {
    pub fn new(
        pool: PgPool,
        settings: Settings,
        witness_directory: PathBuf,
    ) -> Result<Self, KeyError> {
        Self::new_scoped(pool, settings, Scope::Fresh, witness_directory)
    }
    /// `new` for the trust scope `scope` (§19.5).
    pub fn new_scoped(
        pool: PgPool,
        settings: Settings,
        scope: Scope,
        witness_directory: PathBuf,
    ) -> Result<Self, KeyError> {
        check_settings(&settings)?;
        scope.check(&settings)?;
        if !witness_directory.is_absolute() {
            return Err(KeyError::BadInput);
        }
        Ok(Self {
            deadline: None,
            pool,
            settings,
            scope,
            witness_directory,
        })
    }
    /// Applies the caller's single deadline to server-side statement and lock
    /// waits too; transaction-local settings never leak into pooled sessions.
    pub fn with_deadline(mut self, deadline: tokio::time::Instant) -> Self {
        self.deadline = Some(deadline);
        self
    }
    async fn bound_waits(&self, connection: &mut PgConnection) -> Result<(), KeyError> {
        if let Some(deadline) = self.deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(unavailable(CauseKind::Deadline));
            }
            let milliseconds = remaining.as_millis().saturating_add(1);
            sqlx::query("SELECT set_config('statement_timeout', $1, true), set_config('lock_timeout', $1, true)")
                .bind(format!("{milliseconds}ms"))
                .execute(connection).await.map_err(|e| sql(&e))?;
        }
        Ok(())
    }
    async fn witness(&self) -> Result<(String, AsyncWitness), KeyError> {
        check_settings(&self.settings)?;
        let namespace = trust_state_namespace(
            self.scope.issuer(),
            self.scope.profile(),
            &self.settings.key_purpose,
        )
        .map_err(|_| unavailable(CauseKind::Integrity))?;
        let witness = AsyncWitness::lock_detailed(&self.witness_directory, &namespace)
            .await
            .map_err(|fault| witness_fault(&fault))?;
        Ok((namespace, witness))
    }
    /// Idempotent publication binds a never-reused id to one public key forever.
    /// No private material is accepted by this API.
    pub async fn publish(&self, key: &PublicKey) -> Result<(), KeyError> {
        let decoded = decode_key(&key.public_key)?;
        if !valid_id(&key.key_id) {
            return Err(KeyError::BadInput);
        }
        let (namespace, witness) = self.witness().await?;
        let mut tx = self.pool.begin().await.map_err(|e| sql(&e))?;
        self.bound_waits(&mut tx).await?;
        // One transaction lock for both scopes, including their first publication.
        sqlx::query("SELECT pg_advisory_xact_lock(715110005)")
            .execute(&mut *tx)
            .await
            .map_err(|e| sql(&e))?;
        let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(self.scope.issuer()).bind(self.scope.profile()).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|e| sql(&e))?;
        let floor = witness
            .floor_detailed(&namespace)
            .await
            .map_err(|fault| witness_fault(&fault))?;
        let (id, revision) = if let Some(row) = profile {
            let revision: i64 = row
                .try_get("issuer_revision")
                .map_err(|_| unavailable(CauseKind::Decode))?;
            let revoked: bool = row
                .try_get("revoked")
                .map_err(|_| unavailable(CauseKind::Decode))?;
            if revoked || revision <= 0 || floor != Some(revision) {
                return Err(KeyError::InvalidState);
            }
            (
                row.try_get::<Uuid, _>("id")
                    .map_err(|_| unavailable(CauseKind::Decode))?,
                revision,
            )
        } else {
            if floor.is_some() {
                return Err(KeyError::InvalidState);
            }
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO native_game_signing_trust_profiles (id,issuer,profile,key_purpose,profile_version,issuer_revision) VALUES ($1,$2,$3,$4,1,1)")
                .bind(id).bind(self.scope.issuer()).bind(self.scope.profile()).bind(&self.settings.key_purpose).execute(&mut *tx).await.map_err(|e| sql(&e))?;
            (id, 0)
        };
        // §19.5: a kid or key pair is never published in both scopes.
        let reused: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM native_game_signing_trust_key_versions k JOIN native_game_signing_trust_profiles p ON p.id=k.profile_id WHERE p.id<>$5 AND ((p.issuer=$1 AND p.profile=$2 AND p.key_purpose=$3 AND k.key_id=$4) OR (p.issuer<>$1 AND (k.key_id=$4 OR k.public_key=$6))))")
            .bind(self.scope.issuer()).bind(self.scope.profile()).bind(&self.settings.key_purpose).bind(&key.key_id).bind(id).bind(&key.public_key).fetch_one(&mut *tx).await.map_err(|e| sql(&e))?;
        if reused {
            return Err(KeyError::InvalidState);
        }
        let existing = sqlx::query("SELECT public_key, trusted FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 ORDER BY key_revision DESC LIMIT 1")
            .bind(id).bind(&key.key_id).fetch_optional(&mut *tx).await.map_err(|e| sql(&e))?;
        if let Some(row) = existing {
            let encoded: String = row
                .try_get("public_key")
                .map_err(|_| unavailable(CauseKind::Decode))?;
            return if row
                .try_get::<bool, _>("trusted")
                .map_err(|_| unavailable(CauseKind::Decode))?
                && decode_key(&encoded)? == decoded
            {
                Ok(())
            } else {
                Err(KeyError::InvalidState)
            };
        }
        let rebound: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND public_key=$2)")
            .bind(id).bind(&key.public_key).fetch_one(&mut *tx).await.map_err(|e| sql(&e))?;
        if rebound {
            return Err(KeyError::InvalidState);
        }
        let trusted_count: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT DISTINCT ON (key_id) trusted FROM native_game_signing_trust_key_versions WHERE profile_id=$1 ORDER BY key_id, key_revision DESC) latest WHERE trusted")
            .bind(id).fetch_one(&mut *tx).await.map_err(|e| sql(&e))?;
        if trusted_count >= 2 {
            return Err(KeyError::InvalidState);
        }
        let next = revision.checked_add(1).ok_or(KeyError::InvalidState)?;
        sqlx::query("INSERT INTO native_game_signing_trust_key_versions (profile_id,key_id,key_revision,public_key,trusted,trusted_at) VALUES ($1,$2,1,$3,true,clock_timestamp())")
            .bind(id).bind(&key.key_id).bind(&key.public_key).execute(&mut *tx).await.map_err(|e| sql(&e))?;
        sqlx::query("UPDATE native_game_signing_trust_profiles SET issuer_revision=$2, updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).execute(&mut *tx).await.map_err(|e| sql(&e))?;
        witness
            .advance_detailed(&namespace, next)
            .await
            .map_err(|fault| witness_fault(&fault))?;
        tx.commit().await.map_err(|e| sql(&e))
    }
    /// Immediate terminal revocation; key destruction is a separate custody step.
    pub async fn revoke(&self, kid: &str) -> Result<(), KeyError> {
        if !valid_id(kid) {
            return Err(KeyError::BadInput);
        }
        let (namespace, witness) = self.witness().await?;
        let mut tx = self.pool.begin().await.map_err(|e| sql(&e))?;
        self.bound_waits(&mut tx).await?;
        let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(self.scope.issuer()).bind(self.scope.profile()).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|e| sql(&e))?.ok_or(KeyError::InvalidState)?;
        let id: Uuid = row
            .try_get("id")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        let revision: i64 = row
            .try_get("issuer_revision")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        let revoked: bool = row
            .try_get("revoked")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        if revoked
            || revision <= 0
            || witness
                .floor_detailed(&namespace)
                .await
                .map_err(|fault| witness_fault(&fault))?
                != Some(revision)
        {
            return Err(KeyError::InvalidState);
        }
        let key = sqlx::query("SELECT key_revision, public_key, trusted, trusted_at FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 ORDER BY key_revision DESC LIMIT 1")
            .bind(id).bind(kid).fetch_optional(&mut *tx).await.map_err(|e| sql(&e))?.ok_or(KeyError::InvalidState)?;
        if !key
            .try_get::<bool, _>("trusted")
            .map_err(|_| unavailable(CauseKind::Decode))?
        {
            return Ok(());
        }
        let key_revision: i64 = key
            .try_get("key_revision")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        if key_revision <= 0 {
            return Err(KeyError::InvalidState);
        }
        sqlx::query("INSERT INTO native_game_signing_trust_key_versions (profile_id,key_id,key_revision,public_key,trusted,trusted_at,revoked_at) SELECT profile_id,key_id,key_revision+1,public_key,false,trusted_at,clock_timestamp() FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 AND key_revision=$3")
            .bind(id).bind(kid).bind(key_revision).execute(&mut *tx).await.map_err(|e| sql(&e))?;
        let next = revision.checked_add(1).ok_or(KeyError::InvalidState)?;
        sqlx::query("UPDATE native_game_signing_trust_profiles SET issuer_revision=$2, updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).execute(&mut *tx).await.map_err(|e| sql(&e))?;
        witness
            .advance_detailed(&namespace, next)
            .await
            .map_err(|fault| witness_fault(&fault))?;
        tx.commit().await.map_err(|e| sql(&e))
    }
    /// Public trust is exposed to Game by the existing ReadFreshSigningTrustV1
    /// producer, which reads these same immutable rows and retained state witness.
    pub async fn snapshot(&self) -> Result<TrustSnapshot, KeyError> {
        let mut tx = self.pool.begin().await.map_err(|e| sql(&e))?;
        self.snapshot_in(&mut tx).await
    }
    pub(crate) async fn snapshot_in(
        &self,
        tx: &mut PgConnection,
    ) -> Result<TrustSnapshot, KeyError> {
        let (namespace, witness) = self.witness().await?;
        self.bound_waits(tx).await?;
        let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(self.scope.issuer()).bind(self.scope.profile()).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|e| sql(&e))?.ok_or(KeyError::InvalidState)?;
        let revision: i64 = row
            .try_get("issuer_revision")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        if revision <= 0
            || witness
                .floor_detailed(&namespace)
                .await
                .map_err(|fault| witness_fault(&fault))?
                != Some(revision)
        {
            return Err(KeyError::InvalidState);
        }
        let id: Uuid = row
            .try_get("id")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        let revoked: bool = row
            .try_get("revoked")
            .map_err(|_| unavailable(CauseKind::Decode))?;
        let rows = latest_trusted(tx, id).await?;
        let mut keys = BTreeMap::new();
        for row in rows {
            if !revoked
                && row
                    .try_get::<bool, _>("trusted")
                    .map_err(|_| unavailable(CauseKind::Decode))?
            {
                let encoded: String = row
                    .try_get("public_key")
                    .map_err(|_| unavailable(CauseKind::Decode))?;
                let since: i64 = row
                    .try_get("trusted_since")
                    .map_err(|_| unavailable(CauseKind::Decode))?;
                keys.insert(
                    row.try_get("key_id")
                        .map_err(|_| unavailable(CauseKind::Decode))?,
                    TrustedKey {
                        public_key: decode_key(&encoded)?,
                        trusted_since: u64::try_from(since).map_err(|_| KeyError::InvalidState)?,
                    },
                );
            }
        }
        if keys.len() > 2 {
            return Err(KeyError::InvalidState);
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| sql(&e))?;
        Ok(TrustSnapshot {
            scope: self.scope,
            purpose: self.settings.key_purpose.clone(),
            observed_at: u64::try_from(now).map_err(|_| KeyError::InvalidState)?,
            keys,
        })
    }
}
/// The witness fault as an `Unavailable` cause at the caller's source line.
#[track_caller]
fn witness_fault(fault: &WitnessFault) -> KeyError {
    unavailable(match *fault {
        WitnessFault::LockBusy => CauseKind::LockBusy,
        WitnessFault::Io(kind) => CauseKind::Io(kind),
        WitnessFault::Blocking => CauseKind::Blocking,
        WitnessFault::Integrity => CauseKind::Integrity,
        WitnessFault::Entropy => CauseKind::Entropy,
    })
}
pub(super) fn decode_key(encoded: &str) -> Result<[u8; 32], KeyError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| KeyError::BadInput)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != encoded {
        return Err(KeyError::BadInput);
    }
    bytes.try_into().map_err(|_| KeyError::BadInput)
}

/// Fetch at most two trusted keys plus an overflow witness; retired history stays in SQL.
pub(super) async fn latest_trusted(
    tx: &mut PgConnection,
    id: Uuid,
) -> Result<Vec<PgRow>, KeyError> {
    sqlx::query("SELECT key_id, public_key, trusted, trusted_since FROM (SELECT DISTINCT ON (key_id) key_id, public_key, trusted, ceil(extract(epoch FROM trusted_at))::bigint AS trusted_since FROM native_game_signing_trust_key_versions WHERE profile_id=$1 ORDER BY key_id, key_revision DESC) latest WHERE trusted LIMIT 3")
            .bind(id).fetch_all(tx).await.map_err(|e| sql(&e))
}
