use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// All JSON files must be mapped or explicitly excluded by exact relative path.
/// Schema URI aliases resolve locally; they never enable network retrieval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportConfig {
    pub mappings: Vec<ImportMapping>,
    #[serde(default)]
    pub excluded_files: Vec<String>,
    #[serde(default)]
    pub schema_resources: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaScope {
    /// Validate the entire source document, including its envelope.
    #[default]
    Document,
    /// Validate every extracted record using the source's record schema.
    Records,
}

/// Runtime gate. Import, publication, visibility changes and reads are disabled by default.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldOptions {
    pub enabled: bool,
}

/// Source configuration, independent of any particular game's entity families.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportMapping {
    /// Root-relative glob, for example `content/synthetic/*.json`.
    pub path_glob: String,
    /// Root-relative path under `tools/content-schema/`.
    pub schema_file: String,
    pub kind: String,
    /// Object field or JSON pointer in each record. Keys are strings or integers.
    pub key_field: String,
    /// JSON pointer to an array of records, for example `/records`.
    /// An empty pointer selects a document whose root is an array.
    pub records_pointer: String,
    #[serde(default)]
    pub schema_scope: SchemaScope,
    /// Source boolean JSON pointers that veto publication even when allow-listed.
    #[serde(default)]
    pub hidden_when_true: Vec<String>,
}

/// Import diagnostics. An empty pointer identifies the document root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportIssue {
    pub file: String,
    pub pointer: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub version_id: Uuid,
    pub source_commit: String,
    /// `importing`, `ready`, or `failed`. Active state is held separately.
    pub status: VersionStatus,
    pub document_count: i64,
    pub errors: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionStatus {
    Importing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Entity {
    pub version_id: Uuid,
    pub kind: String,
    pub key: String,
    pub data: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("world data feature is disabled")]
    Disabled,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("import {version_id} failed: {issues:?}")]
    ImportFailed {
        version_id: Uuid,
        issues: Vec<ImportIssue>,
    },
    #[error("no previous world version is available")]
    NoPreviousVersion,
    // Avoid including connection information in public diagnostic text.
    #[error("world database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("world validation task failed")]
    ValidationTask(#[from] tokio::task::JoinError),
}

pub(crate) fn validate_label(value: &str, field: &str) -> Result<(), WorldError> {
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(WorldError::InvalidInput(format!(
            "{field} must contain 1 to 512 bytes without control characters"
        )));
    }
    Ok(())
}
