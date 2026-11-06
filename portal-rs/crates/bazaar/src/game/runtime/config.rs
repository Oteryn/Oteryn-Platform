use super::super::game_wire::{decimal, id};
use super::super::{Authority, AuthorityConfig, Error, PINNED_CONTRACT_SHA256, RecoveryFence};
use ed25519_dalek::VerifyingKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, path::PathBuf};
use uuid::Uuid;

pub const ENABLED: &str = "OTERYN_BAZAAR_ENABLED";
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fence {
    pub projection_epoch: u64,
    pub recovery_generation: u64,
    pub incarnation: Uuid,
    pub character_history_complete_since: Option<u64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotKey {
    kid: String,
    public_key_hex: String,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub authority: Authority,
    pub fence: Fence,
    pub fence_file: PathBuf,
    pub keys_file: PathBuf,
    keys_hash: [u8; 32],
    pub bind: SocketAddr,
    pub base_url: String,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub client_ca: PathBuf,
    pub bazaar_spki: [u8; 32],
    pub receipt_spki: [u8; 32],
    pub escrow_account: Option<Uuid>,
    pub worlds: Vec<Uuid>,
    pub public_origin: String,
    pub cooldown_seconds: u64,
    pub financial_retention_days: u64,
}
fn env(key: &str) -> Result<String, Error> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(Error::Unavailable)
}
pub(super) fn hex(raw: &str) -> Result<[u8; 32], Error> {
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Invalid);
    }
    let mut result = [0; 32];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).map_err(|_| Error::Invalid)?;
    }
    Ok(result)
}
impl Config {
    pub fn from_env() -> Result<Option<Self>, Error> {
        if std::env::var(ENABLED).as_deref() != Ok("true") {
            return Ok(None);
        }
        let fence_file = PathBuf::from(env("OTERYN_BAZAAR_FENCE_FILE")?);
        let fence: Fence =
            serde_json::from_slice(&std::fs::read(&fence_file).map_err(|_| Error::Unavailable)?)
                .map_err(|_| Error::Invalid)?;
        id(&fence.incarnation.to_string())?;
        if fence.projection_epoch == 0 || fence.recovery_generation == 0 {
            return Err(Error::Unavailable);
        }
        let keys_file = PathBuf::from(env("OTERYN_BAZAAR_SNAPSHOT_KEYS_FILE")?);
        let keys_bytes = std::fs::read(&keys_file).map_err(|_| Error::Unavailable)?;
        let keys: Vec<SnapshotKey> =
            serde_json::from_slice(&keys_bytes).map_err(|_| Error::Invalid)?;
        let snapshot_keys = keys
            .into_iter()
            .map(|key| {
                Ok((
                    key.kid,
                    VerifyingKey::from_bytes(&hex(&key.public_key_hex)?)
                        .map_err(|_| Error::Invalid)?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut authority = Authority {
            config: AuthorityConfig {
                enabled: true,
                contract_sha256: Some(PINNED_CONTRACT_SHA256),
                v2_receipts_accepted: true,
                eligibility_and_escrow_accepted: true,
                inbox_and_item_classes_accepted: true,
                gold_fee_and_headroom_accepted: true,
                common_recovery_scope_qualified: true,
                signing_and_retention_accepted: true,
                escrow_account_id: None,
                character_cooldown_seconds: None,
                character_history_complete_since: fence.character_history_complete_since,
                receipt_authority: env("OTERYN_BAZAAR_RECEIPT_AUTHORITY")?,
                projection_authority: env("OTERYN_BAZAAR_SOURCE_AUTHORITY")?,
                snapshot_keys,
            },
            fence: RecoveryFence {
                projection_epoch: fence.projection_epoch,
                recovery_generation: fence.recovery_generation,
            },
        };
        authority.enabled()?;
        let bind = env("OTERYN_BAZAAR_INTERNAL_BIND")?
            .parse::<SocketAddr>()
            .map_err(|_| Error::Invalid)?;
        let base_url = env("OTERYN_BAZAAR_INTERNAL_BASE_URL")?;
        let uri: axum::http::Uri = base_url.parse().map_err(|_| Error::Invalid)?;
        if uri.scheme_str() != Some("https")
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || uri.path() != "/"
            || uri.query().is_some()
        {
            return Err(Error::Invalid);
        }
        let escrow_account = std::env::var("OTERYN_BAZAAR_ESCROW_ACCOUNT_ID")
            .ok()
            .map(|v| id(&v))
            .transpose()?;
        authority.config.escrow_account_id = escrow_account;
        authority.config.character_cooldown_seconds = Some(
            std::env::var("OTERYN_BAZAAR_COOLDOWN_SECONDS")
                .map_or(Ok(30 * 86400), |v| decimal(&v, false, u64::MAX))?,
        );
        let mut worlds = env("OTERYN_BAZAAR_WORLD_IDS")?
            .split(',')
            .map(id)
            .collect::<Result<Vec<_>, _>>()?;
        worlds.sort();
        worlds.dedup();
        if worlds.is_empty() {
            return Err(Error::Unavailable);
        }
        let bazaar_spki = hex(&env("OTERYN_BAZAAR_CLIENT_SPKI_SHA256")?)?;
        let receipt_spki = hex(&env("OTERYN_BAZAAR_RECEIPT_CLIENT_SPKI_SHA256")?)?;
        if bazaar_spki == receipt_spki {
            return Err(Error::Denied);
        }
        Ok(Some(Self {
            authority,
            fence,
            fence_file,
            keys_file,
            keys_hash: Sha256::digest(keys_bytes).into(),
            bind,
            base_url,
            certificate: env("OTERYN_BAZAAR_TLS_CERT_FILE")?.into(),
            private_key: env("OTERYN_BAZAAR_TLS_KEY_FILE")?.into(),
            client_ca: env("OTERYN_BAZAAR_CLIENT_CA_FILE")?.into(),
            bazaar_spki,
            receipt_spki,
            escrow_account,
            worlds,
            public_origin: env("OTERYN_PUBLIC_ORIGIN")?,
            cooldown_seconds: std::env::var("OTERYN_BAZAAR_COOLDOWN_SECONDS")
                .map_or(Ok(30 * 86400), |v| decimal(&v, false, u64::MAX))?,
            financial_retention_days: std::env::var("OTERYN_BAZAAR_FINANCIAL_RETENTION_DAYS")
                .map_or(Ok(3650), |v| decimal(&v, false, 36500))?,
        }))
    }
    /// A trust/fence file change suspends this incarnation until a fresh process
    /// reconciles it. File contents never come from HTTP or a restored database.
    pub fn current(&self) -> Result<(), Error> {
        let fence: Fence = serde_json::from_slice(
            &std::fs::read(&self.fence_file).map_err(|_| Error::Unavailable)?,
        )
        .map_err(|_| Error::Invalid)?;
        let keys = std::fs::read(&self.keys_file).map_err(|_| Error::Unavailable)?;
        if fence != self.fence || <[u8; 32]>::from(Sha256::digest(keys)) != self.keys_hash {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
}
