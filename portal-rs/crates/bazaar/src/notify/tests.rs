use super::*;
use sqlx::AssertSqlSafe;
use std::sync::atomic::{AtomicBool, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn notice(kind: NoticeKind) -> BazaarNotice {
    BazaarNotice {
        transition: Uuid::now_v7(),
        listing: Uuid::now_v7(),
        recipient: Uuid::now_v7(),
        evidence: Uuid::now_v7(),
        kind,
    }
}

#[test]
fn all_notices_render_in_both_languages_and_bad_ids_refuse() -> TestResult {
    assert!(!NotificationFlags::default().enabled);
    for kind in [
        NoticeKind::Outbid,
        NoticeKind::NewOffer,
        NoticeKind::EndingSoon,
        NoticeKind::Won,
        NoticeKind::MatchingSearch,
    ] {
        let event = notice(kind);
        let pl = event.render(NoticeLocale::Pl)?;
        let en = event.render(NoticeLocale::En)?;
        assert_ne!(pl.subject, en.subject);
        assert!(!pl.body.is_empty() && !en.body.is_empty());
        assert_eq!(pl.listing_path, format!("/bazaar/{}", event.listing));
        assert_eq!(NoticeKind::parse(kind.key())?, kind);
    }
    assert!(NoticeKind::parse("unknown").is_err());
    let mut event = notice(NoticeKind::Won);
    event.evidence = Uuid::nil();
    assert!(matches!(
        event.render(NoticeLocale::En),
        Err(Error::InvalidInput)
    ));
    Ok(())
}

struct Evidence(AtomicBool);
impl NoticeEvidence for Evidence {
    async fn verify(
        &self,
        _: &mut PgConnection,
        _: &BazaarNotice,
        _: OffsetDateTime,
    ) -> Result<bool, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
struct Sink(AtomicBool);
impl NoticeSink for Sink {
    async fn enqueue(
        &self,
        db: &mut PgConnection,
        id: Uuid,
        _: &BazaarNotice,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO test_deliveries VALUES($1)")
            .bind(id)
            .execute(db)
            .await?;
        if self.0.load(Ordering::SeqCst) {
            return Err(Error::Denied);
        }
        Ok(())
    }
}

struct Harness {
    api: BazaarNotifications<Evidence, Sink>,
    schema: String,
    admin: PgPool,
}
impl Harness {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("skipping PostgreSQL notification test: PORTAL_TEST_DATABASE_URL unset");
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        // Identifiers below are generated solely from a UUID, never from external input.
        let schema = format!("t62_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |db, _| {
                let query = format!("SET search_path TO {path}, public");
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(query)).execute(db).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105230105_t62_notification_outbox.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(
            include_str!("../../../../migrations/20261115110004_fix_04_scheduling.sql")
                .split("-- Authority resync")
                .next()
                .ok_or("missing cursor schema")?,
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE TABLE test_deliveries(id uuid PRIMARY KEY)")
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            api: BazaarNotifications {
                pool,
                evidence: Evidence(AtomicBool::new(true)),
                sink: Sink(AtomicBool::new(false)),
                flags: NotificationFlags { enabled: true },
            },
            schema,
            admin,
        }))
    }
    async fn record(&self, event: &BazaarNotice) -> Result<Uuid, Error> {
        let mut tx = self.api.pool.begin().await?;
        let id = self.api.record(&mut tx, event).await?;
        tx.commit().await?;
        Ok(id)
    }
    async fn count(&self, table: &str) -> Result<i64, Error> {
        Ok(sqlx::query_scalar(match table {
            "bazaar_notification_outbox" => "SELECT count(*) FROM bazaar_notification_outbox",
            "test_deliveries" => "SELECT count(*) FROM test_deliveries",
            _ => return Err(Error::InvalidInput),
        })
        .fetch_one(&self.api.pool)
        .await?)
    }
    async fn finish(self) -> TestResult {
        self.api.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
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
async fn postgres_records_all_kinds_and_replays_once() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    for kind in [
        NoticeKind::Outbid,
        NoticeKind::NewOffer,
        NoticeKind::EndingSoon,
        NoticeKind::Won,
        NoticeKind::MatchingSearch,
    ] {
        let mut event = notice(kind);
        let id = h.record(&event).await?;
        assert_eq!(id, h.record(&event).await?);
        event.evidence = Uuid::now_v7();
        assert!(matches!(
            h.record(&event).await,
            Err(Error::IdempotencyConflict)
        ));
    }
    assert_eq!(h.count("bazaar_notification_outbox").await?, 5);
    while h.api.dispatch_one().await? {}
    assert_eq!(h.count("test_deliveries").await?, 5);
    h.finish().await
}

#[tokio::test]
async fn postgres_disabled_keeps_pending_and_rollback_removes_event() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let Some(mut h) = Harness::new().await? else {
        return Ok(());
    };
    let event = notice(NoticeKind::Outbid);
    let mut tx = h.api.pool.begin().await?;
    h.api.record(&mut tx, &event).await?;
    tx.rollback().await?;
    assert_eq!(h.count("bazaar_notification_outbox").await?, 0);
    h.api.flags = NotificationFlags::default();
    h.record(&event).await?;
    assert!(matches!(h.api.dispatch_one().await, Err(Error::Disabled)));
    assert_eq!(h.count("test_deliveries").await?, 0);
    let state: String = sqlx::query_scalar("SELECT state FROM bazaar_notification_outbox")
        .fetch_one(&h.api.pool)
        .await?;
    assert_eq!(state, "pending");
    h.finish().await
}

#[tokio::test]
async fn postgres_unknown_evidence_and_bad_input_deny() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let mut event = notice(NoticeKind::Won);
    h.api.evidence.0.store(false, Ordering::SeqCst);
    assert!(matches!(h.record(&event).await, Err(Error::Denied)));
    event.recipient = Uuid::nil();
    assert!(matches!(h.record(&event).await, Err(Error::InvalidInput)));
    assert_eq!(h.count("bazaar_notification_outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn postgres_revoked_evidence_suppresses_and_failed_sink_retries_atomically() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    h.record(&notice(NoticeKind::MatchingSearch)).await?;
    h.api.sink.0.store(true, Ordering::SeqCst);
    assert!(h.api.dispatch_one().await.is_err());
    assert_eq!(h.count("test_deliveries").await?, 0);
    h.api.sink.0.store(false, Ordering::SeqCst);
    assert!(h.api.dispatch_one().await?);
    assert_eq!(h.count("test_deliveries").await?, 1);
    h.record(&notice(NoticeKind::EndingSoon)).await?;
    h.api.evidence.0.store(false, Ordering::SeqCst);
    assert!(h.api.dispatch_one().await?);
    assert_eq!(h.count("test_deliveries").await?, 1);
    assert!(!h.api.dispatch_one().await?);
    h.finish().await
}

#[tokio::test]
async fn postgres_concurrent_dispatch_has_one_effect() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    h.record(&notice(NoticeKind::NewOffer)).await?;
    let (a, b) = tokio::join!(h.api.dispatch_one(), h.api.dispatch_one());
    assert_eq!(usize::from(a?) + usize::from(b?), 1);
    assert_eq!(h.count("test_deliveries").await?, 1);
    h.finish().await
}

struct FailRecipient(Uuid);
impl NoticeSink for FailRecipient {
    async fn enqueue(
        &self,
        db: &mut PgConnection,
        id: Uuid,
        notice: &BazaarNotice,
    ) -> Result<(), Error> {
        if notice.recipient == self.0 {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO test_deliveries VALUES($1)")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
#[tokio::test]
async fn postgres_failing_sink_does_not_starve_later_notice() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let started = std::time::Instant::now();
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let blocked = notice(NoticeKind::NewOffer);
    h.record(&blocked).await?;
    h.record(&notice(NoticeKind::Won)).await?;
    let api = BazaarNotifications {
        pool: h.api.pool.clone(),
        evidence: Evidence(AtomicBool::new(true)),
        sink: FailRecipient(blocked.recipient),
        flags: NotificationFlags { enabled: true },
    };
    assert!(api.dispatch_one().await.is_err());
    assert!(api.dispatch_one().await?);
    assert_eq!(h.count("test_deliveries").await?, 1);
    h.finish().await?;
    eprintln!(
        "postgres_failing_sink_does_not_starve_later_notice: {:?}",
        started.elapsed()
    );
    Ok(())
}

struct FencedEvidence(std::sync::Arc<tokio::sync::Notify>);
impl NoticeEvidence for FencedEvidence {
    async fn verify(
        &self,
        db: &mut PgConnection,
        _: &BazaarNotice,
        _: OffsetDateTime,
    ) -> Result<bool, Error> {
        self.0.notify_one();
        sqlx::query("SELECT pg_advisory_xact_lock(64006)")
            .execute(db)
            .await?;
        Ok(true)
    }
}
#[tokio::test]
async fn postgres_notification_record_and_dispatch_share_evidence_lock_order() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let started = std::time::Instant::now();
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let event = notice(NoticeKind::Won);
    let id = h.record(&event).await?;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let api = std::sync::Arc::new(BazaarNotifications {
        pool: h.api.pool.clone(),
        evidence: FencedEvidence(entered.clone()),
        sink: Sink(AtomicBool::new(false)),
        flags: NotificationFlags { enabled: true },
    });
    let mut record = h.api.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(64006)")
        .execute(&mut *record)
        .await?;
    let dispatch = api.clone();
    let task = tokio::spawn(async move { dispatch.dispatch_one().await });
    entered.notified().await;
    assert_eq!(api.record(&mut record, &event).await?, id);
    record.commit().await?;
    assert!(task.await??);
    assert_eq!(h.count("test_deliveries").await?, 1);
    h.finish().await?;
    eprintln!(
        "postgres_notification_record_and_dispatch_share_evidence_lock_order: {:?}",
        started.elapsed()
    );
    Ok(())
}
