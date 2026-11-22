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

/// English and Polish bodies plus explicit approval for a fixture document.
pub const APPROVED_SIGNUP_EDITION: (&str, &str, bool) = (
    "Accepted English fixture edition.",
    "Zatwierdzony dokument testowy.",
    true,
);

/// Seed published bilingual, approved legal editions in an isolated registration fixture.
pub async fn seed_final_signup_editions(
    pool: &sqlx::PgPool,
    version: &str,
) -> Result<(), sqlx::Error> {
    seed_signup_editions(
        pool,
        version,
        APPROVED_SIGNUP_EDITION,
        APPROVED_SIGNUP_EDITION,
    )
    .await
}

/// Seed matching published rows and immutable snapshots. Each document can be
/// backed by a legacy snapshot that lacks explicit approval.
pub async fn seed_signup_editions(
    pool: &sqlx::PgPool,
    version: &str,
    terms: (&str, &str, bool),
    privacy: (&str, &str, bool),
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "WITH texts(slug,english,polish,approved) AS (
             VALUES ('terms-of-service',$2::text,$3::text,$4::boolean),
                    ('privacy-policy',$5::text,$6::text,$7::boolean)
         ), documents AS (
             INSERT INTO pages(id,slug)
             SELECT uuidv7(),slug FROM texts RETURNING id,slug
         ), sources AS (
             INSERT INTO editorial_pages
                 (page_id,locale,title,body,published_at,legal_version,legal_effective_date)
             SELECT id,'en',slug,english,now(),$1,'2026-11-22'
             FROM documents JOIN texts USING (slug) RETURNING *
         ), translations AS (
             INSERT INTO editorial_pages
                 (page_id,locale,title,body,published_at,legal_version,legal_effective_date,source_revision)
             SELECT page_id,'pl',title,polish,now(),legal_version,
                    legal_effective_date,revision
             FROM sources JOIN texts ON texts.slug=sources.title RETURNING *
         )
         INSERT INTO page_legal_versions(page_id,locale,version,content)
         SELECT page_id,locale,legal_version,jsonb_build_object(
             'title',title,'body',body,'effective',legal_effective_date::text)
             || CASE WHEN approved THEN '{\"final_approved\":true}'::jsonb ELSE '{}'::jsonb END
         FROM (SELECT * FROM sources UNION ALL SELECT * FROM translations) editions
         JOIN texts ON texts.slug=editions.title",
    )
    .bind(version)
    .bind(terms.0)
    .bind(terms.1)
    .bind(terms.2)
    .bind(privacy.0)
    .bind(privacy.1)
    .bind(privacy.2)
    .execute(pool)
    .await?;
    Ok(())
}
