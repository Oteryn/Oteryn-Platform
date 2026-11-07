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
