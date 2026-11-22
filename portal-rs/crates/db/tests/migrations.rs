//! Guards the embedded migration set. sqlx keys applied migrations by their
//! numeric version, so two files with the same version prefix make
//! `migrate run` fail on a real database even though everything compiles.

use std::collections::BTreeMap;

#[test]
fn migration_versions_are_unique() {
    let mut seen: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for migration in oteryn_db::MIGRATOR.iter() {
        // A reversible pair shares one version; only up migrations count.
        if migration.migration_type.is_down_migration() {
            continue;
        }
        seen.entry(migration.version)
            .or_default()
            .push(migration.description.to_string());
    }
    let duplicates: Vec<_> = seen.iter().filter(|(_, names)| names.len() > 1).collect();
    assert!(
        duplicates.is_empty(),
        "duplicate migration versions in portal-rs/migrations: {duplicates:?}"
    );
}

#[test]
fn migrations_are_embedded() {
    assert!(oteryn_db::MIGRATOR.iter().next().is_some());
}

/// Versions are 14-digit UTC timestamps (`YYYYMMDDhhmmss`). A shorter or
/// longer prefix still compiles but sorts out of order, so the migration
/// would run before or after the schema it was written against.
#[test]
fn migration_versions_are_fourteen_digit_timestamps() {
    let malformed: Vec<_> = oteryn_db::MIGRATOR
        .iter()
        .filter(|migration| !(10_000_000_000_000..100_000_000_000_000).contains(&migration.version))
        .map(|migration| (migration.version, migration.description.to_string()))
        .collect();
    assert!(
        malformed.is_empty(),
        "migration versions must be 14-digit timestamps: {malformed:?}"
    );
}

#[tokio::test]
async fn reward_due_index_migration_leaves_reads_available()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_db::MIGRATOR;
    use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
    use tokio::time::{Duration, timeout};

    let root = PgPool::connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?).await?;
    let schema: String =
        sqlx::query_scalar("SELECT 'reward_index_' || replace(uuidv7()::text,'-','')")
            .fetch_one(&root)
            .await?;
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |c, _| {
            let path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
        .await?;

    MIGRATOR.run_to(20261118140000, &pool).await?;
    let index_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('referral_campaign_premium_due') IS NOT NULL")
            .fetch_one(&pool)
            .await?;
    assert!(
        !index_exists,
        "the index build belongs in its own migration"
    );

    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(id,email,password_hash,status,locale)
        VALUES(uuidv7(),'reward-index@example.test','test-only','active','en') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let mut writer = pool.begin().await?;
    sqlx::query(
        "INSERT INTO referral_campaign_rewards(operation_key,account_id,kind,amount)
                 VALUES('before',$1,'premium',1)",
    )
    .bind(account)
    .execute(&mut *writer)
    .await?;

    let migration_pool = pool.clone();
    let migration = tokio::spawn(async move { oteryn_db::migrate(&migration_pool).await });
    // Hold a writer open so the index build queues its table lock. A queued
    // ACCESS EXCLUSIVE request would also stall readers arriving behind it.
    timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM pg_locks
                    WHERE relation='referral_campaign_rewards'::regclass
                      AND mode='ShareLock' AND NOT granted
                )",
            )
            .fetch_one(&pool)
            .await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            assert!(
                !migration.is_finished(),
                "index build did not wait for the writer"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;

    let mut reader = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='1s'")
        .execute(&mut *reader)
        .await?;
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM referral_campaign_rewards")
        .fetch_one(&mut *reader)
        .await?;
    assert_eq!(visible, 0);
    reader.commit().await?;
    writer.commit().await?;
    timeout(Duration::from_secs(10), migration).await???;

    let valid: bool = sqlx::query_scalar(
        "SELECT indisvalid FROM pg_index
        WHERE indexrelid='referral_campaign_premium_due'::regclass",
    )
    .fetch_one(&pool)
    .await?;
    assert!(valid);
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}

#[tokio::test]
async fn staff_invite_upgrade_preserves_the_outbox_claim_fence()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_db::{MIGRATOR, outbox};
    use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions, types::Uuid};

    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("invite_claim_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;

    MIGRATOR.run_to(20261122150000, &pool).await?;
    let invite: Uuid = sqlx::query_scalar(
        "WITH issuer AS (
            INSERT INTO accounts(id,email,password_hash,status,locale)
            VALUES(uuidv7(),'invite-issuer@example.invalid','test-only','active','en') RETURNING id
         )
         INSERT INTO staff_invites(id,email,role_ids,invited_by,token_hash,locale,created_at,expires_at)
         SELECT uuidv7(),'invitee@example.invalid',ARRAY[r.id],issuer.id,
                sha256('test-only'::bytea),'en',now(),now()+interval '48 hours'
         FROM issuer CROSS JOIN roles r WHERE r.key='platform_admin' RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let mut tx = pool.begin().await?;
    let job = outbox::enqueue(
        &mut tx,
        "email.staff-invite",
        "{}",
        &invite.to_string(),
        sqlx::types::time::OffsetDateTime::now_utc(),
        5,
    )
    .await?;
    tx.commit().await?;
    let old = outbox::claim(&pool, 64, 3600)
        .await?
        .into_iter()
        .find(|claim| claim.id == job)
        .ok_or("outbox did not return the original invite claim")?;
    assert_eq!(old.attempts, 1);

    MIGRATOR.run_to(20261122150100, &pool).await?;
    let rescheduled: (String, i32, bool, bool) = sqlx::query_as(
        "SELECT o.state,o.attempts,o.leased_until IS NULL,o.run_after=i.expires_at
         FROM outbox o JOIN staff_invites i ON o.idempotency_key=i.id::text WHERE o.id=$1",
    )
    .bind(job)
    .fetch_one(&pool)
    .await?;
    assert_eq!(rescheduled, ("pending".into(), old.attempts, true, true));

    // Model the queue wake-up performed in the invitation approval transaction.
    sqlx::query("UPDATE outbox SET run_after=clock_timestamp() WHERE id=$1")
        .bind(job)
        .execute(&pool)
        .await?;
    let current = outbox::claim(&pool, 64, 3600)
        .await?
        .into_iter()
        .find(|claim| claim.id == job)
        .ok_or("outbox did not return the approved invite claim")?;
    assert_eq!(current.attempts, old.attempts + 1);
    assert!(!outbox::renew(&pool, &old, 3600).await?);
    assert!(!outbox::finish(&pool, &old, None, false).await?);
    assert!(outbox::finish(&pool, &current, None, false).await?);

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
