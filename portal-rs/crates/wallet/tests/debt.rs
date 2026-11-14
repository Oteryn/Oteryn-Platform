#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Coin debt settlement on real PostgreSQL (owner decision 2026-11-14): every
//! credit pays held-reversal debt first, oldest first, audited, in the credit's
//! transaction; outgoing transfers and market listings wait for it.

use oteryn_wallet::*;
use sqlx::{PgConnection, PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tokio::sync::Barrier;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
struct Open;
impl AccountStateReader for Open {
    async fn state(&self, _: &mut PgConnection, _: Uuid) -> AccountState {
        AccountState::Active
    }
}
impl TransferBlockCheck for Open {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Allowed
    }
}
impl AuditHook for Open {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
type TestLedger = Ledger<Open, Open, Open>;

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL wallet debt test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        // SQL identifiers below contain only this fixed prefix and generated UUID hex.
        let schema = format!("wallet_debt_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        // Migrations run without the statement timeout: they wait on the
        // migrator's database-wide lock while other schemas migrate.
        let migrator = Self::pool(&url, &schema, 1, false).await;
        oteryn_db::migrate(&migrator).await.unwrap();
        migrator.close().await;
        let pool = Self::pool(&url, &schema, 25, true).await;
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn pool(url: &str, schema: &str, size: u32, timeout: bool) -> PgPool {
        let search_path = format!("SET search_path TO {schema}");
        PgPoolOptions::new()
            .max_connections(size)
            .after_connect(move |conn, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(sql))
                        .execute(&mut *conn)
                        .await?;
                    if timeout {
                        sqlx::query("SET statement_timeout='10s'")
                            .execute(conn)
                            .await?;
                    }
                    Ok(())
                })
            })
            .connect(url)
            .await
            .unwrap()
    }
    fn ledger(&self) -> TestLedger {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            Open,
            Open,
            Open,
        )
    }
    /// A held reversal of `coins`, `age` seconds old: the debt payments records
    /// when the balance no longer covers a refund or chargeback.
    async fn debt(&self, account: Uuid, coins: i64, age: i64) -> Uuid {
        let (quote, order, reversal) = (id(), id(), id());
        sqlx::query("INSERT INTO coin_purchase_quotes (id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES ($1,$2,$3,$4,100,'EUR',2,'testing',now(),now())")
            .bind(quote).bind(account).bind(id()).bind(coins).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO coin_purchase_orders (id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code,review_required) VALUES ($1,$2,$3,$4,$5,$6,100,'EUR',2,'testing',$7,'processing','review',true)")
            .bind(order).bind(account).bind(id()).bind(quote).bind(id()).bind(coins).bind(id())
            .execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO coin_purchase_reversals (id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome,created_at) VALUES ($1,$2,$3,'chargeback','test',$4,$5,0,'held',now()-make_interval(secs=>$6::double precision))")
            .bind(reversal).bind(order).bind(account).bind(reversal.to_string()).bind(coins).bind(age as f64)
            .execute(&self.pool).await.unwrap();
        reversal
    }
    async fn owed(&self, account: Uuid) -> i64 {
        let mut connection = self.pool.acquire().await.unwrap();
        outstanding_debt(&mut connection, account).await.unwrap()
    }
    async fn owed_on(&self, reversal: Uuid) -> i64 {
        sqlx::query_scalar("SELECT owed FROM coin_outstanding_debts WHERE reversal_id=$1")
            .bind(reversal)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn available(&self, account: Uuid) -> i64 {
        self.ledger().balance(account).await.unwrap().available
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
fn id() -> Uuid {
    Uuid::now_v7()
}
fn refused<T: std::fmt::Debug>(result: Result<T, Error>, expected: Refusal) {
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason == expected),
        "expected {expected:?}, got {result:?}"
    );
}
async fn grant(ledger: &TestLedger, account: Uuid, amount: i64) -> Receipt {
    ledger
        .grant(
            id(),
            account,
            amount,
            id(),
            "Test reward",
            GrantKind::Reward,
        )
        .await
        .unwrap()
}
/// A purchase credit as payments composes it: the top-up adjustment in the
/// caller's transaction.
async fn top_up(db: &Database, account: Uuid, coins: i64) -> Receipt {
    let key = id();
    let mut tx = db.pool.begin().await.unwrap();
    let receipt = db
        .ledger()
        .with_adjustment_kind(AdjustmentKind::TopUp)
        .adjust_in(
            &mut tx,
            key,
            account,
            coins,
            account,
            &format!("wallet-top-up:{key}"),
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    receipt
}

#[tokio::test]
async fn credit_smaller_equal_and_larger_than_the_debt() {
    let Some(db) = Database::open().await else {
        return;
    };
    let ledger = db.ledger();
    let (a, b) = (id(), id());
    ledger.adjust(id(), a, 15, id(), "Before").await.unwrap();
    let debt = db.debt(a, 100, 10).await;
    assert_eq!(db.owed(a).await, 100);

    // Smaller: the whole purchase credit pays debt, the balance is unchanged.
    let smaller = top_up(&db, a, 40).await;
    assert_eq!((smaller.amount, smaller.available_after), (40, 15));
    assert_eq!((db.owed(a).await, db.available(a).await), (60, 15));
    // Equal: the rest of the debt is paid by a staff credit.
    let equal = ledger.adjust(id(), a, 60, id(), "Staff").await.unwrap();
    assert_eq!(equal.available_after, 15);
    assert_eq!((db.owed(a).await, db.owed_on(debt).await), (0, 0));
    // No debt left: a credit reaches the balance whole.
    assert_eq!(grant(&ledger, a, 5).await.available_after, 20);

    // Larger: only the remainder reaches the balance.
    db.debt(b, 50, 10).await;
    let larger = grant(&ledger, b, 80).await;
    assert_eq!(larger.available_after, 30);
    assert_eq!((db.owed(b).await, db.available(b).await), (0, 30));
    // A replay neither credits nor settles again.
    let key = id();
    ledger
        .grant(key, b, 7, b, "Replay", GrantKind::StaffGrant)
        .await
        .unwrap();
    let replay = ledger
        .grant(key, b, 7, b, "Replay", GrantKind::StaffGrant)
        .await
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.available_after, 37);
    assert_eq!(db.available(b).await, 37);

    // The history shows each credit whole and the repayment beside it.
    let history = ledger
        .history(
            a,
            HistoryPage {
                before: None,
                limit: 100,
            },
        )
        .await
        .unwrap();
    let kinds: Vec<(&str, i64)> = history
        .iter()
        .rev()
        .map(|e| (e.kind.as_str(), e.available_delta))
        .collect();
    assert_eq!(
        kinds,
        [
            ("adjustment", 15),
            ("top_up", 40),
            ("debt_settlement", -40),
            ("adjustment", 60),
            ("debt_settlement", -60),
            ("reward", 5),
        ]
    );
    let settlements: Vec<(i64, i64, Uuid)> = sqlx::query_as("SELECT s.coins,s.owed_before,s.credit_operation_id FROM coin_debt_settlements s WHERE s.account_id=$1 ORDER BY s.created_at,s.id")
        .bind(a).fetch_all(&db.pool).await.unwrap();
    assert_eq!(
        settlements,
        [
            (40, 100, smaller.operation_id),
            (60, 60, equal.operation_id)
        ]
    );
    db.close().await;
}

#[tokio::test]
async fn several_debts_are_settled_oldest_first_and_audited() {
    let Some(db) = Database::open().await else {
        return;
    };
    let ledger = db.ledger();
    let a = id();
    // Created out of age order: the oldest is paid first whatever the id.
    let middle = db.debt(a, 20, 200).await;
    let newest = db.debt(a, 50, 100).await;
    let oldest = db.debt(a, 30, 300).await;
    let credit = grant(&ledger, a, 40).await;
    assert_eq!(credit.available_after, 0);
    assert_eq!(
        (
            db.owed_on(oldest).await,
            db.owed_on(middle).await,
            db.owed_on(newest).await
        ),
        (0, 10, 50)
    );
    type Row = (
        Option<Uuid>,
        String,
        i64,
        i64,
        i64,
        String,
        String,
        String,
        Uuid,
    );
    let audit: Vec<Row> = sqlx::query_as(
        "SELECT actor,target,(before->>'owed')::bigint,(after->>'owed')::bigint,(after->>'applied')::bigint, \
         after->>'account_id',after->>'credit_operation_id',after->>'settlement_operation_id',request_id \
         FROM audit_log WHERE action='wallet.coin_debt_settled' ORDER BY created_at,id",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let settlement: Uuid = sqlx::query_scalar(
        "SELECT id FROM wallet_operations WHERE kind='debt_settlement' AND reference_id=$1",
    )
    .bind(credit.operation_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let expected: Vec<Row> = [(oldest, 30, 30), (middle, 20, 10)]
        .into_iter()
        .map(|(debt, owed, applied)| {
            (
                None,
                debt.to_string(),
                owed,
                owed - applied,
                applied,
                a.to_string(),
                credit.operation_id.to_string(),
                settlement.to_string(),
                credit.operation_id,
            )
        })
        .collect();
    assert_eq!(audit, expected);
    // The next credit continues with the next-oldest debt.
    assert_eq!(grant(&ledger, a, 15).await.available_after, 0);
    assert_eq!(
        (db.owed_on(middle).await, db.owed_on(newest).await),
        (0, 45)
    );
    db.close().await;
}

#[tokio::test]
async fn transfer_and_market_listing_wait_for_the_debt() {
    let Some(db) = Database::open().await else {
        return;
    };
    let ledger = db.ledger();
    let (a, b, buyer) = (id(), id(), id());
    ledger.adjust(id(), a, 100, id(), "Funds").await.unwrap();
    // A key used before the debt still replays its committed result.
    let earlier = id();
    ledger.transfer(earlier, a, b, 10).await.unwrap();
    db.debt(a, 25, 10).await;
    refused(
        ledger.transfer(id(), a, b, 10).await,
        Refusal::DebtOutstanding,
    );
    refused(
        ledger.reserve_offer(id(), a, 10, 100).await,
        Refusal::DebtOutstanding,
    );
    assert!(ledger.transfer(earlier, a, b, 10).await.unwrap().replayed);
    // Spending on the platform is not a transfer and stays allowed.
    ledger.reserve(id(), a, 5).await.unwrap();
    assert_eq!(db.available(a).await, 85);
    // The refused listing left no operation behind.
    let listings: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM wallet_operations WHERE account_id=$1 AND kind='reserve' AND reason LIKE 'gold-price:%'",
    )
    .bind(a)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(listings, 0);
    // A transfer in pays the debt of its recipient first.
    ledger.adjust(id(), b, 50, id(), "Funds").await.unwrap();
    ledger.transfer(id(), b, a, 20).await.unwrap();
    assert_eq!((db.owed(a).await, db.available(a).await), (5, 85));
    refused(
        ledger.transfer(id(), a, b, 1).await,
        Refusal::DebtOutstanding,
    );
    // A market fill credits its buyer through the same rule.
    let offer = id();
    ledger.reserve_offer(offer, b, 30, 100).await.unwrap();
    db.debt(buyer, 4, 10).await;
    let fill = ledger.settle(id(), offer, buyer, 10).await.unwrap();
    assert_eq!(fill.account_id, b);
    assert_eq!((db.owed(buyer).await, db.available(buyer).await), (0, 6));
    // Settled: both outgoing paths are open again.
    grant(&ledger, a, 5).await;
    assert_eq!((db.owed(a).await, db.available(a).await), (0, 85));
    ledger.transfer(id(), a, b, 10).await.unwrap();
    ledger.reserve_offer(id(), a, 10, 100).await.unwrap();
    assert_eq!(db.available(a).await, 65);
    db.close().await;
}

#[tokio::test]
async fn concurrent_credits_and_debits_serialize_on_the_account() {
    let Some(db) = Database::open().await else {
        return;
    };
    let ledger = Arc::new(db.ledger());
    let a = id();
    ledger.adjust(id(), a, 30, id(), "Funds").await.unwrap();
    db.debt(a, 45, 10).await;
    let barrier = Arc::new(Barrier::new(24));
    let mut tasks = Vec::new();
    for n in 0..24 {
        let (ledger, barrier) = (ledger.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            if n % 2 == 0 {
                ledger
                    .grant(id(), a, 5, id(), "Credit", GrantKind::Reward)
                    .await
                    .map(|_| 0)
            } else {
                ledger.adjust(id(), a, -5, id(), "Debit").await.map(|_| 5)
            }
        }));
    }
    let mut debited = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(coins) => debited += coins,
            Err(Error::Refused(Refusal::InsufficientFunds)) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }
    // Twelve credits of 5 (60): 45 pay the debt, 15 reach the balance.
    assert_eq!(db.owed(a).await, 0);
    let paid: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(coins),0)::bigint FROM coin_debt_settlements")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(paid, 45);
    assert_eq!(db.available(a).await, 30 + 60 - 45 - debited);
    assert!(db.available(a).await >= 0);
    drop(ledger);
    db.close().await;
}

#[tokio::test]
async fn balance_and_debt_never_go_negative() {
    let Some(db) = Database::open().await else {
        return;
    };
    let ledger = db.ledger();
    let (a, b) = (id(), id());
    ledger
        .adjust(id(), b, 1_000_000, id(), "Funds")
        .await
        .unwrap();
    let (mut balance, mut owed) = (0_i64, 0_i64);
    // A fixed linear congruential sequence: reproducible, no extra crate.
    let mut seed: u64 = 0x5eed;
    for _ in 0..80 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let amount = i64::try_from((seed >> 33) % 40).unwrap() + 1;
        match (seed >> 20) % 5 {
            0 => {
                db.debt(a, amount, 0).await;
                owed += amount;
            }
            1 => {
                grant(&ledger, a, amount).await;
                let paid = amount.min(owed);
                owed -= paid;
                balance += amount - paid;
            }
            2 => {
                ledger.transfer(id(), b, a, amount).await.unwrap();
                let paid = amount.min(owed);
                owed -= paid;
                balance += amount - paid;
            }
            3 => match ledger.adjust(id(), a, -amount, id(), "Debit").await {
                Ok(_) => balance -= amount,
                Err(Error::Refused(Refusal::InsufficientFunds)) => assert!(amount > balance),
                other => panic!("unexpected debit result: {other:?}"),
            },
            _ => match ledger.transfer(id(), a, b, amount).await {
                Ok(_) => {
                    assert_eq!(owed, 0);
                    balance -= amount;
                }
                Err(Error::Refused(Refusal::DebtOutstanding)) => assert!(owed > 0),
                Err(Error::Refused(Refusal::InsufficientFunds)) => assert!(amount > balance),
                other => panic!("unexpected transfer result: {other:?}"),
            },
        }
        assert!(balance >= 0 && owed >= 0);
        assert_eq!(db.available(a).await, balance);
        assert_eq!(db.owed(a).await, owed);
    }
    db.close().await;
}
