//! Scheduling progress must survive a refused or rolled-back financial attempt.
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn cursor(pool: &PgPool, queue: &str) -> Result<Uuid, sqlx::Error> {
    Ok(
        sqlx::query_scalar("SELECT cursor_id FROM bazaar_work_cursors WHERE queue=$1")
            .bind(queue)
            .fetch_optional(pool)
            .await?
            .unwrap_or(Uuid::nil()),
    )
}
pub(crate) async fn advance(pool: &PgPool, queue: &str, cursor: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO bazaar_work_cursors(queue,cursor_id) VALUES($1,$2) ON CONFLICT(queue) DO UPDATE SET cursor_id=EXCLUDED.cursor_id")
        .bind(queue).bind(cursor).execute(pool).await?;
    Ok(())
}
