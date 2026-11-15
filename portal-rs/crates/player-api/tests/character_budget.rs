use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use oteryn_gameauth::tickets::{DenyOAuth, Settings, Tickets};
use oteryn_player_api::{Api, router};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn postgres_invalid_character_bearers_allocate_no_budgets()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPoolOptions::new().connect(&url).await?;
    let schema = format!("character_budget_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let path = path.clone();
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
    oteryn_db::migrate(&pool).await?;
    let app = router(Api {
        tickets: Tickets::new(
            pool.clone(),
            Settings {
                enabled: true,
                nonproduction: true,
            },
            Arc::new(DenyOAuth),
        ),
        projection: None,
    });
    for index in 0..128 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/game-auth/native-characters")
                    .header("Authorization", format!("Bearer fabricated-{index}"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_ticket_budgets")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    assert_eq!(
        count, 0,
        "unauthenticated bearers must not allocate durable budgets"
    );
    eprintln!(
        "postgres_invalid_character_bearers_allocate_no_budgets duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
