#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oteryn_wallet::*;
use sqlx::{PgConnection, PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Accounts(Vec<Uuid>);
impl AccountStateReader for Accounts {
    async fn state(&self, _: &mut PgConnection, account: Uuid) -> AccountState {
        if self.0.contains(&account) {
            AccountState::Active
        } else {
            AccountState::Unknown
        }
    }
}
#[derive(Debug, Clone)]
struct Blocks {
    blocked: Option<Uuid>,
    unknown: bool,
}
impl TransferBlockCheck for Blocks {
    async fn permission(&self, _: &mut PgConnection, account: Uuid) -> TransferPermission {
        if self.unknown {
            TransferPermission::Unknown
        } else if self.blocked == Some(account) {
            TransferPermission::Blocked
        } else {
            TransferPermission::Allowed
        }
    }
}
#[derive(Debug, Clone)]
struct Audit(bool);
impl AuditHook for Audit {
    async fn adjustment(&self, connection: &mut PgConnection, event: &AdjustmentAudit) -> bool {
        let inserted = sqlx::query("INSERT INTO test_audit (id, actor, reason, before_amount, after_amount) VALUES ($1,$2,$3,$4,$5)")
            .bind(event.operation_id).bind(event.actor_id).bind(&event.reason).bind(event.before.available).bind(event.after.available)
            .execute(connection).await.is_ok();
        inserted && self.0
    }
}
#[derive(Debug)]
struct Inactive;
impl AccountStateReader for Inactive {
    async fn state(&self, _: &mut PgConnection, _: Uuid) -> AccountState {
        AccountState::Inactive
    }
}
type TestLedger = Ledger<Accounts, Blocks, Audit>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL wallet test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        // SQL identifiers below contain only this fixed prefix and generated UUID hex.
        let schema = format!("wallet_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search_path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(25)
            .after_connect(move |conn, _| {
                let sql = search_path.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(sql))
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("SET statement_timeout='10s'")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        sqlx::query("CREATE TABLE test_audit (id uuid PRIMARY KEY, actor uuid, reason text, before_amount bigint, after_amount bigint)").execute(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    fn ledger(&self, accounts: &[Uuid]) -> TestLedger {
        Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            Accounts(accounts.to_vec()),
            Blocks {
                blocked: None,
                unknown: false,
            },
            Audit(true),
        )
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
fn refused<T: std::fmt::Debug>(result: Result<T, Error>, expected: Refusal) {
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason == expected),
        "expected {expected:?}"
    );
}
fn id() -> Uuid {
    Uuid::now_v7()
}
async fn fund(ledger: &TestLedger, account: Uuid, amount: i64) {
    ledger
        .adjust(id(), account, amount, id(), "Test grant")
        .await
        .unwrap();
}

#[tokio::test]
async fn twenty_concurrent_debits_cannot_make_balance_negative() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (sender, recipient) = (id(), id());
    let ledger = Arc::new(db.ledger(&[sender, recipient]));
    fund(&ledger, sender, 10).await;
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let (ledger, barrier) = (ledger.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            ledger.transfer(id(), sender, recipient, 1).await
        }));
    }
    let mut succeeded = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => succeeded += 1,
            Err(Error::Refused(Refusal::InsufficientFunds)) => {}
            other => panic!("unexpected debit result: {other:?}"),
        }
    }
    assert_eq!(succeeded, 10);
    assert_eq!(ledger.balance(sender).await.unwrap().available, 0);
    assert_eq!(ledger.balance(recipient).await.unwrap().available, 10);
    let transfers: i64 = sqlx::query_scalar("SELECT count(*) FROM wallet_transfers")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(transfers, 10);
    drop(ledger);
    db.close().await;
}

#[tokio::test]
async fn transfer_replay_is_atomic_and_payload_bound() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (sender, recipient, key) = (id(), id(), id());
    let ledger = Arc::new(db.ledger(&[sender, recipient]));
    fund(&ledger, sender, 20).await;
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let (ledger, barrier) = (ledger.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            ledger.transfer(key, sender, recipient, 7).await.unwrap()
        }));
    }
    let mut fresh = 0;
    for task in tasks {
        if !task.await.unwrap().replayed {
            fresh += 1;
        }
    }
    assert_eq!(fresh, 1);
    refused(
        ledger.transfer(key, sender, recipient, 8).await,
        Refusal::IdempotencyConflict,
    );
    refused(
        ledger.reserve(key, sender, 7).await,
        Refusal::IdempotencyConflict,
    );
    assert_eq!(ledger.balance(sender).await.unwrap().available, 13);
    assert_eq!(ledger.balance(recipient).await.unwrap().available, 7);
    drop(ledger);
    db.close().await;
}

#[tokio::test]
async fn opposite_transfers_use_deterministic_locks() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b) = (id(), id());
    assert_eq!(lock_order(a, b), lock_order(b, a));
    let ledger = Arc::new(db.ledger(&[a, b]));
    fund(&ledger, a, 100).await;
    fund(&ledger, b, 100).await;
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = Vec::new();
    for n in 0..20 {
        let (ledger, barrier) = (ledger.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let (from, to) = if n % 2 == 0 { (a, b) } else { (b, a) };
            ledger.transfer(id(), from, to, 1).await.unwrap();
        }));
    }
    tokio::time::timeout(Duration::from_secs(15), async {
        for task in tasks {
            task.await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(ledger.balance(a).await.unwrap().available, 100);
    assert_eq!(ledger.balance(b).await.unwrap().available, 100);
    drop(ledger);
    db.close().await;
}

#[tokio::test]
async fn reservation_capture_release_and_replay() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b, reservation) = (id(), id(), id());
    let ledger = db.ledger(&[a, b]);
    fund(&ledger, a, 100).await;
    ledger.reserve(reservation, a, 60).await.unwrap();
    assert!(ledger.reserve(reservation, a, 60).await.unwrap().replayed);
    assert_eq!(
        ledger.balance(a).await.unwrap(),
        Balance {
            available: 40,
            transferable: 40,
            non_transferable: 0,
            reserved: 60
        }
    );
    refused(
        ledger.transfer(id(), a, b, 41).await,
        Refusal::InsufficientFunds,
    );
    refused(
        ledger.reserve(id(), a, 41).await,
        Refusal::InsufficientFunds,
    );
    refused(
        ledger.release(id(), b, reservation).await,
        Refusal::ReservationMissing,
    );
    let capture = id();
    ledger.capture(capture, a, reservation).await.unwrap();
    assert!(
        ledger
            .capture(capture, a, reservation)
            .await
            .unwrap()
            .replayed
    );
    refused(
        ledger.release(id(), a, reservation).await,
        Refusal::ReservationSettled,
    );
    refused(
        ledger.capture(id(), a, reservation).await,
        Refusal::ReservationSettled,
    );
    let reservation = id();
    let release = id();
    ledger.reserve(reservation, a, 30).await.unwrap();
    ledger.release(release, a, reservation).await.unwrap();
    assert!(
        ledger
            .release(release, a, reservation)
            .await
            .unwrap()
            .replayed
    );
    refused(
        ledger.capture(id(), a, reservation).await,
        Refusal::ReservationSettled,
    );
    assert_eq!(
        ledger.balance(a).await.unwrap(),
        Balance {
            available: 40,
            transferable: 40,
            non_transferable: 0,
            reserved: 0
        }
    );
    db.close().await;
}

#[tokio::test]
async fn blocks_and_unknown_accounts_fail_closed() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b) = (id(), id());
    let ledger = db.ledger(&[a, b]);
    fund(&ledger, a, 10).await;
    for blocked in [a, b] {
        let denied = Ledger::new(
            db.pool.clone(),
            WalletFlags { enabled: true },
            Accounts(vec![a, b]),
            Blocks {
                blocked: Some(blocked),
                unknown: false,
            },
            Audit(true),
        );
        refused(
            denied.transfer(id(), a, b, 1).await,
            Refusal::TransferBlocked,
        );
    }
    let unknown = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Accounts(vec![a, b]),
        Blocks {
            blocked: None,
            unknown: true,
        },
        Audit(true),
    );
    refused(
        unknown.transfer(id(), a, b, 1).await,
        Refusal::TransferStateUnknown,
    );
    refused(
        ledger.transfer(id(), a, id(), 1).await,
        Refusal::AccountDenied,
    );
    refused(
        ledger.transfer(id(), id(), b, 1).await,
        Refusal::AccountDenied,
    );
    refused(ledger.balance(id()).await, Refusal::AccountDenied);
    refused(ledger.reserve(id(), id(), 1).await, Refusal::AccountDenied);
    let inactive = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Inactive,
        Blocks {
            blocked: None,
            unknown: false,
        },
        Audit(true),
    );
    refused(
        inactive.transfer(id(), a, b, 1).await,
        Refusal::AccountDenied,
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM wallet_transfers")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(ledger.balance(a).await.unwrap().available, 10);
    db.close().await;
}

#[tokio::test]
async fn staff_adjustment_audit_is_atomic_and_idempotent() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, actor, key) = (id(), id(), id());
    let ledger = db.ledger(&[a]);
    ledger
        .adjust(key, a, 15, actor, "Correction")
        .await
        .unwrap();
    assert!(
        ledger
            .adjust(key, a, 15, actor, "Correction")
            .await
            .unwrap()
            .replayed
    );
    refused(
        ledger.adjust(key, a, 15, actor, "Different").await,
        Refusal::IdempotencyConflict,
    );
    let record: (Uuid, String, i64, i64) = sqlx::query_as(
        "SELECT actor,reason,before_amount,after_amount FROM test_audit WHERE id=$1",
    )
    .bind(key)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(record, (actor, "Correction".into(), 0, 15));
    let denied = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Accounts(vec![a]),
        Blocks {
            blocked: None,
            unknown: false,
        },
        Audit(false),
    );
    let key = id();
    refused(
        denied.adjust(key, a, 5, actor, "Denied").await,
        Refusal::AuditDenied,
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_audit")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    ledger
        .adjust(key, a, -5, actor, "Retry after rollback")
        .await
        .unwrap();
    refused(
        ledger.adjust(id(), a, -11, actor, "Too much").await,
        Refusal::InsufficientFunds,
    );
    assert_eq!(ledger.balance(a).await.unwrap().available, 10);
    db.close().await;
}

#[tokio::test]
async fn history_pagination_and_immutable_entries() {
    let Some(db) = Database::open().await else {
        return;
    };
    let a = id();
    let ledger = db.ledger(&[a]);
    for _ in 0..5 {
        fund(&ledger, a, 1).await;
    }
    let first = ledger
        .history(
            a,
            HistoryPage {
                before: None,
                limit: 2,
            },
        )
        .await
        .unwrap();
    let second = ledger
        .history(
            a,
            HistoryPage {
                before: Some(first[1].id),
                limit: 2,
            },
        )
        .await
        .unwrap();
    let third = ledger
        .history(
            a,
            HistoryPage {
                before: Some(second[1].id),
                limit: 2,
            },
        )
        .await
        .unwrap();
    assert_eq!((first.len(), second.len(), third.len()), (2, 2, 1));
    assert!(first[1].id > second[0].id && second[1].id > third[0].id);
    assert_eq!(first[0].available_delta, 1);
    // Identifiers are a fixed allowlist, never external input.
    for table in [
        "wallet_entries",
        "wallet_operations",
        "wallet_adjustments",
        "wallet_transfers",
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&db.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(format!("TRUNCATE {table} CASCADE")))
                .execute(&db.pool)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("UPDATE wallet_entries SET available_delta=999")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert_eq!(ledger.balance(a).await.unwrap().available, 5);
    db.close().await;
}

#[tokio::test]
async fn invalid_input_overflow_and_recipient_failure_roll_back() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b) = (id(), id());
    let ledger = db.ledger(&[a, b]);
    for amount in [0, -1, i64::MIN] {
        refused(
            ledger.transfer(id(), a, b, amount).await,
            Refusal::InvalidAmount,
        );
        refused(
            ledger.reserve(id(), a, amount).await,
            Refusal::InvalidAmount,
        );
    }
    refused(ledger.transfer(id(), a, a, 1).await, Refusal::SameAccount);
    refused(
        ledger.balance(Uuid::nil()).await,
        Refusal::InvalidIdentifier,
    );
    refused(
        ledger.reserve(Uuid::nil(), a, 1).await,
        Refusal::InvalidIdentifier,
    );
    refused(
        ledger.adjust(id(), a, 0, id(), "Invalid").await,
        Refusal::InvalidAmount,
    );
    refused(
        ledger.adjust(id(), a, i64::MIN, id(), "Invalid").await,
        Refusal::InvalidAmount,
    );
    for reason in [" ".to_owned(), "a".repeat(1001)] {
        refused(
            ledger.adjust(id(), a, 1, id(), &reason).await,
            Refusal::InvalidReason,
        );
    }
    refused(
        ledger
            .history(
                a,
                HistoryPage {
                    before: None,
                    limit: 0,
                },
            )
            .await,
        Refusal::InvalidPage,
    );
    let disabled = Ledger::new(
        db.pool.clone(),
        WalletFlags::default(),
        Accounts(vec![a, b]),
        Blocks {
            blocked: None,
            unknown: false,
        },
        Audit(true),
    );
    refused(disabled.balance(a).await, Refusal::Disabled);
    refused(disabled.transfer(id(), a, b, 1).await, Refusal::Disabled);
    refused(disabled.reserve(id(), a, 1).await, Refusal::Disabled);
    refused(disabled.release(id(), a, id()).await, Refusal::Disabled);
    refused(disabled.capture(id(), a, id()).await, Refusal::Disabled);
    refused(
        disabled.adjust(id(), a, 1, id(), "Disabled").await,
        Refusal::Disabled,
    );
    refused(
        disabled.history(a, HistoryPage::default()).await,
        Refusal::Disabled,
    );
    fund(&ledger, a, 10).await;
    fund(&ledger, b, i64::MAX).await;
    refused(ledger.transfer(id(), a, b, 1).await, Refusal::Overflow);
    assert_eq!(ledger.balance(a).await.unwrap().available, 10);
    assert_eq!(ledger.balance(b).await.unwrap().available, i64::MAX);
    db.close().await;
}

#[tokio::test]
async fn coin_classes_store_order_and_audited_grants() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b, actor) = (id(), id(), id());
    let ledger = db.ledger(&[a, b]);
    fund(&ledger, a, 40).await;
    let grant = id();
    let first = ledger
        .credit_non_transferable(grant, a, 60, actor, "Reward", GrantKind::Reward)
        .await
        .unwrap();
    assert_eq!(first.non_transferable_after, 60);
    assert!(
        ledger
            .credit_non_transferable(grant, a, 60, actor, "Reward", GrantKind::Reward)
            .await
            .unwrap()
            .replayed
    );
    refused(
        ledger
            .credit_non_transferable(grant, a, 61, actor, "Reward", GrantKind::Reward)
            .await,
        Refusal::IdempotencyConflict,
    );
    refused(
        ledger.transfer(id(), a, b, 41).await,
        Refusal::InsufficientFunds,
    );
    refused(
        ledger.reserve_offer(id(), a, 41, 100).await,
        Refusal::InsufficientFunds,
    );
    let store = db
        .ledger(&[a, b])
        .with_adjustment_kind(AdjustmentKind::StorePurchase);
    let key = id();
    let mut tx = db.pool.begin().await.unwrap();
    store
        .adjust_in(&mut tx, key, a, -70, a, &format!("store-purchase:{key}"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after = ledger.balance(a).await.unwrap();
    assert_eq!(
        (after.available, after.transferable, after.non_transferable),
        (30, 30, 0)
    );
    let history = ledger.history(a, HistoryPage::default()).await.unwrap();
    assert_eq!(history.iter().filter(|e| e.operation_id == key).count(), 2);
    assert_eq!(
        ledger
            .operation(grant)
            .await
            .unwrap()
            .unwrap()
            .non_transferable_after,
        60
    );
    refused(
        ledger
            .credit_non_transferable(id(), a, 0, actor, "Bad", GrantKind::StaffGrant)
            .await,
        Refusal::InvalidAmount,
    );
    fund(&ledger, b, i64::MAX).await;
    refused(
        ledger
            .credit_non_transferable(id(), b, 1, actor, "Overflow", GrantKind::StaffGrant)
            .await,
        Refusal::Overflow,
    );
    db.close().await;
}

#[tokio::test]
async fn market_partial_fills_replays_release_and_exhaustion() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b, c) = (id(), id(), id());
    let ledger = db.ledger(&[a, b, c]);
    fund(&ledger, a, 100).await;
    let offer = id();
    ledger.reserve_offer(offer, a, 80, 100).await.unwrap();
    refused(
        ledger.reserve_offer(offer, a, 80, 101).await,
        Refusal::IdempotencyConflict,
    );
    let trade = id();
    let first = ledger.settle(trade, offer, b, 30).await.unwrap();
    assert_eq!((first.transferable_after, first.reserved_after), (20, 50));
    ledger.settle(id(), offer, c, 20).await.unwrap();
    let replay = ledger.settle(trade, offer, b, 30).await.unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.reserved_after, 50);
    refused(
        ledger.settle(trade, offer, c, 30).await,
        Refusal::IdempotencyConflict,
    );
    refused(
        ledger.settle(trade, id(), b, 30).await,
        Refusal::IdempotencyConflict,
    );
    let release = id();
    let released = ledger.release(release, a, offer).await.unwrap();
    assert_eq!(released.amount, 30);
    assert_eq!(released.transferable_after, 50);
    assert!(ledger.release(release, a, offer).await.unwrap().replayed);
    refused(
        ledger.settle(id(), offer, b, 1).await,
        Refusal::InsufficientFunds,
    );
    let offer = id();
    ledger.reserve_offer(offer, a, 50, 1).await.unwrap();
    ledger.settle(id(), offer, b, 25).await.unwrap();
    ledger.settle(id(), offer, b, 25).await.unwrap();
    refused(
        ledger.settle(id(), offer, b, 1).await,
        Refusal::InsufficientFunds,
    );
    assert_eq!(ledger.balance(a).await.unwrap().reserved, 0);
    assert_eq!(ledger.balance(b).await.unwrap().transferable, 80);
    assert_eq!(ledger.balance(c).await.unwrap().transferable, 20);
    assert_eq!(
        ledger.history(b, HistoryPage::default()).await.unwrap()[0].counterparty_account_id,
        Some(a)
    );
    db.close().await;
}

#[tokio::test]
async fn concurrent_market_fills_never_over_capture() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b) = (id(), id());
    let ledger = Arc::new(db.ledger(&[a, b]));
    fund(&ledger, a, 100).await;
    let offer = id();
    ledger.reserve_offer(offer, a, 100, 1).await.unwrap();
    let barrier = Arc::new(Barrier::new(12));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let ledger = ledger.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            ledger.settle(id(), offer, b, 10).await
        }));
    }
    let mut successes = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => successes += 1,
            Err(Error::Refused(Refusal::InsufficientFunds)) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(successes, 10);
    assert_eq!(ledger.balance(a).await.unwrap().reserved, 0);
    assert_eq!(ledger.balance(b).await.unwrap().transferable, 100);
    db.close().await;
}

#[tokio::test]
async fn market_refusals_and_key_validation_are_atomic() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (a, b) = (id(), id());
    let ledger = db.ledger(&[a, b]);
    fund(&ledger, a, 10).await;
    refused(
        ledger.reserve_offer(id(), a, 1, 0).await,
        Refusal::InvalidAmount,
    );
    refused(
        ledger.settle(id(), id(), b, 1).await,
        Refusal::ReservationMissing,
    );
    let offer = id();
    ledger.reserve_offer(offer, a, 10, 1).await.unwrap();
    refused(ledger.settle(id(), offer, a, 1).await, Refusal::SameAccount);
    refused(
        ledger.settle(id(), offer, b, 0).await,
        Refusal::InvalidAmount,
    );
    refused(
        ledger.settle(id(), offer, b, 11).await,
        Refusal::InsufficientFunds,
    );
    let blocked = Ledger::new(
        db.pool.clone(),
        WalletFlags { enabled: true },
        Accounts(vec![a, b]),
        Blocks {
            blocked: Some(b),
            unknown: false,
        },
        Audit(true),
    );
    let key = id();
    refused(
        blocked.settle(key, offer, b, 1).await,
        Refusal::TransferBlocked,
    );
    assert!(ledger.operation(key).await.unwrap().is_none());
    assert_eq!(ledger.balance(a).await.unwrap().reserved, 10);
    db.close().await;
}

#[path = "coin_http/mod.rs"]
mod coin_http;
