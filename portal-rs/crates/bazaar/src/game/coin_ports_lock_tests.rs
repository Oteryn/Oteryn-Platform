use super::*;
use crate::game::{AuthorityConfig, PINNED_CONTRACT_SHA256, RecoveryFence};
use sqlx::{PgPool, postgres::PgPoolOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn postgres_bazaar_locks_complete_identity_set_before_wallets() -> TestResult {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await;
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED Bazaar lock order: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("bazaar_lock_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |db, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(&mut *db)
                    .await?;
                sqlx::query("SET statement_timeout='5s'")
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=3")
        .execute(&pool)
        .await?;
    let authority = Authority {
        config: AuthorityConfig {
            enabled: true,
            contract_sha256: Some(PINNED_CONTRACT_SHA256),
            v2_receipts_accepted: true,
            eligibility_and_escrow_accepted: true,
            inbox_and_item_classes_accepted: true,
            gold_fee_and_headroom_accepted: true,
            common_recovery_scope_qualified: true,
            signing_and_retention_accepted: true,
            receipt_authority: "test:receipt".into(),
            projection_authority: "test:projection".into(),
            snapshot_keys: vec![(
                "test-key".into(),
                ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key(),
            )],
            ..Default::default()
        },
        fence: RecoveryFence {
            projection_epoch: 3,
            recovery_generation: 3,
        },
    };
    let mut ids = [Uuid::now_v7(), Uuid::now_v7()];
    ids.sort();
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test','active','en')").bind(id).bind(format!("lock-{index}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1)")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    let mut identity = pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *identity)
        .await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(ids[1])
        .execute(&mut *identity)
        .await?;
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move {
        let mut tx = worker_pool.begin().await?;
        CoinReservations { authority }
            .lock_accounts(&mut tx, &ids)
            .await?;
        tx.commit().await?;
        Ok::<_, Error>(())
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&pool)
            .await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    let mut observer = pool.begin().await?;
    let wallet =
        sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE NOWAIT")
            .bind(ids[0])
            .execute(&mut *observer)
            .await;
    observer.rollback().await?;
    identity.commit().await?;
    worker.await??;
    assert!(
        wallet.is_ok(),
        "Bazaar locked a wallet before the complete identity set"
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    eprintln!(
        "PG postgres_bazaar_locks_complete_identity_set_before_wallets {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
