use super::*;

fn pool() -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")
}
#[tokio::test]
async fn disabled_anonymous_and_bad_locale() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        show(&pool()?, None, "en", false, Settings::default())
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        show(&pool()?, None, "en", true, Settings::default())
            .await
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        show(&pool()?, None, "de", true, Settings::default())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        show(&pool()?, Some(Uuid::nil()), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    Ok(())
}
#[test]
fn translations_and_profile_segments() {
    for lang in ["en", "pl"] {
        assert!(!text(lang, "overview").is_empty());
    }
    assert_eq!(parse_limits("8", "16"), Some((8, 16)));
    assert_eq!(parse_limits("0", "16"), None);
    assert_eq!(parse_limits("8", "7"), None);
    assert_eq!(parse_limits("x", "16"), None);
    assert_eq!(segment("A /?<&"), "A%20%2F%3F%3C%26");
}
#[tokio::test]
async fn postgres_overview_success_and_denial() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL overview test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
    let schema = format!("overview_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105155256_t03_accounts.sql"
    ))
    .execute(&pool)
    .await?;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')").bind(id).bind(format!("overview-{id}@example.invalid")).execute(&pool).await?;
    let response = show(&pool, Some(id), "en", true, Settings::default()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("Account overview"));
    assert!(html.contains("href=\"/en/shop/history\""));
    assert!(!html.contains("/panel/balance"));
    assert!(html.contains("Unavailable"));
    assert!(!html.contains("test-hash"));
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105145653_t18_account_characters.sql"
    ))
    .execute(&pool)
    .await?;
    let authority = "oteryn:character-authority:primary";
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority=$1,complete_through=EXTRACT(EPOCH FROM now())::bigint,observed_at=EXTRACT(EPOCH FROM now())::bigint WHERE id=1").bind(authority).execute(&pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'1','1',0,$3)").bind(id).bind(authority).bind(vec![0u8;32]).execute(&pool).await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(id).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&pool).await?;
    let settings = Settings {
        enabled: true,
        environment: oteryn_core::Environment::Testing,
        source_authority: authority.into(),
        ..Settings::default()
    };
    let response = show(&pool, Some(id), "en", true, settings.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(html.contains("/characters/Aldric"));
    assert!(html.contains("Current game connection"));
    sqlx::query("UPDATE account_character_feed SET complete_through=0 WHERE id=1")
        .execute(&pool)
        .await?;
    let response = show(&pool, Some(id), "pl", true, settings).await;
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(!html.contains("Aldric"));
    assert!(html.contains("nieaktualne"));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        show(&pool, Some(id), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        show(&pool, Some(id), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
