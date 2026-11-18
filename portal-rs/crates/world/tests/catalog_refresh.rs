#![allow(clippy::expect_used)]
use oteryn_world::{
    WorldOptions, WorldStore,
    catalog::{CatalogStore, Error, Fence, RetryPolicy, Source, refresh_source_in_background},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use uuid::Uuid;
#[path = "support/catalog.rs"]
mod catalog_fixture;
struct Database {
    root: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED catalog refresh PostgreSQL test: PORTAL_TEST_DATABASE_URL unset");
            return Ok(None);
        };
        let root = PgPool::connect(&url).await?;
        let schema = format!("catalog_refresh_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self { root, pool, schema }))
    }
    async fn close(self) -> Result<(), sqlx::Error> {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}
fn source(fixture: &catalog_fixture::Fixture) -> Source {
    Source::local(
        PathBuf::from("test-only-unused-file"),
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
struct Producer {
    task: tokio::task::JoinHandle<()>,
    url: url::Url,
    requests: Arc<AtomicUsize>,
}
impl Drop for Producer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn producer(body: Vec<u8>, failures: usize) -> Result<Producer, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = url::Url::parse(&format!("http://{}/catalog", listener.local_addr()?))?;
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0; 4096];
            if !matches!(socket.read(&mut buffer).await, Ok(n) if n > 0) {
                continue;
            }
            let number = count.fetch_add(1, Ordering::SeqCst);
            let (status, bytes) = if number < failures {
                ("503 Service Unavailable", &b"unavailable"[..])
            } else {
                ("200 OK", body.as_slice())
            };
            if socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await
                .is_ok()
            {
                let _ = socket.write_all(bytes).await;
            }
        }
    });
    Ok(Producer {
        task,
        url,
        requests,
    })
}
#[tokio::test]
async fn unavailable_then_available_producer_recovers_without_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let fixture = catalog_fixture::supported();
    let producer = producer(fixture.bytes.clone(), 1).await?;
    let mut source = source(&fixture);
    source.file = PathBuf::new();
    source.url = Some(producer.url.clone());
    let ready = Arc::new(AtomicBool::new(false));
    let refresh = refresh_source_in_background(
        db.pool.clone(),
        source.clone(),
        true,
        RetryPolicy::new(1, 2, 3)?,
        ready.clone(),
    );
    assert!(!ready.load(Ordering::Acquire));
    let snapshot = tokio::time::timeout(Duration::from_secs(5), refresh).await???;
    assert_eq!(
        snapshot.as_deref(),
        Some(fixture.text("snapshot_id").as_str())
    );
    assert!(ready.load(Ordering::Acquire));
    assert_eq!(producer.requests.load(Ordering::SeqCst), 2);
    let catalogue = CatalogStore::new(db.pool.clone(), Some(source)).with_readiness(ready);
    assert!(catalogue.active_version_id().await?.is_some());
    assert!(
        catalogue
            .get_entity("Item", "oteryn:item.training_sword")
            .await?
            .is_none()
    );
    WorldStore::new(db.pool.clone(), WorldOptions { enabled: true })
        .allow("Item", "oteryn:item.training_sword")
        .await?;
    assert!(
        catalogue
            .get_entity("Item", "oteryn:item.training_sword")
            .await?
            .is_some()
    );
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn stored_matching_publication_is_ready_without_any_refetch()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let fixture = catalog_fixture::supported();
    let mut source = source(&fixture);
    let candidate = oteryn_world::catalog::verify(&fixture.bytes, &fixture.sidecar, &source.fence)?;
    candidate.persist_inactive(&db.pool).await?;
    candidate.publish(&db.pool).await?;
    let producer = producer(fixture.bytes.clone(), usize::MAX).await?;
    source.file = PathBuf::new();
    source.url = Some(producer.url.clone());
    let ready = Arc::new(AtomicBool::new(false));
    let refresh = refresh_source_in_background(
        db.pool.clone(),
        source.clone(),
        true,
        RetryPolicy::new(5, 300, 12)?,
        ready.clone(),
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), refresh).await???;
    assert!(ready.load(Ordering::Acquire));
    assert_eq!(producer.requests.load(Ordering::SeqCst), 0);
    // A hash/fence mismatch or publication OFF cannot borrow that stored readiness.
    for mode in ["hash", "revision", "disabled"] {
        let mut wrong = source.clone();
        if mode == "hash" {
            wrong.expected_sha256 = "0".repeat(64);
        }
        if mode == "revision" {
            wrong.fence.source_revision = "b".repeat(40);
        }
        let ready = Arc::new(AtomicBool::new(false));
        let result = refresh_source_in_background(
            db.pool.clone(),
            wrong,
            mode != "disabled",
            RetryPolicy::new(1, 1, 1)?,
            ready.clone(),
        )
        .await?;
        assert_eq!(result, Err(Error::Unavailable));
        assert!(!ready.load(Ordering::Acquire));
    }
    assert_eq!(producer.requests.load(Ordering::SeqCst), 3);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn retry_limit_and_integrity_refusal_leave_source_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let fixture = catalog_fixture::supported();
    let failed = producer(fixture.bytes.clone(), usize::MAX).await?;
    let mut source = source(&fixture);
    source.file = PathBuf::new();
    source.url = Some(failed.url.clone());
    let ready = Arc::new(AtomicBool::new(false));
    assert_eq!(
        refresh_source_in_background(
            db.pool.clone(),
            source.clone(),
            true,
            RetryPolicy::new(1, 1, 2)?,
            ready.clone()
        )
        .await?,
        Err(Error::Unavailable)
    );
    assert!(!ready.load(Ordering::Acquire));
    assert_eq!(failed.requests.load(Ordering::SeqCst), 2);
    let bad = producer(b"wrong bytes".to_vec(), 0).await?;
    source.url = Some(bad.url.clone());
    assert_eq!(
        refresh_source_in_background(
            db.pool.clone(),
            source,
            true,
            RetryPolicy::new(1, 2, 3)?,
            ready.clone()
        )
        .await?,
        Err(Error::IntegrityMismatch)
    );
    assert!(!ready.load(Ordering::Acquire));
    assert_eq!(bad.requests.load(Ordering::SeqCst), 1);
    db.close().await?;
    Ok(())
}
#[tokio::test]
async fn identity_approval_intentionally_survives_legacy_to_catalogue_switch()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let legacy = Uuid::now_v7();
    let key = "oteryn:item.training_sword";
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
        .bind(legacy)
        .bind("b".repeat(40))
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'Item',$2,'{\"name\":\"Legacy source\"}',true)").bind(legacy).bind(key).execute(&db.pool).await?;
    sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
        .bind(legacy)
        .execute(&db.pool)
        .await?;
    let world = WorldStore::new(db.pool.clone(), WorldOptions { enabled: true });
    world.allow("Item", key).await?;
    let fixture = catalog_fixture::supported();
    let source = source(&fixture);
    let candidate = oteryn_world::catalog::verify(&fixture.bytes, &fixture.sidecar, &source.fence)?;
    candidate.persist_inactive(&db.pool).await?;
    candidate.publish(&db.pool).await?;
    let catalogue = CatalogStore::new(db.pool.clone(), Some(source));
    let entity = catalogue
        .get_entity("Item", key)
        .await?
        .expect("stable identity remains approved");
    assert_ne!(entity.version_id, legacy);
    assert_eq!(entity.data["name"], "Training Sword");
    assert_eq!(catalogue.list_by_kind("Item", None, 10).await?.len(), 1);
    world.deny("Item", key).await?;
    assert!(catalogue.get_entity("Item", key).await?.is_none());
    assert!(catalogue.list_by_kind("Item", None, 10).await?.is_empty());
    assert!(world.get_entity("Item", key).await?.is_none());
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn retired_epoch_delayed_refresh_cannot_replace_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let a = catalog_fixture::supported();
    let sa = source(&a);
    let a = oteryn_world::catalog::verify(&a.bytes, &a.sidecar, &sa.fence)?;
    a.persist_inactive(&db.pool).await?;
    a.publish(&db.pool).await?;
    // A refresh has verified its candidate but is paused before publication.
    let release = Arc::new(tokio::sync::Notify::new());
    let signal = release.clone();
    let pool = db.pool.clone();
    let delayed = tokio::spawn(async move {
        signal.notified().await;
        a.publish(&pool).await
    });
    let mut input = catalog_fixture::supported_source();
    input["authority_epoch"] = "native-epoch-2".into();
    let b = catalog_fixture::emit(input);
    let sb = source(&b);
    let b = oteryn_world::catalog::verify(&b.bytes, &b.sidecar, &sb.fence)?;
    b.persist_inactive(&db.pool).await?;
    b.publish(&db.pool).await?;
    release.notify_one();
    assert_eq!(delayed.await?, Err(Error::ProvenanceConflict));
    assert!(
        CatalogStore::new(db.pool.clone(), Some(sb))
            .active_version_id()
            .await?
            .is_some()
    );
    assert_eq!(
        CatalogStore::new(db.pool.clone(), Some(sa))
            .active_version_id()
            .await?,
        None
    );
    db.close().await?;
    eprintln!(
        "retired_epoch_delayed_refresh_cannot_replace_publication: {:?}",
        started.elapsed()
    );
    Ok(())
}
#[tokio::test]
async fn nul_opaque_category_does_not_poison_valid_npc_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut input = catalog_fixture::supported_source();
    input["entities"][0]["data"]["opaque"] = "opaque\0carrier".into();
    input["capability_manifest"][1]["support"] = "supported".into();
    input["completeness_manifest"][1]["state"] = "complete".into();
    input["entities"].as_array_mut().ok_or("entities")?.push(serde_json::json!({"type":"npc","content_key":"oteryn:npc.test","capability_id":"npc","data":{"name":"Guide"}}));
    let fixture = catalog_fixture::emit(input);
    let configured = source(&fixture);
    let candidate =
        oteryn_world::catalog::verify(&fixture.bytes, &fixture.sidecar, &configured.fence)?;
    candidate.persist_inactive(&db.pool).await?;
    candidate.publish(&db.pool).await?;
    let store = CatalogStore::new(db.pool.clone(), Some(configured));
    assert!(!store.available("Item").await?);
    assert!(store.available("NPC").await?);
    let original: Vec<u8> =
        sqlx::query_scalar("SELECT artifact FROM world_catalog_candidates WHERE snapshot_id=$1")
            .bind(candidate.snapshot_id())
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(original, fixture.bytes);
    db.close().await?;
    eprintln!(
        "nul_opaque_category_does_not_poison_valid_npc_publication: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn paused_first_epoch_refresh_is_fenced_before_its_first_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let a = catalog_fixture::supported();
    let mut sa = source(&a);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    sa.file = PathBuf::new();
    sa.url = Some(url::Url::parse(&format!(
        "http://{}/catalog",
        listener.local_addr()?
    ))?);
    let (arrived, paused) = tokio::sync::oneshot::channel();
    let (release, resume) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;
        let mut buffer = [0; 4096];
        let received = socket.read(&mut buffer).await?;
        assert!(received > 0);
        let _ = arrived.send(());
        let _ = resume.await;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    a.bytes.len()
                )
                .as_bytes(),
            )
            .await?;
        socket.write_all(&a.bytes).await?;
        Ok::<_, std::io::Error>(())
    });
    let policy = RetryPolicy::new(1, 1, 1)?;
    let delayed = refresh_source_in_background(
        db.pool.clone(),
        sa,
        true,
        policy,
        Arc::new(AtomicBool::new(false)),
    );
    tokio::time::timeout(Duration::from_secs(3), paused).await??;
    let mut input = catalog_fixture::supported_source();
    input["authority_epoch"] = "native-epoch-2".into();
    let b = catalog_fixture::emit(input);
    let producer = producer(b.bytes.clone(), 0).await?;
    let mut sb = source(&b);
    sb.file = PathBuf::new();
    sb.url = Some(producer.url.clone());
    refresh_source_in_background(
        db.pool.clone(),
        sb.clone(),
        true,
        policy,
        Arc::new(AtomicBool::new(false)),
    )
    .await??;
    let _ = release.send(());
    assert_eq!(delayed.await?, Err(Error::ProvenanceConflict));
    server.await??;
    assert!(
        CatalogStore::new(db.pool.clone(), Some(sb))
            .active_version_id()
            .await?
            .is_some()
    );
    db.close().await?;
    eprintln!(
        "paused_first_epoch_refresh_is_fenced_before_its_first_publication: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn migration_copies_aliases_and_denies_conflicting_approvals()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Err("PORTAL_TEST_DATABASE_URL required".into());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("alias_migration_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let preceding = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            oteryn_db::MIGRATOR
                .iter()
                .filter(|m| m.version < 20261115120014)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    {
        let mut connection = oteryn_db::migration_connection(&pool).await?;
        preceding.run(&mut *connection).await?;
    }
    let version = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready')",
    )
    .bind(version)
    .execute(&pool)
    .await?;
    sqlx::query("UPDATE world_active_version SET version_id=$1")
        .bind(version)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'creatures','rat','{}',true),($1,'npcs','guide','{}',true)").bind(version).execute(&pool).await?;
    sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES('creatures','rat',true),('npcs','guide',true),('NPC','guide',false)").execute(&pool).await?;
    sqlx::query(
        "INSERT INTO world_admin_names(kind,key,pl,en) VALUES('creatures','rat','Szczur','Rat')",
    )
    .execute(&pool)
    .await?;
    // Populate the predecessor schema with two previously accepted epochs.
    let old = catalog_fixture::supported();
    let old_source = source(&old);
    let old = oteryn_world::catalog::verify(&old.bytes, &old.sidecar, &old_source.fence)?;
    old.persist_inactive(&pool).await?;
    let mut current_input = catalog_fixture::supported_source();
    current_input["authority_epoch"] = "migration-current-epoch".into();
    let current = catalog_fixture::emit(current_input);
    let current_source = source(&current);
    let current =
        oteryn_world::catalog::verify(&current.bytes, &current.sidecar, &current_source.fence)?;
    current.persist_inactive(&pool).await?;
    for candidate in [&old, &current] {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready')",
        )
        .bind(id)
        .execute(&pool)
        .await?;
        sqlx::query("INSERT INTO world_catalog_versions(snapshot_id,version_id,capabilities) VALUES($1,$2,ARRAY['item'])").bind(candidate.snapshot_id()).bind(id).execute(&pool).await?;
    }
    sqlx::query("UPDATE world_catalog_publication SET snapshot_id=$1")
        .bind(current.snapshot_id())
        .execute(&pool)
        .await?;
    oteryn_db::migrate(&pool).await?;
    assert_eq!(old.publish(&pool).await, Err(Error::ProvenanceConflict));
    assert!(
        CatalogStore::new(pool.clone(), Some(current_source))
            .active_version_id()
            .await?
            .is_some()
    );
    let world = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
    assert!(world.get_entity("Creature", "rat").await?.is_some());
    assert!(world.get_entity("creatures", "rat").await?.is_some());
    assert!(world.get_entity("NPC", "guide").await?.is_none());
    let name: String =
        sqlx::query_scalar("SELECT en FROM world_admin_names WHERE kind='Creature' AND key='rat'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(name, "Rat");
    world.allow("npcs", "guide").await?;
    assert!(world.get_entity("NPC", "guide").await?.is_some());
    let db = Database { root, pool, schema };
    db.close().await?;
    eprintln!(
        "migration_copies_aliases_and_denies_conflicting_approvals: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn retired_epoch_requires_exact_audited_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let a = catalog_fixture::supported();
    let sa = source(&a);
    let a = oteryn_world::catalog::verify(&a.bytes, &a.sidecar, &sa.fence)?;
    a.persist_inactive(&db.pool).await?;
    a.publish(&db.pool).await?;
    let mut input = catalog_fixture::supported_source();
    input["authority_epoch"] = "recovery-epoch-2".into();
    let b = catalog_fixture::emit(input);
    let sb = source(&b);
    let b = oteryn_world::catalog::verify(&b.bytes, &b.sidecar, &sb.fence)?;
    b.persist_inactive(&db.pool).await?;
    b.publish(&db.pool).await?;
    assert_eq!(a.publish(&db.pool).await, Err(Error::ProvenanceConflict));
    for (ruleset, evidence) in [
        ("wrong", "operator evidence"),
        (sa.fence.ruleset_id.as_str(), " "),
    ] {
        assert!(
            sqlx::query("SELECT world_catalog_recover_authority($1,$2,$3,$4,$5,$6)")
                .bind(Uuid::now_v7())
                .bind(a.snapshot_id())
                .bind(ruleset)
                .bind(&sa.fence.content_profile_id)
                .bind(evidence)
                .bind("Restore current epoch if validation fails")
                .execute(&db.pool)
                .await
                .is_err()
        );
    }
    assert!(
        CatalogStore::new(db.pool.clone(), Some(sb))
            .active_version_id()
            .await?
            .is_some()
    );
    let recovery: i64 =
        sqlx::query_scalar("SELECT world_catalog_recover_authority($1,$2,$3,$4,$5,$6)")
            .bind(Uuid::now_v7())
            .bind(a.snapshot_id())
            .bind(&sa.fence.ruleset_id)
            .bind(&sa.fence.content_profile_id)
            .bind("Explicit operator authority evidence for retained test snapshot")
            .bind("Restore epoch 2 after verifying authority")
            .fetch_one(&db.pool)
            .await?;
    assert!(recovery > 0);
    a.publish(&db.pool).await?;
    assert!(
        CatalogStore::new(db.pool.clone(), Some(sa))
            .active_version_id()
            .await?
            .is_some()
    );
    assert_eq!(b.publish(&db.pool).await, Err(Error::ProvenanceConflict));
    assert!(
        sqlx::query("DELETE FROM world_catalog_authority_recoveries")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await?;
    eprintln!(
        "retired_epoch_requires_exact_audited_recovery: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn publication_reservation_recovers_after_pool_exhaustion()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let fixture = catalog_fixture::supported();
    let source = source(&fixture);
    let candidate = oteryn_world::catalog::verify(&fixture.bytes, &fixture.sidecar, &source.fence)?;
    candidate.persist_inactive(&db.pool).await?;
    candidate.publish(&db.pool).await?;
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(db.pool.acquire().await?);
    }
    let ready = Arc::new(AtomicBool::new(false));
    let task = refresh_source_in_background(
        db.pool.clone(),
        source,
        true,
        RetryPolicy::new(1, 1, 3)?,
        ready.clone(),
    );
    tokio::time::sleep(Duration::from_secs(31)).await;
    drop(held);
    tokio::time::timeout(Duration::from_secs(5), task).await???;
    assert!(ready.load(Ordering::Acquire));
    db.close().await?;
    eprintln!(
        "publication_reservation_recovers_after_pool_exhaustion: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn publication_reservation_row_lock_obeys_attempt_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut tx = db.pool.begin().await?;
    sqlx::query("SELECT singleton FROM world_catalog_publication WHERE singleton FOR UPDATE")
        .fetch_one(&mut *tx)
        .await?;
    let ready = Arc::new(AtomicBool::new(false));
    let task = refresh_source_in_background(
        db.pool.clone(),
        source(&catalog_fixture::supported()),
        true,
        RetryPolicy::new(1, 1, 1)?,
        ready.clone(),
    );
    let result = tokio::time::timeout(Duration::from_secs(63), task).await??;
    assert!(matches!(result, Err(Error::Unavailable)));
    assert!(!ready.load(Ordering::Acquire));
    tx.rollback().await?;
    db.close().await?;
    eprintln!(
        "publication_reservation_row_lock_obeys_attempt_deadline: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn r7_failed_verification_preserves_publication() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open().await?.ok_or("PostgreSQL required")?;
    let fixture = catalog_fixture::supported();
    let original = source(&fixture);
    let candidate =
        oteryn_world::catalog::verify(&fixture.bytes, &fixture.sidecar, &original.fence)?;
    candidate.persist_inactive(&db.pool).await?;
    candidate.publish(&db.pool).await?;
    let store = CatalogStore::new(db.pool.clone(), Some(original.clone()));
    let mut replacement = original;
    replacement.fence.snapshot_id = format!("sha256:{}", "0".repeat(64));
    let bad = producer(b"corrupt".to_vec(), 0).await?;
    replacement.file = PathBuf::new();
    replacement.url = Some(bad.url.clone());
    assert_eq!(
        refresh_source_in_background(
            db.pool.clone(),
            replacement,
            true,
            RetryPolicy::new(1, 1, 1)?,
            Arc::new(AtomicBool::new(false))
        )
        .await?,
        Err(Error::IntegrityMismatch)
    );
    let active = store.active_version_id().await?;
    db.close().await?;
    assert!(
        active.is_some(),
        "failed verification hid active publication"
    );
    Ok(())
}
