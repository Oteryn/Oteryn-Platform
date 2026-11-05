use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn package_configuration_fails_closed() {
    assert!(!valid_packages(&[]));
    let p = Package {
        key: "fixture".into(),
        coins: 100,
        minor_units: 250,
        currency: "PLN".into(),
    };
    assert!(valid_packages(std::slice::from_ref(&p)));
    assert!(!valid_packages(&[p.clone(), p.clone()]));
    assert!(!valid_packages(&[Package {
        coins: 0,
        ..p.clone()
    }]));
    assert!(!valid_packages(&[Package {
        currency: "pln".into(),
        ..p
    }]));
}
#[tokio::test]
async fn postgres_payment_intent_replay_conflict_and_interrupted_claim() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED shop payment PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("shop_payments_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'fixture@example.invalid',now(),'unused-test-hash','active','en')").bind(account).execute(&pool).await?;
    let topups = PaymentTopUps {
        pool: pool.clone(),
        payments: Payments::new(
            pool.clone(),
            TestProvider::new(&[5; 32])?,
            Accounts(pool.clone()),
            PaymentFlags { enabled: true },
        ),
        packages: vec![
            Package {
                key: "fixture".into(),
                coins: 100,
                minor_units: 250,
                currency: "PLN".into(),
            },
            Package {
                key: "other".into(),
                coins: 200,
                minor_units: 500,
                currency: "PLN".into(),
            },
        ],
    };
    let operation = Uuid::now_v7();
    let first = topups.create(account, operation, "fixture".into()).await;
    assert!(first.is_ok());
    assert_eq!(
        first,
        topups.create(account, operation, "fixture".into()).await
    );
    assert_eq!(
        topups.create(account, operation, "other".into()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        topups.create(account, Uuid::nil(), "fixture".into()).await,
        Err(Error::Invalid)
    );
    assert_eq!(
        topups
            .create(account, Uuid::now_v7(), "unknown".into())
            .await,
        Err(Error::Invalid)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_entries")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    let interrupted = Uuid::now_v7();
    sqlx::query("INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency) VALUES($1,$2,'fixture',100,250,'PLN')").bind(interrupted).bind(account).execute(&pool).await?;
    assert_eq!(
        topups.create(account, interrupted, "fixture".into()).await,
        Err(Error::Unavailable)
    );
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert_eq!(
        topups
            .create(account, Uuid::now_v7(), "fixture".into())
            .await,
        Err(Error::Unavailable)
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payment_intents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
