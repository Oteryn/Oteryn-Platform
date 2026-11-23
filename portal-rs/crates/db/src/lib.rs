//! PostgreSQL pool and migrations for the Oteryn portal.

use sqlx::{
    Connection, PgConnection, Postgres, Transaction,
    migrate::{Migrate, MigrateError, Migrator},
    pool::PoolConnection,
    postgres::{PgPool, PgPoolOptions},
};
use std::time::Duration;
use tokio::time::Instant;

/// Embedded migrations from `portal-rs/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Creates a lazily connecting pool, so the web process starts without a reachable database.
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .connect_lazy(database_url)
}

struct MigrationConnection<'a> {
    connection: &'a mut PoolConnection<Postgres>,
    unlocked: bool,
}

impl Drop for MigrationConnection<'_> {
    fn drop(&mut self) {
        if !self.unlocked {
            self.connection.close_on_drop();
        }
    }
}

/// Run database work within one acquisition and execution deadline, then
/// finish returning or closing the connection before releasing the caller's
/// admission permit. A timed-out connection is discarded rather than reused.
async fn with_connection_deadline<T, E>(
    pool: &PgPool,
    deadline: Instant,
    work: impl AsyncFnOnce(&mut sqlx::PgConnection) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<sqlx::Error>,
{
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use tokio::time::{timeout, timeout_at};
    check_deadline(deadline)?;
    let mut connection = timeout_at(deadline, pool.acquire())
        .await
        .map_err(|_| sqlx::Error::PoolTimedOut)??;
    let outcome = {
        let mut work = std::pin::pin!(work(&mut connection));
        // Timeout polls its inner future first. Check before resuming any SQL.
        timeout_at(
            deadline,
            poll_fn(|cx| match check_deadline(deadline) {
                Ok(()) => work.as_mut().poll(cx),
                Err(error) => Poll::Ready(Err(error.into())),
            }),
        )
        .await
    };
    let cleanup = Duration::from_millis(250);
    match outcome {
        Ok(result) if check_deadline(deadline).is_ok() => {
            // SQLx checks the connection and flushes any queued rollback.
            // Dropping this future on a network stall discards the socket.
            let _ = timeout(cleanup, connection.return_to_pool()).await;
            result
        }
        _ => {
            let _ = timeout(cleanup, connection.close()).await;
            Err(sqlx::Error::PoolTimedOut.into())
        }
    }
}

/// Check an absolute request deadline before starting another phase of work.
pub fn check_deadline(deadline: Instant) -> Result<(), sqlx::Error> {
    if Instant::now() >= deadline {
        return Err(sqlx::Error::PoolTimedOut);
    }
    Ok(())
}

/// Bound pooled SQL and its transaction-local timeouts by the original deadline.
pub async fn with_transaction_deadline<T, E>(
    pool: &PgPool,
    deadline: Instant,
    lock_limit: Duration,
    statement_limit: Duration,
    work: impl AsyncFnOnce(&mut Transaction<'_, Postgres>) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<sqlx::Error>,
{
    with_connection_deadline(pool, deadline, async |connection| {
        let mut tx = connection.begin().await?;
        check_deadline(deadline)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let millis = |duration: Duration| format!("{}ms", duration.as_millis().max(1));
        sqlx::query(
            "SELECT set_config('lock_timeout', $1, true), set_config('statement_timeout', $2, true),
             CASE WHEN current_setting('server_version_num')::integer >= 170000
             THEN set_config('transaction_timeout', $3, true) END",
        )
        .bind(millis(remaining.min(lock_limit)))
        .bind(millis(remaining.min(statement_limit)))
        .bind(millis(remaining))
        .execute(&mut *tx)
        .await?;
        check_deadline(deadline)?;
        let result = work(&mut tx).await?;
        check_deadline(deadline)?;
        tx.commit().await?;
        Ok(result)
    })
    .await
}

/// Applies pending migrations.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    let mut connection = pool.acquire().await?;
    // Closing on failure or cancellation releases the session-level migration lock.
    let mut guard = MigrationConnection {
        connection: &mut connection,
        unlocked: false,
    };
    let connection = &mut **guard.connection;
    // Match SQLx's PostgreSQL lock key and release each statement snapshot
    // between attempts.
    let mut retry = tokio::time::interval(std::time::Duration::from_millis(25));
    loop {
        retry.tick().await;
        if sqlx::query_scalar(
            "SELECT pg_try_advisory_lock(x'3d32ad9e'::int * crc32(convert_to(current_database(), 'UTF8')))",
        )
        .fetch_one(&mut *connection)
        .await?
        {
            break;
        }
    }
    let migrator = Migrator {
        migrations: MIGRATOR.migrations.clone(),
        locking: false,
        ..Migrator::DEFAULT
    };
    connection
        .ensure_migrations_table(&migrator.table_name)
        .await?;
    if let Some(version) = connection.dirty_version(&migrator.table_name).await? {
        return Err(MigrateError::Dirty(version));
    }
    let applied = connection
        .list_applied_migrations(&migrator.table_name)
        .await?;
    for (version, index, drop_index) in [
        (
            20261122160701,
            "accounts_operations_created_at",
            "DROP INDEX accounts_operations_created_at",
        ),
        (
            20261122160702,
            "security_events_operations_login_created_at",
            "DROP INDEX security_events_operations_login_created_at",
        ),
    ] {
        if applied.iter().any(|migration| migration.version == version) {
            continue;
        }
        let valid: Option<bool> = sqlx::query_scalar(
            "SELECT i.indisvalid FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid
             WHERE c.relname = $1 AND c.relnamespace = to_regnamespace(current_schema())",
        )
        .bind(index)
        .fetch_optional(&mut *connection)
        .await?;
        match valid {
            Some(false) => {
                sqlx::query(drop_index).execute(&mut *connection).await?;
            }
            Some(true) => {
                // The build finished before its migration receipt was written.
                // Record the original checksum without rebuilding the index.
                migrator
                    .run_direct(Some(version - 1), connection, false)
                    .await?;
                let migration = migrator
                    .iter()
                    .find(|migration| migration.version == version)
                    .ok_or(MigrateError::VersionMissing(version))?;
                connection.skip(&migrator.table_name, migration).await?;
            }
            None => {}
        }
    }
    migrator.run_direct(None, connection, false).await?;
    connection.unlock().await?;
    guard.unlocked = true;
    Ok(())
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

pub mod transaction;

/// Establish the shared three-second transaction-wide mutation lock deadline.
pub async fn mutation_budget(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('oteryn.mutation_deadline', coalesce(nullif(current_setting('oteryn.mutation_deadline',true),''),(extract(epoch FROM clock_timestamp())+3)::text),true)")
        .execute(&mut *connection).await?;
    sqlx::query("SELECT set_config('lock_timeout', greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint::text || 'ms',true), set_config('statement_timeout', greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint::text || 'ms',true)")
        .execute(connection).await?;
    Ok(())
}

/// Isolation for database-wide advisory locks in schema-based PostgreSQL tests.
#[cfg(feature = "test-support")]
pub mod test_support;

pub mod ingest_budget;

/// The caller holds the RBAC writer fence after locking the affected accounts.
/// Every protected membership being removed needs an eligible survivor.
pub async fn protected_role_survives(
    connection: &mut PgConnection,
    account: sqlx::types::Uuid,
    role: Option<sqlx::types::Uuid>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT NOT EXISTS (
            SELECT 1 FROM account_roles mine JOIN roles r ON r.id=mine.role_id
            WHERE mine.account_id=$1 AND ($2::uuid IS NULL OR r.id=$2)
              AND r.key IN ('owner','platform_admin')
              AND NOT EXISTS (
                SELECT 1 FROM account_roles other JOIN accounts a ON a.id=other.account_id
                WHERE other.role_id=r.id AND a.id<>$1
                  AND a.status='active' AND a.email_verified_at IS NOT NULL
                  AND NOT EXISTS (
                    SELECT 1 FROM account_terminations t
                    WHERE t.account_id=a.id AND t.cancelled_at IS NULL AND t.finalized_at IS NULL
                  )
              )
        )",
    )
    .bind(account)
    .bind(role)
    .fetch_one(connection)
    .await
}
