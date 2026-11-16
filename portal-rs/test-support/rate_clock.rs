//! Freeze only the statement clock, leaving session and MFA wall clocks intact.
//! The disposable schema must precede an explicitly listed pg_catalog.
pub async fn install(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "CREATE TABLE audit_rate_clock AS SELECT date_trunc('minute', pg_catalog.statement_timestamp()) + interval '59.999 seconds' AS instant;
         CREATE FUNCTION statement_timestamp() RETURNS timestamptz LANGUAGE SQL STABLE
         AS 'SELECT instant FROM audit_rate_clock';",
    )
    .execute(pool)
    .await?;
    Ok(())
}
