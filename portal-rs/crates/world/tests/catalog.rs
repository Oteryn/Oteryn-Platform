use oteryn_world::{
    WorldOptions, WorldStore,
    catalog::{Candidate, Error, Fence, Source, verify},
};
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use uuid::Uuid;

fn source() -> Source {
    Source {
        file: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/catalog-d34807f5/game-platform-catalog.json"),
        fence: Fence {
            source_revision: "a2a5da955dd8f580c9e768c8ac6a741db388cb22".into(),
            authority_epoch: "preproduction-unallocated".into(),
            ruleset_id: "oteryn:ruleset.unbound".into(),
            content_profile_id: "oteryn:profile.unbound".into(),
            snapshot_id: "sha256:6d599c15564e1ad1b0ec307a73fa52e517a452ae6b0eab4056a0cd67e2d27f8e"
                .into(),
        },
    }
}
#[tokio::test]
async fn verified_catalogue_is_idempotent_and_cannot_publish()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED catalog PostgreSQL test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&db)
        .await?;
    assert_eq!(version, "180006");
    let schema = format!("catalog_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&db)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&db)
        .await?;
    oteryn_db::migrate(&db).await?;
    let store = WorldStore::new(db.clone(), WorldOptions { enabled: true });
    assert_eq!(store.active_version_id().await?, None);
    let source = source();
    let mut mismatched = source.clone();
    mismatched.fence.source_revision = "a".repeat(40);
    assert_eq!(mismatched.read().err(), Some(Error::ProvenanceConflict));
    let missing = Source {
        file: source.file.with_file_name("absent.json"),
        ..source.clone()
    };
    assert_eq!(missing.read().err(), Some(Error::Unavailable));
    let raw = std::fs::read(&source.file)?;
    let mut dirty = raw.clone();
    dirty.push(b' ');
    let sidecar = std::fs::read(source.file.with_extension("json.sha256"))?;
    assert_eq!(
        verify(&dirty, &sidecar, &source.fence).err(),
        Some(Error::IntegrityMismatch)
    );
    let candidate: Candidate = source.read()?;
    candidate.persist_inactive(&db).await?;
    candidate.persist_inactive(&db).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM world_catalog_candidates WHERE status='inactive'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 1);
    for table in ["world_versions", "world_entities", "world_visibility"] {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&db)
                .await?;
        assert_eq!(count, 0);
    }
    assert_eq!(store.active_version_id().await?, None);
    assert!(store.list_by_kind("Creature", None, 100).await?.is_empty());
    // JSON carrier strings may contain escaped NUL. Retain exact artifact bytes,
    // rather than normalizing through PostgreSQL JSONB (which rejects them).
    let control_file = source
        .file
        .with_file_name("synthetic-control-character.json");
    let control_raw = std::fs::read(&control_file)?;
    let value: serde_json::Value = serde_json::from_slice(&control_raw)?;
    let text = |name: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(value[name].as_str().ok_or("fixture field missing")?.into())
    };
    let control = Source {
        file: control_file,
        fence: Fence {
            source_revision: text("source_revision")?,
            authority_epoch: text("authority_epoch")?,
            ruleset_id: text("ruleset_id")?,
            content_profile_id: text("content_profile_id")?,
            snapshot_id: text("snapshot_id")?,
        },
    }
    .read()?;
    control.persist_inactive(&db).await?;
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT artifact FROM world_catalog_candidates WHERE snapshot_id=$1")
            .bind(control.snapshot_id())
            .fetch_one(&db)
            .await?;
    assert_eq!(stored, control_raw);
    assert_eq!(store.active_version_id().await?, None);
    // The additive migration is reversible, without touching old world tables.
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120001_world_catalog_candidates.down.sql"
    ))
    .execute(&db)
    .await?;
    assert_eq!(store.active_version_id().await?, None);
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120001_world_catalog_candidates.up.sql"
    ))
    .execute(&db)
    .await?;
    candidate.persist_inactive(&db).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}
