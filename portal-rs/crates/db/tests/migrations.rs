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
async fn reward_due_index_build_allows_reads_and_award_inserts()
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
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
        .await?;

    {
        let mut connection = oteryn_db::migration_connection(&pool).await?;
        MIGRATOR.run_to(20261118140000, &mut *connection).await?;
    }
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
    timeout(Duration::from_secs(10), async {
        loop {
            let building: bool = sqlx::query_scalar("SELECT EXISTS (
                SELECT 1 FROM pg_stat_progress_create_index
                WHERE relid='referral_campaign_rewards'::regclass
                  AND command='CREATE INDEX CONCURRENTLY' AND phase='waiting for writers before build'
            )").fetch_one(&pool).await?;
            if building {
                return Ok::<_, sqlx::Error>(());
            }
            assert!(!migration.is_finished(), "migration exited before entering the concurrent build");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await??;

    let migration_locks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_locks l JOIN pg_stat_progress_create_index p USING(pid)
         WHERE p.relid='referral_campaign_rewards'::regclass AND l.locktype='advisory' AND l.granted",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        migration_locks, 1,
        "the prelock must use SQLx's advisory key"
    );

    let contender_pool = pool.clone();
    let contender = tokio::spawn(async move { oteryn_db::migrate(&contender_pool).await });
    timeout(Duration::from_secs(10), async {
        loop {
            let attempting_lock: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                SELECT 1 FROM pg_stat_activity WHERE application_name=$1
                  AND query LIKE 'SELECT pg%advisory_lock(%'
            )",
            )
            .bind(&schema)
            .fetch_one(&pool)
            .await?;
            if attempting_lock {
                return Ok::<_, sqlx::Error>(());
            }
            assert!(
                !contender.is_finished(),
                "second migrator exited before waiting for the first"
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
    sqlx::query(
        "INSERT INTO referral_campaign_rewards(operation_key,account_id,kind,amount)
                 VALUES('during',$1,'premium',1)",
    )
    .bind(account)
    .execute(&mut *reader)
    .await?;
    reader.commit().await?;
    writer.commit().await?;
    timeout(Duration::from_secs(10), async {
        migration.await??;
        contender.await??;
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;

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
