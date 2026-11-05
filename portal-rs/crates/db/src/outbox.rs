//! Transactional, at-least-once job delivery. Handlers must make their effects idempotent.

pub use sqlx::types::{Uuid, time::OffsetDateTime};
pub use sqlx::{Error, PgPool, Postgres, Transaction};

/// A claim's attempt number fences acknowledgements from expired owners.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub kind: String,
    /// JSON text, decoded and validated by the owning domain handler.
    pub payload: String,
    pub attempts: i32,
}

/// Enqueues atomically with the caller's domain changes. Duplicate keys preserve the original job.
/// Invalid JSON, blank kinds/keys and invalid attempt limits are refused by PostgreSQL.
pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    payload: &str,
    key: &str,
    run_after: OffsetDateTime,
    max_attempts: i32,
) -> Result<Uuid, Error> {
    sqlx::query_scalar(
        "INSERT INTO outbox (kind, payload, idempotency_key, run_after, max_attempts)
         VALUES ($1, $2::text::jsonb, $3, $4, $5)
         ON CONFLICT (kind, idempotency_key) DO UPDATE SET idempotency_key = outbox.idempotency_key
         RETURNING id",
    )
    .bind(kind)
    .bind(payload)
    .bind(key)
    .bind(run_after)
    .bind(max_attempts)
    .fetch_one(&mut **tx)
    .await
}

/// Claims a bounded batch without waiting for other workers' row locks.
pub async fn claim(pool: &PgPool, batch: i64, lease_seconds: i32) -> Result<Vec<Job>, Error> {
    if !(1..=1000).contains(&batch) || !(1..=86400).contains(&lease_seconds) {
        return Err(Error::Protocol(
            "invalid outbox batch or lease duration".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    // Exhausted jobs also become dead after a crash on their final attempt.
    sqlx::query(
        "WITH exhausted AS (
           SELECT id FROM outbox WHERE state = 'running' AND leased_until <= now()
             AND attempts >= max_attempts ORDER BY leased_until
           FOR UPDATE SKIP LOCKED LIMIT $1
         )
         UPDATE outbox o SET state = 'dead', leased_until = NULL,
           last_error = 'lease expired on final attempt', updated_at = now()
         FROM exhausted WHERE o.id = exhausted.id",
    )
    .bind(batch)
    .execute(&mut *tx)
    .await?;
    let rows: Vec<(Uuid, String, String, i32)> = sqlx::query_as(
        "WITH due AS (
           SELECT id FROM outbox WHERE
             (state = 'pending' AND run_after <= now()) OR
             (state = 'running' AND leased_until <= now() AND attempts < max_attempts)
           ORDER BY run_after, id FOR UPDATE SKIP LOCKED LIMIT $1
         )
         UPDATE outbox o SET state = 'running', attempts = attempts + 1,
           leased_until = now() + make_interval(secs => $2), updated_at = now()
         FROM due WHERE o.id = due.id RETURNING o.id, o.kind, o.payload::text, o.attempts",
    )
    .bind(batch)
    .bind(f64::from(lease_seconds))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(id, kind, payload, attempts)| Job {
            id,
            kind,
            payload,
            attempts,
        })
        .collect())
}

/// Extends a live lease only while its original claim still owns it.
pub async fn renew(pool: &PgPool, job: &Job, lease_seconds: i32) -> Result<bool, Error> {
    if !(1..=86400).contains(&lease_seconds) {
        return Err(Error::Protocol("invalid outbox lease duration".into()));
    }
    Ok(sqlx::query(
        "UPDATE outbox SET leased_until = now() + make_interval(secs => $3), updated_at = now()
         WHERE id = $1 AND attempts = $2 AND state = 'running' AND leased_until > now()",
    )
    .bind(job.id)
    .bind(job.attempts)
    .bind(f64::from(lease_seconds))
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

/// Finishes a live claim. Unknown kinds bypass retries. Retry delay is capped at one hour,
/// with equal jitter in the upper half of the exponential window; errors are size bounded.
pub async fn finish(
    pool: &PgPool,
    job: &Job,
    error: Option<&str>,
    permanent: bool,
) -> Result<bool, Error> {
    Ok(sqlx::query(
        "UPDATE outbox SET
           state = CASE WHEN $3::text IS NULL THEN 'done'
                        WHEN $4 OR attempts >= max_attempts THEN 'dead' ELSE 'pending' END,
           run_after = CASE WHEN $3::text IS NULL THEN run_after ELSE
             now() + make_interval(secs => LEAST(3600.0, power(2.0, LEAST(attempts, 12))) * (0.5 + random() * 0.5)) END,
           leased_until = NULL, last_error = left($3, 2048), updated_at = now()
         WHERE id = $1 AND attempts = $2 AND state = 'running' AND leased_until > now()",
    ).bind(job.id).bind(job.attempts).bind(error).bind(permanent).execute(pool).await?.rows_affected() == 1)
}

/// Counts for metrics exporters, including zero values for empty states. No payloads or errors.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub pending: i64,
    pub running: i64,
    pub done: i64,
    pub dead: i64,
}

pub async fn counts(pool: &PgPool) -> Result<Counts, Error> {
    let (pending, running, done, dead) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE state = 'pending'), count(*) FILTER (WHERE state = 'running'),
         count(*) FILTER (WHERE state = 'done'), count(*) FILTER (WHERE state = 'dead') FROM outbox",
    ).fetch_one(pool).await?;
    Ok(Counts {
        pending,
        running,
        done,
        dead,
    })
}

/// A database-clock schedule slot makes concurrent schedulers and restarts deduplicate.
pub async fn schedule(pool: &PgPool, kind: &str, interval_seconds: i32) -> Result<(), Error> {
    if interval_seconds <= 0 {
        return Err(Error::Protocol("schedule interval must be positive".into()));
    }
    sqlx::query(
        "INSERT INTO outbox (kind, payload, idempotency_key)
         VALUES ($1, '{}'::jsonb, 'schedule:' || floor(extract(epoch FROM now()) / $2)::text)
         ON CONFLICT (kind, idempotency_key) DO NOTHING",
    )
    .bind(kind)
    .bind(interval_seconds)
    .execute(pool)
    .await?;
    Ok(())
}

// Re-export the query and pool builders for domain repositories and integration harnesses.
pub use sqlx::{AssertSqlSafe, postgres::PgPoolOptions, query, query_as, query_scalar};

/// Shared asynchronous result used by periodic domain cleanup handlers.
pub type CleanupFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;

/// Implemented by session, token and support retention repositories.
pub trait Cleanup: std::fmt::Debug + Send + Sync {
    fn clean(&self) -> CleanupFuture<'_>;
}
