//! Shared per-publisher HTTP route admission across portal processes.
pub async fn admit(
    db: &sqlx::PgPool,
    publisher: &[u8],
    route: &str,
    limit: u16,
) -> Result<bool, sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO community_ingest_budgets(publisher,route) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(publisher).bind(route).execute(&mut *tx).await?;
    // Lock before sampling time so waiting requests use the actual admission time.
    sqlx::query(
        "SELECT 1 FROM community_ingest_budgets WHERE publisher=$1 AND route=$2 FOR UPDATE",
    )
    .bind(publisher)
    .bind(route)
    .execute(&mut *tx)
    .await?;
    let allowed = sqlx::query_scalar::<_, bool>(
        "WITH moment AS MATERIALIZED (
            SELECT extract(epoch FROM clock_timestamp())::double precision AS now
        ), live AS (
            SELECT ARRAY(SELECT t FROM unnest(admitted_at) t
                         WHERE t > moment.now - 60) AS times, moment.now
            FROM community_ingest_budgets, moment WHERE publisher=$1 AND route=$2
        )
        UPDATE community_ingest_budgets b
        SET admitted_at=CASE WHEN cardinality(live.times)<$3
            THEN array_append(live.times,live.now) ELSE live.times END
        FROM live WHERE b.publisher=$1 AND b.route=$2
        RETURNING cardinality(live.times)<$3",
    )
    .bind(publisher)
    .bind(route)
    .bind(i32::from(limit))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(allowed)
}
