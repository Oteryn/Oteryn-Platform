use super::*;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn owner_history_and_appeal_submission() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("sanction_history_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |db, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search_path)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    for id in [owner, other] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'fixture','active','en')")
            .bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
    }
    let sanction: Uuid = sqlx::query_scalar(
        "INSERT INTO sanctions(account_id,category,status,public_reason,moderator_notes,effective_at,expires_at,created_by)
         VALUES ($1,'restriction','active','Public reason','Staff note',now(),now()+interval '1 day',$2) RETURNING id",
    )
    .bind(owner).bind(other).fetch_one(&pool).await?;
    let pending: Uuid = sqlx::query_scalar(
        "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,created_by)
         VALUES ($1,'restriction','pending','Second decision',now(),$2) RETURNING id",
    )
    .bind(owner)
    .bind(other)
    .fetch_one(&pool)
    .await?;

    let items = history(&pool, owner, 1).await?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, sanction);
    assert!(history(&pool, other, 1).await?.is_empty());
    assert!(matches!(
        history(&pool, owner, 0).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        detail(&pool, owner, pending).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        update(&pool, other, sanction, 1, PlayerAction::Acknowledge).await,
        Err(Error::NotFound)
    ));

    let submit = |message: &str| PlayerAction::Submit {
        message: message.to_owned(),
    };
    assert!(matches!(
        update(&pool, owner, sanction, 1, submit("   ")).await,
        Err(Error::Invalid)
    ));
    assert_eq!(detail(&pool, owner, sanction).await?.version, 1);
    update(&pool, owner, sanction, 1, PlayerAction::Acknowledge).await?;
    assert!(matches!(
        update(&pool, owner, sanction, 1, submit("Please review")).await,
        Err(Error::Conflict)
    ));
    update(&pool, owner, sanction, 2, submit("Please review")).await?;
    let item = detail(&pool, owner, sanction).await?;
    assert!(item.acknowledged);
    assert_eq!(item.appeal_status, "requested");
    assert_eq!(item.message.as_deref(), Some("Please review"));
    assert!(matches!(
        update(&pool, owner, sanction, 3, submit("Please review again")).await,
        Err(Error::Conflict)
    ));

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
