use sqlx::{PgPool, postgres::PgPoolOptions};
pub async fn database(url: &str) -> Result<(PgPool, PgPool, String), sqlx::Error> {
    let admin = PgPool::connect(url).await?;
    let schema: String = sqlx::query_scalar("SELECT 'privacy_' || replace(uuidv7()::text,'-','')")
        .fetch_one(&admin)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(4)
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
        .connect(url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    Ok((pool, admin, schema))
}
pub async fn close(pool: PgPool, admin: PgPool, schema: String) -> Result<(), sqlx::Error> {
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
