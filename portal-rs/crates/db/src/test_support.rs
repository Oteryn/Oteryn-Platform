//! Test-only isolation for schemas sharing database-wide advisory lock keys.
//!
//! Acquire once at the outer test entry, before creating fixtures. Keep the guard
//! through fixture cleanup. Concurrent actors *within* a test remain concurrent.

use sqlx::{Connection, PgConnection};
use tokio::sync::{Mutex, MutexGuard};

static GLOBAL_LOCK_TEST: Mutex<()> = Mutex::const_new(());

/// Retains test ownership independently of fixture pools and mutation budgets.
#[derive(Debug)]
pub struct GlobalLockGuard {
    // Closing this dedicated session releases the test-only advisory lock, even
    // when a test unwinds. Never return a locked session to a connection pool.
    _connection: PgConnection,
    _local: MutexGuard<'static, ()>,
}

/// Serialize PostgreSQL tests using the global lock hierarchy on one database.
///
/// The local mutex bounds waiting connections to one per test binary; the session
/// lock also coordinates different crates/binaries. This key is test-only and
/// distinct from production gates (64002, 71010, and 22005160000). Waiting here
/// happens before the unchanged three-second mutation acquisition budget begins.
///
/// Returns `Ok(None)` when no test database is configured.
pub async fn global_lock_guard() -> Result<Option<GlobalLockGuard>, sqlx::Error> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(None);
    };
    let local = GLOBAL_LOCK_TEST.lock().await;
    let mut connection = PgConnection::connect(&url).await?;
    sqlx::query("SELECT pg_advisory_lock(684326110016002)")
        .execute(&mut connection)
        .await?;
    Ok(Some(GlobalLockGuard {
        _connection: connection,
        _local: local,
    }))
}

/// Persist a chargeback hold and its unresolved reversal for an existing payer.
pub async fn coin_chargeback_hold(
    pool: &sqlx::PgPool,
    account: sqlx::types::Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let order: sqlx::types::Uuid = sqlx::query_scalar("WITH quote AS (
        INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at)
        VALUES(uuidv7(),$1,uuidv7(),10,100,'EUR',2,'testing',now(),now())
        RETURNING id,package_id
    ) INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code,review_required)
      SELECT uuidv7(),$1,uuidv7(),id,package_id,10,100,'EUR',2,'testing',uuidv7(),'processing','review',true FROM quote RETURNING id")
        .bind(account).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome)
                 VALUES(uuidv7(),$1,$2,'chargeback','test',$1::text,10,0,'held')")
        .bind(order).bind(account).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES ($1,$2,'reversal_insufficient_balance')")
        .bind(account).bind(order).execute(&mut *tx).await?;
    tx.commit().await
}
