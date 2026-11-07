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

#[tokio::test]
async fn paid_topup_webhook_outbox_worker_balance() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    use oteryn_payments::{
        Amount, IntentState, PaymentEvent, PaymentFlags, Payments, TestProvider,
    };
    struct Active;
    impl oteryn_payments::AccountReader for Active {
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
    let day = OffsetDateTime::now_utc().unix_timestamp() as u64 / 86400 * 86400;
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
    assert_eq!(outbox::counts(&db.pool).await?.dead, 21);
    let audit: i64 =
        query_scalar("SELECT count(*) FROM audit_log WHERE action='wallet.top_up.denied'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audit, 21);
    db.close().await
}
