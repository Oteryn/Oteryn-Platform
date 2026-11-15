use sqlx::PgPool;
use uuid::Uuid;
pub(super) type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(super) struct Database {
    pub db: PgPool,
    root: PgPool,
    schema: String,
}
impl Database {
    pub async fn new() -> Result<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPool::connect(&url).await?;
        let schema = format!("audit06_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let db = PgPool::connect_with(
            root.connect_options()
                .as_ref()
                .clone()
                .options([("search_path", schema.as_str())]),
        )
        .await?;
        oteryn_db::migrate(&db).await?;
        Ok(Self { db, root, schema })
    }
    pub async fn close(self) -> Result {
        self.db.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}
