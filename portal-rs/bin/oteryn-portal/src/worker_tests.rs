use super::*;
use outbox::{OffsetDateTime, Uuid, query, query_scalar};
use std::sync::atomic::{AtomicUsize, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = outbox::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("t37_{}", oteryn_core::new_id().simple());
        // Schema identifiers contain only a fixed prefix and generated UUID hex.
        query(outbox::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = outbox::PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |connection, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    query("SELECT set_config('search_path', $1, false)")
                        .bind(sql)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        query(outbox::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
    async fn enqueue(&self, kind: &str, key: &str, attempts: i32) -> Result<Uuid, outbox::Error> {
        let mut tx = self.pool.begin().await?;
        let id = outbox::enqueue(
            &mut tx,
            kind,
            "{}",
            key,
            OffsetDateTime::now_utc(),
            attempts,
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
}

#[tokio::test]
async fn enqueue_rolls_back_with_caller() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mut tx = db.pool.begin().await?;
    outbox::enqueue(
        &mut tx,
        "test",
        "{}",
        "rollback",
        OffsetDateTime::now_utc(),
        5,
    )
    .await?;
    tx.rollback().await?;
    assert_eq!(outbox::counts(&db.pool).await?, outbox::Counts::default());
    db.close().await
}

#[tokio::test]
async fn idempotency_deduplicates_per_kind() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (a, b) = tokio::join!(db.enqueue("a", "key", 5), db.enqueue("a", "key", 5));
    assert_eq!(a?, b?);
    db.enqueue("b", "key", 5).await?;
    assert_eq!(outbox::counts(&db.pool).await?.pending, 2);
    db.close().await
}

#[derive(Debug)]
struct Counter(Arc<AtomicUsize>);
impl Handler for Counter {
    fn run<'a>(&'a self, _job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
impl Cleanup for Counter {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

#[tokio::test]
async fn two_workers_execute_each_job_once() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for i in 0..40 {
        db.enqueue("count", &i.to_string(), 5).await?;
    }
    let counter = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    registry.register("count", Arc::new(Counter(counter.clone())))?;
    for _ in 0..3 {
        let (a, b) = tokio::join!(registry.batch(&db.pool), registry.batch(&db.pool));
        a?;
        b?;
    }
    assert_eq!(counter.load(Ordering::SeqCst), 40);
    assert_eq!(outbox::counts(&db.pool).await?.done, 40);
    db.close().await
}

#[derive(Debug)]
struct Failure;
impl Handler for Failure {
    fn run<'a>(&'a self, _job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async { Err("domain refused job".into()) })
    }
}
#[tokio::test]
async fn retry_then_dead_and_unknown_kind() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enqueue("failure", "key", 2).await?;
    let mut registry = Registry::default();
    registry.register("failure", Arc::new(Failure))?;
    registry.batch(&db.pool).await?;
    let (state, attempts, delay): (String, i32, f64) = outbox::query_as(
        "SELECT state, attempts, extract(epoch FROM (run_after - updated_at))::float8 FROM outbox",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!((state.as_str(), attempts), ("pending", 1));
    assert!((1.0..=2.0).contains(&delay));
    assert_eq!(registry.batch(&db.pool).await?, 0);
    query("UPDATE outbox SET run_after = now() - interval '1 second'")
        .execute(&db.pool)
        .await?;
    registry.batch(&db.pool).await?;
    let error: String = query_scalar("SELECT last_error FROM outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(error, "domain refused job");
    db.enqueue("unknown", "key", 5).await?;
    registry.batch(&db.pool).await?;
    assert_eq!(outbox::counts(&db.pool).await?.dead, 2);
    let error: String = query_scalar("SELECT last_error FROM outbox WHERE kind = 'unknown'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(error, "unknown job kind: unknown");
    db.close().await
}

#[tokio::test]
async fn lease_expiry_fences_old_owner_and_exhausts_final_attempt() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enqueue("lease", "key", 2).await?;
    let old = outbox::claim(&db.pool, 1, 60).await?.remove(0);
    assert!(outbox::renew(&db.pool, &old, 120).await?);
    assert!(outbox::claim(&db.pool, 1, 60).await?.is_empty());
    query("UPDATE outbox SET leased_until = now() - interval '1 second'")
        .execute(&db.pool)
        .await?;
    let new = outbox::claim(&db.pool, 1, 60).await?.remove(0);
    assert_eq!(new.attempts, 2);
    assert!(!outbox::finish(&db.pool, &old, None, false).await?);
    assert!(!outbox::renew(&db.pool, &old, 60).await?);
    query("UPDATE outbox SET leased_until = now() - interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert!(outbox::claim(&db.pool, 1, 60).await?.is_empty());
    assert_eq!(outbox::counts(&db.pool).await?.dead, 1);
    db.close().await
}

#[tokio::test]
async fn schedules_run_all_cleanup_implementations() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mut registry = Registry::default();
    let counter = Arc::new(AtomicUsize::new(0));
    for kind in ["cleanup.sessions", "cleanup.tokens", "cleanup.support"] {
        registry.cleanup(kind, 3600, Arc::new(Counter(counter.clone())))?;
    }
    registry.schedule(&db.pool).await?;
    registry.schedule(&db.pool).await?;
    assert_eq!(registry.batch(&db.pool).await?, 3);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    NoopCleanup.clean().await?;
    db.close().await
}

#[tokio::test]
async fn invalid_input_is_refused() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for (kind, payload, key, attempts) in [
        ("", "{}", "k", 5),
        ("a", "bad json", "k", 5),
        ("a", "{}", "", 5),
        ("a", "{}", "k", 0),
    ] {
        let mut tx = db.pool.begin().await?;
        assert!(
            outbox::enqueue(
                &mut tx,
                kind,
                payload,
                key,
                OffsetDateTime::now_utc(),
                attempts
            )
            .await
            .is_err()
        );
        tx.rollback().await?;
    }
    assert!(outbox::claim(&db.pool, 0, 60).await.is_err());
    assert!(outbox::claim(&db.pool, 1, 0).await.is_err());
    assert!(outbox::schedule(&db.pool, "test", 0).await.is_err());
    db.close().await
}

#[test]
fn registry_refuses_duplicate_and_invalid_registration() {
    let mut registry = Registry::default();
    assert!(registry.cleanup("test", 1, Arc::new(NoopCleanup)).is_ok());
    assert!(registry.cleanup("test", 1, Arc::new(NoopCleanup)).is_err());
    assert!(registry.cleanup("", 1, Arc::new(NoopCleanup)).is_err());
    assert!(registry.cleanup("other", 0, Arc::new(NoopCleanup)).is_err());
}

#[derive(Debug)]
struct GatedHandler {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl Handler for GatedHandler {
    fn run<'a>(&'a self, _job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            self.started.notify_one();
            self.release.notified().await;
            Ok(())
        })
    }
}

#[tokio::test]
async fn shutdown_drains_in_flight_job() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enqueue("gated", "key", 5).await?;
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let shutdown = tokio::sync::Notify::new();
    let mut registry = Registry::default();
    registry.register(
        "gated",
        Arc::new(GatedHandler {
            started: started.clone(),
            release: release.clone(),
        }),
    )?;
    let worker = run_until(db.pool.clone(), registry, shutdown.notified());
    let control = async {
        started.notified().await;
        shutdown.notify_one();
        tokio::task::yield_now().await;
        assert_eq!(outbox::counts(&db.pool).await?.running, 1);
        release.notify_one();
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (result, control_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(worker, control)
    })
    .await?;
    result?;
    control_result?;
    assert_eq!(outbox::counts(&db.pool).await?.done, 1);
    db.close().await
}
