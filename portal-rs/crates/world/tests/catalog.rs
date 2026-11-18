#![allow(clippy::expect_used)]
use oteryn_world::{
    WorldOptions, WorldStore,
    catalog::{Candidate, Error, Fence, Source, verify},
};
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use uuid::Uuid;

#[path = "support/catalog.rs"]
mod catalog_fixture;
fn source(file: PathBuf, fixture: &catalog_fixture::Fixture) -> Source {
    std::fs::write(&file, &fixture.bytes).expect("test artifact");
    Source::local(
        file,
        Fence {
            source_revision: fixture.text("source_revision"),
            authority_epoch: fixture.text("authority_epoch"),
            ruleset_id: fixture.text("ruleset_id"),
            content_profile_id: fixture.text("content_profile_id"),
            snapshot_id: fixture.text("snapshot_id"),
        },
        fixture.sha256(),
    )
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
    let directory = tempfile::tempdir()?;
    let source = source(
        directory.path().join("negative.json"),
        &catalog_fixture::negative(),
    );
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
    let sidecar = format!("{}\n", source.expected_sha256).into_bytes();
    assert_eq!(
        verify(&dirty, &sidecar, &source.fence).err(),
        Some(Error::IntegrityMismatch)
    );
    let candidate: Candidate = source.read()?;
    candidate.persist_inactive(&db).await?;
    candidate.persist_inactive(&db).await?;
    assert_eq!(
        candidate.require_complete(&["item"]),
        Err(Error::UnsupportedCapability)
    );
    assert_eq!(
        candidate.cached_entities(&db, "item").await.err(),
        Some(Error::UnsupportedCapability)
    );
    let records: i64 = sqlx::query_scalar("SELECT count(*) FROM world_catalog_records")
        .fetch_one(&db)
        .await?;
    assert_eq!(records, 0);
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
    let fixture = catalog_fixture::control_character();
    let control_raw = fixture.bytes.clone();
    let control = crate::source(directory.path().join("control.json"), &fixture).read()?;
    control.persist_inactive(&db).await?;
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT artifact FROM world_catalog_candidates WHERE snapshot_id=$1")
            .bind(control.snapshot_id())
            .fetch_one(&db)
            .await?;
    assert_eq!(stored, control_raw);
    let entities = control.cached_entities(&db, "item").await?;
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].content_key, "oteryn:item.training_sword");
    assert_eq!(entities[0].data["name"], "Control\0Name");
    assert_eq!(
        control.cached_entities(&db, "npc").await.err(),
        Some(Error::UnsupportedCapability)
    );
    // Failure halfway through a rebuild must preserve the previous cache.
    sqlx::raw_sql("CREATE FUNCTION reject_catalog_record() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test rejection'; END $$; CREATE TRIGGER reject_catalog_record BEFORE INSERT ON world_catalog_records FOR EACH ROW EXECUTE FUNCTION reject_catalog_record();").execute(&db).await?;
    assert_eq!(control.persist_inactive(&db).await, Err(Error::Unavailable));
    assert_eq!(control.cached_entities(&db, "item").await?.len(), 1);
    sqlx::raw_sql("DROP TRIGGER reject_catalog_record ON world_catalog_records; DROP FUNCTION reject_catalog_record();").execute(&db).await?;
    sqlx::query("DELETE FROM world_catalog_records WHERE snapshot_id=$1")
        .bind(control.snapshot_id())
        .execute(&db)
        .await?;
    assert_eq!(
        control.cached_entities(&db, "item").await.err(),
        Some(Error::IntegrityMismatch)
    );
    control.persist_inactive(&db).await?;
    assert_eq!(control.cached_entities(&db, "item").await?.len(), 1);
    let search_path = schema.clone();
    let parallel = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search_path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let (a, b, c, d) = tokio::join!(
        control.persist_inactive(&parallel),
        control.persist_inactive(&parallel),
        control.persist_inactive(&parallel),
        control.persist_inactive(&parallel)
    );
    a?;
    b?;
    c?;
    d?;
    parallel.close().await;
    assert_eq!(control.cached_entities(&db, "item").await?.len(), 1);
    assert_eq!(store.active_version_id().await?, None);
    // Rollback must remove only catalogue-owned versions and their dependents,
    // preserving legacy data and the independent identity approval store.
    let fixture = catalog_fixture::supported();
    let published = crate::source(directory.path().join("published.json"), &fixture).read()?;
    published.persist_inactive(&db).await?;
    published.publish(&db).await?;
    let native: Uuid =
        sqlx::query_scalar("SELECT version_id FROM world_catalog_versions WHERE snapshot_id=$1")
            .bind(published.snapshot_id())
            .fetch_one(&db)
            .await?;
    let legacy = Uuid::now_v7();
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
        .bind(legacy)
        .bind("c".repeat(40))
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'Item','legacy-only','{\"name\":\"Legacy only\"}',true)").bind(legacy).execute(&db).await?;
    store.allow("Item", "legacy-only").await?;
    sqlx::query(
        "UPDATE world_active_version SET version_id=$1,previous_version_id=$2 WHERE singleton",
    )
    .bind(legacy)
    .bind(native)
    .execute(&db)
    .await?;
    let legacy_puzzle = Uuid::now_v7();
    let native_puzzle = Uuid::now_v7();
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,'rollback@local.test','non-loginable-test','active','en')").bind(account).execute(&db).await?;
    sqlx::query("INSERT INTO puzzles_daily(id,day,kind,world_version,definition) VALUES($1,'2026-11-01','quiz',$2,'{}'),($3,'2026-11-02','quiz',$4,'{}')")
        .bind(legacy_puzzle).bind(legacy).bind(native_puzzle).bind(native).execute(&db).await?;
    sqlx::query("INSERT INTO puzzles_answers(account_id,puzzle_id,choice,correct) VALUES($1,$2,0,false),($1,$3,0,false)").bind(account).bind(legacy_puzzle).bind(native_puzzle).execute(&db).await?;
    // The additive migration is reversible, without touching old world tables.
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120003_world_catalog_publication.down.sql"
    ))
    .execute(&db)
    .await?;
    let remaining: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM world_versions")
        .fetch_all(&db)
        .await?;
    assert_eq!(remaining, vec![legacy]);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM world_entities WHERE version_id=$1")
        .bind(native)
        .fetch_one(&db)
        .await?;
    assert_eq!(rows, 0);
    assert_eq!(store.active_version_id().await?, Some(legacy));
    let previous: Option<Uuid> =
        sqlx::query_scalar("SELECT previous_version_id FROM world_active_version WHERE singleton")
            .fetch_one(&db)
            .await?;
    assert_eq!(previous, None);
    assert!(store.get_entity("Item", "legacy-only").await?.is_some());
    let puzzles: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM puzzles_daily")
        .fetch_all(&db)
        .await?;
    assert_eq!(puzzles, vec![legacy_puzzle]);
    let answers: Vec<Uuid> = sqlx::query_scalar("SELECT puzzle_id FROM puzzles_answers")
        .fetch_all(&db)
        .await?;
    assert_eq!(answers, vec![legacy_puzzle]);
    // The legacy immutable guard is restored after the scoped rollback cleanup.
    assert!(
        sqlx::query("DELETE FROM puzzles_daily WHERE id=$1")
            .bind(legacy_puzzle)
            .execute(&db)
            .await
            .is_err()
    );
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120002_world_catalog_records.down.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120001_world_catalog_candidates.down.sql"
    ))
    .execute(&db)
    .await?;
    assert_eq!(store.active_version_id().await?, Some(legacy));
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120001_world_catalog_candidates.up.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120002_world_catalog_records.up.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261107120003_world_catalog_publication.up.sql"
    ))
    .execute(&db)
    .await?;
    candidate.persist_inactive(&db).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&db)
        .await?;
    Ok(())
}
