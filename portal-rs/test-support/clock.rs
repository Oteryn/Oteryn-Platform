//! SQL clock for disposable test schemas only. Pools must explicitly place
//! pg_catalog last in search_path so production SQL resolves these functions.
//! Install after migrations; no shared database clock or production code changes.
pub async fn install(
    pool: &sqlx::PgPool,
    instant: time::OffsetDateTime,
) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "CREATE TABLE audit_test_clock (instant timestamptz NOT NULL);
         CREATE FUNCTION clock_timestamp() RETURNS timestamptz LANGUAGE SQL VOLATILE
         AS 'SELECT instant FROM audit_test_clock';
         CREATE FUNCTION now() RETURNS timestamptz LANGUAGE SQL STABLE
         AS 'SELECT instant FROM audit_test_clock';",
    )
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO audit_test_clock VALUES ($1)")
        .bind(instant)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set(pool: &sqlx::PgPool, instant: time::OffsetDateTime) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE audit_test_clock SET instant=$1")
        .bind(instant)
        .execute(pool)
        .await?;
    Ok(())
}
