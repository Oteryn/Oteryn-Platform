use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn audit05_08_recovery_seed_rejects_fresh_public_key_under_another_kid() -> TestResult {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("seed_audit05_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search_path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |db, _| {
            let path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let directory = std::env::temp_dir().join(format!("seed-audit05-{}", Uuid::now_v7()));
    std::fs::create_dir(&directory)?;
    let public = URL_SAFE_NO_PAD.encode(
        ed25519_dalek::SigningKey::from_bytes(&[17; 32])
            .verifying_key()
            .to_bytes(),
    );
    let fresh = PublicTrustRegistry::new(
        pool.clone(),
        keys::Settings {
            enabled: true,
            key_purpose: "fresh_entry".into(),
            clock_uncertainty_seconds: 0,
        },
        directory.clone(),
    )?;
    fresh
        .publish(&keys::PublicKey {
            key_id: "fresh-owner".into(),
            public_key: public.clone(),
        })
        .await?;
    assert!(matches!(
        publish_recovery(&pool, &directory, "another-kid", &public).await,
        Err(SeedError::Conflict(_))
    ));
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM native_game_signing_trust_key_versions")
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 1);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
