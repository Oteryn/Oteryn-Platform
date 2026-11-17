//! `run_transaction` bounds lock waits and idle time by the mutation budget and
//! owns commit and rollback.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oteryn_db::transaction::run_transaction;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::{Duration, Instant};

async fn pool(connections: u32) -> Option<PgPool> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED budgeted transaction test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    Some(
        PgPoolOptions::new()
            .max_connections(connections)
            .connect(&url)
            .await
            .unwrap(),
    )
}

async fn key(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT ('x' || substr(md5(gen_random_uuid()::text), 1, 15))::bit(60)::bigint",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn held_lock_sheds_within_budget() {
    let Some(pool) = pool(2).await else { return };
    let key = key(&pool).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut *holder)
        .await
        .unwrap();

    let started = Instant::now();
    let outcome = tokio::time::timeout(
        Duration::from_secs(10),
        run_transaction(&pool, async |tx| {
            sqlx::query("SELECT pg_advisory_xact_lock($1)")
                .bind(key)
                .execute(tx.connection().await?)
                .await
                .map(|_| ())
        }),
    )
    .await
    .expect("lock wait outlived the mutation budget");
    let elapsed = started.elapsed();
    holder.rollback().await.unwrap();

    let error = outcome.expect_err("held lock was acquired");
    let code = error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned());
    // Both timeouts carry the same remaining budget; whichever fires first
    // (lock 55P03 or statement 57014) is the budget shedding the wait.
    assert!(
        matches!(code.as_deref(), Some("55P03" | "57014")),
        "{error:?}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
    pool.close().await;
}

#[tokio::test]
async fn timeouts_are_armed_to_the_remaining_budget_and_never_zero() {
    let Some(pool) = pool(1).await else { return };
    let (lock, statement, transaction, server): (String, String, Option<String>, i32) =
        run_transaction(&pool, async |tx| {
            sqlx::query_as(
                "SELECT current_setting('lock_timeout'), current_setting('statement_timeout'), \
                 current_setting('transaction_timeout', true), \
                 current_setting('server_version_num')::integer",
            )
            .fetch_one(tx.connection().await?)
            .await
        })
        .await
        .unwrap();
    let millis = |value: &str| -> i64 {
        let value = value.trim();
        if let Some(ms) = value.strip_suffix("ms") {
            ms.parse().unwrap()
        } else if let Some(s) = value.strip_suffix('s') {
            s.parse::<i64>().unwrap() * 1000
        } else {
            value.parse().unwrap()
        }
    };
    for (name, value) in [("lock_timeout", &lock), ("statement_timeout", &statement)] {
        let ms = millis(value);
        assert!((1..=3000).contains(&ms), "{name} = {value}");
    }
    if server >= 170000 {
        let ms = millis(transaction.as_deref().unwrap());
        assert!(
            (1..=3500).contains(&ms),
            "transaction_timeout = {transaction:?}"
        );
    }
    // The settings are local: the pooled session keeps no budget afterwards.
    let after: String = sqlx::query_scalar("SELECT current_setting('lock_timeout')")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, "0");
    pool.close().await;
}

#[tokio::test]
async fn idle_transaction_past_the_budget_is_ended() {
    let Some(pool) = pool(1).await else { return };
    let server: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::integer")
        .fetch_one(&pool)
        .await
        .unwrap();
    if server < 170000 {
        eprintln!("SKIPPED idle budget test: transaction_timeout needs PostgreSQL 17");
        return;
    }
    let outcome: Result<(), sqlx::Error> = run_transaction(&pool, async |tx| {
        sqlx::query("SELECT 1")
            .execute(tx.connection().await?)
            .await?;
        tokio::time::sleep(Duration::from_millis(4000)).await;
        // Lending the connection again must not extend the budget either.
        sqlx::query("SELECT 1")
            .execute(tx.connection().await?)
            .await?;
        Ok(())
    })
    .await;
    assert!(outcome.is_err(), "transaction outlived its budget");
    pool.close().await;
}

#[tokio::test]
async fn ok_commits_and_err_rolls_back() {
    let Some(pool) = pool(1).await else { return };
    sqlx::query("CREATE TEMP TABLE budget_marks(mark text PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    run_transaction(&pool, async |tx| {
        sqlx::query("INSERT INTO budget_marks VALUES ('kept')")
            .execute(tx.connection().await?)
            .await
            .map(|_| ())
    })
    .await
    .unwrap();
    let refused: Result<(), sqlx::Error> = run_transaction(&pool, async |tx| {
        sqlx::query("INSERT INTO budget_marks VALUES ('dropped')")
            .execute(tx.connection().await?)
            .await?;
        Err(sqlx::Error::RowNotFound)
    })
    .await;
    assert!(matches!(refused, Err(sqlx::Error::RowNotFound)));
    let marks: Vec<String> = sqlx::query_scalar("SELECT mark FROM budget_marks ORDER BY mark")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(marks, ["kept"]);
    pool.close().await;
}
