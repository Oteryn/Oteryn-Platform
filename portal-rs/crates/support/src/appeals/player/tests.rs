use super::*;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn postgres_confirmed_sanction_history_and_player_actions()
-> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("player_appeal_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path', $1, false)")
                    .bind(path)
                    .execute(connection)
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
    let active: Uuid = sqlx::query_scalar(
        "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by)
         VALUES ($1,'restriction','active','Timed decision',now()-interval '1 hour',now()+interval '1 hour',$2)
         RETURNING id",
    ).bind(owner).bind(other).fetch_one(&pool).await?;
    let pending: Uuid = sqlx::query_scalar(
        "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,created_by)
         VALUES ($1,'restriction','pending','Awaiting approval',now(),$2) RETURNING id",
    )
    .bind(owner)
    .bind(other)
    .fetch_one(&pool)
    .await?;
    let expired: Uuid = sqlx::query_scalar(
        "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by,acknowledged_at)
         VALUES ($1,'restriction','active','Earlier decision',now()-interval '2 hours',now()-interval '1 hour',$2,now())
         RETURNING id",
    ).bind(owner).bind(other).fetch_one(&pool).await?;

    let rows = history(&pool, owner, 1).await?;
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [expired, active]
    );
    assert_eq!(rows[0].state, "expired");
    assert!(rows[0].acknowledged_at.is_some());
    assert_eq!(rows[1].public_reason, "Timed decision");
    assert_eq!(rows[1].state, "active");
    assert!(rows[1].can_appeal());
    assert!(history(&pool, other, 1).await?.is_empty());
    assert!(matches!(
        history(&pool, owner, 0).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        detail(&pool, other, active).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        detail(&pool, owner, pending).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        detail(&pool, owner, Uuid::nil()).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        change(&pool, other, active, 1, PlayerAction::Acknowledge).await,
        Err(Error::NotFound)
    ));

    for message in ["  ".into(), "x".repeat(4001)] {
        assert!(matches!(
            change(&pool, owner, active, 1, PlayerAction::Appeal(&message)).await,
            Err(Error::Invalid)
        ));
    }
    let initialized: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM support_appeal_workflows WHERE sanction_id=$1)",
    )
    .bind(active)
    .fetch_one(&pool)
    .await?;
    assert!(!initialized);

    change(&pool, owner, active, 1, PlayerAction::Acknowledge).await?;
    let acknowledged = detail(&pool, owner, active).await?;
    assert!(acknowledged.acknowledged_at.is_some());
    assert_eq!(acknowledged.version, 2);
    assert!(matches!(
        change(&pool, owner, active, 1, PlayerAction::Appeal("Outdated")).await,
        Err(Error::Conflict)
    ));
    change(
        &pool,
        owner,
        active,
        2,
        PlayerAction::Appeal("  Please review  "),
    )
    .await?;
    let filed = detail(&pool, owner, active).await?;
    assert_eq!(filed.acknowledged_at, acknowledged.acknowledged_at);
    assert_eq!(filed.appeal_status, "requested");
    assert_eq!(filed.message.as_deref(), Some("Please review"));
    assert!(!filed.can_appeal());
    assert!(matches!(
        change(
            &pool,
            owner,
            active,
            filed.version,
            PlayerAction::Appeal("Another reason")
        )
        .await,
        Err(Error::Conflict)
    ));

    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
