use crate::{
    Amount, IntentState, PaymentEvent, TestProvider, WebhookOutcome,
    coin::{CoinOrderRequest, CoinSettings, OrderState, parse_packages},
    coin_store::{
        CoinError, CoinHost, CoinPurchases, Debit, HostFuture, Ownership, WalletSnapshot,
        purchase_hold, release_purchase_hold_in,
    },
    provider_webhook::{
        PATH, ProviderWebhook, SIGNATURE_HEADER, TIMESTAMP_HEADER, WebhookSettings, router, sign,
    },
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_store::{
    catalog::{self, CatalogAccess, CatalogFlags, CatalogStore},
    purchase,
};
use oteryn_wallet::{
    self as wallet, AdjustmentAudit, AdjustmentKind, AuditHook, Ledger, TransferBlockCheck,
    TransferPermission, WalletFlags,
};
use serde_json::{Value, json};
use sqlx::{Acquire, PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::{sync::Arc, time::Instant};
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SEEDS: std::ops::Range<u64> = 0x5041_0000..0x5041_00c8;
const PACKAGE: &str = r#"[{"id":"01934f10-7c02-7001-805b-3b1122334402","title":"500 coins","coins":500,"price":{"minor_units":499,"currency":"EUR","exponent":2},"valid_for_seconds":120}]"#;
const CATALOG: &[u8] = br#"{"contract_version":1,"offers":[{"offer_key":"item","currency":"COINS","unit_price":"1","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.store.item_grant","product_version":"2","unit_kind":"ITEM","grant_key":"item:great_health_potion","grant_revision":"12","quantity":10}]}]}"#;
const SECRET: &str = "local-payment-property-signing-key-123456789";

#[derive(Debug, Clone)]
struct Host(PgPool);

impl Host {
    fn ledger(&self) -> Ledger<purchase::DatabaseAccounts, Self, Self> {
        Ledger::new(
            self.0.clone(),
            WalletFlags { enabled: true },
            purchase::DatabaseAccounts,
            self.clone(),
            self.clone(),
        )
    }
}
impl TransferBlockCheck for Host {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Blocked
    }
}
impl AuditHook for Host {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
impl CatalogAccess for Host {
    async fn account_state(
        &self,
        _: Uuid,
        _: &mut PgConnection,
    ) -> Result<catalog::AccountState, catalog::Error> {
        Ok(catalog::AccountState::Active)
    }
    async fn permitted(
        &self,
        _: Uuid,
        _: catalog::Action,
        _: &mut PgConnection,
    ) -> Result<bool, catalog::Error> {
        Ok(true)
    }
}
impl CoinHost for Host {
    fn payer<'a>(&'a self, _: &'a str) -> HostFuture<'a, Result<Uuid, CoinError>> {
        Box::pin(async { Err(CoinError::Unauthorized) })
    }
    fn ownership(&self, _: Uuid, _: Uuid) -> HostFuture<'_, Ownership> {
        Box::pin(async { Ownership::NotOwned })
    }
    fn restriction_in<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        payer: Uuid,
    ) -> HostFuture<'a, Result<Option<String>, CoinError>> {
        Box::pin(async move {
            sqlx::query_scalar(
                "SELECT account_value_restriction(id) FROM accounts WHERE id=$1 FOR SHARE",
            )
            .bind(payer)
            .fetch_one(conn)
            .await
            .map_err(Into::into)
        })
    }
    fn credit_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<(), CoinError>> {
        Box::pin(async move {
            self.ledger()
                .with_adjustment_kind(AdjustmentKind::TopUp)
                .adjust_in(
                    tx,
                    key,
                    payer,
                    coins,
                    payer,
                    &format!("wallet-top-up:{key}"),
                )
                .await
                .map(|_| ())
                .map_err(|_| CoinError::Unavailable)
        })
    }
    fn reverse_in<'a>(
        &'a self,
        tx: &'a mut Transaction<'static, Postgres>,
        key: Uuid,
        payer: Uuid,
        coins: i64,
    ) -> HostFuture<'a, Result<Debit, CoinError>> {
        Box::pin(async move {
            let mut attempt = tx.begin().await?;
            let result = self
                .ledger()
                .adjust_in(
                    &mut attempt,
                    key,
                    payer,
                    -coins,
                    payer,
                    &format!("coin-purchase-reversal:{key}"),
                )
                .await;
            match result {
                Ok(_) => {
                    attempt.commit().await?;
                    Ok(Debit::Debited)
                }
                Err(wallet::Error::Refused(wallet::Refusal::InsufficientFunds)) => {
                    attempt.rollback().await?;
                    Ok(Debit::Insufficient)
                }
                Err(_) => Err(CoinError::Unavailable),
            }
        })
    }
    fn wallet(&self, payer: Uuid) -> HostFuture<'_, Result<WalletSnapshot, CoinError>> {
        Box::pin(async move {
            let mut tx = self.0.begin().await?;
            let (balance, revision) = self
                .ledger()
                .snapshot_in(&mut tx, payer)
                .await
                .map_err(|_| CoinError::Unavailable)?;
            Ok(WalletSnapshot {
                revision,
                total: balance.available + balance.reserved,
                available: balance.available,
            })
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Step {
    Open(usize),
    Settle(usize, bool),
    SettleWhileHeld,
    Reverse {
        order: usize,
        kind: &'static str,
        applied: bool,
    },
    Adjust(i64),
    Buy,
    Release,
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    available: i64,
    reserved: i64,
    debt: i64,
    held: bool,
    journal: Vec<(Uuid, i64, i64)>,
    orders: Vec<(Uuid, String, i64)>,
    reversals: i64,
    releases: i64,
    store_orders: i64,
}

async fn snapshot(host: &Host, payer: Uuid) -> TestResult<Snapshot> {
    let pool = &host.0;
    let balance = host.ledger().balance(payer).await?;
    let orders: Vec<(Uuid, String, i64)> = sqlx::query_as(
        "SELECT id,state,revision FROM coin_purchase_orders WHERE payer_account_id=$1 ORDER BY id",
    )
    .bind(payer)
    .fetch_all(pool)
    .await?;
    let journal = sqlx::query_as("SELECT operation_id,available_delta,reserved_delta FROM wallet_entries WHERE account_id=$1 ORDER BY entry_sequence")
        .bind(payer).fetch_all(pool).await?;
    let (reversals, releases, store_orders) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM coin_purchase_reversals WHERE payer_account_id=$1),
                (SELECT count(*) FROM coin_purchase_hold_releases WHERE account_id=$1),
                (SELECT count(*) FROM store_orders WHERE account_id=$1)",
    )
    .bind(payer)
    .fetch_one(pool)
    .await?;
    let mut conn = pool.acquire().await?;
    Ok(Snapshot {
        available: balance.available,
        reserved: balance.reserved,
        debt: wallet::outstanding_debt(&mut conn, payer).await?,
        held: purchase_hold(&mut conn, payer).await?.is_some(),
        journal,
        orders,
        reversals,
        releases,
        store_orders,
    })
}

async fn assert_bindings(pool: &PgPool, payer: Uuid, context: &str) -> TestResult {
    let broken: bool = sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM coin_purchase_orders o
           LEFT JOIN payment_intents p ON p.id=o.payment_intent_id AND p.account_id=o.payer_account_id
           LEFT JOIN wallet_operations w ON w.id=o.payment_intent_id AND w.account_id=o.payer_account_id AND w.kind='top_up'
           WHERE o.payer_account_id=$1
             AND (p.id IS NULL OR (o.receipt_id IS NULL) <> (w.id IS NULL)
                  OR (w.id IS NOT NULL AND (w.amount<>o.coins OR NOT EXISTS
                      (SELECT 1 FROM wallet_entries e WHERE e.operation_id=w.id AND e.account_id=$1 AND e.available_delta=o.coins))))
           UNION ALL
           SELECT 1 FROM payment_intents p WHERE p.account_id=$1 AND NOT EXISTS
              (SELECT 1 FROM coin_purchase_orders o WHERE o.payment_intent_id=p.id AND o.payer_account_id=$1)
           UNION ALL
           SELECT 1 FROM wallet_operations w WHERE w.account_id=$1 AND w.kind='top_up' AND NOT EXISTS
              (SELECT 1 FROM coin_purchase_orders o WHERE o.payment_intent_id=w.id AND o.payer_account_id=$1)
           UNION ALL
           SELECT 1 FROM coin_purchase_reversals r LEFT JOIN wallet_operations w
              ON w.id=r.id AND w.account_id=r.payer_account_id
              WHERE r.payer_account_id=$1 AND ((r.debited=0) <> (w.id IS NULL)
                 OR (w.id IS NOT NULL AND NOT EXISTS
                    (SELECT 1 FROM wallet_entries e WHERE e.operation_id=w.id AND e.account_id=$1 AND e.available_delta=-r.debited)))
           UNION ALL
           SELECT 1 FROM coin_purchase_reversals WHERE payer_account_id=$1 GROUP BY order_id HAVING count(*)>1
         )",
    ).bind(payer).fetch_one(pool).await?;
    assert!(!broken, "inconsistent payment/ledger bindings: {context}");
    Ok(())
}

async fn reversal(app: Router, body: Vec<u8>) -> TestResult<(StatusCode, Value)> {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let response = app
        .oneshot(
            Request::post(PATH)
                .header("content-type", "application/json")
                .header(TIMESTAMP_HEADER, timestamp.to_string())
                .header(
                    SIGNATURE_HEADER,
                    sign(SECRET.as_bytes(), timestamp, &body).ok_or("invalid signature fixture")?,
                )
                .body(Body::from(body))?,
        )
        .await?;
    let status = response.status();
    let raw = to_bytes(response.into_body(), 4096).await?;
    Ok((status, serde_json::from_slice(&raw)?))
}

async fn sequence(
    host: &Host,
    coins: &Arc<CoinPurchases>,
    app: &Router,
    provider: &TestProvider,
    store: &purchase::DatabasePurchases,
    revision: &str,
    seed: u64,
) -> TestResult {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        state >> 32
    };
    let cushion = i64::try_from(2 + next() % 98)?;
    let spend = i64::try_from(105 + next() % 280)?;
    let refund_early = next().is_multiple_of(2);
    let mut refunds = [
        Step::Reverse {
            order: 0,
            kind: "refund",
            applied: true,
        },
        Step::Reverse {
            order: 0,
            kind: "refund",
            applied: false,
        },
        Step::Reverse {
            order: 0,
            kind: "chargeback",
            applied: true,
        },
    ];
    if next().is_multiple_of(2) {
        refunds.swap(1, 2);
    }
    let mut steps = vec![
        Step::Adjust(cushion),
        Step::Open(0),
        Step::Open(1),
        Step::Open(2),
        Step::Open(3),
    ];
    let mut settlements = [
        Step::Settle(0, true),
        Step::Settle(1, false),
        Step::Settle(2, true),
    ];
    settlements.swap(1, (next() % 3) as usize);
    steps.extend(settlements);
    if refund_early {
        steps.extend(refunds);
    }
    steps.extend([Step::Buy, Step::Adjust(-spend)]);
    if !refund_early {
        steps.extend(refunds);
    }
    steps.extend([
        Step::Reverse {
            order: 2,
            kind: "chargeback",
            applied: true,
        },
        Step::Buy,
        Step::SettleWhileHeld,
        Step::Reverse {
            order: 3,
            kind: "refund",
            applied: true,
        },
    ]);
    // Isolate the gates independently: some cases clear the hold first,
    // others pay the debt first while staff review is still required.
    let (first, second) = if next().is_multiple_of(2) {
        (Step::Release, Step::Adjust(500))
    } else {
        (Step::Adjust(500), Step::Release)
    };
    steps.extend([first, Step::Buy, second, Step::Buy]);
    let payer = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'unused','active','en')")
        .bind(payer).bind(format!("{payer}@example.invalid")).execute(&host.0).await?;

    let mut orders = [None; 4];
    let mut net = 0;
    let mut balance = 0;
    let mut debt = 0;
    let mut held = false;
    let mut reversed = [false; 4];
    let mut expected_store_orders = 0;
    for (index, &step) in steps.iter().enumerate() {
        let context = format!("seed={seed:#x} step={index} sequence={steps:?}");
        let result: TestResult = async {
            match step {
                Step::Open(slot) => {
                    let catalogue = coins.catalogue(payer, None).await?;
                    let package = catalogue.packages.first().ok_or("missing package")?;
                    let request = CoinOrderRequest { request_id: Uuid::now_v7(), package_id: package.id, quote_id: package.quote_id, character_id: None };
                    let order = coins.create_order(payer, &request).await?;
                    let before = snapshot(host, payer).await?;
                    assert_eq!(coins.create_order(payer, &request).await?, order, "{context}");
                    assert!(matches!(coins.order(Uuid::now_v7(), request.request_id).await, Err(CoinError::NotFound)), "{context}");
                    assert_eq!(snapshot(host, payer).await?, before, "{context}");
                    orders[slot] = Some((order.order_id, order.request_id));
                }
                Step::Settle(slot, paid) => {
                    let (order, _) = orders[slot].ok_or("order not opened")?;
                    let first = coins.sandbox_complete(payer, order, paid).await?;
                    let before = snapshot(host, payer).await?;
                    assert_eq!(first.state, if paid { OrderState::Paid } else { OrderState::Failed }, "{context}");
                    assert_eq!(coins.sandbox_complete(payer, order, paid).await?, first, "{context}");
                    assert_eq!(snapshot(host, payer).await?, before, "{context}");
                    if paid {
                        net += 500;
                        balance += 500;
                    }
                }
                Step::SettleWhileHeld => {
                    let (order, request) = orders[3].ok_or("order not opened")?;
                    let (intent_id, provider_reference) = sqlx::query_as(
                        "SELECT p.id,p.provider_reference FROM payment_intents p JOIN coin_purchase_orders o ON o.payment_intent_id=p.id WHERE o.id=$1"
                    ).bind(order).fetch_one(&host.0).await?;
                    let body = serde_json::to_vec(&PaymentEvent {
                        event_id: format!("{payer}-held-settlement"),
                        intent_id,
                        provider_reference,
                        amount: Amount {minor_units:499, currency:"EUR".into()},
                        state: IntentState::Succeeded,
                    })?;
                    let signature = provider.sign(&body)?;
                    assert_eq!(coins.webhook(&body, &signature).await?, WebhookOutcome::Applied, "{context}");
                    let before = snapshot(host, payer).await?;
                    assert_eq!(coins.webhook(&body, &signature).await?, WebhookOutcome::Replay, "{context}");
                    assert_eq!(snapshot(host, payer).await?, before, "{context}");
                    let order = coins.order(payer, request).await?;
                    assert_eq!(order.state, OrderState::Processing, "{context}");
                    assert!(order.review_required && order.receipt_id.is_none(), "{context}");
                }
                Step::Reverse { order, kind, applied } => {
                    let (id, request) = orders[order].ok_or("order not opened")?;
                    let reference: String = sqlx::query_scalar(
                        "SELECT p.provider_reference FROM payment_intents p JOIN coin_purchase_orders o ON o.payment_intent_id=p.id WHERE o.id=$1"
                    ).bind(id).fetch_one(&host.0).await?;
                    let body = serde_json::to_vec(&json!({
                        "version": 1, "event_id": format!("{payer}-{index}"), "type": kind,
                        "order_reference": reference, "amount": {"minor_units": 499, "currency":"EUR"}
                    }))?;
                    let barrier = Arc::new(Barrier::new(2));
                    let spawn = || {
                        let (app, body, barrier) = (app.clone(), body.clone(), barrier.clone());
                        tokio::spawn(async move {
                            barrier.wait().await;
                            reversal(app, body).await.map_err(|error| format!("{error:?}"))
                        })
                    };
                    let first = spawn();
                    let second = spawn();
                    let (status, mut answer) = first.await??;
                    let (other_status, mut other) = second.await??;
                    if applied {
                        assert_eq!(status, StatusCode::OK, "{context}: {answer}");
                        assert_eq!(other_status, status, "{context}");
                        assert_ne!(answer["replay"], other["replay"], "{context}");
                        answer["replay"] = Value::Bool(false);
                        other["replay"] = Value::Bool(false);
                        assert_eq!(answer, other, "{context}");
                        assert_eq!(answer["outcome"], "applied", "{context}");
                    } else {
                        // OTERYN_PAYMENT_PROVIDER_WEBHOOK_V1 defines a 200
                        // acknowledgement with the same stored refusal on replay.
                        let (refusal, replay) = if status == StatusCode::UNPROCESSABLE_ENTITY {
                            ((status, answer), (other_status, other))
                        } else {
                            ((other_status, other), (status, answer))
                        };
                        assert_eq!(refusal.0, StatusCode::UNPROCESSABLE_ENTITY, "{context}");
                        assert_eq!(refusal.1["code"], "PROVIDER_WEBHOOK_ILLEGAL_TRANSITION", "{context}");
                        assert_eq!(replay.0, StatusCode::OK, "{context}");
                        assert_eq!(replay.1["event_id"], refusal.1["event_id"], "{context}");
                        assert_eq!(replay.1["outcome"], "illegal_transition", "{context}");
                        assert_eq!(replay.1["replay"], true, "{context}");
                    }
                    if applied {
                        let expected_state = match (order, kind) {
                            // A refund of a paid-but-uncredited order fences it
                            // in Processing. See payments README, phase B.
                            (3, _) => OrderState::Processing,
                            (_, "refund") => OrderState::Refunded,
                            _ => OrderState::Chargeback,
                        };
                        let record = coins.order(payer, request).await?;
                        assert_eq!(record.state, expected_state, "{context}");
                        if !reversed[order] {
                            reversed[order] = true;
                            if order != 3 {
                                net -= 500;
                                if balance >= 500 {
                                    balance -= 500;
                                } else {
                                    debt += 500;
                                    held = true;
                                }
                            }
                        }
                    }
                }
                Step::Adjust(amount) => {
                    let key = Uuid::now_v7();
                    let ledger = host.ledger();
                    let mut first = ledger.adjust(key, payer, amount, payer, "property test").await?;
                    let before = snapshot(host, payer).await?;
                    let replay = ledger.adjust(key, payer, amount, payer, "property test").await?;
                    assert!(replay.replayed, "{context}");
                    first.replayed = true;
                    assert_eq!(replay, first, "{context}");
                    assert_eq!(snapshot(host, payer).await?, before, "{context}");
                    net += amount;
                    let repaid = amount.max(0).min(debt);
                    debt -= repaid;
                    balance += amount - repaid;
                }
                Step::Buy => {
                    let key = Uuid::now_v7();
                    let raw = serde_json::to_vec(&purchase::PurchaseRequest {
                        contract_version: 1,
                        operation: "PurchaseStoreOfferV1".into(),
                        source_authority: "property-test".into(),
                        purchase_request_id: key.to_string(),
                        account_id: payer.to_string(),
                        character_id: payer.to_string(),
                        world_id: payer.to_string(),
                        catalog_revision: revision.into(),
                        offer_key: "item".into(),
                        currency: "COINS".into(),
                        unit_price: "1".into(),
                        quantity: 1,
                    })?;
                    let answer = store.purchase(&raw).await?;
                    let decoded: Value = serde_json::from_slice(&answer)?;
                    let before = snapshot(host, payer).await?;
                    assert_eq!(store.purchase(&raw).await?, answer, "{context}");
                    assert_eq!(snapshot(host, payer).await?, before, "{context}");
                    if held || debt > 0 {
                        assert_eq!(decoded["result"], "ACCOUNT_RESTRICTED", "{context}");
                    } else {
                        assert_eq!(decoded["result"], "ACCEPTED", "{context}");
                        balance -= 1;
                        net -= 1;
                        expected_store_orders += 1;
                    }
                }
                Step::Release => {
                    let mut tx = host.0.begin().await?;
                    let hold = purchase_hold(&mut tx, payer).await?.ok_or("missing active hold")?;
                    assert!(release_purchase_hold_in(&mut tx, payer, payer, hold.hold_id, "property release").await?.is_some(), "{context}");
                    assert!(release_purchase_hold_in(&mut tx, payer, payer, hold.hold_id, "property release").await?.is_none(), "{context}");
                    tx.commit().await?;
                    held = false;
                }
            }
            let actual = snapshot(host, payer).await?;
            assert!(actual.available >= 0 && actual.reserved >= 0, "{context}");
            assert_eq!((actual.available, actual.debt, actual.held), (balance, debt, held), "{context}");
            assert_eq!(actual.available + actual.reserved - actual.debt, net, "{context}");
            assert_eq!(actual.journal.iter().map(|(_, a, r)| a+r).sum::<i64>(), actual.available + actual.reserved, "{context}");
            assert_eq!(actual.reversals, reversed.iter().map(|&value| i64::from(value)).sum::<i64>(), "{context}");
            assert_eq!(actual.store_orders, expected_store_orders, "{context}");
            assert_eq!(actual.releases, i64::from(steps[..=index].iter().any(|s| matches!(s, Step::Release))), "{context}");
            assert_bindings(&host.0, payer, &context).await
        }.await;
        result.map_err(|error| format!("{context}: {error:?}"))?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payment_sequences_conserve_coins_across_reversals_and_holds() -> TestResult {
    let started = Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("payment_properties_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .after_connect({
            let schema = schema.clone();
            move |conn, _| {
                let schema = schema.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(schema)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            }
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let host = Host(pool.clone());
    let provider = TestProvider::new(&[9; 32])?;
    let coins = Arc::new(CoinPurchases::new(
        pool.clone(),
        CoinSettings {
            enabled: true,
            packages: parse_packages(PACKAGE).ok_or("invalid package fixture")?,
            origin: "https://portal.example".into(),
            environment: "testing".into(),
        },
        provider.clone(),
        Arc::new(host.clone()),
    ));
    let webhook = WebhookSettings::from_lookup(|key| match key {
        "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED" => Some("true".into()),
        "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET" => Some(SECRET.into()),
        _ => None,
    })
    .ok_or("invalid webhook settings")?;
    let app = router(Some(Arc::new(ProviderWebhook::new(
        pool.clone(),
        coins.clone(),
        provider.clone(),
        webhook,
    ))));
    let catalog = CatalogStore::new(pool.clone(), CatalogFlags { enabled: true });
    let actor = Uuid::now_v7();
    let revision = catalog.import(CATALOG, actor, &host).await?;

    catalog.activate(&revision, actor, &host).await?;
    let store = purchase::database(
        pool.clone(),
        purchase::Settings {
            purchase_enabled: true,
            delivery_enabled: true,
            environment: "testing".into(),
            source_authority: "property-test".into(),
            peer_identity: "property-peer".into(),
            enabled_unit_kinds: vec!["ITEM".into()],
            max_pending_operations: 4000,
            ..Default::default()
        },
        false,
    );
    let mut result = Ok(());
    for seed in SEEDS {
        if let Err(error) = sequence(&host, &coins, &app, &provider, &store, &revision, seed).await
        {
            result = Err(error);
            break;
        }
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    eprintln!(
        "payment_sequences_conserve_coins_across_reversals_and_holds: {:?}",
        started.elapsed()
    );
    result
}
