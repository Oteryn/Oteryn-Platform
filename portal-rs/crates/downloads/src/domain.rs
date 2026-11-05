use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Game integration remains unavailable unless explicitly enabled by the caller.
#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureFlags {
    pub downloads: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            _ => Err(Error::Invalid("channel")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }

    pub fn policy_path(self) -> String {
        format!("channels/{}/policy-v1.json", self.as_str())
    }
}

/// Only a positively established active state permits staff mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Inactive,
    Unknown,
    Ambiguous,
}

/// The account and RBAC owners implement this adapter later. Errors must map to denial.
#[async_trait]
pub trait PublishAccess: Send + Sync {
    async fn account_state(&self, account_id: Uuid) -> AccountState;
    async fn may_publish(&self, account_id: Uuid, channel: Channel) -> Option<bool>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("downloads are disabled")]
    Disabled,
    #[error("publication denied")]
    Denied,
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error("untrusted or inconsistent signed metadata")]
    Verification,
    #[error("publication would replay or rewrite trusted history")]
    Conflict,
    #[error("download not found")]
    NotFound,
    #[error("download storage failed")]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    Optional,
    Recommended,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackAuthorization {
    None,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub platform: String,
    pub architecture: String,
    pub target_path: String,
}

/// Platform policy schema v1; the document is a TUF-authenticated target, not an unsigned authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdaterPolicy {
    pub schema_version: u32,
    pub policy_revision: i64,
    pub channel: Channel,
    pub current_release_id: String,
    pub current_release_sequence: i64,
    pub current_version_display: String,
    pub minimum_supported_release_sequence: i64,
    pub update_mode: UpdateMode,
    pub artifacts: Vec<Artifact>,
    pub revoked_release_ids: Vec<String>,
    pub revoked_artifact_targets: Vec<String>,
    pub rollback_authorization: RollbackAuthorization,
}

impl UpdaterPolicy {
    pub fn validate(&self, channel: Channel) -> Result<(), Error> {
        if self.schema_version != 1
            || self.channel != channel
            || self.policy_revision <= 0
            || self.current_release_sequence <= 0
            || self.minimum_supported_release_sequence <= 0
            || self.minimum_supported_release_sequence > self.current_release_sequence
            || !identifier(&self.current_release_id)
            || self.current_version_display.is_empty()
            || self.current_version_display.len() > 96
            || self.current_version_display.chars().any(char::is_control)
            || self.artifacts.is_empty()
            || self.artifacts.len() > 32
            || self.revoked_release_ids.len() > 1024
            || self.revoked_artifact_targets.len() > 1024
            || self.revoked_release_ids.contains(&self.current_release_id)
        {
            return Err(Error::Invalid("policy"));
        }
        let prefix = format!(
            "channels/{}/releases/{}/",
            channel.as_str(),
            self.current_release_id
        );
        let mut variants = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            if !matches!(artifact.platform.as_str(), "windows" | "linux" | "macos")
                || !matches!(artifact.architecture.as_str(), "x86_64" | "arm64" | "x86")
                || !safe_path(&artifact.target_path)
                || !artifact.target_path.starts_with(&prefix)
                || self
                    .revoked_artifact_targets
                    .contains(&artifact.target_path)
                || !variants.insert((&artifact.platform, &artifact.architecture))
                || !paths.insert(&artifact.target_path)
            {
                return Err(Error::Invalid("artifact"));
            }
        }
        let channel_prefix = format!("channels/{}/", channel.as_str());
        if self.revoked_release_ids.iter().any(|id| !identifier(id))
            || self
                .revoked_artifact_targets
                .iter()
                .any(|path| !safe_path(path) || !path.starts_with(&channel_prefix))
        {
            return Err(Error::Invalid("revocation"));
        }
        Ok(())
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && !matches!(value, "." | "..")
}

pub(crate) fn safe_path(value: &str) -> bool {
    value.len() <= 512 && !value.is_empty() && value.split('/').all(identifier)
}

/// Exact uploaded bytes. Metadata filenames use consistent-snapshot version prefixes.
/// Artifact packages are never uploaded, fetched or proxied by the portal.
#[derive(Debug, Clone)]
pub struct Upload {
    pub channel: Channel,
    pub metadata: BTreeMap<String, Vec<u8>>,
    pub policy: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Versions {
    pub root: u64,
    pub targets: u64,
    pub delegated: u64,
    pub snapshot: u64,
    pub timestamp: u64,
}
