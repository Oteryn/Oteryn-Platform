use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use oteryn_payments::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
struct Accounts(Arc<AtomicU8>);
impl AccountReader for Accounts {
    async fn state_in(&self, _: &mut sqlx::PgConnection, id: Uuid) -> Result<AccountState, Error> {
        self.state(id).await
    }
    async fn state(&self, _: Uuid) -> Result<AccountState, Error> {
        match self.0.load(Ordering::SeqCst) {
            0 => Ok(AccountState::Active),
            1 => Ok(AccountState::Restricted),
            2 => Ok(AccountState::Unknown),
            3 => Ok(AccountState::Ambiguous),
            _ => Err(Error::NotFound),
        }
    }
}

/// Answers `Active` except on the `fail_on`-th call (1-based), which fails as a
/// timed-out account read does.
#[derive(Debug, Clone)]
struct FlakyAccounts {
    calls: Arc<AtomicU8>,
    fail_on: u8,
    restrict: bool,
}
impl AccountReader for FlakyAccounts {
    async fn state_in(&self, _: &mut sqlx::PgConnection, id: Uuid) -> Result<AccountState, Error> {
        self.state(id).await
    }
    async fn state(&self, _: Uuid) -> Result<AccountState, Error> {
        if self.calls.fetch_add(1, Ordering::SeqCst) + 1 == self.fail_on {
            if self.restrict {
                return Ok(AccountState::Restricted);
            }
            return Err(Error::Database(sqlx::Error::PoolTimedOut));
        }
        Ok(AccountState::Active)
    }
}

struct Harness {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    provider: TestProvider,
    accounts: Accounts,
}
impl Harness {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL payment test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("payments_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |conn, _| {
                let statement = search_path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(statement))
                        .execute(conn)
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
            provider: TestProvider::new(&[42; 32])?,
            accounts: Accounts(Arc::new(AtomicU8::new(0))),
        }))
    }
    fn service(&self, enabled: bool) -> Payments<TestProvider, Accounts> {
        Payments::new(
            self.pool.clone(),
            self.provider.clone(),
            self.accounts.clone(),
            PaymentFlags { enabled },
        )
    }
    async fn create(&self) -> Result<Intent, Error> {
        self.service(true)
            .create_intent(Uuid::now_v7(), amount())
            .await
    }
    fn event(&self, intent: &Intent, state: IntentState) -> PaymentEvent {
        PaymentEvent {
            event_id: Uuid::now_v7().to_string(),
            intent_id: intent.id,
            provider_reference: intent.provider_reference.clone(),
            amount: intent.amount.clone(),
            state,
        }
    }
    fn signed(
        &self,
        event: &PaymentEvent,
    ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(event)?;
        let signature = self.provider.sign(&body)?;
        Ok((body, signature))
    }
    async fn count(&self, table: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
    }
    async fn finish(self) -> TestResult {
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
fn amount() -> Amount {
    Amount {
        minor_units: 1000,
        currency: "PLN".into(),
    }
}

#[tokio::test]
async fn bad_signature_rejected() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let (body, mut signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    signature[0] ^= 1;
    assert!(matches!(
        h.service(true).webhook(&body, &signature).await,
        Err(Error::BadSignature)
    ));
    assert_eq!(h.count("payment_events").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn replay_ignored() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Failed))?;
    let service = h.service(true);
    assert_eq!(
        service.webhook(&body, &signature).await?,
        WebhookOutcome::Applied
    );
    assert_eq!(
        service.webhook(&body, &signature).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(service.intent(intent.id).await?.state, IntentState::Failed);
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn illegal_transition_rolls_back() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let service = h.service(true);
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Failed))?;
    service.webhook(&body, &signature).await?;
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    assert!(matches!(
        service.webhook(&body, &signature).await,
        Err(Error::IllegalTransition)
    ));
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("invoices").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn concurrent_success_enqueues_exactly_once() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    let service = h.service(true);
    let barrier = tokio::sync::Barrier::new(2);
    let deliver = || async {
        barrier.wait().await;
        service.webhook(&body, &signature).await
    };
    let (a, b) = tokio::join!(deliver(), deliver());
    let results = [a?, b?];
    assert!(results.contains(&WebhookOutcome::Applied));
    assert!(results.contains(&WebhookOutcome::Replay));
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("invoices").await?, 1);
    assert_eq!(h.count("outbox").await?, 1);
    let (kind, key, payload): (String, String, String) =
        sqlx::query_as("SELECT kind,idempotency_key,payload::text FROM outbox")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(kind, "wallet.top_up");
    assert_eq!(key, intent.id.to_string());
    assert_eq!(
        serde_json::from_str::<TopUpJob>(&payload)?,
        TopUpJob {
            intent_id: intent.id,
            account_id: intent.account_id,
            amount: 1000,
            currency: "PLN".into()
        }
    );
    h.finish().await
}

#[tokio::test]
async fn disabled_and_ineligible_accounts_deny() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    assert!(!PaymentFlags::default().enabled);
    assert!(matches!(
        h.service(false)
            .create_intent(Uuid::now_v7(), amount())
            .await,
        Err(Error::Disabled)
    ));
    let intent = h.create().await?;
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    assert!(matches!(
        h.service(false).intent(intent.id).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        h.service(false).webhook(&body, &signature).await,
        Err(Error::Disabled)
    ));
    for state in 1..=4 {
        h.accounts.0.store(state, Ordering::SeqCst);
        // State 4 is a failed account read: still refused, but as retryable Unavailable.
        let refused = |e: &Result<Intent, Error>| match state {
            4 => matches!(e, Err(Error::Unavailable)),
            _ => matches!(e, Err(Error::AccountDenied)),
        };
        assert!(refused(&h.create().await));
        assert!(refused(&h.service(true).intent(intent.id).await));
        let webhook = h.service(true).webhook(&body, &signature).await;
        assert!(match state {
            4 => matches!(webhook, Err(Error::Unavailable)),
            _ => matches!(webhook, Err(Error::AccountDenied)),
        });
    }
    assert_eq!(h.count("payment_events").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn invalid_input_and_signed_mismatch_deny() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let service = h.service(true);
    for invalid in [
        Amount {
            minor_units: 0,
            ..amount()
        },
        Amount {
            minor_units: -1,
            ..amount()
        },
        Amount {
            currency: "pln".into(),
            ..amount()
        },
    ] {
        assert!(matches!(
            service.create_intent(Uuid::now_v7(), invalid).await,
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(
        service.create_intent(Uuid::nil(), amount()).await,
        Err(Error::InvalidInput)
    ));
    let intent = h.create().await?;
    let mut event = h.event(&intent, IntentState::Succeeded);
    event.amount.minor_units += 1;
    let (body, signature) = h.signed(&event)?;
    assert!(matches!(
        service.webhook(&body, &signature).await,
        Err(Error::InvalidInput)
    ));
    event.amount = amount();
    event.provider_reference = "wrong".into();
    let (body, signature) = h.signed(&event)?;
    assert!(matches!(
        service.webhook(&body, &signature).await,
        Err(Error::InvalidInput)
    ));
    let body = b"{invalid";
    assert!(matches!(
        service.webhook(body, &h.provider.sign(body)?).await,
        Err(Error::InvalidInput)
    ));
    assert_eq!(h.count("payment_events").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    h.finish().await
}

#[tokio::test]
async fn refund_transition_has_no_second_credit() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let service = h.service(true);
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    service.webhook(&body, &signature).await?;
    assert_eq!(
        h.provider.refund(intent.id, &intent.amount).await?,
        h.provider.refund(intent.id, &intent.amount).await?
    );
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Refunded))?;
    assert!(matches!(
        service.webhook(&body, &signature).await,
        Err(Error::IllegalTransition)
    ));
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("outbox").await?, 1);
    h.finish().await
}

#[test]
fn state_machine_exhaustive() {
    let states = [
        IntentState::Created,
        IntentState::Pending,
        IntentState::Succeeded,
        IntentState::Failed,
        IntentState::Refunded,
        IntentState::Chargeback,
    ];
    for from in states {
        for to in states {
            let legal = matches!(
                (from, to),
                (
                    IntentState::Created,
                    IntentState::Pending | IntentState::Succeeded | IntentState::Failed
                ) | (
                    IntentState::Pending,
                    IntentState::Succeeded | IntentState::Failed
                ) | (
                    IntentState::Succeeded,
                    IntentState::Refunded | IntentState::Chargeback
                ) | (IntentState::Refunded, IntentState::Chargeback)
            );
            assert_eq!(from.transition(to).is_ok(), legal);
        }
    }
}

#[tokio::test]
async fn provider_deterministic_and_signature_binds_body() -> TestResult {
    assert!(TestProvider::new(&[1; 16]).is_err());
    let provider = TestProvider::new(&[42; 32])?;
    let id = Uuid::now_v7();
    assert_eq!(
        provider.create_intent(id, &amount()).await?,
        provider.create_intent(id, &amount()).await?
    );
    let signature = provider.sign(b"body")?;
    provider.verify_webhook_signature(b"body", &signature)?;
    assert!(matches!(
        provider.verify_webhook_signature(b"tampered", &signature),
        Err(Error::BadSignature)
    ));
    assert!(provider.verify_webhook_signature(b"body", &[]).is_err());
    Ok(())
}

#[tokio::test]
async fn distinct_concurrent_success_events_cannot_double_credit() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let (body_a, sig_a) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    let (body_b, sig_b) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    let service = h.service(true);
    let (a, b) = tokio::join!(
        service.webhook(&body_a, &sig_a),
        service.webhook(&body_b, &sig_b)
    );
    assert!(matches!(
        (&a, &b),
        (Ok(WebhookOutcome::Applied), Err(Error::IllegalTransition))
            | (Err(Error::IllegalTransition), Ok(WebhookOutcome::Applied))
    ));
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("invoices").await?, 1);
    assert_eq!(h.count("outbox").await?, 1);
    h.finish().await
}

#[tokio::test]
async fn outbox_failure_rolls_back_and_delivery_can_retry() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    let service = h.service(true);
    let (body, signature) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    sqlx::query("ALTER TABLE outbox ADD CONSTRAINT refuse_top_up CHECK (kind <> 'wallet.top_up')")
        .execute(&h.pool)
        .await?;
    assert!(matches!(
        service.webhook(&body, &signature).await,
        Err(Error::Database(_))
    ));
    assert_eq!(service.intent(intent.id).await?.state, IntentState::Pending);
    assert_eq!(h.count("payment_events").await?, 0);
    assert_eq!(h.count("invoices").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    sqlx::query("ALTER TABLE outbox DROP CONSTRAINT refuse_top_up")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        service.webhook(&body, &signature).await?,
        WebhookOutcome::Applied
    );
    assert_eq!(h.count("outbox").await?, 1);
    h.finish().await
}

#[test]
fn hmac_matches_rfc4231_case_six() -> TestResult {
    let provider = TestProvider::new(&[0xaa; 131])?;
    let signature = provider.sign(b"Test Using Larger Than Block-Size Key - Hash Key First")?;
    let hex: String = signature.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
    Ok(())
}

#[tokio::test]
async fn audit_09_02_changed_replay_binding_is_refused() -> TestResult {
    let started = std::time::Instant::now();
    let h = Harness::new().await?.ok_or("PostgreSQL is required")?;
    let intent = h.create().await?;
    let event = h.event(&intent, IntentState::Succeeded);
    let service = h.service(true);
    let (body, signature) = h.signed(&event)?;
    assert_eq!(
        service.webhook(&body, &signature).await?,
        WebhookOutcome::Applied
    );
    let mut changes = Vec::new();
    let mut changed = event.clone();
    changed.state = IntentState::Failed;
    changes.push(changed);
    let mut changed = event.clone();
    changed.amount.minor_units += 1;
    changes.push(changed);
    let mut changed = event.clone();
    changed.amount.currency = "USD".into();
    changes.push(changed);
    let mut changed = event.clone();
    changed.provider_reference.push('x');
    changes.push(changed);
    let other = h.create().await?;
    let mut changed = h.event(&other, IntentState::Succeeded);
    changed.event_id.clone_from(&event.event_id);
    changes.push(changed);
    for changed in changes {
        let (body, signature) = h.signed(&changed)?;
        assert!(matches!(
            service.webhook(&body, &signature).await,
            Err(Error::InvalidInput)
        ));
    }
    assert_eq!(
        service.webhook(&body, &signature).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(service.intent(other.id).await?.state, IntentState::Pending);
    assert_eq!(h.count("payment_events").await?, 1);
    assert_eq!(h.count("invoices").await?, 1);
    assert_eq!(h.count("outbox").await?, 1);
    h.finish().await?;
    eprintln!(
        "PG audit_09_02_changed_replay_binding_is_refused: {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn l6_03_browser_checkout_refund_refused_twice_after_credit() -> TestResult {
    let started = std::time::Instant::now();
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'test-package',25,$3,$4,$1)").bind(intent.id).bind(intent.account_id).bind(intent.amount.minor_units).bind(&intent.amount.currency).execute(&h.pool).await?;
    let service = h.service(true);
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    service.webhook(&body, &sig).await?;
    // Same ledger binding and exact persisted quantity as the top-up worker.
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(intent.account_id)
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount,reference_id,reason) VALUES($1,'top_up',$2,25,$2,$3)").bind(intent.id).bind(intent.account_id).bind(format!("wallet-top-up:{}",intent.id)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,25,0)").bind(Uuid::now_v7()).bind(intent.id).bind(intent.account_id).execute(&h.pool).await?;
    for _ in 0..2 {
        let (body, sig) = h.signed(&h.event(&intent, IntentState::Refunded))?;
        assert!(matches!(
            service.webhook(&body, &sig).await,
            Err(Error::IllegalTransition)
        ));
    }
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    assert_eq!(h.count("wallet_entries").await?, 1);
    assert_eq!(h.count("payment_events").await?, 1);
    h.finish().await?;
    eprintln!(
        "PG l6_03_browser_checkout_refund_refused_twice_after_credit {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn l6_03_undelivered_browser_checkout_can_still_be_refunded() -> TestResult {
    let started = std::time::Instant::now();
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'test-package',25,$3,$4,$1)").bind(intent.id).bind(intent.account_id).bind(intent.amount.minor_units).bind(&intent.amount.currency).execute(&h.pool).await?;
    let service = h.service(true);
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    service.webhook(&body, &sig).await?;
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Refunded))?;
    assert_eq!(service.webhook(&body, &sig).await?, WebhookOutcome::Applied);
    assert_eq!(service.webhook(&body, &sig).await?, WebhookOutcome::Replay);
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Refunded
    );
    assert_eq!(h.count("wallet_entries").await?, 0);
    assert_eq!(h.count("payment_events").await?, 2);
    h.finish().await?;
    eprintln!(
        "PG l6_03_undelivered_browser_checkout_can_still_be_refunded {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn account_state_read_failure_is_unavailable_not_denied() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    for state in 1..=3 {
        h.accounts.0.store(state, Ordering::SeqCst);
        assert!(matches!(h.create().await, Err(Error::AccountDenied)));
        assert!(matches!(
            h.service(true).intent(intent.id).await,
            Err(Error::AccountDenied)
        ));
    }
    h.accounts.0.store(4, Ordering::SeqCst);
    assert!(matches!(h.create().await, Err(Error::Unavailable)));
    assert!(matches!(
        h.service(true).intent(intent.id).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        h.service(true)
            .ensure_intent(Uuid::now_v7(), Uuid::now_v7(), amount())
            .await,
        Err(Error::Unavailable)
    ));
    assert_eq!(h.count("payment_intents").await?, 1);
    h.finish().await
}

#[tokio::test]
async fn ensure_intent_reread_failure_is_unavailable_and_replay_returns_same_intent() -> TestResult
{
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    // The second eligibility read fails after provider creation.
    let flaky = FlakyAccounts {
        calls: Arc::new(AtomicU8::new(0)),
        fail_on: 2,
        restrict: false,
    };
    let service = Payments::new(
        h.pool.clone(),
        h.provider.clone(),
        flaky.clone(),
        PaymentFlags { enabled: true },
    );
    let (id, account) = (Uuid::now_v7(), Uuid::now_v7());
    assert!(matches!(
        service.ensure_intent(id, account, amount()).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(flaky.calls.load(Ordering::SeqCst), 2);
    assert_eq!(h.count("payment_intents").await?, 0);
    let created = service.ensure_intent(id, account, amount()).await?;
    let replay = service.ensure_intent(id, account, amount()).await?;
    assert_eq!(created, replay);
    assert_eq!(replay.id, id);
    assert_eq!(replay.account_id, account);
    assert_eq!(replay.state, IntentState::Pending);
    assert_eq!(h.count("payment_intents").await?, 1);
    h.finish().await
}

#[tokio::test]
async fn webhook_account_read_failure_is_unavailable_and_redelivery_applies_once() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let intent = h.create().await?;
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'test-package',25,$3,$4,$1)").bind(intent.id).bind(intent.account_id).bind(intent.amount.minor_units).bind(&intent.amount.currency).execute(&h.pool).await?;
    let service = h.service(true);
    let (paid, paid_sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    let (refund, refund_sig) = h.signed(&h.event(&intent, IntentState::Refunded))?;
    // A failed account read on a settlement: retryable, and nothing is written.
    h.accounts.0.store(4, Ordering::SeqCst);
    assert!(matches!(
        service.webhook(&paid, &paid_sig).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(h.count("payment_events").await?, 0);
    assert_eq!(h.count("invoices").await?, 0);
    assert_eq!(h.count("outbox").await?, 0);
    h.accounts.0.store(0, Ordering::SeqCst);
    assert_eq!(service.intent(intent.id).await?.state, IntentState::Pending);
    assert_eq!(
        service.webhook(&paid, &paid_sig).await?,
        WebhookOutcome::Applied
    );
    assert_eq!(
        service.webhook(&paid, &paid_sig).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(h.count("invoices").await?, 1);
    assert_eq!(h.count("outbox").await?, 1);
    // The same on a refund, which reads through refund_state_in.
    h.accounts.0.store(4, Ordering::SeqCst);
    assert!(matches!(
        service.webhook(&refund, &refund_sig).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(h.count("payment_events").await?, 1);
    h.accounts.0.store(0, Ordering::SeqCst);
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    assert_eq!(
        service.webhook(&refund, &refund_sig).await?,
        WebhookOutcome::Applied
    );
    assert_eq!(
        service.webhook(&refund, &refund_sig).await?,
        WebhookOutcome::Replay
    );
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Refunded
    );
    assert_eq!(h.count("payment_events").await?, 2);
    assert_eq!(h.count("outbox").await?, 1);
    h.finish().await
}

#[tokio::test]
async fn restricted_at_creation_leaves_no_intent() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let accounts = FlakyAccounts {
        calls: Arc::new(AtomicU8::new(0)),
        fail_on: 2,
        restrict: true,
    };
    let payments = Payments::new(
        h.pool.clone(),
        h.provider.clone(),
        accounts,
        PaymentFlags { enabled: true },
    );
    assert!(matches!(
        payments
            .ensure_intent(Uuid::now_v7(), Uuid::now_v7(), amount())
            .await,
        Err(Error::AccountDenied)
    ));
    assert_eq!(
        h.count("payment_intents").await?,
        0,
        "restricted creation persisted"
    );
    h.finish().await
}

async fn browser_intent(h: &Harness, account: Uuid) -> Result<Intent, Box<dyn std::error::Error>> {
    let intent = h.service(true).create_intent(account, amount()).await?;
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'test-package',25,$3,$4,$1)")
        .bind(intent.id).bind(account).bind(intent.amount.minor_units).bind(&intent.amount.currency).execute(&h.pool).await?;
    Ok(intent)
}

async fn credit(h: &Harness, account: Uuid, operation: Uuid, coins: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount,reference_id,reason) VALUES($1,'top_up',$2,$3,$2,$4)").bind(operation).bind(account).bind(coins).bind(format!("wallet-top-up:{operation}")).execute(&mut *tx).await?;
    oteryn_wallet::credit_in(&mut tx, operation, account, coins).await?;
    tx.commit().await?;
    Ok(())
}

async fn held_reversal(h: &Harness, account: Uuid) -> TestResult {
    let order = Uuid::now_v7();
    sqlx::query("INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES ($1,$2,$1,25,100,'EUR',2,'testing',now(),now())").bind(order).bind(account).execute(&h.pool).await?;
    sqlx::query("INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code) VALUES ($1,$2,$1,$1,$1,25,100,'EUR',2,'testing',$1,'processing','review')").bind(order).bind(account).execute(&h.pool).await?;
    sqlx::query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES ($1,$1,$2,'chargeback','test','reversal',25,0,'held')").bind(order).bind(account).execute(&h.pool).await?;
    sqlx::query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance')").bind(account).bind(order).execute(&h.pool).await?;
    Ok(())
}

#[tokio::test]
async fn browser_delivered_chargeback() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let intent = browser_intent(&h, account).await?;
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    h.service(true).webhook(&body, &sig).await?;
    credit(&h, account, intent.id, 25).await?;
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Chargeback))?;
    assert!(
        matches!(
            h.service(true).webhook(&body, &sig).await,
            Err(Error::IllegalTransition)
        ),
        "delivered chargeback must be refused"
    );
    assert_eq!(
        h.service(true).intent(intent.id).await?.state,
        IntentState::Succeeded
    );
    h.finish().await
}

#[tokio::test]
async fn browser_hold_after_repayment() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    held_reversal(&h, account).await?;
    credit(&h, account, Uuid::now_v7(), 25).await?;
    let intent = browser_intent(&h, account).await?;
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    assert!(
        matches!(
            h.service(true).webhook(&body, &sig).await,
            Err(Error::AccountDenied)
        ),
        "active purchase hold must block checkout"
    );
    h.finish().await
}

#[tokio::test]
async fn browser_checkout_repays_debt_at_ceiling() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    credit(&h, account, Uuid::now_v7(), oteryn_wallet::MAX_BALANCE).await?;
    held_reversal(&h, account).await?;
    sqlx::query("DELETE FROM coin_purchase_holds WHERE account_id=$1")
        .bind(account)
        .execute(&h.pool)
        .await?;
    let intent = browser_intent(&h, account).await?;
    let (body, sig) = h.signed(&h.event(&intent, IntentState::Succeeded))?;
    assert!(
        matches!(
            h.service(true).webhook(&body, &sig).await,
            Ok(WebhookOutcome::Applied)
        ),
        "debt-covered checkout must settle"
    );
    credit(&h, account, intent.id, 25).await?;
    h.finish().await
}
