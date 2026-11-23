use std::time::Duration;

use serde::Serialize;
use sqlx::{FromRow, PgPool, postgres::PgRow};

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const WORKER_STALE_AFTER_SECONDS: i64 = 60;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProbeError {
    #[error("system database probe timed out")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("system database probe failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub(super) enum Probe<T> {
    Ok(T),
    Observed(T),
    Stale(T),
    Mismatch(T),
    Unknown,
    Unavailable,
}

impl<T> Probe<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Ok(value)
            | Self::Observed(value)
            | Self::Stale(value)
            | Self::Mismatch(value) => Some(value),
            Self::Unknown | Self::Unavailable => None,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Ok(_) => "system-ok",
            Self::Observed(_) => "system-observed",
            Self::Stale(_) => "system-stale",
            Self::Mismatch(_) => "system-mismatch",
            Self::Unknown => "system-unknown",
            Self::Unavailable => "system-unavailable",
        }
    }
}

// Bound pool acquisition and round trips as well as execution on PostgreSQL.
async fn query<T>(pool: &PgPool, sql: &'static str) -> Result<T, ProbeError>
where
    T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
{
    tokio::time::timeout(PROBE_TIMEOUT, async {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL statement_timeout = '1500ms'")
            .execute(&mut *tx)
            .await?;
        let value = sqlx::query_as(sql).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(value)
    })
    .await?
    .map_err(Into::into)
}

pub(super) async fn postgres(pool: &PgPool) -> Probe<String> {
    match query::<(String,)>(pool, "SELECT current_setting('server_version')").await {
        Ok((version,)) => Probe::Ok(version),
        Err(_) => Probe::Unavailable,
    }
}

#[derive(Debug, Serialize)]
pub(super) struct MigrationStatus {
    pub applied: Option<i64>,
    pub expected: Option<i64>,
}

pub(super) async fn migrations(pool: &PgPool) -> Probe<MigrationStatus> {
    let expected: Vec<_> = oteryn_db::MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| (migration.version, migration.checksum.as_ref()))
        .collect();
    let result = query::<(Vec<i64>, Vec<Vec<u8>>, bool)>(
        pool,
        "SELECT coalesce(array_agg(version ORDER BY version) FILTER (WHERE success), '{}'),
                coalesce(array_agg(checksum ORDER BY version) FILTER (WHERE success), '{}'),
                coalesce(bool_and(success), true)
         FROM _sqlx_migrations",
    )
    .await;
    match result {
        Ok((versions, checksums, all_succeeded)) => {
            migration_status(&expected, &versions, &checksums, all_succeeded)
        }
        Err(error) if table_absent(&error) => migration_status(&expected, &[], &[], true),
        Err(_) => Probe::Unavailable,
    }
}

fn migration_status(
    expected: &[(i64, &[u8])],
    applied: &[i64],
    checksums: &[Vec<u8>],
    all_succeeded: bool,
) -> Probe<MigrationStatus> {
    let status = MigrationStatus {
        applied: applied.last().copied(),
        expected: expected.last().map(|&(version, _)| version),
    };
    let installed = applied
        .iter()
        .copied()
        .zip(checksums.iter().map(Vec::as_slice));
    if all_succeeded && installed.eq(expected.iter().copied()) {
        Probe::Ok(status)
    } else {
        Probe::Mismatch(status)
    }
}

fn table_absent(error: &ProbeError) -> bool {
    matches!(error, ProbeError::Database(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01"))
}

async fn observation<T>(pool: &PgPool, sql: &'static str) -> Probe<T>
where
    (Option<T>,): for<'r> FromRow<'r, PgRow> + Send + Unpin,
{
    match query::<(Option<T>,)>(pool, sql).await {
        Ok((Some(value),)) => Probe::Observed(value),
        Ok((None,)) => Probe::Unknown,
        Err(error) if table_absent(&error) => Probe::Unknown,
        Err(_) => Probe::Unavailable,
    }
}

pub(super) async fn worker(pool: &PgPool) -> Probe<i64> {
    match observation(
        pool,
        "SELECT floor(extract(epoch FROM clock_timestamp() - max(seen_at)))::bigint
         FROM portal_worker_heartbeat",
    )
    .await
    {
        Probe::Observed(age) if (0..WORKER_STALE_AFTER_SECONDS).contains(&age) => Probe::Ok(age),
        Probe::Observed(age) => Probe::Stale(age),
        result => result,
    }
}

pub(crate) async fn record_worker_heartbeat(pool: &PgPool) -> Result<(), ProbeError> {
    query::<(bool,)>(
        pool,
        "INSERT INTO portal_worker_heartbeat (singleton, seen_at) VALUES (true, clock_timestamp())
         ON CONFLICT (singleton) DO UPDATE SET seen_at = clock_timestamp()
         RETURNING singleton",
    )
    .await
    .map(|_| ())
}

#[derive(Debug, Serialize)]
pub(super) struct Backlog {
    pub backlog: i64,
    pub oldest_age_seconds: Option<i64>,
}

pub(super) async fn outbox(pool: &PgPool) -> Probe<Backlog> {
    let result = query::<(i64, Option<i64>)>(
        pool,
        "SELECT count(*), floor(extract(epoch FROM statement_timestamp() - min(created_at)))::bigint
         FROM outbox WHERE state IN ('pending', 'running')",
    )
    .await;
    match result {
        Ok((backlog, oldest_age_seconds)) => Probe::Ok(Backlog {
            backlog,
            oldest_age_seconds,
        }),
        Err(_) => Probe::Unavailable,
    }
}

pub(super) async fn mail_outbox(pool: &PgPool) -> Probe<i64> {
    match query::<(i64,)>(
        pool,
        "SELECT count(*) FROM email_outbox AS mail
         JOIN outbox AS job ON job.kind = 'notify.email' AND job.idempotency_key = mail.id::text
         WHERE mail.delivered_at IS NULL AND job.state IN ('pending', 'running')",
    )
    .await
    {
        Ok((count,)) => Probe::Ok(count),
        Err(_) => Probe::Unavailable,
    }
}

pub(super) async fn gateway(pool: &PgPool) -> Probe<String> {
    // An authenticated, recorded native admission is evidence of traffic.
    observation(
        pool,
        r#"SELECT to_char(max(created_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
           FROM native_admissions"#,
    )
    .await
}

pub(super) async fn game_link(pool: &PgPool) -> Probe<String> {
    // The admin control store records contact with authenticated Game agents.
    observation(
        pool,
        r#"SELECT to_char(max(last_seen_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
           FROM admin_control_agents"#,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_status_checks_all_versions_and_failed_rows() {
        let expected = [(1, &[1][..]), (2, &[2][..])];
        let checksums = [vec![1], vec![2]];
        assert!(matches!(
            migration_status(&expected, &[1, 2], &checksums, true),
            Probe::Ok(_)
        ));
        for (applied, checksums, success) in [
            (&[2][..], &[vec![2]][..], true),
            (&[1, 2, 3], &[vec![1], vec![2], vec![3]], true),
            (&[1, 2], &checksums, false),
            (&[1, 2], &[vec![2], vec![1]], true),
        ] {
            assert!(matches!(
                migration_status(&expected, applied, checksums, success),
                Probe::Mismatch(MigrationStatus {
                    applied: Some(_),
                    expected: Some(2)
                })
            ));
        }
    }
}
