//! PostgreSQL pool and migrations for the Oteryn portal.

use sqlx::{
    PgConnection,
    postgres::{PgPool, PgPoolOptions},
};

/// Embedded migrations from `portal-rs/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Creates a lazily connecting pool, so the web process starts without a reachable database.
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .connect_lazy(database_url)
}

/// Applies pending migrations.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Readiness probe: one round trip to PostgreSQL.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_embedded_in_order() {
        let versions: Vec<i64> = MIGRATOR.iter().map(|m| m.version).collect();
        let mut sorted = versions.clone();
        sorted.sort_unstable();
        assert!(!versions.is_empty());
        assert_eq!(versions, sorted);
    }
}

pub mod outbox;

pub mod retry;

/// Establish the shared three-second transaction-wide mutation lock deadline.
pub async fn mutation_budget(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("WITH deadline AS MATERIALIZED (SELECT set_config('oteryn.mutation_deadline',coalesce(nullif(current_setting('oteryn.mutation_deadline',true),''),(extract(epoch FROM clock_timestamp())+3)::text),true) AS value), budget AS (SELECT greatest(1,ceil((value::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint AS remaining FROM deadline) SELECT set_config('lock_timeout',least(remaining,coalesce(nullif((extract(epoch FROM current_setting('lock_timeout')::interval)*1000)::bigint,0),remaining))::text || 'ms',true), set_config('statement_timeout',least(remaining,coalesce(nullif((extract(epoch FROM current_setting('statement_timeout')::interval)*1000)::bigint,0),remaining))::text || 'ms',true) FROM budget")
        .execute(connection).await?;
    Ok(())
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[tokio::test]
    async fn postgres_drc_budget_retains_deadline_and_shorter_timeouts()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool = lazy_pool(&std::env::var("PORTAL_TEST_DATABASE_URL")?)?;
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout='100ms'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='2s'")
            .execute(&mut *tx)
            .await?;
        mutation_budget(&mut tx).await?;
        let first: String =
            sqlx::query_scalar("SELECT current_setting('oteryn.mutation_deadline')")
                .fetch_one(&mut *tx)
                .await?;
        mutation_budget(&mut tx).await?;
        let second: String =
            sqlx::query_scalar("SELECT current_setting('oteryn.mutation_deadline')")
                .fetch_one(&mut *tx)
                .await?;
        assert_eq!(first, second);
        let lock: i64 =
            sqlx::query_scalar("SELECT setting::bigint FROM pg_settings WHERE name='lock_timeout'")
                .fetch_one(&mut *tx)
                .await?;
        let statement: i64 = sqlx::query_scalar(
            "SELECT setting::bigint FROM pg_settings WHERE name='statement_timeout'",
        )
        .fetch_one(&mut *tx)
        .await?;
        assert!(
            lock <= 100,
            "budget increased a shorter lock timeout: {lock}"
        );
        assert!(
            statement <= 2000,
            "budget increased a shorter statement timeout: {statement}"
        );
        tx.rollback().await?;
        let mut tx = pool.begin().await?;
        let deadline: Option<String> = sqlx::query_scalar(
            "SELECT nullif(current_setting('oteryn.mutation_deadline',true),'')",
        )
        .fetch_one(&mut *tx)
        .await?;
        assert!(deadline.is_none());
        tx.rollback().await?;
        pool.close().await;
        Ok(())
    }
}
