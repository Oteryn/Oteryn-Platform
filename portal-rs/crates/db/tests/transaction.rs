//! `run_transaction` bounds lock waits and idle time by the mutation budget and
//! owns commit and rollback.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oteryn_db::transaction::{arm_commit, run_transaction};
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

#[tokio::test]
async fn deferred_commit_check_is_the_authorization_point() {
    let pool = pool(3).await.expect("PostgreSQL required");
    let schema = format!("commit_deadline_{}", sqlx::types::Uuid::now_v7().simple());
    let mut setup = pool.begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&mut *setup)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('search_path', $1, true)")
        .bind(&schema)
        .execute(&mut *setup)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261122173041_mutation_commit_deadline.sql"
    ))
    .execute(&mut *setup)
    .await
    .unwrap();
    sqlx::raw_sql(
        "CREATE TABLE guarded_work (lock_key bigint);
         CREATE TABLE deferred_work (lock_key bigint);
         CREATE FUNCTION queue_deferred_work() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             INSERT INTO deferred_work VALUES (NEW.lock_key);
             RETURN NULL;
         END $$;
         CREATE FUNCTION block_deferred_work() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             -- Isolate the absolute fence from the relative watchdog in this test.
             SET LOCAL transaction_timeout = 0;
             PERFORM pg_advisory_xact_lock(NEW.lock_key);
             RETURN NULL;
         END $$;
         CREATE CONSTRAINT TRIGGER queue_deferred_work AFTER INSERT ON guarded_work
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION queue_deferred_work();
         CREATE CONSTRAINT TRIGGER block_deferred_work AFTER INSERT ON deferred_work
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION block_deferred_work()",
    )
    .execute(&mut *setup)
    .await
    .unwrap();
    setup.commit().await.unwrap();

    let key = key(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('search_path', $1, true)")
        .bind(&schema)
        .execute(&mut *writer)
        .await
        .unwrap();
    oteryn_db::mutation_budget(&mut writer).await.unwrap();
    sqlx::query("INSERT INTO guarded_work VALUES ($1)")
        .bind(key)
        .execute(&mut *writer)
        .await
        .unwrap();
    let (expiry, pid): (sqlx::types::time::OffsetDateTime, i32) =
        sqlx::query_as("SELECT clock_timestamp() + interval '1500 milliseconds', pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
    assert!(arm_commit(&mut writer, Some(expiry)).await.unwrap());
    let committing = tokio::spawn(writer.commit());
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND query='COMMIT' AND wait_event='advisory')")
                .bind(pid).fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("transitive deferred work must block inside COMMIT");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let expired: bool = sqlx::query_scalar("SELECT clock_timestamp() >= $1")
                .bind(expiry)
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
            if expired {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    blocker.rollback().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(2), committing)
        .await
        .unwrap()
        .unwrap()
        .expect_err("expired transaction must roll back");
    assert_eq!(
        error.as_database_error().unwrap().message(),
        "mutation commit deadline exceeded"
    );

    let mut check = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('search_path', $1, true)")
        .bind(&schema)
        .execute(&mut *check)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM guarded_work) + (SELECT count(*) FROM deferred_work)
              + (SELECT count(*) FROM mutation_commit_deadlines)",
    )
    .fetch_one(&mut *check)
    .await
    .unwrap();
    assert_eq!(count, 0);
    oteryn_db::mutation_budget(&mut check).await.unwrap();
    sqlx::query("INSERT INTO guarded_work VALUES ($1)")
        .bind(key)
        .execute(&mut *check)
        .await
        .unwrap();
    let expiry: sqlx::types::time::OffsetDateTime =
        sqlx::query_scalar("SELECT clock_timestamp() + interval '1500 milliseconds'")
            .fetch_one(&mut *check)
            .await
            .unwrap();
    assert!(arm_commit(&mut check, Some(expiry)).await.unwrap());
    check.commit().await.unwrap();

    // An authorized commit stays visible after the deadline has passed.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let expired: bool = sqlx::query_scalar("SELECT clock_timestamp() >= $1")
                .bind(expiry)
                .fetch_one(&pool)
                .await
                .unwrap();
            if expired {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    let mut check = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('search_path', $1, true)")
        .bind(&schema)
        .execute(&mut *check)
        .await
        .unwrap();
    let committed: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM guarded_work), (SELECT count(*) FROM deferred_work),
                (SELECT count(*) FROM mutation_commit_deadlines)",
    )
    .fetch_one(&mut *check)
    .await
    .unwrap();
    assert_eq!(
        committed,
        (1, 1, 0),
        "successful commits consume the deadline row"
    );

    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&mut *check)
        .await
        .unwrap();
    check.commit().await.unwrap();
}
