use super::*;
use outbox::{OffsetDateTime, Uuid, query, query_scalar};
use std::sync::atomic::{AtomicUsize, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) struct Database {
    pub(super) pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    pub(super) async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
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
                    query("SELECT set_config('search_path', $1, false), set_config('application_name', $1, false)")
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
    pub(super) async fn close(self) -> TestResult {
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
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::{Layer, layer::SubscriberExt};

    struct ObserveDrain(Arc<tokio::sync::Notify>);
    impl<S: tracing::Subscriber> Layer<S> for ObserveDrain {
        fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
            if event.metadata().name() == "outbox_worker_drain_started" {
                self.0.notify_one();
            }
        }
    }

    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.enqueue("gated", "key", 5).await?;
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let shutdown = tokio::sync::Notify::new();
    let draining = Arc::new(tokio::sync::Notify::new());
    let mut registry = Registry::default();
    registry.register(
        "gated",
        Arc::new(GatedHandler {
            started: started.clone(),
            release: release.clone(),
        }),
    )?;
    let subscriber = tracing_subscriber::registry().with(ObserveDrain(draining.clone()));
    let worker = run_until(db.pool.clone(), registry, shutdown.notified()).with_subscriber(subscriber);
    let control = async {
        started.notified().await;
        shutdown.notify_one();
        draining.notified().await;
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

#[tokio::test]
async fn paid_topup_webhook_outbox_worker_balance() -> TestResult {
    paid_topup_and_refund_scenario().await
}

#[tokio::test]
async fn generic_refund_after_real_worker_credit_is_refused() -> TestResult {
    paid_topup_and_refund_scenario().await
}

async fn paid_topup_and_refund_scenario() -> TestResult {
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    use oteryn_payments::{
        Amount, IntentState, PaymentEvent, PaymentFlags, Payments, TestProvider,
    };
    struct Active;
    impl oteryn_payments::AccountReader for Active {
        async fn state_in(
            &self,
            _: &mut sqlx::PgConnection,
            id: Uuid,
        ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
            self.state(id).await
        }
        async fn state(
            &self,
            _: Uuid,
        ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
            Ok(oteryn_payments::AccountState::Active)
        }
    }
    // Empty reversible migration preserves all history through rollback/reapply.
    oteryn_db::MIGRATOR.undo(&db.pool, 20261107170001).await?;
    oteryn_db::migrate(&db.pool).await?;
    // Financial aggregation uses a closed fixture day, independent of UTC now.
    let day = 1_789_948_800_u64;
    let credited_at = OffsetDateTime::from_unix_timestamp(i64::try_from(day + 3600)?)?;
    // Set a literal default in this disposable schema before the real worker writes.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "ALTER TABLE wallet_entries ALTER COLUMN created_at SET DEFAULT to_timestamp({})",
        credited_at.unix_timestamp()
    )))
    .execute(&db.pool)
    .await?;
    let account = Uuid::now_v7();
    query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'topup@example.invalid',now(),'unused','active','en')").bind(account).execute(&db.pool).await?;
    let provider = TestProvider::new(&[5; 32])?;
    let payments = Payments::new(
        db.pool.clone(),
        provider.clone(),
        Active,
        PaymentFlags { enabled: true },
    );
    let intent = payments
        .create_intent(
            account,
            Amount {
                minor_units: 250,
                currency: "PLN".into(),
            },
        )
        .await?;
    query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'fixed',1234,250,'PLN',$3)").bind(Uuid::now_v7()).bind(account).bind(intent.id).execute(&db.pool).await?;
    let body = serde_json::to_vec(&PaymentEvent {
        event_id: "paid".into(),
        intent_id: intent.id,
        provider_reference: intent.provider_reference,
        amount: intent.amount,
        state: IntentState::Succeeded,
    })?;
    payments.webhook(&body, &provider.sign(&body)?).await?;
    // Deliver two distinct outbox jobs for the same intent concurrently on the
    // first credit, then replay again after acknowledgement.
    let payload: String =
        query_scalar("SELECT payload::text FROM outbox WHERE kind='wallet.top_up'")
            .fetch_one(&db.pool)
            .await?;
    let mut tx = db.pool.begin().await?;
    outbox::enqueue(
        &mut tx,
        "wallet.top_up",
        &payload,
        "concurrent-copy",
        OffsetDateTime::now_utc(),
        10,
    )
    .await?;
    tx.commit().await?;
    let mut registry = Registry::default();
    registry.register(
        "wallet.top_up",
        Arc::new(crate::wallet_topup::Delivery {
            pool: db.pool.clone(),
            payments_enabled: true,
            wallet_enabled: true,
        }),
    )?;
    registry.batch(&db.pool).await?;
    let balance: i64 = query_scalar(
        "SELECT COALESCE(SUM(available_delta),0)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(
        balance, 1234,
        "paid checkout must credit package coins, not minor units"
    );
    let mut refund: PaymentEvent = serde_json::from_slice(&body)?;
    refund.event_id = "refund-after-worker-credit".into();
    refund.state = IntentState::Refunded;
    let refund = serde_json::to_vec(&refund)?;
    assert!(matches!(
        payments.webhook(&refund, &provider.sign(&refund)?).await,
        Err(oteryn_payments::Error::IllegalTransition)
    ));
    assert_eq!(
        payments.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    let refunds: i64 = query_scalar("SELECT count(*) FROM payment_events WHERE state='refunded'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(refunds, 0);
    assert_eq!(outbox::counts(&db.pool).await?.done, 2);
    let payload: String =
        query_scalar("SELECT payload::text FROM outbox WHERE kind='wallet.top_up'")
            .fetch_one(&db.pool)
            .await?;
    let delivery = crate::wallet_topup::Delivery {
        pool: db.pool.clone(),
        payments_enabled: true,
        wallet_enabled: true,
    };
    let replay = Job {
        id: Uuid::now_v7(),
        kind: "wallet.top_up".into(),
        payload,
        attempts: 1,
    };
    delivery.run(&replay).await?;
    let other = Job {
        id: Uuid::now_v7(),
        ..replay.clone()
    };
    let (a, b) = tokio::join!(delivery.run(&replay), delivery.run(&other));
    a?;
    b?;
    let count: i64 = query_scalar("SELECT count(*) FROM wallet_entries WHERE operation_id=$1")
        .bind(intent.id)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    let audit: i64 =
        query_scalar("SELECT count(*) FROM audit_log WHERE action='wallet.top_up.credited'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audit, 1);
    let mut tx = db.pool.begin().await?;
    let aggregate = oteryn_admin::analytics::purchased::read_in(&mut tx, day, day + 86400).await?;
    assert_eq!(aggregate.count, "1");
    assert_eq!(aggregate.coins, "1234");
    assert_eq!(aggregate.paid[0].minor_units, "250");
    assert_eq!(aggregate.paid[0].currency, "PLN");
    let empty = oteryn_admin::analytics::purchased::read_in(&mut tx, day - 86400, day).await?;
    assert_eq!(empty.count, "0");
    assert_eq!(empty.coins, "0");
    assert!(empty.paid.is_empty());
    tx.rollback().await?;
    let mut tx = db.pool.begin().await?;
    assert!(
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261107180001_wallet_topup.down.sql"
        ))
        .execute(&mut *tx)
        .await
        .is_err(),
        "populated top_up rollback must refuse"
    );
    tx.rollback().await?;
    oteryn_db::migrate(&db.pool).await?;

    let paid = payments.intent(intent.id).await?;
    for attempt in 0..2 {
        let body = serde_json::to_vec(&PaymentEvent {
            event_id: format!("refund-{attempt}"),
            intent_id: paid.id,
            provider_reference: paid.provider_reference.clone(),
            amount: paid.amount.clone(),
            state: IntentState::Refunded,
        })?;
        assert!(matches!(
            payments.webhook(&body, &provider.sign(&body)?).await,
            Err(oteryn_payments::Error::IllegalTransition)
        ));
    }
    assert_eq!(
        payments.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    let balance: i64 =
        query_scalar("SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(balance, 1234);
    eprintln!(
        "PG paid_topup_webhook_outbox_worker_balance (L6-03) {:.3}s",
        started.elapsed().as_secs_f64()
    );
    db.close().await
}

#[tokio::test]
async fn paid_topup_survives_ten_deferrals_and_credits_once_after_unlock() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let intent = Uuid::now_v7();
    query(
        "INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale,email_change_blocked_until)
         VALUES($1,'deferred-topup@example.invalid',now(),'unused','active','en',now()+interval '72 hours')",
    )
    .bind(account)
    .execute(&db.pool)
    .await?;
    query(
        "INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state)
         VALUES($1,$2,'test',$3,250,'PLN','succeeded')",
    )
    .bind(intent)
    .bind(account)
    .bind(intent.to_string())
    .execute(&db.pool)
    .await?;
    query(
        "INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id)
         VALUES($1,$2,'fixed',1234,250,'PLN',$3)",
    )
    .bind(Uuid::now_v7())
    .bind(account)
    .bind(intent)
    .execute(&db.pool)
    .await?;
    let payload = serde_json::to_string(&oteryn_payments::TopUpJob {
        intent_id: intent,
        account_id: account,
        amount: 250,
        currency: "PLN".into(),
    })?;
    let mut tx = db.pool.begin().await?;
    let job = outbox::enqueue(
        &mut tx,
        "wallet.top_up",
        &payload,
        &intent.to_string(),
        OffsetDateTime::now_utc(),
        10,
    )
    .await?;
    tx.commit().await?;
    let mut registry = Registry::default();
    registry.register(
        "wallet.top_up",
        Arc::new(crate::wallet_topup::Delivery {
            pool: db.pool.clone(),
            payments_enabled: true,
            wallet_enabled: true,
        }),
    )?;

    for _ in 0..10 {
        // Advance the retry schedule without waiting, leaving the restriction active.
        query("UPDATE outbox SET run_after=now() WHERE id=$1 AND state='pending'")
            .bind(job)
            .execute(&db.pool)
            .await?;
        assert_eq!(registry.batch(&db.pool).await?, 1);
    }
    let deferred: (String, i32, bool, bool) = sqlx::query_as(
        "SELECT state,attempts,run_after>now(),leased_until IS NULL FROM outbox WHERE id=$1",
    )
    .bind(job)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(deferred, ("pending".into(), 0, true, true));
    let balance = || {
        query_scalar::<_, i64>("SELECT COALESCE(sum(available_delta),0)::bigint FROM wallet_entries WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
    };
    assert_eq!(balance().await?, 0);
    assert_eq!(registry.batch(&db.pool).await?, 0);

    query("UPDATE accounts SET email_change_blocked_until=DEFAULT WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    query("UPDATE outbox SET run_after=now() WHERE id=$1 AND state='pending'")
        .bind(job)
        .execute(&db.pool)
        .await?;
    assert_eq!(registry.batch(&db.pool).await?, 1);
    assert_eq!(registry.batch(&db.pool).await?, 0);
    let completed: (String, i32) = sqlx::query_as("SELECT state,attempts FROM outbox WHERE id=$1")
        .bind(job)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(completed, ("done".into(), 1));
    assert_eq!(balance().await?, 1234);
    let audits: (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE action='wallet.top_up.credited'),
                count(*) FILTER (WHERE action='wallet.top_up.denied') FROM audit_log",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(audits, (1, 0));
    db.close().await
}

#[tokio::test]
async fn topup_refusals_are_audited_and_dead_lettered_without_credit() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    for case in [
        "amount",
        "currency",
        "account",
        "intent_amount",
        "intent_currency",
        "intent_account",
        "dispatched_refund",
        "checkout_amount",
        "checkout_currency",
        "checkout_account",
        "audit_outage",
        "missing_checkout",
        "missing_intent",
        "suspended",
        "closed",
        "unverified",
        "unknown_account",
        "payments_off",
        "wallet_off",
        "pending",
        "refunded",
        "malformed",
    ] {
        let account = Uuid::now_v7();
        query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'unused','active','en')").bind(account).bind(format!("{case}@example.invalid")).execute(&db.pool).await?;
        let intent = Uuid::now_v7();
        query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'PLN','succeeded')").bind(intent).bind(account).bind(intent.to_string()).execute(&db.pool).await?;
        query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'fixed',1234,250,'PLN',$3)").bind(Uuid::now_v7()).bind(account).bind(intent).execute(&db.pool).await?;
        let mut payload = oteryn_payments::TopUpJob {
            intent_id: intent,
            account_id: account,
            amount: 250,
            currency: "PLN".into(),
        };
        match case {
            "dispatched_refund" => {
                let op = Uuid::now_v7();
                query("INSERT INTO commerce_operations(id,actor_id,target_id,command,reason_sha256,requires_approval) VALUES($1,$2,$3,'{}',repeat('0',64),false)").bind(op).bind(account).bind(intent).execute(&db.pool).await?;
                query("INSERT INTO commerce_refunds(operation_id,intent_id,account_id,amount,currency,provider,state,provider_reference) VALUES($1,$2,$3,250,'PLN','test','done','accepted')").bind(op).bind(intent).bind(account).execute(&db.pool).await?;
            }
            "amount" => payload.amount = 251,
            "currency" => payload.currency = "EUR".into(),
            "account" => payload.account_id = Uuid::now_v7(),
            "intent_amount" => {
                query("UPDATE payment_intents SET amount=251 WHERE id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "intent_currency" => {
                query("UPDATE payment_intents SET currency='EUR' WHERE id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "intent_account" => {
                query("UPDATE payment_intents SET account_id=$2 WHERE id=$1")
                    .bind(intent)
                    .bind(Uuid::now_v7())
                    .execute(&db.pool)
                    .await?;
            }
            "checkout_amount" => {
                query("UPDATE store_web_topups SET minor_units=251 WHERE payment_intent_id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "checkout_currency" => {
                query("UPDATE store_web_topups SET currency='EUR' WHERE payment_intent_id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "checkout_account" => {
                query("UPDATE store_web_topups SET account_id=$2 WHERE payment_intent_id=$1")
                    .bind(intent)
                    .bind(Uuid::now_v7())
                    .execute(&db.pool)
                    .await?;
            }
            "audit_outage" => {
                sqlx::raw_sql("CREATE FUNCTION reject_test_topup_credit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='wallet.top_up.credited' THEN RAISE EXCEPTION 'test credit audit outage'; END IF; RETURN NEW; END $$; CREATE TRIGGER test_topup_credit_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_test_topup_credit()") .execute(&db.pool).await?;
            }
            "missing_checkout" => {
                query("DELETE FROM store_web_topups WHERE payment_intent_id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "missing_intent" => {
                query("DELETE FROM payment_intents WHERE id=$1")
                    .bind(intent)
                    .execute(&db.pool)
                    .await?;
            }
            "suspended" | "closed" => {
                query("UPDATE accounts SET status=$2 WHERE id=$1")
                    .bind(account)
                    .bind(case)
                    .execute(&db.pool)
                    .await?;
            }
            "unverified" => {
                query("UPDATE accounts SET email_verified_at=NULL WHERE id=$1")
                    .bind(account)
                    .execute(&db.pool)
                    .await?;
            }
            "unknown_account" => {
                query("DELETE FROM accounts WHERE id=$1")
                    .bind(account)
                    .execute(&db.pool)
                    .await?;
            }
            "pending" | "refunded" => {
                query("UPDATE payment_intents SET state=$2 WHERE id=$1")
                    .bind(intent)
                    .bind(case)
                    .execute(&db.pool)
                    .await?;
            }
            _ => {}
        }
        let body = if case == "malformed" {
            "{}".into()
        } else {
            serde_json::to_string(&payload)?
        };
        let mut tx = db.pool.begin().await?;
        outbox::enqueue(
            &mut tx,
            "wallet.top_up",
            &body,
            &intent.to_string(),
            OffsetDateTime::now_utc(),
            1,
        )
        .await?;
        tx.commit().await?;
        let mut registry = Registry::default();
        registry.register(
            "wallet.top_up",
            Arc::new(crate::wallet_topup::Delivery {
                pool: db.pool.clone(),
                payments_enabled: case != "payments_off",
                wallet_enabled: case != "wallet_off",
            }),
        )?;
        registry.batch(&db.pool).await?;
        let count: i64 = query_scalar("SELECT count(*) FROM wallet_entries")
            .fetch_one(&db.pool)
            .await?;
        assert_eq!(count, 0, "{case}");
        if case == "audit_outage" {
            sqlx::raw_sql("DROP TRIGGER test_topup_credit_audit ON audit_log; DROP FUNCTION reject_test_topup_credit()") .execute(&db.pool).await?;
        }
    }
    assert_eq!(outbox::counts(&db.pool).await?.dead, 22);
    let audit: i64 =
        query_scalar("SELECT count(*) FROM audit_log WHERE action='wallet.top_up.denied'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audit, 22);
    db.close().await
}

// Exercise the actual payment-intent FOR UPDATE, not a simulated slow handler.
async fn locked_topup(
    db: &Database,
) -> Result<(Uuid, sqlx::Transaction<'static, sqlx::Postgres>), Box<dyn std::error::Error>> {
    let account = Uuid::now_v7();
    let intent = Uuid::now_v7();
    query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'deadline@example.invalid',now(),'unused','active','en')")
        .bind(account).execute(&db.pool).await?;
    query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'PLN','succeeded')")
        .bind(intent).bind(account).bind(intent.to_string()).execute(&db.pool).await?;
    query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'fixed',1234,250,'PLN',$3)")
        .bind(Uuid::now_v7()).bind(account).bind(intent).execute(&db.pool).await?;
    let payload = serde_json::to_string(&oteryn_payments::TopUpJob {
        intent_id: intent,
        account_id: account,
        amount: 250,
        currency: "PLN".into(),
    })?;
    let mut tx = db.pool.begin().await?;
    outbox::enqueue(
        &mut tx,
        "wallet.top_up",
        &payload,
        &intent.to_string(),
        OffsetDateTime::now_utc(),
        5,
    )
    .await?;
    tx.commit().await?;
    let mut lock = db.pool.begin().await?;
    query("SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE")
        .bind(intent)
        .execute(&mut *lock)
        .await?;
    Ok((intent, lock))
}

fn topup_registry(db: &Database) -> Result<Registry, String> {
    let mut registry = Registry::default();
    registry.register(
        "wallet.top_up",
        Arc::new(crate::wallet_topup::Delivery {
            pool: db.pool.clone(),
            payments_enabled: true,
            wallet_enabled: true,
        }),
    )?;
    Ok(registry)
}

#[tokio::test]
async fn locked_topup_deadline_allows_later_batch_and_idempotent_retry() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let (intent, lock) = locked_topup(&db).await?;
    let mut registry = topup_registry(&db)?;
    registry.execution_budget = Duration::from_millis(200);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), registry.batch(&db.pool)).await??,
        1
    );
    assert_eq!(outbox::counts(&db.pool).await?.running, 1);
    let counter = Arc::new(AtomicUsize::new(0));
    registry.register("later", Arc::new(Counter(counter.clone())))?;
    db.enqueue("later", "later", 5).await?;
    assert_eq!(registry.batch(&db.pool).await?, 1);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    // The cancelled transaction must not resume and credit after this unlock.
    lock.rollback().await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let entries: i64 = query_scalar("SELECT count(*) FROM wallet_entries")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(
        entries, 0,
        "cancelled delivery must never resume after unlock"
    );
    query("UPDATE outbox SET leased_until=now()-interval '1 second' WHERE kind='wallet.top_up'")
        .execute(&db.pool)
        .await?;
    registry.execution_budget = Duration::from_secs(5);
    registry.batch(&db.pool).await?;
    let entries: i64 = query_scalar("SELECT count(*) FROM wallet_entries WHERE operation_id=$1")
        .bind(intent)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(entries, 1);
    assert_eq!(outbox::counts(&db.pool).await?.done, 2);
    db.close().await?;
    eprintln!(
        "PG locked_topup_deadline_allows_later_batch_and_idempotent_retry duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn shutdown_budget_cancels_locked_topup_and_leaves_retryable_lease() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let (_, lock) = locked_topup(&db).await?;
    let mut registry = topup_registry(&db)?;
    registry.shutdown_budget = Duration::from_millis(100);
    let shutdown = tokio::sync::Notify::new();
    let control = async {
        // Observe the real lock wait before sending the shutdown signal.
        loop {
            let waiting: i64 = query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock' AND query LIKE 'SELECT account_id,amount,currency,state FROM payment_intents%'").bind(&db.schema).fetch_one(&db.pool).await?;
            if waiting > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let signalled = std::time::Instant::now();
        shutdown.notify_one();
        Ok::<_, Box<dyn std::error::Error>>(signalled)
    };
    let (result, signalled) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            run_until(db.pool.clone(), registry, shutdown.notified()),
            control
        )
    })
    .await?;
    result?;
    assert!(signalled?.elapsed() < Duration::from_secs(1));
    lock.rollback().await?;
    let entries: i64 = query_scalar("SELECT count(*) FROM wallet_entries")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(entries, 0);
    assert_eq!(outbox::counts(&db.pool).await?.running, 1);
    query("UPDATE outbox SET leased_until=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    topup_registry(&db)?.batch(&db.pool).await?;
    assert_eq!(outbox::counts(&db.pool).await?.done, 1);
    db.close().await?;
    eprintln!(
        "PG shutdown_budget_cancels_locked_topup_and_leaves_retryable_lease duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn sixteen_locked_topups_finish_after_heartbeat_with_exhausted_handler_pool() -> TestResult {
    let started = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let (intent, lock) = locked_topup(&db).await?;
    let payload: String =
        query_scalar("SELECT payload::text FROM outbox WHERE kind='wallet.top_up'")
            .fetch_one(&db.pool)
            .await?;
    for index in 1..16 {
        let mut tx = db.pool.begin().await?;
        outbox::enqueue(
            &mut tx,
            "wallet.top_up",
            &payload,
            &format!("saturation-{index}"),
            OffsetDateTime::now_utc(),
            5,
        )
        .await?;
        tx.commit().await?;
    }
    let handlers = db
        .pool
        .options()
        .clone()
        .max_connections(16)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with((*db.pool.connect_options()).clone())
        .await?;
    let mut registry = Registry {
        heartbeat_interval: Duration::from_millis(100),
        ..Default::default()
    };
    registry.register(
        "wallet.top_up",
        Arc::new(crate::wallet_topup::Delivery {
            pool: handlers.clone(),
            payments_enabled: true,
            wallet_enabled: true,
        }),
    )?;
    let unlock = async {
        loop {
            let waiting: i64 = query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock' AND query LIKE 'SELECT account_id,amount,currency,state FROM payment_intents%'")
                .bind(&db.schema).fetch_one(&db.pool).await?;
            if waiting == 16 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let baseline: time::OffsetDateTime =
            query_scalar("SELECT max(leased_until) FROM outbox WHERE state='running'")
                .fetch_one(&db.pool)
                .await?;
        let renewal = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let renewed: i64 = query_scalar(
                    "SELECT count(*) FROM outbox WHERE state='running' AND leased_until > $1",
                )
                .bind(baseline)
                .fetch_one(&db.pool)
                .await?;
                if renewed == 16 {
                    return Ok::<_, sqlx::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        lock.rollback().await?;
        assert!(
            renewal.is_ok(),
            "heartbeat must successfully extend all sixteen live leases before releasing handlers"
        );
        renewal??;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (batch, unlock) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(registry.batch(&handlers), unlock)
    })
    .await?;
    unlock?;
    assert_eq!(batch?, 16);
    assert_eq!(outbox::counts(&db.pool).await?.done, 16);
    let entries: i64 = query_scalar("SELECT count(*) FROM wallet_entries WHERE operation_id=$1")
        .bind(intent)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(entries, 1);
    db.enqueue("wallet.top_up", "later", 1).await?;
    assert_eq!(registry.batch(&handlers).await?, 1);
    handlers.close().await;
    db.close().await?;
    eprintln!(
        "PG sixteen_locked_topups_finish_after_heartbeat_with_exhausted_handler_pool duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn handler_finishes_while_renewal_waits_for_capacity() -> TestResult {
    let started_at = std::time::Instant::now();
    let db = Database::new()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    db.enqueue("gated", "renewal-pending", 5).await?;
    let job = outbox::claim(&db.pool, 1, 60)
        .await?
        .pop()
        .ok_or("job missing")?;
    let leases = db
        .pool
        .options()
        .clone()
        .max_connections(1)
        .min_connections(0)
        .connect_with((*db.pool.connect_options()).clone())
        .await?;
    let held = leases.acquire().await?;
    let renewal_started = tokio::sync::Notify::new();
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let handler = Arc::new(GatedHandler {
        started: started.clone(),
        release: release.clone(),
    });
    let control = async {
        started.notified().await;
        renewal_started.notified().await;
        assert_eq!(
            leases.num_idle(),
            0,
            "renewal must wait for exhausted capacity"
        );
        release.notify_one();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(
            execute_observed(
                &db.pool,
                &leases,
                job,
                Some(handler),
                Duration::from_secs(15),
                || renewal_started.notify_one(),
            ),
            control
        )
    })
    .await?;
    result?;
    assert_eq!(outbox::counts(&db.pool).await?.done, 1);
    drop(held);
    leases.close().await;
    db.close().await?;
    eprintln!(
        "PG handler_finishes_while_renewal_waits_for_capacity duration={:.3}s",
        started_at.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn postgres_journal_registry_expires_and_erases_lifecycle_payloads() -> TestResult {
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let character = Uuid::now_v7();
    let transferred = Uuid::now_v7();
    query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,'journal@example.test','x','active','en')").bind(account).execute(&db.pool).await?;
    // Payload rows represent each retention class and an unexpired sibling.
    for (key, family, progress, expired) in [
        ("bank", "BANK", false, true),
        ("login", "game_login", true, true),
        ("stamina", "stamina", true, true),
        ("live", "stamina", true, false),
    ] {
        query("INSERT INTO pd_streams(stream_key,authority,account_id,family,progress) VALUES($1,'journal-test',$2,$3,$4)").bind(key).bind(account.to_string()).bind(family).bind(progress).execute(&db.pool).await?;
        let record: serde_json::Value = serde_json::from_str(if family == "BANK" {
            include_str!(
                "../../../../docs/contracts/fixtures/game-journal-items-v1/record-bank-entry.valid.json"
            )
        } else {
            include_str!(
                "../../../../docs/contracts/fixtures/game-journal-progress-v1/record-stamina.valid.json"
            )
        })?;
        let payload = if family == "game_login" {
            include_bytes!("../../../../docs/contracts/fixtures/game-journal-progress-v1/payload-game-login.valid.json").to_vec()
        } else {
            serde_json::to_vec(&record["payload"])?
        };
        query("INSERT INTO pd_records(stream_key,source_revision,authority,recovery_generation,record_id,entity_key,character_id,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms) VALUES($1,1,'journal-test','3',$1,$1,$2,'test','UPSERT',0,0,decode(repeat('00',32),'hex'),$4,CASE WHEN $3 THEN 0 ELSE 9000000000000 END)").bind(key).bind(character.to_string()).bind(expired).bind(payload).execute(&db.pool).await?;
    }
    let mut registry = Registry::default();
    register_journal_cleanup(&mut registry, &db.pool)?;
    registry.schedule(&db.pool).await?;
    assert_eq!(registry.batch(&db.pool).await?, 1);
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    for (key, subject) in [
        ("transferred", Some(transferred.to_string())),
        ("account", None),
    ] {
        query("INSERT INTO pd_records SELECT stream_key,2,'journal-test','3',$1,$1,$2,scope_key,kind,occurred_ms,coverage_ms,digest,payload,expires_ms FROM pd_records WHERE stream_key='live' AND source_revision=1")
            .bind(key).bind(subject).execute(&db.pool).await?;
        // Distinct revisions in the same stream retain independent subjects.
        query("UPDATE pd_records SET source_revision=3 WHERE record_id='transferred'")
            .execute(&db.pool)
            .await?;
    }
    // A scheduled deletion is not terminal and must retain current payloads.
    query("INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json) VALUES($1,$2,1,1,'1','{\"lifecycle\":\"DELETION_PENDING\"}')").bind(account).bind(character).execute(&db.pool).await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_erasure_jobs")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    query("UPDATE character_service_states SET snapshot_json='{\"lifecycle\":\"DELETED\"}',revision=2").execute(&db.pool).await?;
    db.enqueue("cleanup.journal-payloads", "character-erasure", 3)
        .await?;
    registry.batch(&db.pool).await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?,
        2
    );
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_deleted_scopes")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    // The same authoritative handoff also covers transfers and account closure.
    query("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json,receipt_json,state) VALUES($1,$1,'transfer','TransferCharacterOwnership',$2,$3,'{}','{}',json_build_object('outcome','COMMITTED','from_account_id',$2::text,'character_id',$4::text)::text,'TERMINAL')").bind(Uuid::now_v7()).bind(account).bind(Uuid::now_v7()).bind(transferred).execute(&db.pool).await?;
    db.enqueue("cleanup.journal-payloads", "transfer-erasure", 3)
        .await?;
    registry.batch(&db.pool).await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_records WHERE payload IS NOT NULL")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    query("UPDATE accounts SET status='closed' WHERE id=$1")
        .bind(account)
        .execute(&db.pool)
        .await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_erasure_jobs")
            .fetch_one(&db.pool)
            .await?,
        1
    );
    db.enqueue("cleanup.journal-payloads", "account-erasure", 3)
        .await?;
    registry.batch(&db.pool).await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_streams WHERE NOT erased")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM pd_erasure_jobs")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await?;
    eprintln!(
        "postgres_journal_registry_expires_and_erases_lifecycle_payloads duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn audit02_credentials_only_delivers_password_notification() -> TestResult {
    if std::env::var_os("A2_CREDENTIALS_ONLY_CHILD").is_some() {
        return tokio::runtime::Runtime::new()?.block_on(credentials_only_delivery());
    }
    for credentials_enabled in ["true", "false"] {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "worker::tests::audit02_credentials_only_delivers_password_notification",
                "--nocapture",
            ])
            .env("A2_CREDENTIALS_ONLY_CHILD", "1")
            .env("OTERYN_ACCOUNT_CREDENTIALS", credentials_enabled)
            .env("OTERYN_ACCOUNT_LOGIN", "false")
            .env("OTERYN_ACCOUNT_SESSIONS", "false")
            .env("OTERYN_ACCOUNT_PASSKEYS", "false")
            .env("OTERYN_SMTP_HOST", "localhost")
            .env("OTERYN_SMTP_USERNAME", "test-sender")
            .env("OTERYN_SMTP_PASSWORD", "test-smtp-password")
            .env("OTERYN_SMTP_FROM", "security@example.test")
            .output()?;
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
async fn credentials_only_delivery() -> TestResult {
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('credential-notify@example.test',$1,'active','en',now()) RETURNING id")
        .bind(oteryn_identity::password::hash("current test password")?).fetch_one(&db.pool).await?;
    let session = vec![21u8; 32];
    query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,repeat('c',64),'test',now()+interval '1 hour',now()+interval '1 day')").bind(&session).bind(id).execute(&db.pool).await?;
    // Complete, empty breach-cache range for the replacement password's SHA-1 prefix.
    query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES('287B4',ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id())").execute(&db.pool).await?;
    let credentials = oteryn_identity::credentials::Credentials {
        pool: db.pool.clone(),
        enabled: true,
        revoker: Arc::new(oteryn_identity::credentials::TransactionalRevoker),
        notifications: Arc::new(
            oteryn_identity::sessions::security_notifications(db.pool.clone())
                .map_err(|e| e.to_string())?,
        ),
    };
    assert_eq!(
        credentials
            .change(
                &session,
                oteryn_identity::credentials::PasswordChange {
                    current_password: "current test password".into(),
                    password: "replacement test password".into(),
                    password_confirmation: "replacement test password".into(),
                }
            )
            .await?,
        oteryn_identity::credentials::Outcome::Changed
    );
    let mut delivery = crate::sessions_http::delivery(db.pool.clone())?
        .ok_or("credentials producer has no notification consumer")?;
    let sender = Arc::new(oteryn_notify::CaptureSender::default());
    delivery.sender = sender.clone();
    let mut registry = Registry::default();
    register_security_notifications(&mut registry, db.pool.clone())?;
    *registry
        .handlers
        .get_mut(oteryn_notify::EMAIL_JOB)
        .ok_or("production registry omitted notify.email")? = Arc::new(delivery);
    assert_eq!(registry.batch(&db.pool).await?, 1);
    {
        let emails = sender.emails.lock().map_err(|e| e.to_string())?;
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].recipient, "credential-notify@example.test");
        assert!(emails[0].subject.to_lowercase().contains("password"));
    }
    let failed: i64 =
        query_scalar("SELECT count(*) FROM outbox WHERE state<>'done' OR last_error IS NOT NULL")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(failed, 0);
    eprintln!(
        "audit02_credentials_only_delivers_password_notification duration: {:?}",
        started.elapsed()
    );
    db.close().await
}

#[tokio::test]
async fn scheduling_conflict_has_a_deadline_and_closes_blocked_session() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mut blocker = db.pool.begin().await?;
    // A long slot avoids crossing the schedule boundary during this test.
    query("INSERT INTO outbox(kind,payload,idempotency_key) VALUES('blocked','{}','schedule:' || floor(extract(epoch FROM now()) / 2000000000)::text)")
        .execute(&mut *blocker).await?;
    let mut registry = Registry {
        execution_budget: Duration::from_millis(150),
        ..Registry::default()
    };
    registry.cleanup("blocked", 2_000_000_000, Arc::new(NoopCleanup))?;
    let start = std::time::Instant::now();
    let error = tokio::time::timeout(Duration::from_secs(2), registry.schedule(&db.pool))
        .await?
        .err()
        .ok_or("conflicting schedule must time out")?;
    assert!(error.to_string().contains("scheduling deadline"));
    assert!(start.elapsed() >= Duration::from_millis(150));
    // Cancellation must terminate the database session while its blocker is
    // still held, rather than leave a query waiting in a pooled connection.
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: i64 = query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock'")
                .bind(&db.schema).fetch_one(&db.admin).await?;
            if waiting == 0 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_, outbox::Error>(())
    }).await??;
    blocker.rollback().await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM outbox WHERE kind='blocked'")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}

#[tokio::test]
async fn claiming_table_lock_has_a_deadline_and_closes_blocked_session() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mut blocker = db.pool.begin().await?;
    query("LOCK TABLE outbox IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await?;
    let registry = Registry {
        execution_budget: Duration::from_millis(150),
        ..Registry::default()
    };
    let start = std::time::Instant::now();
    let error = tokio::time::timeout(Duration::from_secs(2), registry.batch(&db.pool))
        .await?
        .err()
        .ok_or("table lock must time out")?;
    assert!(error.to_string().contains("claiming deadline"));
    assert!(start.elapsed() >= Duration::from_millis(150));
    // Cancellation must terminate the database session while its blocker is
    // still held, rather than leave a query waiting in a pooled connection.
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: i64 = query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1 AND wait_event_type='Lock'")
                .bind(&db.schema).fetch_one(&db.admin).await?;
            if waiting == 0 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_, outbox::Error>(())
    }).await??;
    blocker.rollback().await?;
    assert_eq!(
        query_scalar::<_, i64>("SELECT count(*) FROM outbox WHERE state='running'")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}

struct LockingPaymentAccounts;
impl oteryn_payments::AccountReader for LockingPaymentAccounts {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
        Ok(oteryn_payments::AccountState::Unknown)
    }
    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> Result<oteryn_payments::AccountState, oteryn_payments::Error> {
        let active: Option<bool> =
            query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account)
                .fetch_optional(connection)
                .await?;
        Ok(if active == Some(true) {
            oteryn_payments::AccountState::Active
        } else {
            oteryn_payments::AccountState::Unknown
        })
    }
}

#[tokio::test]
async fn lock_order_topup_and_refund_prepare_account_before_intent() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let mut inversions = Vec::new();
    for refund in [false, true] {
        let db = Database::new()
            .await?
            .ok_or("PostgreSQL required for lock-order regression")?;
        let (intent, lock) = locked_topup(&db).await?;
        lock.rollback().await?;
        let account: Uuid = query_scalar("SELECT account_id FROM payment_intents WHERE id=$1")
            .bind(intent)
            .fetch_one(&db.pool)
            .await?;
        let payload: String =
            query_scalar("SELECT payload::text FROM outbox WHERE kind='wallet.top_up'")
                .fetch_one(&db.pool)
                .await?;
        let mut writer = db.pool.begin().await?;
        let blocker: i32 = query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await?;
        query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .execute(&mut *writer)
            .await?;
        let pool = db.pool.clone();
        let effect = tokio::spawn(async move {
            if refund {
                let provider =
                    oteryn_payments::TestProvider::new(&[5; 32]).map_err(|e| e.to_string())?;
                let event = oteryn_payments::PaymentEvent {
                    event_id: "ordered-refund".into(),
                    intent_id: intent,
                    provider_reference: intent.to_string(),
                    amount: oteryn_payments::Amount {
                        minor_units: 250,
                        currency: "PLN".into(),
                    },
                    state: oteryn_payments::IntentState::Refunded,
                };
                let raw = serde_json::to_vec(&event).map_err(|e| e.to_string())?;
                let signature = provider.sign(&raw).map_err(|e| e.to_string())?;
                oteryn_payments::Payments::new(
                    pool,
                    provider,
                    LockingPaymentAccounts,
                    oteryn_payments::PaymentFlags { enabled: true },
                )
                .webhook(&raw, &signature)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
            } else {
                crate::wallet_topup::Delivery {
                    pool,
                    payments_enabled: true,
                    wallet_enabled: true,
                }
                .run(&Job {
                    id: Uuid::now_v7(),
                    kind: "wallet.top_up".into(),
                    payload,
                    attempts: 1,
                })
                .await
                .map_err(|e| e.to_string())
            }
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let waiting: bool = query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock')")
                    .bind(blocker).fetch_one(&db.pool).await?;
                if waiting { return Ok::<_, sqlx::Error>(()); }
                tokio::task::yield_now().await;
            }
        }).await.map_err(|_| "payment effect did not reach observable account wait within 2 seconds")??;
        query("SET LOCAL lock_timeout='250ms'")
            .execute(&mut *writer)
            .await?;
        let intent_lock = query("SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE")
            .bind(intent)
            .execute(&mut *writer)
            .await;
        writer.rollback().await?;
        let outcome = tokio::time::timeout(Duration::from_secs(4), effect)
            .await
            .map_err(|_| "payment effect did not finish after account release")??;
        if intent_lock.is_err() {
            inversions.push((refund, format!("{intent_lock:?}")));
        }
        db.close().await?;
        outcome.map_err(std::io::Error::other)?;
    }
    assert!(
        inversions.is_empty(),
        "payment effects retained intents before account waits: {inversions:?}"
    );
    Ok(())
}

#[tokio::test]
async fn disabled_consumers_preserve_approved_refund_and_resume_once() -> TestResult {
    use oteryn_admin::commerce::*;
    // A child process supplies enabled configuration without racing global
    // environment mutation against the rest of the test suite.
    if let Ok(schema) = std::env::var("OTERYN_AUDIT3_REFUND_SCHEMA") {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let pool = outbox::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let schema = schema.clone();
                Box::pin(async move {
                    query("SELECT set_config('search_path', $1, false)")
                        .bind(schema)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        let registry = configured(pool.clone()).await?;
        assert!(registry.handlers.contains_key("commerce.refund"));
        registry.batch(&pool).await?;
        registry.batch(&pool).await?;
        pool.close().await;
        return Ok(());
    }
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let staff = Uuid::now_v7();
    let approver = Uuid::now_v7();
    let buyer = Uuid::now_v7();
    for id in [staff, approver, buyer] {
        query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES ($1,$2,'test-only','active',now(),'en')")
            .bind(id).bind(format!("{id}@example.test")).execute(&db.pool).await?;
    }
    for id in [staff, approver] {
        query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
            .bind(id).execute(&db.pool).await?;
    }
    let intent = Uuid::now_v7();
    query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,100,'PLN','succeeded')")
        .bind(intent).bind(buyer).bind(format!("test:{intent}")).execute(&db.pool).await?;
    query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES(uuidv7(),$1,'fixed',100,100,'PLN',$2)")
        .bind(buyer).bind(intent).execute(&db.pool).await?;
    let adapter = || -> Result<TestPaymentAdapter, Box<dyn std::error::Error>> {
        Ok(TestPaymentAdapter::new(
            db.pool.clone(),
            oteryn_payments::TestProvider::new(&[7; 32])?,
            oteryn_payments::PaymentFlags { enabled: true },
        ))
    };
    let flags = Flags {
        enabled: true,
        payments: true,
        refunds: true,
        premium: false,
        wallet: false,
    };
    let service = Commerce::new(
        db.pool.clone(),
        oteryn_admin::web::PortalIdentity::configured(db.pool.clone()),
        domain_effects(db.pool.clone(), flags, Some(adapter()?)),
        flags,
    );
    let staff_proof = crate::test_staff::proof(&db.pool, staff).await.ok();
    let approver_proof = crate::test_staff::proof(&db.pool, approver).await.ok();
    let context = |account| oteryn_admin::rbac::CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: if account == staff {
            staff_proof
        } else {
            approver_proof
        },
    };
    let operation = Uuid::now_v7();
    service
        .submit(
            context(staff),
            Request {
                request_id: operation,
                target: intent,
                reason: "Approved customer refund request".into(),
                command: Command::Refund,
            },
        )
        .await?;
    service.approve(context(approver), operation).await?;
    let registry = configured(db.pool.clone()).await?;
    assert!(
        registry.disabled.iter().any(|k| k == "commerce.refund"),
        "run this regression with refunds disabled"
    );
    // Exercise every conditionally absent consumer, including a final-attempt
    // expired lease: disabled kinds must be excluded from exhaustion cleanup too.
    for kind in [
        "wallet.top_up",
        "email.password-reset",
        "email.verify",
        "email.account_exists",
        "notify.email",
        "bazaar.authority",
        "league.ingest",
        "identity.account-closure-audit",
        "identity.email-change",
        "admin.control.expire",
    ] {
        assert_eq!(
            registry.disabled.iter().any(|disabled| disabled == kind),
            kind != "identity.account-closure-audit",
            "unexpected default availability for {kind}"
        );
        db.enqueue(kind, "disabled", 1).await?;
    }
    // Disabled work exceeding the claim batch must not starve unknown-kind
    // rejection or other enabled deliveries.
    for i in 0..20 {
        db.enqueue("email.account_exists", &format!("backlog-{i}"), 1)
            .await?;
    }
    query("UPDATE outbox SET state='running', attempts=max_attempts, leased_until=now()-interval '1 second' WHERE kind='email.verify'")
        .execute(&db.pool).await?;
    db.enqueue("unknown.audit3", "unknown", 1).await?;
    for _ in 0..3 {
        registry.batch(&db.pool).await?;
    }
    let audit_state: String =
        query_scalar("SELECT state FROM outbox WHERE kind='identity.account-closure-audit'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audit_state, "done");
    let refund: (String, i32) =
        outbox::query_as("SELECT state,attempts FROM outbox WHERE kind='commerce.refund'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(refund, ("pending".into(), 0));
    let dead: i64 =
        query_scalar("SELECT count(*) FROM outbox WHERE state='dead' AND kind<>'unknown.audit3'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(dead, 0);
    assert_eq!(outbox::counts(&db.pool).await?.dead, 1);
    let key_file = tempfile::NamedTempFile::new()?;
    std::fs::write(key_file.path(), [7u8; 32])?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "worker::tests::disabled_consumers_preserve_approved_refund_and_resume_once",
        ])
        .env("OTERYN_AUDIT3_REFUND_SCHEMA", &db.schema)
        .env("OTERYN_ADMIN_COMMERCE", "true")
        .env("OTERYN_REFUNDS_ENABLED", "true")
        .env("OTERYN_PAYMENTS_ENABLED", "true")
        .env("OTERYN_ENV", "testing")
        .env("OTERYN_COMMERCE_TEST_PROVIDER_KEY_FILE", key_file.path())
        .output()?;
    assert!(
        output.status.success(),
        "configured refund worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dispatched: i64 =
        query_scalar("SELECT count(*) FROM audit_log WHERE action='commerce.refund.dispatched'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(dispatched, 1);
    let state: String = query_scalar("SELECT state FROM outbox WHERE kind='commerce.refund'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(state, "done");
    assert!(matches!(
        crate::account_lifecycle_http::request_test_termination(&db.pool, buyer).await?,
        Err(oteryn_identity::termination::Error::UnknownState)
    ));
    let provider = oteryn_payments::TestProvider::new(&[7; 32])?;
    let payments = oteryn_payments::Payments::new(
        db.pool.clone(), provider.clone(), crate::shop_web::Accounts(db.pool.clone()),
        oteryn_payments::PaymentFlags { enabled: true },
    );
    let refund = serde_json::to_vec(&oteryn_payments::PaymentEvent {
        event_id: "verified-refund".into(),
        intent_id: intent,
        provider_reference: format!("test:{intent}"),
        amount: oteryn_payments::Amount { minor_units: 100, currency: "PLN".into() },
        state: oteryn_payments::IntentState::Refunded,
    })?;
    payments.webhook(&refund, &provider.sign(&refund)?).await?;
    crate::account_lifecycle_http::request_test_termination(&db.pool, buyer).await??;

    db.close().await?;
    eprintln!(
        "PG disabled_consumers_preserve_approved_refund_and_resume_once duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn configured_token_cleanup_preserves_live_tokens_and_resend_window() -> TestResult {
    let started = std::time::Instant::now();
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let account: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,locale) VALUES ('cleanup@example.test','test-only','en') RETURNING id")
        .fetch_one(&db.pool).await?;
    // Old expired, old consumed, live, and recently consumed respectively.
    for (byte, expired, consumed, recent) in [
        (1u8, true, false, false),
        (2, false, true, false),
        (3, false, false, false),
        (4, false, true, true),
    ] {
        query("INSERT INTO account_email_tokens(token_hash,account_id,purpose,expires_at,used_at,created_at,updated_at) VALUES ($1,$2,'verify_email',now()+make_interval(secs=>$3),CASE WHEN $4 THEN now()-make_interval(secs=>$5) ELSE NULL END,now()-make_interval(secs=>$5),now()-make_interval(secs=>$5))")
            .bind(vec![byte;32]).bind(account).bind(if expired { -1.0 } else { 86400.0 })
            .bind(consumed).bind(if recent { 0.0 } else { 120.0 }).execute(&db.pool).await?;
    }
    let registry = configured(db.pool.clone()).await?;
    db.enqueue("cleanup.tokens", "cleanup", 5).await?;
    registry.batch(&db.pool).await?;
    let hashes: Vec<Vec<u8>> =
        query_scalar("SELECT token_hash FROM account_email_tokens ORDER BY token_hash")
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(hashes, vec![vec![3; 32], vec![4; 32]]);
    assert!(
        !oteryn_identity::verify::resend(&db.pool, account).await?,
        "cleanup must preserve the resend throttle"
    );
    assert_eq!(outbox::counts(&db.pool).await?.done, 1);
    db.close().await?;
    eprintln!(
        "PG configured_token_cleanup_preserves_live_tokens_and_resend_window duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn terminal_refusal_releases_topup() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let (intent, mut tx) = locked_topup(&db).await?;
    let op = Uuid::now_v7();
    query("INSERT INTO commerce_operations(id,actor_id,target_id,command,reason_sha256,requires_approval,result) VALUES($1,$2,$3,'{}',repeat('0',64),false,jsonb_build_object('refund_outcome','refused'::text))").bind(op).bind(Uuid::now_v7()).bind(intent).execute(&mut *tx).await?;
    query("INSERT INTO commerce_refunds(operation_id,intent_id,account_id,amount,currency,provider) SELECT $1,id,account_id,amount,currency,provider FROM payment_intents WHERE id=$2").bind(op).bind(intent).execute(&mut *tx).await?;
    tx.commit().await?;
    topup_registry(&db)?.batch(&db.pool).await?;
    let credited: bool = query_scalar(
        "SELECT EXISTS(SELECT 1 FROM wallet_operations WHERE id=$1 AND kind='top_up')",
    )
    .bind(intent)
    .fetch_one(&db.pool)
    .await?;
    assert!(credited, "terminal refusal blocked wallet credit");
    db.close().await
}

#[tokio::test]
async fn expired_token_keeps_latest_resend_cooldown() -> TestResult {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    let account: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,locale,status) VALUES ('recent@example.test','unused','en','pending_verification') RETURNING id").fetch_one(&db.pool).await?;
    query("INSERT INTO account_email_tokens(token_hash,account_id,purpose,expires_at,created_at,updated_at) VALUES (sha256('recent'),$1,'verify_email',now()-interval '1 second',now()-interval '24 hours 1 second',now()-interval '2 seconds')").bind(account).execute(&db.pool).await?;
    VerificationTokenCleanup(db.pool.clone()).clean().await?;
    let resent = oteryn_identity::verify::resend(&db.pool, account).await?;
    db.close().await?;
    assert!(!resent, "second resend within cooldown was accepted");
    Ok(())
}

#[tokio::test]
async fn scheduling_prunes_completed_legacy_slots() -> TestResult {
    let db = Database::new().await?.ok_or("PostgreSQL required")?;
    query("INSERT INTO outbox(kind,payload,idempotency_key,state) VALUES ('league.retention','{}','schedule:1','done'),('league.retention','{}','manual','done')").execute(&db.pool).await?;
    outbox::schedule(&db.pool, "league.retention", 1).await?;
    let keys: Vec<String> = query_scalar(
        "SELECT idempotency_key FROM outbox WHERE state='done' ORDER BY idempotency_key",
    )
    .fetch_all(&db.pool)
    .await?;
    db.close().await?;
    assert_eq!(keys, ["manual"], "legacy schedule history retained");
    Ok(())
}

#[tokio::test]
async fn delivered_checkout_releases_capacity() -> TestResult {
    let Some(db) = Database::new().await? else { return Ok(()) };
    let (intent, mut tx) = locked_topup(&db).await?;
    let account: Uuid = query_scalar("SELECT account_id FROM payment_intents WHERE id=$1")
        .bind(intent).fetch_one(&mut *tx).await?;
    query("INSERT INTO wallet_accounts(account_id) VALUES($1)").bind(account).execute(&mut *tx).await?;
    let existing = Uuid::now_v7();
    let balance = oteryn_wallet::MAX_BALANCE - 1234;
    query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'adjustment',$2,$3)")
        .bind(existing).bind(account).bind(balance).execute(&mut *tx).await?;
    query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(existing).bind(account).bind(balance).execute(&mut *tx).await?;
    assert_eq!(oteryn_wallet::incoming_capacity_with_topup_in(&mut tx,account,0).await?,1234, "pending checkout must reserve its package");
    tx.commit().await?;
    topup_registry(&db)?.batch(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    let credited: i64 = query_scalar("SELECT available_delta FROM wallet_entries WHERE operation_id=$1")
        .bind(intent).fetch_one(&mut *tx).await?;
    assert_eq!(credited,1234);
    assert_eq!(oteryn_wallet::incoming_capacity_with_topup_in(&mut tx,account,0).await?,0);
    tx.commit().await?;
    db.close().await
}

#[tokio::test]
async fn retention_login_history_schedule_handles_thousand_expirations_per_hour() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let registry = configured(db.pool.clone()).await?;
    let interval = registry.schedules["cleanup.game-logins"];
    assert!(
        3600 / interval * 512 >= 1000,
        "cleanup interval {interval}s"
    );
    db.close().await
}

#[tokio::test]
async fn retention_closure_audit_scheduled_with_game_disabled() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert!(!crate::account_closure::configured(db.pool.clone())?.game_enabled());
    let registry = configured(db.pool.clone()).await?;
    assert_eq!(
        registry.schedules.get("identity.account-closure-audit"),
        Some(&3600)
    );
    db.close().await
}

#[tokio::test]
async fn configured_death_retention_expires_malformed_rows_with_statistics_disabled() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_ne!(std::env::var("PORTAL_STATS_ENABLED").as_deref(), Ok("true"));
    let character = Uuid::now_v7();
    query(r#"INSERT INTO stats_character_events (character_ref,day,kind,event_key,details) VALUES
        ($1,CURRENT_DATE-32,'death','old-invalid','{"occurred_at":"malformed","_statistics_epoch":"1"}'),
        ($1,CURRENT_DATE-31,'death','old-missing','{"_statistics_epoch":"1"}'),
        ($1,CURRENT_DATE,'death','recent-invalid','{"occurred_at":"malformed","_statistics_epoch":"1"}')"#)
        .bind(character).execute(&db.pool).await?;
    let registry = configured(db.pool.clone()).await?;
    assert_eq!(registry.schedules.get("stats.death-retention"), Some(&3600));
    let job = db.enqueue("stats.death-retention", "death-retention", 1).await?;
    assert_eq!(registry.batch(&db.pool).await?, 1);
    let state: String = query_scalar("SELECT state FROM outbox WHERE id=$1")
        .bind(job).fetch_one(&db.pool).await?;
    assert_eq!(state, "done");
    let remaining: Vec<String> = query_scalar("SELECT event_key FROM stats_character_events WHERE character_ref=$1 ORDER BY event_key")
        .bind(character).fetch_all(&db.pool).await?;
    assert_eq!(remaining, ["recent-invalid"]);
    db.close().await
}

#[tokio::test]
async fn configured_referral_job_commits_stalled_delivery_backoffs_before_deadline() -> TestResult {
    const TEST: &str = "worker::tests::configured_referral_job_commits_stalled_delivery_backoffs_before_deadline";
    if std::env::var_os("PORTAL_TEST_REFERRAL_SMTP_CHILD").is_none() {
        // The local server accepts TCP connections but never sends an SMTP greeting.
        let smtp = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", TEST, "--nocapture"])
            .env("PORTAL_TEST_REFERRAL_SMTP_CHILD", "1")
            .env("PORTAL_REFERRALS_ENABLED", "true")
            .env("OTERYN_ACCOUNT_EMAIL_CHANGE", "true")
            .env("OTERYN_MFA_KEY", "19".repeat(32))
            .env("OTERYN_PUBLIC_ORIGIN", "https://portal.example.test")
            .env("OTERYN_SMTP_HOST", "127.0.0.1")
            .env("OTERYN_SMTP_TLS_MODE", "starttls")
            .env("OTERYN_SMTP_PORT", smtp.local_addr()?.port().to_string())
            .env("OTERYN_SMTP_USERNAME", "fake")
            .env("OTERYN_SMTP_PASSWORD", "fake")
            .env("OTERYN_MAIL_FROM", "sender@example.test")
            .env("OTERYN_SMTP_FROM", "sender@example.test")
            .output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        return Ok(());
    }
    let db = Database::new().await?.ok_or("PostgreSQL is required")?;
    query("UPDATE referral_campaign_settings SET enabled=true").execute(&db.pool).await?;
    let inviter: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('inviter@example.test','unused','active','en',now()) RETURNING id")
        .fetch_one(&db.pool).await?;
    query("WITH invitations AS (
            INSERT INTO referral_email_invitations(inviter_id,recipient,kind)
            SELECT $1,'recipient-'||n||'@example.test','new' FROM generate_series(1,8) n
            RETURNING id,inviter_id
        ) INSERT INTO referral_email_attempts(invitation_id,inviter_id) SELECT id,inviter_id FROM invitations")
        .bind(inviter).execute(&db.pool).await?;
    query("WITH change AS (
        INSERT INTO account_email_changes(account_id,kind,old_email,new_email,confirmation_hash,revert_hash,immediate,apply_after,expires_at,revert_until)
        VALUES($1,'primary','inviter@example.test','new@example.test',decode(repeat('ab',32),'hex'),decode(repeat('cd',32),'hex'),false,now()+interval '1 day',now()+interval '2 days',now()+interval '30 days')
        RETURNING id
    ) INSERT INTO account_email_change_mail(change_id,recipient,subject,body)
      SELECT id,'change-'||n||'@example.test','Email change',
        E'Test body\n\nhttps://portal.example.test/email-change/confirm/' || lpad(to_hex(n),64,'0')
      FROM change CROSS JOIN generate_series(1,8) n")
      .bind(inviter).execute(&db.pool).await?;
    let registry = configured(db.pool.clone()).await?;
    let referrals = db.enqueue("referrals.process", "stalled-smtp", 1).await?;
    let changes = db.enqueue("identity.email-change", "stalled-smtp", 1).await?;
    assert_eq!(registry.batch(&db.pool).await?, 2);
    let states: Vec<(String, String)> = outbox::query_as("SELECT kind,state FROM outbox WHERE id IN ($1,$2) ORDER BY kind")
        .bind(referrals).bind(changes).fetch_all(&db.pool).await?;
    assert_eq!(states, [
        ("identity.email-change".into(), "done".into()),
        ("referrals.process".into(), "done".into()),
    ]);
    let outcomes: Vec<(i64, i64)> = outbox::query_as("WITH attempts AS (
        SELECT 'referral' AS kind,delivery_attempts,delivered_at,next_attempt_at FROM referral_email_attempts
        UNION ALL
        SELECT 'change',delivery_attempts,delivered_at,next_attempt_at FROM account_email_change_mail
    ) SELECT
        count(*) FILTER (WHERE delivery_attempts=1 AND delivered_at IS NULL AND next_attempt_at>now()),
        count(*) FILTER (WHERE delivery_attempts=0 AND delivered_at IS NULL)
    FROM attempts GROUP BY kind").fetch_all(&db.pool).await?;
    assert_eq!(outcomes.len(), 2);
    for (deferred, untouched) in outcomes {
        assert!(deferred >= 3, "deferred {deferred}");
        assert_eq!(deferred + untouched, 8);
        assert!(untouched > 0);
    }
    db.close().await
}

#[tokio::test]
async fn termination_boundary_blocks_chargeback_hold_and_debt_after_release() -> TestResult {
    let db = Database::new().await?.ok_or("PostgreSQL is required")?;
    let account: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('chargeback@example.test','unused','active','en',now()) RETURNING id")
        .fetch_one(&db.pool).await?;
    oteryn_db::test_support::coin_chargeback_hold(&db.pool, account).await?;
    for held in [true, false] {
        if !held {
            query("DELETE FROM coin_purchase_holds WHERE account_id=$1")
                .bind(account).execute(&db.pool).await?;
        }
        assert!(matches!(
            crate::account_lifecycle_http::request_test_termination(&db.pool, account).await?,
            Err(oteryn_identity::termination::Error::UnknownState)
        ), "held={held}");
    }
    let mut tx = db.pool.begin().await?;
    query("INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(account).execute(&mut *tx).await?;
    query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account).execute(&mut *tx).await?;
    let credit = Uuid::now_v7();
    query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'top_up',$2,10)")
        .bind(credit).bind(account).execute(&mut *tx).await?;
    oteryn_wallet::credit_in(&mut tx, credit, account, 10).await?;
    query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) SELECT $1,order_id,'reversal_insufficient_balance' FROM coin_purchase_reversals WHERE payer_account_id=$1")
        .bind(account).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(matches!(
        crate::account_lifecycle_http::request_test_termination(&db.pool, account).await?,
        Err(oteryn_identity::termination::Error::UnknownState)
    ));
    db.close().await
}

#[tokio::test]
async fn unpaid_browser_credit_blocks_termination() -> TestResult {
    let db = Database::new().await?.ok_or("PostgreSQL is required")?;
    let account: Uuid = query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES ('uncredited@example.test','unused','active','en',now()) RETURNING id")
        .fetch_one(&db.pool).await?;
    let intent = Uuid::now_v7();
    query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($2,$1,'test','checkout',100,'PLN','succeeded')").bind(account).bind(intent).execute(&db.pool).await?;
    query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,intended_payment_intent_id) VALUES(uuidv7(),$1,'test',100,100,'PLN',$2)").bind(account).bind(intent).execute(&db.pool).await?;
    assert!(matches!(
        crate::account_lifecycle_http::request_test_termination(&db.pool, account).await?,
        Err(oteryn_identity::termination::Error::UnknownState)
    ));
    db.close().await
}
