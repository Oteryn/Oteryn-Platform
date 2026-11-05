#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

use oteryn_world::{ImportMapping, WorldError, WorldOptions, WorldStore};

fn mapping() -> ImportMapping {
    ImportMapping {
        path_glob: "content/synthetic/*.json".into(),
        schema_file: "tools/content-schema/envelope.schema.json".into(),
        kind: "synthetic-entry".into(),
        key_field: "code".into(),
        records_pointer: "/payload".into(),
        schema_scope: oteryn_world::SchemaScope::Document,
        hidden_when_true: Vec::new(),
    }
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/valid")
}

// Unit validation is exercised through the same internal validator, in the crate's
// unit tests. This integration check proves gates run before any connection attempt.
#[tokio::test]
async fn disabled_and_bad_commit_refuse_without_database_access() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let disabled = WorldStore::new(pool.clone(), WorldOptions::default());
    assert!(matches!(
        disabled
            .import_directory(fixture(), &"a".repeat(40), vec![mapping()])
            .await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.active_version_id().await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.allow("kind", "key").await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.deny("kind", "key").await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.get_entity("kind", "key").await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.list_by_kind("kind", None, 10).await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.rollback().await,
        Err(WorldError::Disabled)
    ));
    assert!(matches!(
        disabled.import_report(uuid::Uuid::now_v7()).await,
        Err(WorldError::Disabled)
    ));
    let enabled = WorldStore::new(pool, WorldOptions { enabled: true });
    for bad in ["main", "", "deadbeef", &"Z".repeat(40)] {
        assert!(matches!(
            enabled
                .import_directory(fixture(), bad, vec![mapping()])
                .await,
            Err(WorldError::InvalidInput(_))
        ));
    }
    assert!(matches!(
        enabled.get_entity("", "x").await,
        Err(WorldError::InvalidInput(_))
    ));
    assert!(matches!(
        enabled.allow("kind", "bad\0key").await,
        Err(WorldError::InvalidInput(_))
    ));
    assert!(matches!(
        enabled.list_by_kind("kind", None, 0).await,
        Err(WorldError::InvalidInput(_))
    ));
    assert!(matches!(
        enabled.list_by_kind("kind", None, 1001).await,
        Err(WorldError::InvalidInput(_))
    ));
}
