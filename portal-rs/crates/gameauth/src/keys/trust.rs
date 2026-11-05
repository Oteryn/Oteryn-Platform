//! Public-only registry shared with the existing native-evidence producer. State
//! witness advance precedes SQL commit; a witness-ahead ambiguity fails closed.
use super::{
    ISSUER, KeyError, PROFILE, PublicKey, Settings, SigningTrust, TrustedKey, check_settings,
    valid_id,
};
use crate::evidence::{Witness, trust_state_namespace};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sqlx::{PgPool, Row};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PublicTrustRegistry {
    pool: PgPool,
    settings: Settings,
    witness_directory: PathBuf,
}
/// A bounded self-check cache. Refresh before every signing operation (no later
/// than 5 seconds); a stale snapshot cannot authorize signing or key destruction.
#[derive(Debug)]
pub struct TrustSnapshot {
    purpose: String,
    observed_at: u64,
    keys: BTreeMap<String, TrustedKey>,
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
        if issuer != ISSUER
            || profile != PROFILE
            || purpose != self.purpose
            || now < self.observed_at
            || now - self.observed_at >= 5
        {
            return Err(KeyError::Unavailable);
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
        check_settings(&settings)?;
        if !witness_directory.is_absolute() {
            return Err(KeyError::BadInput);
        }
        Ok(Self {
            pool,
            settings,
            witness_directory,
        })
    }
    fn witness(&self) -> Result<(String, Witness), KeyError> {
        check_settings(&self.settings)?;
        let namespace = trust_state_namespace(ISSUER, PROFILE, &self.settings.key_purpose)
            .map_err(|_| KeyError::Unavailable)?;
        let witness = Witness::lock(&self.witness_directory, &namespace)
            .map_err(|_| KeyError::Unavailable)?;
        Ok((namespace, witness))
    }
    /// Idempotent publication binds a never-reused id to one public key forever.
    /// No private material is accepted by this API.
    pub async fn publish(&self, key: &PublicKey) -> Result<(), KeyError> {
        let decoded = decode_key(&key.public_key)?;
        if !valid_id(&key.key_id) {
            return Err(KeyError::BadInput);
        }
        let (namespace, witness) = self.witness()?;
        let mut tx = self.pool.begin().await.map_err(|_| KeyError::Unavailable)?;
        let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(ISSUER).bind(PROFILE).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        let floor = witness
            .floor(&namespace)
            .map_err(|_| KeyError::Unavailable)?;
        let (id, revision) = if let Some(row) = profile {
            let revision: i64 = row
                .try_get("issuer_revision")
                .map_err(|_| KeyError::Unavailable)?;
            let revoked: bool = row.try_get("revoked").map_err(|_| KeyError::Unavailable)?;
            if revoked || revision <= 0 || floor != Some(revision) {
                return Err(KeyError::InvalidState);
            }
            (
                row.try_get::<Uuid, _>("id")
                    .map_err(|_| KeyError::Unavailable)?,
                revision,
            )
        } else {
            if floor.is_some() {
                return Err(KeyError::InvalidState);
            }
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO native_game_signing_trust_profiles (id,issuer,profile,key_purpose,profile_version,issuer_revision) VALUES ($1,$2,$3,$4,1,1)")
                .bind(id).bind(ISSUER).bind(PROFILE).bind(&self.settings.key_purpose).execute(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
            (id, 0)
        };
        let reused: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM native_game_signing_trust_key_versions k JOIN native_game_signing_trust_profiles p ON p.id=k.profile_id WHERE p.issuer=$1 AND p.profile=$2 AND p.key_purpose=$3 AND k.key_id=$4 AND p.id<>$5)")
            .bind(ISSUER).bind(PROFILE).bind(&self.settings.key_purpose).bind(&key.key_id).bind(id).fetch_one(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        if reused {
            return Err(KeyError::InvalidState);
        }
        let rows = sqlx::query("SELECT DISTINCT ON (key_id) key_id, public_key, trusted FROM native_game_signing_trust_key_versions WHERE profile_id=$1 ORDER BY key_id, key_revision DESC")
            .bind(id).fetch_all(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        let mut trusted_count = 0;
        for row in rows {
            let kid: String = row.try_get("key_id").map_err(|_| KeyError::Unavailable)?;
            let encoded: String = row
                .try_get("public_key")
                .map_err(|_| KeyError::Unavailable)?;
            let trusted: bool = row.try_get("trusted").map_err(|_| KeyError::Unavailable)?;
            let public = decode_key(&encoded)?;
            if kid == key.key_id {
                return if trusted && public == decoded {
                    Ok(())
                } else {
                    Err(KeyError::InvalidState)
                };
            }
            if public == decoded {
                return Err(KeyError::InvalidState);
            }
            trusted_count += usize::from(trusted);
        }
        if trusted_count >= 2 {
            return Err(KeyError::InvalidState);
        }
        let next = revision.checked_add(1).ok_or(KeyError::InvalidState)?;
        sqlx::query("INSERT INTO native_game_signing_trust_key_versions (profile_id,key_id,key_revision,public_key,trusted,trusted_at) VALUES ($1,$2,1,$3,true,clock_timestamp())")
            .bind(id).bind(&key.key_id).bind(&key.public_key).execute(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        sqlx::query("UPDATE native_game_signing_trust_profiles SET issuer_revision=$2, updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).execute(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        witness
            .advance(&namespace, next)
            .map_err(|_| KeyError::Unavailable)?;
        tx.commit().await.map_err(|_| KeyError::Unavailable)
    }
    /// Immediate terminal revocation; key destruction is a separate custody step.
    pub async fn revoke(&self, kid: &str) -> Result<(), KeyError> {
        if !valid_id(kid) {
            return Err(KeyError::BadInput);
        }
        let (namespace, witness) = self.witness()?;
        let mut tx = self.pool.begin().await.map_err(|_| KeyError::Unavailable)?;
        let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(ISSUER).bind(PROFILE).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|_| KeyError::Unavailable)?.ok_or(KeyError::InvalidState)?;
        let id: Uuid = row.try_get("id").map_err(|_| KeyError::Unavailable)?;
        let revision: i64 = row
            .try_get("issuer_revision")
            .map_err(|_| KeyError::Unavailable)?;
        let revoked: bool = row.try_get("revoked").map_err(|_| KeyError::Unavailable)?;
        if revoked
            || revision <= 0
            || witness
                .floor(&namespace)
                .map_err(|_| KeyError::Unavailable)?
                != Some(revision)
        {
            return Err(KeyError::InvalidState);
        }
        let key = sqlx::query("SELECT key_revision, public_key, trusted, trusted_at FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 ORDER BY key_revision DESC LIMIT 1")
            .bind(id).bind(kid).fetch_optional(&mut *tx).await.map_err(|_| KeyError::Unavailable)?.ok_or(KeyError::InvalidState)?;
        if !key
            .try_get::<bool, _>("trusted")
            .map_err(|_| KeyError::Unavailable)?
        {
            return Ok(());
        }
        let key_revision: i64 = key
            .try_get("key_revision")
            .map_err(|_| KeyError::Unavailable)?;
        if key_revision <= 0 {
            return Err(KeyError::InvalidState);
        }
        sqlx::query("INSERT INTO native_game_signing_trust_key_versions (profile_id,key_id,key_revision,public_key,trusted,trusted_at,revoked_at) SELECT profile_id,key_id,key_revision+1,public_key,false,trusted_at,clock_timestamp() FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 AND key_revision=$3")
            .bind(id).bind(kid).bind(key_revision).execute(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        let next = revision.checked_add(1).ok_or(KeyError::InvalidState)?;
        sqlx::query("UPDATE native_game_signing_trust_profiles SET issuer_revision=$2, updated_at=clock_timestamp() WHERE id=$1")
            .bind(id).bind(next).execute(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        witness
            .advance(&namespace, next)
            .map_err(|_| KeyError::Unavailable)?;
        tx.commit().await.map_err(|_| KeyError::Unavailable)
    }
    /// Public trust is exposed to Game by the existing ReadFreshSigningTrustV1
    /// producer, which reads these same immutable rows and retained state witness.
    pub async fn snapshot(&self) -> Result<TrustSnapshot, KeyError> {
        let (namespace, witness) = self.witness()?;
        let mut tx = self.pool.begin().await.map_err(|_| KeyError::Unavailable)?;
        let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
            .bind(ISSUER).bind(PROFILE).bind(&self.settings.key_purpose).fetch_optional(&mut *tx).await.map_err(|_| KeyError::Unavailable)?.ok_or(KeyError::InvalidState)?;
        let revision: i64 = row
            .try_get("issuer_revision")
            .map_err(|_| KeyError::Unavailable)?;
        if revision <= 0
            || witness
                .floor(&namespace)
                .map_err(|_| KeyError::Unavailable)?
                != Some(revision)
        {
            return Err(KeyError::InvalidState);
        }
        let id: Uuid = row.try_get("id").map_err(|_| KeyError::Unavailable)?;
        let revoked: bool = row.try_get("revoked").map_err(|_| KeyError::Unavailable)?;
        let rows = sqlx::query("SELECT DISTINCT ON (key_id) key_id, public_key, trusted, ceil(extract(epoch FROM trusted_at))::bigint AS trusted_since FROM native_game_signing_trust_key_versions WHERE profile_id=$1 ORDER BY key_id, key_revision DESC")
            .bind(id).fetch_all(&mut *tx).await.map_err(|_| KeyError::Unavailable)?;
        let mut keys = BTreeMap::new();
        for row in rows {
            if !revoked
                && row
                    .try_get::<bool, _>("trusted")
                    .map_err(|_| KeyError::Unavailable)?
            {
                let encoded: String = row
                    .try_get("public_key")
                    .map_err(|_| KeyError::Unavailable)?;
                let since: i64 = row
                    .try_get("trusted_since")
                    .map_err(|_| KeyError::Unavailable)?;
                keys.insert(
                    row.try_get("key_id").map_err(|_| KeyError::Unavailable)?,
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
                .map_err(|_| KeyError::Unavailable)?;
        tx.commit().await.map_err(|_| KeyError::Unavailable)?;
        Ok(TrustSnapshot {
            purpose: self.settings.key_purpose.clone(),
            observed_at: u64::try_from(now).map_err(|_| KeyError::InvalidState)?,
            keys,
        })
    }
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
