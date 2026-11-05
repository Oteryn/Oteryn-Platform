#![allow(clippy::unwrap_used, clippy::panic)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use oteryn_world::{ImportMapping, VersionStatus, WorldError, WorldOptions, WorldStore};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KIND: &str = "synthetic-entry";

fn mapping() -> ImportMapping {
    ImportMapping {
        path_glob: "content/synthetic/*.json".into(),
        schema_file: "tools/content-schema/envelope.schema.json".into(),
        kind: KIND.into(),
        key_field: "code".into(),
        records_pointer: "/payload".into(),
        schema_scope: oteryn_world::SchemaScope::Document,
        hidden_when_true: Vec::new(),
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    store: WorldStore,
}

impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let url = match std::env::var("PORTAL_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => {
                eprintln!("SKIPPED PostgreSQL world test: PORTAL_TEST_DATABASE_URL is unset");
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&admin)
            .await?;
        assert_eq!(
            version, "180006",
            "world integration tests require PostgreSQL 18.6"
        );
        let schema = format!("world_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = PgPoolOptions::new().max_connections(8).after_connect(move |conn, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path', $1, false), set_config('application_name', $1, false)")
                    .bind(search_path).execute(conn).await?;
                Ok(())
            })
        }).connect(&url).await?;
        oteryn_db::migrate(&pool).await?;
        let store = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
        Ok(Some(Self {
            admin,
            pool,
            schema,
            store,
        }))
    }

    async fn import(
        &self,
        name: &str,
        commit: char,
    ) -> Result<oteryn_world::ImportReport, WorldError> {
        self.store
            .import_directory(
                fixture(name),
                &commit.to_string().repeat(40),
                vec![mapping()],
            )
            .await
    }

    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}

#[tokio::test]
async fn valid_import_switch_and_rollback_preserve_documents() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(db.store.active_version_id().await?, None);
    assert!(matches!(
        db.store.rollback().await,
        Err(WorldError::NoPreviousVersion)
    ));
    let first = db.import("valid", 'a').await?;
    assert_eq!(first.status, VersionStatus::Ready);
    assert_eq!(first.document_count, 2);
    assert_eq!(first.version_id.get_version_num(), 7);
    assert_eq!(
        db.store.import_report(first.version_id).await?,
        Some(first.clone())
    );
    db.store.allow(KIND, "fixture.alpha").await?;
    let second = db.import("updated", 'b').await?;
    assert_eq!(db.store.active_version_id().await?, Some(second.version_id));
    assert_eq!(
        db.store
            .get_entity(KIND, "fixture.alpha")
            .await?
            .unwrap()
            .data["weight"],
        10
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM world_entities")
            .fetch_one(&db.pool)
            .await?,
        4
    );
    assert_eq!(db.store.rollback().await?, first.version_id);
    assert_eq!(
        db.store
            .get_entity(KIND, "fixture.alpha")
            .await?
            .unwrap()
            .data["weight"],
        1
    );
    assert_eq!(db.store.rollback().await?, second.version_id);
    db.close().await
}

#[tokio::test]
async fn schema_failure_keeps_old_active_and_records_pointer() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.import("valid", 'a').await?;
    let error = db.import("invalid", 'b').await.unwrap_err();
    let WorldError::ImportFailed { version_id, issues } = error else {
        panic!("expected validation error")
    };
    assert_eq!(issues[0].file, "content/synthetic/entries.json");
    assert_eq!(issues[0].pointer, "/payload/0/weight");
    assert!(!issues[0].message.is_empty());
    let failed = db.store.import_report(version_id).await?.unwrap();
    assert_eq!(failed.status, VersionStatus::Failed);
    assert_eq!(failed.errors, issues);
    assert_eq!(failed.document_count, 0);
    assert_eq!(db.store.active_version_id().await?, Some(first.version_id));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM world_entities WHERE version_id = $1")
            .bind(version_id)
            .fetch_one(&db.pool)
            .await?,
        0
    );
    assert!(matches!(
        db.store.rollback().await,
        Err(WorldError::NoPreviousVersion)
    ));
    db.close().await
}

#[tokio::test]
async fn allow_list_is_hidden_by_default_and_scoped_to_kind_and_key() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.import("valid", 'a').await?;
    assert!(db.store.get_entity(KIND, "fixture.alpha").await?.is_none());
    assert!(db.store.list_by_kind(KIND, None, 10).await?.is_empty());
    db.store.allow("another-kind", "fixture.alpha").await?;
    assert!(db.store.get_entity(KIND, "fixture.alpha").await?.is_none());
    db.store.allow(KIND, "fixture.alpha").await?;
    db.store.allow(KIND, "fixture.beta").await?;
    let first = db.store.list_by_kind(KIND, None, 1).await?;
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].key, "fixture.alpha");
    let second = db.store.list_by_kind(KIND, Some(&first[0].key), 1).await?;
    assert_eq!(second[0].key, "fixture.beta");
    db.store.deny(KIND, "fixture.alpha").await?;
    db.store.deny(KIND, "fixture.alpha").await?;
    assert!(db.store.get_entity(KIND, "fixture.alpha").await?.is_none());
    let updated = db.import("updated", 'b').await?;
    let rows = db.store.list_by_kind(KIND, None, 10).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "fixture.beta");
    assert_eq!(rows[0].version_id, updated.version_id);
    assert!(db.store.get_entity(KIND, "absent").await?.is_none());
    db.close().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_switches_wait_for_the_same_lock_and_keep_the_actual_predecessor() -> TestResult
{
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.import("valid", 'a').await?;
    let mut lock = db.pool.begin().await?;
    sqlx::query("SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE")
        .execute(&mut *lock)
        .await?;
    let store = db.store.clone();
    let left = tokio::spawn(async move {
        store
            .import_directory(fixture("updated"), &"b".repeat(40), vec![mapping()])
            .await
    });
    let store = db.store.clone();
    let right = tokio::spawn(async move {
        store
            .import_directory(fixture("valid"), &"c".repeat(40), vec![mapping()])
            .await
    });
    timeout(Duration::from_secs(10), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name = $1 AND wait_event_type = 'Lock'")
                .bind(&db.schema).fetch_one(&db.admin).await.unwrap();
            if waiting >= 2 { break; }
            sleep(Duration::from_millis(10)).await;
        }
    }).await?;
    assert!(!left.is_finished());
    assert!(!right.is_finished());
    assert_eq!(db.store.active_version_id().await?, Some(first.version_id));
    lock.commit().await?;
    let left = left.await??;
    let right = right.await??;
    let active = db.store.active_version_id().await?.unwrap();
    let previous = if active == left.version_id {
        right.version_id
    } else {
        left.version_id
    };
    assert!(active == left.version_id || active == right.version_id);
    assert_eq!(db.store.rollback().await?, previous);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM world_versions WHERE status = 'ready'")
            .fetch_one(&db.pool)
            .await?,
        3
    );
    db.close().await
}

#[tokio::test]
async fn database_failure_rolls_back_partial_documents_and_switch() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.import("valid", 'a').await?;
    sqlx::query("ALTER TABLE world_entities ADD CONSTRAINT synthetic_write_refusal CHECK ((data->>'weight')::integer < 2) NOT VALID").execute(&db.pool).await?;
    assert!(matches!(
        db.import("valid", 'b').await,
        Err(WorldError::Database(_))
    ));
    assert_eq!(db.store.active_version_id().await?, Some(first.version_id));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM world_entities")
            .fetch_one(&db.pool)
            .await?,
        2
    );
    let failed: Uuid = sqlx::query_scalar("SELECT id FROM world_versions WHERE status = 'failed'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        db.store.import_report(failed).await?.unwrap().errors[0].message,
        "database publication failed; transaction rolled back"
    );
    db.close().await
}

#[tokio::test]
async fn malformed_unmapped_and_duplicate_input_fail_closed() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.import("valid", 'a').await?;
    for (file, data, pointer) in [
        ("entries.json", "{broken", ""),
        (
            "entries.json",
            r#"{"payload":[{"code":"same","weight":1},{"code":"same","weight":2}]}"#,
            "/payload/1/code",
        ),
        (
            "entries.json",
            r#"{"payload":[{"code":"","weight":1}]}"#,
            "/payload/0/code",
        ),
    ] {
        let temp = TempDir::new()?;
        copy_tree(&fixture("valid"), temp.path())?;
        fs::write(temp.path().join("content/synthetic").join(file), data)?;
        let error = db
            .store
            .import_directory(temp.path(), &"b".repeat(40), vec![mapping()])
            .await
            .unwrap_err();
        let WorldError::ImportFailed { issues, .. } = error else {
            panic!("expected import failure")
        };
        assert_eq!(issues[0].pointer, pointer);
    }
    let temp = TempDir::new()?;
    copy_tree(&fixture("valid"), temp.path())?;
    fs::write(temp.path().join("content/unmapped.json"), "{}")?;
    assert!(matches!(
        db.store
            .import_directory(temp.path(), &"b".repeat(40), vec![mapping()])
            .await,
        Err(WorldError::ImportFailed { .. })
    ));
    assert_eq!(db.store.active_version_id().await?, Some(first.version_id));
    db.close().await
}

#[tokio::test]
async fn invalid_mapping_and_external_schema_reference_are_recorded() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for mappings in [
        vec![],
        vec![mapping(), mapping()],
        vec![ImportMapping {
            schema_file: "../outside.json".into(),
            ..mapping()
        }],
        vec![ImportMapping {
            records_pointer: "/bad~pointer".into(),
            ..mapping()
        }],
    ] {
        assert!(matches!(
            db.store
                .import_directory(fixture("valid"), &"a".repeat(40), mappings)
                .await,
            Err(WorldError::ImportFailed { .. })
        ));
    }
    let temp = TempDir::new()?;
    copy_tree(&fixture("valid"), temp.path())?;
    fs::write(
        temp.path()
            .join("tools/content-schema/envelope.schema.json"),
        r#"{"$ref":"https://example.invalid/forbidden-schema"}"#,
    )?;
    assert!(matches!(
        db.store
            .import_directory(temp.path(), &"a".repeat(40), vec![mapping()])
            .await,
        Err(WorldError::ImportFailed { .. })
    ));
    assert_eq!(db.store.active_version_id().await?, None);
    db.close().await
}

#[cfg(unix)]
#[tokio::test]
async fn content_symlinks_and_schema_escape_are_refused() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let temp = TempDir::new()?;
    copy_tree(&fixture("valid"), temp.path())?;
    std::os::unix::fs::symlink(
        fixture("valid").join("content/synthetic/entries.json"),
        temp.path().join("content/synthetic/linked.json"),
    )?;
    assert!(matches!(
        db.store
            .import_directory(temp.path(), &"a".repeat(40), vec![mapping()])
            .await,
        Err(WorldError::ImportFailed { .. })
    ));
    fs::remove_file(temp.path().join("content/synthetic/linked.json"))?;
    fs::write(temp.path().join("outside.schema.json"), "true")?;
    fs::write(
        temp.path()
            .join("tools/content-schema/envelope.schema.json"),
        r#"{"$ref":"../../outside.schema.json"}"#,
    )?;
    assert!(matches!(
        db.store
            .import_directory(temp.path(), &"a".repeat(40), vec![mapping()])
            .await,
        Err(WorldError::ImportFailed { .. })
    ));
    db.close().await
}

#[tokio::test]
async fn small_pinned_game_fixture_imports_without_a_game_checkout() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let config: oteryn_world::ImportConfig =
        serde_json::from_str(include_str!("../config/game-857f15f3.json"))?;
    let mapping = config
        .mappings
        .iter()
        .find(|m| m.path_glob == "content/world/areas/cities/areas-*.json")
        .unwrap()
        .clone();
    let report = db
        .store
        .import_configured_directory(
            fixture("game"),
            "857f15f376907c592bb28d854fccff18a15cdf87",
            oteryn_world::ImportConfig {
                mappings: vec![mapping],
                excluded_files: Vec::new(),
                schema_resources: config
                    .schema_resources
                    .into_iter()
                    .filter(|(uri, _)| uri == "urn:oteryn:monster-authoring:candidate:1")
                    .collect(),
            },
        )
        .await?;
    assert_eq!(report.document_count, 22);
    assert!(db.store.list_by_kind("Area", None, 100).await?.is_empty());
    db.close().await
}

#[tokio::test]
async fn real_pinned_game_tree_imports_with_its_own_schemas() -> TestResult {
    const COMMIT: &str = "857f15f376907c592bb28d854fccff18a15cdf87";
    let root = std::env::var_os("PORTAL_TEST_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/oteryn-game"));
    if !root.join("content").is_dir() || !root.join("tools/content-schema").is_dir() {
        eprintln!(
            "SKIPPED real Game tree import: pinned checkout unavailable (set PORTAL_TEST_GAME_DIR)"
        );
        return Ok(());
    }
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    assert!(head.status.success(), "Game reference must be a checkout");
    assert_eq!(
        String::from_utf8(head.stdout)?.trim(),
        COMMIT,
        "Game reference must be pinned to the authorized commit"
    );
    let dirty = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "status",
            "--porcelain",
            "--",
            "content",
            "tools/content-schema",
        ])
        .output()?;
    assert!(dirty.status.success());
    assert!(
        dirty.stdout.is_empty(),
        "Game content and schemas must be unmodified"
    );
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let config: oteryn_world::ImportConfig =
        serde_json::from_str(include_str!("../config/game-857f15f3.json"))?;
    let report = db
        .store
        .import_configured_directory(root, COMMIT, config)
        .await?;
    eprintln!(
        "Real Game tree import: {} documents from {COMMIT}",
        report.document_count
    );
    assert!(report.document_count > 20_000);
    assert_eq!(db.store.active_version_id().await?, Some(report.version_id));
    assert!(
        db.store
            .list_by_kind("Achievement", None, 1000)
            .await?
            .is_empty()
    );
    let secret_key: String = sqlx::query_scalar("SELECT key FROM world_entities WHERE version_id = $1 AND kind = 'Achievement' AND data->>'secret' = 'true' LIMIT 1")
        .bind(report.version_id).fetch_one(&db.pool).await?;
    db.store.allow("Achievement", &secret_key).await?;
    assert!(
        db.store
            .get_entity("Achievement", &secret_key)
            .await?
            .is_none()
    );
    assert!(
        db.store
            .list_by_kind("Achievement", None, 1000)
            .await?
            .is_empty()
    );
    db.close().await
}
