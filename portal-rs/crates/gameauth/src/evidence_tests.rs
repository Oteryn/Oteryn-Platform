use super::*;
use crate::runtime::tests::{self as support, ACCOUNT, Database};
use axum::{
    body::to_bytes,
    http::{StatusCode, header},
};
use tower::ServiceExt;

use support::TestResult;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Result<Self, std::io::Error> {
        let path = std::env::temp_dir().join(format!(
            "portal-evidence-{}",
            sqlx::types::Uuid::now_v7().simple()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn settings(directory: &Directory) -> Settings {
    Settings {
        enabled: true,
        peer_identity: oteryn_internal_api::guard::Identity::san("evidence").ok(),
        activated: true,
        witness_directory: directory.0.clone(),
        source_authority: "platform".into(),
        fresh_account_purpose: "security".into(),
        fresh_account_scope: "admission".into(),
        fresh_key_purpose: "admission".into(),
        ..Settings::default()
    }
}
fn account(operation: &str) -> Value {
    let recovery = operation == RECOVERY_ACCOUNT;
    object([
        ("version", Value::Unsigned(if recovery { 2 } else { 1 })),
        ("operation", string(operation)),
        ("account_id", string(ACCOUNT)),
        (
            "purpose",
            string(if recovery {
                "platform_security"
            } else {
                "security"
            }),
        ),
        (
            "scope",
            string(if recovery {
                "existing_actor_recovery"
            } else {
                "admission"
            }),
        ),
    ])
}
fn trust(recovery: bool) -> Value {
    object([
        ("version", Value::Unsigned(if recovery { 2 } else { 1 })),
        (
            "operation",
            string(if recovery {
                RECOVERY_TRUST
            } else {
                FRESH_TRUST
            }),
        ),
        (
            "issuer",
            string(if recovery {
                "urn:oteryn:platform:game-recovery"
            } else {
                "urn:oteryn:platform:game-admission"
            }),
        ),
        (
            "profile",
            string(if recovery {
                "oteryn-reauth-recovery-v1"
            } else {
                "oteryn-pre-admission-v1"
            }),
        ),
        (
            "key_purpose",
            string(if recovery {
                "existing_actor_recovery"
            } else {
                "admission"
            }),
        ),
        ("key_id", string("key-1")),
    ])
}
async fn sources(db: &Database) -> TestResult {
    // Accounts come from the embedded migrations; public trust is fixture-only.
    sqlx::raw_sql("CREATE TABLE native_game_signing_trust_profiles (id uuid PRIMARY KEY,issuer text,profile text,key_purpose text,profile_version bigint,issuer_revision bigint,revoked_at timestamptz); CREATE TABLE native_game_signing_trust_key_versions (profile_id uuid,key_id text,key_revision bigint,public_key text,trusted boolean);").execute(&db.pool).await?;
