//! PostgreSQL serializes each publisher/route window across portal processes.
use axum::{
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use oteryn_internal_api::guard::{self, PublisherKey};
use sqlx::PgPool;
pub(crate) fn configured_limit() -> u16 {
    std::env::var("PORTAL_PUBLIC_PROJECTIONS_REQUESTS_PER_MINUTE")
        .map_or(6000, |value| value.parse().unwrap_or(0))
}
pub(crate) async fn check(
    db: &PgPool,
    publisher: &PublisherKey,
    route: &str,
    limit: u16,
) -> Result<(), Response> {
    if !(1..=6000).contains(&limit) {
        return Err(guard::failure(StatusCode::SERVICE_UNAVAILABLE));
    }
    match admit(db, publisher.as_bytes(), route, limit).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
            Err(response)
        }
        Err(_) => Err(guard::failure(StatusCode::SERVICE_UNAVAILABLE)),
    }
}
async fn admit(
    db: &PgPool,
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

#[cfg(test)]
#[path = "ingest_budget_tests.rs"]
mod tests;
