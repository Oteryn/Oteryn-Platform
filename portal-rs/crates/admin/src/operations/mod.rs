use crate::{analytics::IngestAvailability, rbac::Permission, web};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::ratelimit::NamedPolicy;
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::time::Instant;

type Rows = Vec<Vec<String>>;
type ReadResult = Result<Response, StatusCode>;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    operations_router(pool, IngestAvailability::configured()?, &|key| {
        std::env::var(key).ok()
    })
}

fn operations_router(
    pool: PgPool,
    ingest: IngestAvailability,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let stats = Router::new().route("/admin/operations/statistics", get(statistics));
    let limits = Router::new()
        .route("/admin/operations/rate-limits", get(rate_limits))
        .route("/admin/operations/rate-limits.csv", get(rate_export));
    let flags = Router::new()
        .route("/admin/operations/flags", get(effective_flags))
        .layer(Extension(ingest));
    let health = Router::new().route("/admin/operations/health", get(system_health));
    let mut routes = Router::new();
    for (router, permission) in [
        (stats, Permission::AdminAnalyticsView),
        (limits, Permission::ViewRateLimits),
        (flags, Permission::ViewEffectiveFlags),
        (health, Permission::ViewPlatformHealth),
    ] {
        routes = routes.merge(web::configured_guard_from(
            router.with_state(pool.clone()),
            pool.clone(),
            permission,
            lookup,
        )?);
    }
    Ok(routes)
}

#[derive(Template)]
#[template(path = "page.html", config = "src/operations/askama.toml")]
struct Page {
    lang: &'static str,
    title: &'static str,
    columns: &'static [&'static str],
    rows: Rows,
    note: &'static str,
    days: Option<u8>,
    export: bool,
    incidents: Rows,
}

impl Page {
    fn new(
        lang: &'static str,
        title: &'static str,
        columns: &'static [&'static str],
        rows: Rows,
    ) -> Self {
        Self {
            lang,
            title,
            columns,
            rows,
            note: "",
            days: None,
            export: false,
            incidents: vec![],
        }
    }

    fn text(&self, key: &str) -> String {
        oteryn_ui::text(self.lang, key)
    }

    fn response(&self) -> ReadResult {
        let html = self.render().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(([(header::CACHE_CONTROL, "no-store")], Html(html)).into_response())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    days: u8,
}

fn range(raw: Option<&str>) -> Result<u8, StatusCode> {
    let Some(raw) = raw.filter(|query| !query.is_empty()) else {
        return Ok(7);
    };
    serde_urlencoded::from_str::<Range>(raw)
        .ok()
        .map(|range| range.days)
        .filter(|days| (1..=30).contains(days))
        .ok_or(StatusCode::BAD_REQUEST)
}

fn no_query(raw: Option<&str>) -> Result<(), StatusCode> {
    if raw.is_some_and(|query| !query.is_empty()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

const READ_BUDGET: Duration = Duration::from_millis(2500);
const HTTP_BUDGET: Duration = Duration::from_secs(3);

async fn bounded<T>(
    pool: &PgPool,
    read: impl for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<T, sqlx::Error>> + Send + 'a>>,
) -> Result<T, StatusCode> {
    let start = Instant::now();
    tokio::time::timeout_at(start + HTTP_BUDGET, async {
        let mut tx = read_transaction(pool, start + READ_BUDGET).await?;
        let result = read(&mut tx).await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(result)
    })
    .await
    .ok()
    .and_then(Result::ok)
    .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

async fn read_transaction(
    pool: &PgPool,
    deadline: Instant,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .as_millis();
    if remaining == 0 {
        return Err(sqlx::Error::PoolTimedOut);
    }
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(format!("{remaining}ms"))
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

const STATISTICS_SQL: &str =
        "WITH bounds AS (
            SELECT (statement_timestamp() AT TIME ZONE 'UTC')::date AS today
         ), dates AS (
            SELECT today - n AS day FROM bounds, generate_series(0, $1::int - 1) AS n
         ), registrations AS (
            SELECT (created_at AT TIME ZONE 'UTC')::date AS day, count(*) AS total
            FROM accounts
            WHERE created_at >= (((statement_timestamp() AT TIME ZONE 'UTC')::date - ($1::int - 1))::timestamp AT TIME ZONE 'UTC')
              AND created_at <= statement_timestamp()
            GROUP BY 1
         ), logins AS (
            SELECT (created_at AT TIME ZONE 'UTC')::date AS day, count(*) AS total
            FROM security_events
            WHERE kind = 'login_succeeded'
              AND created_at >= (((statement_timestamp() AT TIME ZONE 'UTC')::date - ($1::int - 1))::timestamp AT TIME ZONE 'UTC')
              AND created_at <= statement_timestamp()
            GROUP BY 1
         )
         SELECT to_char(dates.day, 'YYYY-MM-DD'), coalesce(registrations.total, 0), coalesce(logins.total, 0)
         FROM dates LEFT JOIN registrations USING(day) LEFT JOIN logins USING(day)
         ORDER BY dates.day DESC";

async fn statistics(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> ReadResult {
    let days = range(raw.as_deref())?;
    let rows = bounded(&pool, |connection| {
        Box::pin(
            sqlx::query_as::<_, (String, i64, i64)>(STATISTICS_SQL)
                .bind(i32::from(days))
                .fetch_all(connection),
        )
    })
    .await?;
    let mut page = Page::new(
        web::page_language(&headers, chrome.as_deref()),
        "ops-statistics",
        &["ops-date", "ops-registrations", "ops-logins"],
        rows.into_iter()
            .map(|(day, registrations, logins)| {
                vec![day, registrations.to_string(), logins.to_string()]
            })
            .collect(),
    );
    page.days = Some(days);
    page.response()
}

async fn rate_rows(pool: &PgPool) -> Result<Rows, StatusCode> {
    // These older identity adapters call abuse::http::budget directly.
    let identity = [
        ("identity-consent-mutation", 10, 60),
        ("identity-registration-attempt", 5, 60),
        ("identity-registration-challenge", 5, 60),
    ];
    let mut policies: Vec<_> = NamedPolicy::ALL
        .iter()
        .map(|policy| (policy.name(), policy.limit(), policy.window_seconds()))
        .chain(identity)
        .collect();
    policies.sort_unstable_by_key(|policy| policy.0);
    let names: Vec<_> = policies.iter().map(|policy| policy.0).collect();
    let thresholds: Vec<_> = policies.iter().map(|policy| policy.1).collect();
    // The counter saturates at limit + 1. Each saturated window proves at least
    // one denied key charge, but cannot tell us how many later charges it denied.
    let counts = bounded(pool, |connection| {
        Box::pin(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(r.limiter_name)
         FROM unnest($1::text[], $2::bigint[]) WITH ORDINALITY AS p(name, threshold, ord)
         LEFT JOIN web_rate_limits r ON r.limiter_name = p.name AND r.count > p.threshold
           AND r.window_start >= statement_timestamp() - interval '1 hour'
           AND r.window_start <= statement_timestamp()
         GROUP BY p.ord ORDER BY p.ord",
            )
            .bind(names)
            .bind(thresholds)
            .fetch_all(connection),
        )
    })
    .await?;
    Ok(policies
        .iter()
        .zip(counts)
        .map(|((name, limit, window_seconds), count)| {
            vec![
                (*name).to_owned(),
                limit.to_string(),
                window_seconds.to_string(),
                count.to_string(),
            ]
        })
        .collect())
}

async fn rate_limits(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> ReadResult {
    no_query(raw.as_deref())?;
    let mut page = Page::new(
        web::page_language(&headers, chrome.as_deref()),
        "ops-rate-limits",
        &["ops-policy", "ops-limit", "ops-window", "ops-saturated"],
        rate_rows(&pool).await?,
    );
    page.note = "ops-rate-note";
    page.export = true;
    page.response()
}

async fn rate_export(State(pool): State<PgPool>, RawQuery(raw): RawQuery) -> ReadResult {
    no_query(raw.as_deref())?;
    let mut csv = String::from("policy,limit,window_seconds,saturated_key_windows_last_hour\r\n");
    for row in rate_rows(&pool).await? {
        // Policy names are static identifiers. All other columns are integers.
        csv.push_str(&row.join(","));
        csv.push_str("\r\n");
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rate-limits.csv\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        csv,
    )
        .into_response())
}

const BOOLEAN_FLAGS: &[&str] = &[
    "OTERYN_ACCOUNT_EXPORT_ENABLED",
    "OTERYN_ADMIN_ANALYTICS_INGEST",
    "OTERYN_ADMIN_ANALYTICS_PRESENT",
    "OTERYN_ADMIN_CONSOLE",
    "OTERYN_ADMIN_PANEL",
    "OTERYN_ADMIN_PLAYERS",
    "OTERYN_BAZAAR_MODERATION",
    "OTERYN_DOWNLOADS_ENABLED",
];

fn flag_rows(enabled: impl Fn(&str) -> bool) -> Rows {
    BOOLEAN_FLAGS
        .iter()
        .map(|name| vec![(*name).into(), enabled(name).to_string()])
        .collect()
}

async fn effective_flags(
    Extension(ingest): Extension<IngestAvailability>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> ReadResult {
    no_query(raw.as_deref())?;
    Page::new(
        web::page_language(&headers, chrome.as_deref()),
        "ops-flags",
        &["ops-name", "ops-value"],
        flag_rows(|name| match name {
            "OTERYN_ADMIN_ANALYTICS_INGEST" => ingest.0,
            _ => std::env::var(name).as_deref() == Ok("true"),
        }),
    )
    .response()
}

async fn system_health(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> ReadResult {
    no_query(raw.as_deref())?;
    let mut page = Page::new(
        web::page_language(&headers, chrome.as_deref()),
        "ops-health",
        &["ops-component", "ops-status"],
        vec![],
    );
    page.rows
        .push(vec![page.text("ops-http"), page.text("ops-available")]);
    let (database, queue, incidents) = tokio::join!(
        bounded(&pool, |connection| Box::pin(
            sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(connection)
        )),
        bounded(&pool, |connection| {
            Box::pin(sqlx::query_as::<_, (i64, i64, bool)>(
            "SELECT count(*) FILTER (WHERE state = 'pending' AND run_after <= statement_timestamp()),
                    count(*) FILTER (WHERE state = 'running' AND leased_until <= statement_timestamp()),
                    coalesce(bool_or(kind = 'cleanup.rate-limits' AND state = 'done'
                      AND updated_at > statement_timestamp() - interval '5 minutes'), false)
             FROM outbox WHERE state IN ('pending', 'running')
                OR (kind = 'cleanup.rate-limits' AND state = 'done')",
        ).fetch_one(connection))
        }),
        bounded(&pool, |connection| {
            Box::pin(sqlx::query_as::<_, (String, bool)>(
            "SELECT to_char(observed AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS'), binding_conflict
             FROM (
               SELECT created_at AS observed, true AS binding_conflict FROM audit_log
               WHERE action = 'ops.playerdata_binding_conflict' AND actor IS NULL
               UNION ALL
               SELECT updated_at AS observed, false FROM outbox WHERE state = 'dead'
             ) AS incidents
             WHERE observed > statement_timestamp() - interval '7 days'
             ORDER BY observed DESC LIMIT 50",
        ).fetch_all(connection))
        }),
    );
    let database_status = page.text(if database.is_ok() {
        "ops-available"
    } else {
        "ops-unavailable"
    });
    page.rows.push(vec![page.text("ops-db"), database_status]);
    match queue {
        Ok((due, expired, worker_observed)) => {
            let worker_status = page.text(if worker_observed {
                "ops-worker-observed"
            } else {
                "ops-worker-unconfirmed"
            });
            page.rows.push(vec![page.text("ops-worker"), worker_status]);
            for (label, value) in [("ops-due", due), ("ops-leases", expired)] {
                page.rows.push(vec![page.text(label), value.to_string()]);
            }
        }
        Err(_) => page
            .rows
            .push(vec![page.text("ops-worker"), page.text("ops-unavailable")]),
    }
    match incidents {
        Ok(incidents) => {
            page.incidents = incidents
                .into_iter()
                .map(|(at, binding_conflict)| {
                    let label = if binding_conflict {
                        "ops-binding-conflict"
                    } else {
                        "ops-dead-job"
                    };
                    vec![at, page.text(label)]
                })
                .collect()
        }
        Err(_) => page.note = "ops-incidents-unavailable",
    }
    page.response()
}

#[cfg(test)]
mod tests;
