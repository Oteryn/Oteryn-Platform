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
        assert!(matches!(h.create().await, Err(Error::AccountDenied)));
        assert!(matches!(
            h.service(true).intent(intent.id).await,
            Err(Error::AccountDenied)
        ));
        assert!(matches!(
            h.service(true).webhook(&body, &signature).await,
            Err(Error::AccountDenied)
        ));
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
    service.webhook(&body, &signature).await?;
    assert_eq!(
        service.intent(intent.id).await?.state,
        IntentState::Refunded
    );
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
                ) | (IntentState::Succeeded, IntentState::Refunded)
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
