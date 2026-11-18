use super::*;
use sqlx::{PgConnection, PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Instant};
use tokio::sync::Barrier;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SEEDS: std::ops::Range<u64> = 0x5741_0000..0x5741_00c8;

#[derive(Clone, Copy, Debug)]
struct Policy;
impl AccountStateReader for Policy {
    async fn state(&self, _: &mut PgConnection, _: Uuid) -> AccountState {
        AccountState::Active
    }
}
impl TransferBlockCheck for Policy {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Allowed
    }
}
impl AuditHook for Policy {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug)]
enum Step {
    Adjust(i64),
    Reserve(i64),
    Finish { capture: bool },
}

async fn apply(
    ledger: &Ledger<Policy, Policy, Policy>,
    account: Uuid,
    key: Uuid,
    step: Step,
    reservation: Option<(Uuid, i64)>,
) -> Result<Receipt, Error> {
    match step {
        Step::Adjust(amount) => {
            ledger
                .adjust(key, account, amount, account, "property test")
                .await
        }
        Step::Reserve(amount) => ledger.reserve(key, account, amount).await,
        Step::Finish { capture } => {
            let id = reservation.map_or(Uuid::nil(), |(id, _)| id);
            if capture {
                ledger.capture(key, account, id).await
            } else {
                ledger.release(key, account, id).await
            }
        }
    }
}

fn replay_result(result: Result<Receipt, Error>) -> Result<Result<Receipt, Refusal>, Error> {
    match result {
        Ok(mut receipt) => {
            receipt.replayed = false;
            Ok(Ok(receipt))
        }
        Err(Error::Refused(refusal)) => Ok(Err(refusal)),
        Err(error) => Err(error),
    }
}

async fn sequence(pool: &PgPool, seed: u64) -> TestResult {
    // Fixed LCG. Use high bits so sampling small ranges does not use the short low-bit cycle.
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        state >> 32
    };
    let steps: Vec<_> = (0..12)
        .map(|index| {
            let owner = (next() % 2) as usize;
            let amount = i64::try_from(1 + next() % 50)?;
            let step = match (index, next() % 6) {
                (0, _) | (_, 0 | 1) => Step::Adjust(amount),
                (_, 2) => Step::Adjust(-amount),
                (_, 3) => Step::Reserve(amount),
                (_, kind) => Step::Finish { capture: kind == 4 },
            };
            Ok::<_, std::num::TryFromIntError>((owner, step, next().is_multiple_of(3)))
        })
        .collect::<Result<_, _>>()?;
    let ledger = Arc::new(Ledger::new(
        pool.clone(),
        WalletFlags { enabled: true },
        Policy,
        Policy,
        Policy,
    ));
    let accounts = [Uuid::now_v7(), Uuid::now_v7()];
    let mut available = [0; 2];
    let mut reserved = [0; 2];
    let mut reservations = [Vec::new(), Vec::new()];
    let mut accepted = [0; 2];
    let mut previous = None;
    for (index, &(owner, step, concurrent)) in steps.iter().enumerate() {
        let context = format!("seed={seed:#x} step={index} sequence={steps:?}");
        let checked: TestResult = async {
            let account = accounts[owner];
            let key = Uuid::now_v7();
            let reservation = reservations[owner].last().copied();
            let result = if index == 0 || concurrent {
                let barrier = Arc::new(Barrier::new(2));
                let spawn = || {
                    let ledger = ledger.clone();
                    let barrier = barrier.clone();
                    tokio::spawn(async move {
                        barrier.wait().await;
                        apply(&ledger, account, key, step, reservation).await
                    })
                };
                let first = spawn();
                let second = spawn();
                let first = first.await?;
                let second = second.await?;
                if let (Ok(a), Ok(b)) = (&first, &second) {
                    assert_ne!(a.replayed, b.replayed, "{context}");
                }
                assert_eq!(replay_result(first)?, replay_result(second)?, "{context}");
                apply(&ledger, account, key, step, reservation).await
            } else {
                apply(&ledger, account, key, step, reservation).await
            };
            let expected = match step {
                Step::Adjust(amount) if available[owner] + amount >= 0 => {
                    available[owner] += amount;
                    Ok(())
                }
                Step::Reserve(amount) if available[owner] >= amount => {
                    available[owner] -= amount;
                    reserved[owner] += amount;
                    reservations[owner].push((key, amount));
                    Ok(())
                }
                Step::Finish { capture } => {
                    if let Some((_, amount)) = reservations[owner].pop() {
                        reserved[owner] -= amount;
                        if !capture {
                            available[owner] += amount;
                        }
                        Ok(())
                    } else {
                        Err(Refusal::InvalidIdentifier)
                    }
                }
                _ => Err(Refusal::InsufficientFunds),
            };
            let result = replay_result(result)?;
            assert_eq!(
                result.as_ref().map(|_| ()).map_err(|err| *err),
                expected,
                "{context}"
            );
            let replay = apply(&ledger, account, key, step, reservation).await;
            if let Ok(receipt) = &replay {
                assert!(receipt.replayed, "{context}");
            }
            assert_eq!(result, replay_result(replay)?, "{context}");
            if let Some((old_account, old_key, old_step, old_reservation, old_receipt)) =
                previous.take()
            {
                assert_eq!(
                    replay_result(
                        apply(&ledger, old_account, old_key, old_step, old_reservation).await
                    )?,
                    Ok(old_receipt),
                    "{context}"
                );
            }
            if let Ok(receipt) = result {
                accepted[owner] += 1;
                previous = Some((account, key, step, reservation, receipt));
            }
            for (i, &id) in accounts.iter().enumerate() {
                let balance = ledger.balance(id).await?;
                assert_eq!(
                    (balance.available, balance.reserved),
                    (available[i], reserved[i]),
                    "{context}"
                );
                assert!(balance.available >= 0 && balance.reserved >= 0, "{context}");
                let history = ledger.history(id, HistoryPage::default()).await?;
                assert_eq!(history.len(), accepted[i], "{context}");
                assert_eq!(
                    history
                        .iter()
                        .map(|entry| entry.available_delta + entry.reserved_delta)
                        .sum::<i64>(),
                    available[i] + reserved[i],
                    "{context}"
                );
                let orphan: bool = sqlx::query_scalar(
                    "SELECT EXISTS (
                     SELECT 1 FROM wallet_operations o
                     FULL JOIN wallet_entries e ON e.operation_id=o.id AND e.account_id=o.account_id
                     WHERE (o.account_id=$1 OR e.account_id=$1) AND (o.id IS NULL OR e.id IS NULL)
                 )",
                )
                .bind(id)
                .fetch_one(pool)
                .await?;
                assert!(!orphan, "{context}");
            }
            Ok(())
        }
        .await;
        checked.map_err(|error| format!("{context}: {error:?}"))?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wallet_sequences_preserve_balances_and_keyed_effects() -> TestResult {
    let started = Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("wallet_properties_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(4)
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
    let mut result = Ok(());
    for seed in SEEDS {
        if let Err(error) = sequence(&pool, seed).await {
            result = Err(format!("seed={seed:#x}: {error:?}").into());
            break;
        }
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    eprintln!(
        "wallet_sequences_preserve_balances_and_keyed_effects: {:?}",
        started.elapsed()
    );
    result
}
