use super::{Chart, Error, Range};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::BTreeMap;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

pub(super) type Values = BTreeMap<String, BTreeMap<String, i128>>;

pub(super) async fn snapshot(pool: &PgPool) -> Result<Transaction<'_, Postgres>, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '3s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

pub(super) async fn worlds(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<(Uuid, String)>, Error> {
    Ok(
        sqlx::query_as("SELECT id,name FROM world_status ORDER BY name,id LIMIT 500")
            .fetch_all(&mut **tx)
            .await?,
    )
}

pub(super) async fn world_exists(
    tx: &mut Transaction<'_, Postgres>,
    world: Uuid,
) -> Result<(), Error> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM world_status WHERE id=$1)")
        .bind(world)
        .fetch_one(&mut **tx)
        .await?;
    if exists { Ok(()) } else { Err(Error::Invalid) }
}

pub(super) async fn first_failure_day(tx: &mut Transaction<'_, Postgres>) -> Result<Date, Error> {
    Ok(
        sqlx::query_scalar("SELECT min(day) FROM dashboard_login_failures")
            .fetch_one(&mut **tx)
            .await?,
    )
}

pub(super) async fn last_online_update(
    tx: &mut Transaction<'_, Postgres>,
    selected: Option<Uuid>,
    now: OffsetDateTime,
) -> Result<Option<String>, Error> {
    let (sample, rollup): (Option<OffsetDateTime>, Option<Date>) = if let Some(world) = selected {
        sqlx::query_as(
            "SELECT (SELECT max(sampled_at) FROM stats_world_samples WHERE world=$2 AND sampled_at <= $1),
                    (SELECT max(day) FROM stats_world_daily WHERE world=$2 AND day <= ($1 AT TIME ZONE 'UTC')::date)",
        )
        .bind(now)
        .bind(world)
        .fetch_one(&mut **tx)
        .await?
    } else {
        sqlx::query_as(
            "SELECT (SELECT max(sampled_at) FROM stats_world_samples WHERE sampled_at <= $1),
                    (SELECT max(day) FROM stats_world_daily WHERE day <= ($1 AT TIME ZONE 'UTC')::date)",
        )
        .bind(now)
        .fetch_one(&mut **tx)
        .await?
    };
    if let Some(sample) = sample.filter(|sample| rollup.is_none_or(|day| sample.date() >= day)) {
        return Ok(Some(format!(
            "{} {:02}:{:02} UTC",
            sample.date(),
            sample.hour(),
            sample.minute()
        )));
    }
    Ok(rollup.map(|day| format!("{day} UTC")))
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    chart: Chart,
    range: Range,
    now: OffsetDateTime,
) -> Result<Values, Error> {
    let start = range.previous.midnight().assume_utc();
    let end = range.end.midnight().assume_utc().min(now);
    let rows: Vec<(String, String, String)> = match chart {
        Chart::Sessions => {
            if !range.days().contains(&now.date()) {
                Vec::new()
            } else {
                sqlx::query_as(
                    "SELECT ($1::timestamptz AT TIME ZONE 'UTC')::date::text, '', count(*)::text
                     FROM web_sessions WHERE account_id IS NOT NULL
                     AND idle_expires_at > $1 AND absolute_expires_at > $1",
                )
                .bind(now)
                .fetch_all(&mut **tx)
                .await?
            }
        }
        Chart::LoginsFailed => sqlx::query_as(
            "SELECT day::text, '', failures::text FROM dashboard_login_failures WHERE day >= $1 AND day < $2",
        )
        .bind(range.previous)
        .bind(range.end)
        .fetch_all(&mut **tx)
        .await?,
        Chart::Response => sqlx::query_as(RESPONSE)
            .bind(start)
            .bind(end)
            .bind(range.from)
            .bind(now)
            .fetch_all(&mut **tx)
            .await?,
        Chart::Online => {
            if let Some(world) = range.world {
                sqlx::query_as(ONLINE_WORLD)
                    .bind(start)
                    .bind(end)
                    .bind(world)
                    .fetch_all(&mut **tx)
                    .await?
            } else {
                sqlx::query_as(ONLINE)
                    .bind(start)
                    .bind(end)
                    .fetch_all(&mut **tx)
                    .await?
            }
        }
        metric => sqlx::query_as(metric.sql().ok_or(Error::Invalid)?)
            .bind(start)
            .bind(end)
            .fetch_all(&mut **tx)
            .await?,
    };
    let mut values = Values::new();
    for (bucket, series, value) in rows {
        values
            .entry(series)
            .or_default()
            .insert(bucket, value.parse()?);
    }
    Ok(values)
}

impl Chart {
    fn sql(self) -> Option<&'static str> {
        Some(match self {
            Self::Registrations => REGISTRATIONS,
            Self::Verified => VERIFIED,
            Self::LoginsSucceeded => SUCCEEDED,
            Self::Opened => OPENED,
            Self::Resolved => RESOLVED,
            Self::Sanctions => SANCTIONS,
            Self::Ledger => LEDGER,
            Self::Sessions | Self::LoginsFailed | Self::Response | Self::Online => return None,
        })
    }
}

const REGISTRATIONS: &str = "
    SELECT (created_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM accounts WHERE created_at >= $1 AND created_at < $2 GROUP BY 1";
const VERIFIED: &str = "
    SELECT (email_verified_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM accounts WHERE email_verified_at >= $1 AND email_verified_at < $2 GROUP BY 1";
const SUCCEEDED: &str = "
    SELECT (created_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM security_events WHERE kind='login_succeeded' AND created_at >= $1 AND created_at < $2 GROUP BY 1";
const OPENED: &str = "
    SELECT (created_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM tickets WHERE created_at >= $1 AND created_at < $2 GROUP BY 1";
const RESOLVED: &str = "
    SELECT (created_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM audit_log WHERE action='support.ticket_status_changed'
      AND after->>'status'='resolved' AND before->>'status' IS DISTINCT FROM 'resolved'
      AND created_at >= $1 AND created_at < $2 GROUP BY 1";
const SANCTIONS: &str = "
    SELECT (created_at AT TIME ZONE 'UTC')::date::text, '', count(*)::text
    FROM audit_log WHERE created_at >= $1 AND created_at < $2
      AND (action = 'support.enforcement_approved'
           OR (action = 'support.enforcement_created'
               AND (after->>'status' = 'active' OR after->>'pending' = 'false')))
    GROUP BY 1";
const LEDGER: &str = "
    SELECT (w.created_at AT TIME ZONE 'UTC')::date::text,
        CASE WHEN w.kind='top_up' AND p.provider='test' THEN 'top_up_sandbox' ELSE w.kind END,
        sum(abs(w.amount::numeric))::text
    FROM wallet_operations w LEFT JOIN payment_intents p ON w.kind='top_up' AND p.id=w.id
    WHERE w.created_at >= $1 AND w.created_at < $2 GROUP BY 1,2";
const RESPONSE: &str = "
    WITH replies AS (
        SELECT (t.created_at AT TIME ZONE 'UTC')::date AS day,
               extract(epoch FROM (m.created_at - t.created_at)) AS seconds
        FROM tickets t
        JOIN LATERAL (
            SELECT created_at FROM ticket_messages WHERE ticket_id=t.id
              AND author_kind='staff' AND NOT internal AND created_at >= t.created_at AND created_at < $4
            ORDER BY created_at LIMIT 1
        ) m ON true
        WHERE t.created_at >= $1 AND t.created_at < $2
    ), cohorts AS (
        SELECT day, day < $3 AS previous, seconds FROM replies
    )
    SELECT coalesce(day::text, CASE WHEN previous THEN 'previous' ELSE 'current' END), '',
        round(percentile_cont(0.5) WITHIN GROUP (ORDER BY seconds))::bigint::text
    FROM cohorts GROUP BY GROUPING SETS ((day), (previous))";
const ONLINE: &str = "
    SELECT day::text, '', max(peak)::text FROM (
        SELECT (sampled_at AT TIME ZONE 'UTC')::date AS day, max(players_online) AS peak
        FROM stats_world_samples WHERE sampled_at >= $1 AND sampled_at < $2 GROUP BY 1
        UNION ALL
        SELECT day, max(players_max) FROM stats_world_daily
        WHERE day >= ($1 AT TIME ZONE 'UTC')::date AND day <= (($2 - interval '1 microsecond') AT TIME ZONE 'UTC')::date
        GROUP BY day
    ) observed GROUP BY day";
const ONLINE_WORLD: &str = "
    SELECT day::text, '', max(peak)::text FROM (
        SELECT (sampled_at AT TIME ZONE 'UTC')::date AS day, max(players_online) AS peak
        FROM stats_world_samples WHERE world=$3 AND sampled_at >= $1 AND sampled_at < $2 GROUP BY 1
        UNION ALL
        SELECT day, players_max FROM stats_world_daily WHERE world=$3
          AND day >= ($1 AT TIME ZONE 'UTC')::date AND day <= (($2 - interval '1 microsecond') AT TIME ZONE 'UTC')::date
    ) observed GROUP BY day";
