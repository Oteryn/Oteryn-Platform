use crate::{
    catalog::{self, CatalogAccess, CatalogFlags, CatalogStore},
    delivery::Delivery,
    purchase::{self, Settings},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_identity::{
    password::PasswordConfig,
    termination::{
        self, ActorProof, BoundaryFuture, Terminations,
        game_closure::runtime::financial_obligations_clear,
    },
};
use oteryn_internal_api::{
    canonical,
    guard::{Identity, VerifiedClient},
};
use oteryn_wallet::{
    AdjustmentAudit, AuditHook, Ledger, TransferBlockCheck, TransferPermission, WalletFlags,
};
use serde_json::Value;
use sqlx::{PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::{sync::Arc, time::Instant};
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SEEDS: std::ops::Range<u64> = 0x5354_0000..0x5354_00c8;
const CATALOG: &[u8] = br#"{"contract_version":1,"offers":[{"offer_key":"item","currency":"COINS","unit_price":"10","max_quantity":10,"purchasable":true,"units":[{"product_id":"oteryn.store.item_grant","product_version":"2","unit_kind":"ITEM","grant_key":"item:great_health_potion","grant_revision":"12","quantity":10}]}]}"#;
const PEER: &str = "property-store-peer";
const PASSWORD: &str = "local property test password";

#[derive(Clone, Copy, Debug)]
struct Policy;
impl TransferBlockCheck for Policy {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Blocked
    }
}
impl AuditHook for Policy {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
impl CatalogAccess for Policy {
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
impl termination::Boundary for Policy {
    fn authorize<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: Uuid,
        _: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> BoundaryFuture<'a> {
        Box::pin(async move {
            if financial_obligations_clear(tx, account).await? {
                Ok(())
            } else {
                Err(termination::Error::UnknownState)
            }
        })
    }
    fn revoke<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: Uuid,
        _: bool,
    ) -> BoundaryFuture<'a> {
        // Game proof and credential storage are supplied by the application.
        Box::pin(async { Ok(()) })
    }
}

async fn post(
    app: Router,
    path: &'static str,
    bytes: Vec<u8>,
) -> TestResult<(StatusCode, Vec<u8>)> {
    let mut request = Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(bytes))?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
            PEER,
        )?));
    let response = app.oneshot(request).await?;
    let status = response.status();
    Ok((status, to_bytes(response.into_body(), 4096).await?.to_vec()))
}

async fn duplicates(
    app: &Router,
    path: &'static str,
    bytes: &[u8],
    context: &str,
) -> TestResult<Vec<u8>> {
    let barrier = Arc::new(Barrier::new(2));
    let spawn = || {
        let (app, bytes, barrier) = (app.clone(), bytes.to_vec(), barrier.clone());
        tokio::spawn(async move {
            barrier.wait().await;
            post(app, path, bytes)
                .await
                .map_err(|error| format!("{error:?}"))
        })
    };
    let first = spawn();
    let second = spawn();
    let first = first.await??;
    let second = second.await??;
    assert_eq!(first.0, StatusCode::OK, "{context}: {first:?}");
    assert_eq!(first, second, "{context}");
    Ok(first.1)
}

#[derive(Clone, Copy, Debug)]
enum Step {
    Fund(i64),
    Buy(u8),
    Decide(usize, bool),
    Debit(i64),
}

struct StoreFixture {
    pool: PgPool,
    buy: Router,
    delivery: Delivery,
    revision: String,
}

impl StoreFixture {
    fn wallet(&self) -> Ledger<purchase::DatabaseAccounts, Policy, Policy> {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            purchase::DatabaseAccounts,
            Policy,
            Policy,
        )
    }

    async fn sequence(&self, seed: u64, steps: &[Step]) -> TestResult {
        let account = account(&self.pool, "unused").await?;
        let wallet = self.wallet();
        let mut expected_balance = 0;
        let mut expected_orders = 0;
        let mut expected_receipts = 0;
        let mut expected_claims = 0;
        let mut funded = 0;
        let mut orders = Vec::new();
        for (index, &step) in steps.iter().enumerate() {
            let context = format!("seed={seed:#x} step={index} sequence={steps:?}");
            let result: TestResult = async {
                match step {
                    Step::Fund(amount) | Step::Debit(amount) => {
                        wallet.adjust(Uuid::now_v7(), account, amount, account, "property funding").await?;
                        expected_balance += amount;
                        funded += amount;
                    }
                    Step::Buy(quantity) => {
                        let key = Uuid::now_v7();
                        let raw = serde_json::to_vec(&purchase::PurchaseRequest {
                            contract_version: 1,
                            operation: "PurchaseStoreOfferV1".into(),
                            source_authority: "property-test".into(),
                            purchase_request_id: key.to_string(),
                            account_id: account.to_string(),
                            character_id: account.to_string(),
                            world_id: account.to_string(),
                            catalog_revision: self.revision.clone(),
                            offer_key: "item".into(),
                            currency: "COINS".into(),
                            unit_price: "10".into(),
                            quantity,
                        })?;
                        let answer = duplicates(&self.buy, purchase::http::PURCHASE_PATH, &raw, &context).await?;
                        let value: Value = serde_json::from_slice(&answer)?;
                        let price = 10 * i64::from(quantity);
                        if expected_balance >= price {
                            expected_balance -= price;
                            expected_orders += 1;
                            assert_eq!(value["result"], "ACCEPTED", "{context}");
                        } else {
                            assert_eq!(value["result"], "INSUFFICIENT_BALANCE", "{context}");
                        }
                        orders.push((raw, answer, value));
                    }
                    Step::Decide(slot, delivered) => {
                        let (purchase, answer, value) = orders.get(slot).ok_or("missing purchase answer")?;
                        if value["result"] == "ACCEPTED" {
                            let id: Uuid = serde_json::from_value(value["units"][0]["delivery_operation_id"].clone())?;
                            let digest: String = sqlx::query_scalar("SELECT binding_digest FROM store_operations WHERE operation_id=$1")
                                .bind(id).fetch_one(&self.pool).await?;
                            let mut receipt: canonical::Value = serde_json::from_str(include_str!(
                                "../../../../docs/contracts/fixtures/store-wire-v1/receipt.deliver-item.committed.json"
                            ))?;
                            let entitlement = value["units"][0]["entitlement_id"]
                                .as_str().ok_or("missing entitlement")?;
                            let canonical::Value::Object(fields) = &mut receipt else {
                                return Err("invalid receipt fixture".into());
                            };
                            for (key, value) in fields.iter_mut() {
                                *value = match key.as_str() {
                                    "source_authority" => canonical::Value::String("property-test".into()),
                                    "account_id" => canonical::Value::String(account.to_string()),
                                    "operation_id" => canonical::Value::String(id.to_string()),
                                    "entitlement_id" => canonical::Value::String(entitlement.into()),
                                    "binding_digest" => canonical::Value::String(digest.clone()),
                                    "delivered_quantity" => canonical::Value::Unsigned(value_for_quantity(purchase)?),
                                    "outcome" if !delivered => canonical::Value::String("REJECTED".into()),
                                    "result_code" if !delivered => canonical::Value::String("STORE_GRANT_UNKNOWN".into()),
                                    _ => continue,
                                };
                            }
                            if delivered {
                                expected_claims += 1;
                            } else {
                                // The Store wire contract §6.3 supports item reversal
                                // after delivery. A rejected delivery only opens review.
                                fields.retain(|(key, _)| key != "delivered_quantity");
                            }
                            let raw = canonical::encode(&receipt)?;
                            let wrong_owner = String::from_utf8(raw.clone())?.replace(
                                &account.to_string(), &Uuid::now_v7().to_string()
                            );
                            assert_eq!(
                                self.delivery.receipt(wrong_owner.as_bytes()).await,
                                Err(purchase::Error::Invalid),
                                "{context}"
                            );
                            let first = self.delivery.receipt(&raw).await?;
                            assert_eq!(self.delivery.receipt(&raw).await?, first, "{context}");
                            let state: String = sqlx::query_scalar(
                                "SELECT state FROM store_fulfilment_units WHERE entitlement_id=$1"
                            ).bind(Uuid::parse_str(entitlement)?).fetch_one(&self.pool).await?;
                            assert_eq!(state, if delivered { "DELIVERED" } else { "MANUAL_REVIEW" }, "{context}");
                            expected_receipts += 1;
                        }
                        let replay = post(self.buy.clone(), purchase::http::PURCHASE_PATH, purchase.clone()).await?;
                        assert_eq!(replay, (StatusCode::OK, answer.clone()), "{context}");
                    }
                }
                let balance = wallet.balance(account).await?;
                assert!(balance.available >= 0 && balance.reserved >= 0, "{context}");
                assert_eq!((balance.available,balance.reserved), (expected_balance,0), "{context}");
                let (order_count, spent, receipts, claims, broken): (i64, i64, i64, i64, bool) = sqlx::query_as(
                    "SELECT
                       (SELECT count(*) FROM store_orders WHERE account_id=$1),
                       (SELECT coalesce(sum(total_price),0)::bigint FROM store_orders WHERE account_id=$1),
                       (SELECT count(*) FROM store_receipts r JOIN store_operations op USING(operation_id)
                           JOIN store_fulfilment_units u USING(entitlement_id) WHERE u.account_id=$1),
                       (SELECT count(*) FROM store_operations op JOIN store_fulfilment_units u USING(entitlement_id)
                           WHERE u.account_id=$1 AND op.operation_kind='CLAIM'),
                       EXISTS (
                         SELECT 1 FROM store_orders o LEFT JOIN wallet_entries e
                            ON e.operation_id=o.wallet_operation_id AND e.account_id=o.account_id
                            WHERE o.account_id=$1 AND (e.id IS NULL OR e.available_delta<>-o.total_price OR e.reserved_delta<>0
                               OR (SELECT count(*) FROM wallet_entries e2 WHERE e2.operation_id=o.wallet_operation_id)<>1)
                         UNION ALL
                         SELECT 1 FROM wallet_operations w WHERE w.account_id=$1 AND w.kind='store_purchase'
                            AND NOT EXISTS(SELECT 1 FROM store_orders o WHERE o.wallet_operation_id=w.id AND o.account_id=$1)
                         UNION ALL
                         SELECT 1 FROM store_orders o LEFT JOIN store_order_lines l USING(order_id)
                            LEFT JOIN store_fulfilment_units u USING(line_id)
                            LEFT JOIN store_operations op ON op.entitlement_id=u.entitlement_id AND op.operation_kind='DELIVER'
                            WHERE o.account_id=$1 AND (u.account_id IS DISTINCT FROM o.account_id OR op.operation_id IS NULL)
                         UNION ALL
                         SELECT 1 FROM store_operations op JOIN store_fulfilment_units u USING(entitlement_id)
                            WHERE u.account_id=$1 AND op.operation_kind='DELIVER'
                            GROUP BY u.entitlement_id HAVING count(*)<>1
                       )",
                ).bind(account).fetch_one(&self.pool).await?;
                assert!(!broken, "broken order, debit or delivery binding: {context}");
                assert_eq!((order_count, receipts, claims), (expected_orders, expected_receipts, expected_claims), "{context}");
                assert_eq!(funded - spent, balance.available, "{context}");
                Ok(())
            }.await;
            result.map_err(|error| format!("{context}: {error:?}"))?;
        }
        Ok(())
    }
}

fn value_for_quantity(purchase: &[u8]) -> TestResult<u64> {
    let value: Value = serde_json::from_slice(purchase)?;
    Ok(value["quantity"]
        .as_u64()
        .ok_or("missing purchase quantity")?
        * 10)
}

async fn account(pool: &PgPool, hash: &str) -> TestResult<Uuid> {
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),$3,'active','en')")
        .bind(account).bind(format!("{account}@example.invalid")).bind(hash).execute(pool).await?;
    Ok(account)
}

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> TestResult<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let admin = PgPool::connect(&url).await?;
        let schema = format!("store_properties_{}", Uuid::now_v7().simple());
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
        Ok(Self {
            admin,
            pool,
            schema,
        })
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        Ok(())
    }
}

fn generator(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        state >> 32
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn store_sequences_bind_debits_orders_and_delivery_intents() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = Instant::now();
    let db = Database::new().await?;
    let settings = Settings {
        purchase_enabled: true,
        delivery_enabled: true,
        environment: "testing".into(),
        source_authority: "property-test".into(),
        peer_identity: PEER.into(),
        enabled_unit_kinds: vec!["ITEM".into()],
        requests_per_minute: 6000,
        max_pending_operations: 4000,
        ..Default::default()
    };
    let catalog = CatalogStore::new(db.pool.clone(), CatalogFlags { enabled: true });
    let actor = Uuid::now_v7();
    let revision = catalog.import(CATALOG, actor, &Policy).await?;
    catalog.activate(&revision, actor, &Policy).await?;
    let fixture = StoreFixture {
        pool: db.pool.clone(),
        buy: purchase::http::router(
            purchase::database(db.pool.clone(), settings.clone(), false),
            vec![],
        ),
        delivery: Delivery::new(db.pool.clone(), settings),
        revision,
    };
    let mut result = Ok(());
    for seed in SEEDS {
        let mut next = generator(seed);
        let mut steps = [
            Step::Fund(i64::try_from(75 + next() % 75)?),
            Step::Buy(u8::try_from(1 + next() % 4)?),
            Step::Debit(-i64::try_from(1 + next() % 20)?),
            Step::Buy(u8::try_from(1 + next() % 10)?),
            Step::Decide(0, next().is_multiple_of(2)),
            Step::Decide(1, next().is_multiple_of(2)),
        ];
        if next().is_multiple_of(2) {
            steps.swap(4, 5);
        }
        if let Err(error) = fixture.sequence(seed, &steps).await {
            result = Err(error);
            break;
        }
    }
    db.close().await?;
    eprintln!(
        "store_sequences_bind_debits_orders_and_delivery_intents: {:?}",
        started.elapsed()
    );
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Obligation {
    None,
    Reservation,
    BrowserTopUp,
    NativeTopUp,
    Debt,
    Hold,
}

// Historical payment snapshots for the closure worker. The payment property
// test exercises live creation and reversal through the owning service.
async fn payment_fixture(
    pool: &PgPool,
    account: Uuid,
    obligation: Obligation,
    coins: i64,
) -> TestResult {
    let order = Uuid::now_v7();
    let intent = Uuid::now_v7();
    let state = match obligation {
        Obligation::Debt | Obligation::Hold => "chargeback",
        _ => "succeeded",
    };
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,100,'EUR',$4)")
        .bind(intent).bind(account).bind(intent.to_string()).bind(state).execute(pool).await?;
    if obligation == Obligation::BrowserTopUp {
        sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id) VALUES($1,$2,'fixture',$3,100,'EUR',$1)")
            .bind(intent).bind(account).bind(coins).execute(pool).await?;
        return Ok(());
    }
    let quote = Uuid::now_v7();
    sqlx::query("INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES($1,$2,$3,$4,100,'EUR',2,'testing',now(),now())")
        .bind(quote).bind(account).bind(order).bind(coins).execute(pool).await?;
    // A held native success has no invoice yet; already credited chargebacks do.
    let receipt = if obligation == Obligation::NativeTopUp {
        None
    } else {
        let invoice = Uuid::now_v7();
        sqlx::query("INSERT INTO invoices(id,intent_id,account_id,amount,currency) VALUES($1,$2,$3,100,'EUR')")
            .bind(invoice).bind(intent).bind(account).execute(pool).await?;
        Some(invoice)
    };
    sqlx::query("INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,receipt_id,code,review_required) VALUES($1,$2,$1,$3,$1,$4,100,'EUR',2,'testing',$5,$6,$7,'review_required',true)")
        .bind(order).bind(account).bind(quote).bind(coins).bind(intent)
        .bind(if receipt.is_none() { "processing" } else { "chargeback" }).bind(receipt).execute(pool).await?;
    if obligation == Obligation::Debt || obligation == Obligation::Hold {
        // Build the prior credit and spend through the wallet. A held reversal
        // represents the provider having recovered its money without a debit.
        let ledger = Ledger::new(
            pool.clone(),
            WalletFlags { enabled: true },
            purchase::DatabaseAccounts,
            Policy,
            Policy,
        );
        let mut tx = pool.begin().await?;
        ledger
            .clone()
            .with_adjustment_kind(oteryn_wallet::AdjustmentKind::TopUp)
            .adjust_in(
                &mut tx,
                intent,
                account,
                coins,
                account,
                &format!("wallet-top-up:{intent}"),
            )
            .await?;
        tx.commit().await?;
        ledger
            .adjust(Uuid::now_v7(), account, -coins, account, "historical spend")
            .await?;
        sqlx::query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES($1,$2,$3,'chargeback','test',$4,$5,0,'held')")
            .bind(Uuid::now_v7()).bind(order).bind(account).bind(intent.to_string()).bind(coins).execute(pool).await?;
        if obligation == Obligation::Hold {
            sqlx::query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance')")
                .bind(account).bind(order).execute(pool).await?;
            ledger
                .adjust(
                    Uuid::now_v7(),
                    account,
                    coins,
                    account,
                    "historical debt repayment",
                )
                .await?;
        }
    }
    Ok(())
}

async fn closure_sequence(
    pool: &PgPool,
    service: &Terminations,
    password_hash: &str,
    seed: u64,
    obligation: Obligation,
) -> TestResult {
    let mut next = generator(seed);
    let coins = i64::try_from(1 + next() % 100)?;
    let reservation = i64::try_from(1 + next() % 100)?;
    let context = format!(
        "seed={seed:#x} sequence=[fund({reservation}), reservation({obligation:?}, {reservation}), payment_fixture({obligation:?}, {coins}), closure_request, due_worker, replay]"
    );
    let result: TestResult = async {
        let payer = account(pool, password_hash).await?;
        let ledger = Ledger::new(pool.clone(), WalletFlags { enabled:true }, purchase::DatabaseAccounts, Policy, Policy);
        let key = Uuid::now_v7();
        let barrier = Arc::new(Barrier::new(2));
        let spawn = || {
            let ledger = ledger.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                ledger.adjust(key,payer,reservation,payer,"closure property funding").await
            })
        };
        let first = spawn();
        let second = spawn();
        let mut first = first.await??;
        let mut second = second.await??;
        assert_ne!(first.replayed, second.replayed, "{context}");
        first.replayed = false;
        second.replayed = false;
        assert_eq!(first, second, "{context}");
        assert_eq!(ledger.balance(payer).await?.available, reservation, "{context}");
        match obligation {
            Obligation::None => (),
            Obligation::Reservation => {
                ledger.reserve(Uuid::now_v7(), payer, reservation).await?;
            }
            other => payment_fixture(pool, payer, other, coins).await?,
        }
        let balance = ledger.balance(payer).await?;
        let proof = ActorProof("test proof".into());
        let blocks = obligation != Obligation::None;
        let request = service.request(payer, &proof, PASSWORD, "TERMINATE").await;
        let accepted = request.is_ok();
        // The worker must recheck obligations even when the account was
        // already scheduled before the obligation appeared.
        let scheduled = match request {
            Ok(requested) => {
                sqlx::query("UPDATE account_terminations SET created_at=now()-interval '2 days', scheduled_for=now()-interval '1 day' WHERE id=$1")
                    .bind(requested.id).execute(pool).await?;
                requested.id
            }
            Err(termination::Error::UnknownState) if blocks => {
                sqlx::query_scalar("INSERT INTO account_terminations(account_id,created_at,scheduled_for) VALUES($1,now()-interval '2 days',now()-interval '1 day') RETURNING id")
                    .bind(payer).fetch_one(pool).await?
            }
            Err(error) => return Err(error.into()),
        };
        let work = service.run_worker(1).await?;
        let finalized: bool = sqlx::query_scalar("SELECT finalized_at IS NOT NULL FROM account_terminations WHERE id=$1")
            .bind(scheduled).fetch_one(pool).await?;
        let status: String = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1").bind(payer).fetch_one(pool).await?;
        if blocks && !finalized {
            assert_eq!(work.blocked, 1, "{context}");
            let replay = service.run_worker(1).await?;
            assert_eq!(replay.blocked, work.blocked, "{context}");
            assert_eq!(replay.finalized, 0, "{context}");
            service.cancel(payer, &proof, PASSWORD).await?;
        }
        if accepted == blocks || finalized == blocks || (status == "closed") == blocks {
            return Err(format!("closure request accepted={accepted}, worker finalized={finalized}, account status={status}").into());
        }
        assert_eq!(work.finalized, usize::from(!blocks), "{context}");
        let mut tx = pool.begin().await?;
        assert_eq!(ledger.read_balance_in(&mut tx, payer).await?, balance, "{context}");
        tx.commit().await?;
        Ok(())
    }.await;
    result.map_err(|error| format!("{context}: {error:?}").into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closure_sequences_never_finalize_open_obligations() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = Instant::now();
    let db = Database::new().await?;
    let hash = PasswordConfig {
        memory_kib: 64,
        iterations: 1,
        parallelism: 1,
    }
    .hash(PASSWORD)?;
    let service = Terminations::new(
        db.pool.clone(),
        termination::Config {
            enabled: true,
            grace_days: 1,
        },
        Arc::new(Policy),
    )?;
    let obligations = [
        Obligation::None,
        Obligation::Reservation,
        Obligation::BrowserTopUp,
        Obligation::NativeTopUp,
        Obligation::Debt,
        Obligation::Hold,
    ];
    let mut failures = Vec::new();
    for (index, seed) in SEEDS.enumerate() {
        if let Err(error) = closure_sequence(
            &db.pool,
            &service,
            &hash,
            seed,
            obligations[index % obligations.len()],
        )
        .await
        {
            failures.push(error.to_string());
        }
        if index % obligations.len() == obligations.len() - 1 && !failures.is_empty() {
            break;
        }
    }
    db.close().await?;
    eprintln!(
        "closure_sequences_never_finalize_open_obligations: {:?}",
        started.elapsed()
    );
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}
