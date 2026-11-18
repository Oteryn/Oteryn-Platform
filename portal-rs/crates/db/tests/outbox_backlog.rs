use oteryn_db::outbox;
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

// Count tuples visited by the production transaction, independent of machine speed.
async fn visits(pool: &PgPool) -> Result<i64> {
    sqlx::query("SELECT pg_stat_force_next_flush()")
        .execute(pool)
        .await?;
    let count = sqlx::query_scalar(
        "SELECT seq_tup_read + idx_tup_fetch FROM pg_stat_all_tables
         WHERE relid = 'outbox'::regclass",
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}

async fn backlog(state: &str) -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED outbox backlog test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let schema = format!("backlog_{}", sqlx::types::Uuid::now_v7().simple());
    let options = url
        .parse::<sqlx::postgres::PgConnectOptions>()?
        .options([("search_path", schema.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    oteryn_db::migrate(&pool).await?;
    sqlx::query(
        "INSERT INTO outbox (kind, payload, idempotency_key, state)
         SELECT 'task', '{}', 'schedule:' || i, $1 FROM generate_series(1, 20000) i",
    )
    .bind(state)
    .execute(&pool)
    .await?;
    sqlx::query("ANALYZE outbox").execute(&pool).await?;
    let before = visits(&pool).await?;
    let bound = if state == "pending" {
        assert_eq!(outbox::claim(&pool, 16, 60).await?.len(), 16);
        160
    } else {
        outbox::schedule(&pool, "task", 1).await?;
        4000
    };
    let read = visits(&pool).await? - before;
    if state == "done" {
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox WHERE state='done'")
            .fetch_one(&pool)
            .await?;
        assert_eq!(remaining, 19000);
    }
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    assert!(
        read <= bound,
        "visited {read} tuples for a single batch (limit {bound})"
    );
    Ok(())
}

#[tokio::test]
async fn claim_bounds_pending_backlog_reads() -> Result {
    backlog("pending").await
}

#[tokio::test]
async fn prune_bounds_legacy_backlog_reads() -> Result {
    backlog("done").await
}
