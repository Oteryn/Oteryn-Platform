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
#[allow(clippy::expect_used)]
pub async fn global_lock_guard() -> Option<GlobalLockGuard> {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").ok()?;
    let local = GLOBAL_LOCK_TEST.lock().await;
    let mut connection = PgConnection::connect(&url)
        .await
        .expect("connect PostgreSQL test isolation session");
    sqlx::query("SELECT pg_advisory_lock(684326110016002)")
        .execute(&mut connection)
        .await
        .expect("acquire PostgreSQL test isolation guard");
    Some(GlobalLockGuard {
        _connection: connection,
        _local: local,
    })
}
