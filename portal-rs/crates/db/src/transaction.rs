//! Writer transactions bounded by the shared mutation budget.
//!
//! `run_transaction` opens the transaction, starts the three-second mutation
//! budget before the first statement of the closure, and owns commit and
//! rollback: `Ok` commits, `Err` rolls back. A path that must persist a refusal
//! returns it inside `Ok` and maps it after the commit.
//!
//! `lock_timeout` and `statement_timeout` are re-armed to the time remaining on
//! every `BudgetTx::connection` call and are never zero. On PostgreSQL 17 and
//! later `transaction_timeout` also bounds the whole transaction, including
//! time the client spends between statements.

use sqlx::{PgConnection, PgPool, Postgres, Transaction};

/// Grace beyond the mutation deadline before PostgreSQL ends the transaction.
const TRANSACTION_GRACE_MS: i64 = 500;

/// A transaction whose waits are bounded by the shared mutation budget.
///
/// There is no public commit or rollback; `run_transaction` decides.
pub struct BudgetTx {
    tx: Transaction<'static, Postgres>,
}

impl std::fmt::Debug for BudgetTx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BudgetTx")
    }
}

impl BudgetTx {
    /// Re-arms the lock and statement timeouts to the time remaining and lends
    /// the connection for the statements that follow.
    pub async fn connection(&mut self) -> Result<&mut PgConnection, sqlx::Error> {
        crate::mutation_budget(&mut self.tx).await?;
        Ok(&mut self.tx)
    }
}

/// Runs `work` in one budgeted transaction: commit on `Ok`, rollback on `Err`.
pub async fn run_transaction<T, E, F>(pool: &PgPool, work: F) -> Result<T, E>
where
    F: AsyncFnOnce(&mut BudgetTx) -> Result<T, E>,
    E: From<sqlx::Error>,
{
    let mut tx = pool.begin().await?;
    crate::mutation_budget(&mut tx).await?;
    sqlx::query(
        "SELECT CASE WHEN current_setting('server_version_num')::integer >= 170000 \
         THEN set_config('transaction_timeout', \
         (greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint + $1)::text || 'ms', true) END",
    )
    .bind(TRANSACTION_GRACE_MS)
    .execute(&mut *tx)
    .await?;
    let mut budget = BudgetTx { tx };
    match work(&mut budget).await {
        Ok(value) => {
            budget.tx.commit().await?;
            Ok(value)
        }
        Err(error) => {
            // A failed rollback leaves the connection to the pool's own cleanup;
            // the caller's error is the one that matters.
            let _ = budget.tx.rollback().await;
            Err(error)
        }
    }
}

/// Arm the timer and queue an absolute deadline check as the transaction's last write.
/// The timer bounds deferred waits. The guard checks the database clock after deferred
/// work, even if scheduling delayed timer arming or delivery.
pub async fn arm_commit(
    connection: &mut PgConnection,
    authorization_expiry: Option<sqlx::types::time::OffsetDateTime>,
) -> Result<bool, sqlx::Error> {
    sqlx::query("SET CONSTRAINTS mutation_commit_deadline DEFERRED")
        .execute(&mut *connection)
        .await?;
    // PostgreSQL does not re-arm an already active transaction timeout.
    sqlx::query("SET LOCAL transaction_timeout = 0")
        .execute(&mut *connection)
        .await?;
    let armed = sqlx::query(
        "WITH deadline AS MATERIALIZED (
            SELECT least($1::timestamptz,
                to_timestamp(current_setting('oteryn.mutation_deadline')::double precision)) AS expires_at
         ), remaining AS MATERIALIZED (
            SELECT expires_at, floor(extract(epoch FROM (expires_at - clock_timestamp()))
                * 1000)::bigint AS ms FROM deadline
         ), armed AS MATERIALIZED (
            SELECT expires_at, set_config('transaction_timeout', ms::text || 'ms', true)
            FROM remaining WHERE ms > 0
         )
         INSERT INTO mutation_commit_deadlines (expires_at)
         SELECT expires_at FROM armed",
    )
    .bind(authorization_expiry)
    .execute(connection)
    .await?;
    Ok(armed.rows_affected() == 1)
}
