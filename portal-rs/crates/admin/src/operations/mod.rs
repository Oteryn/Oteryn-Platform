use crate::{rbac::Permission, web};
use askama::Template;
use axum::{
    Router,
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::ratelimit::NamedPolicy;
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;

type Rows = Vec<Vec<String>>;
type ReadResult = Result<Response, StatusCode>;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    operations_router(pool, &|key| std::env::var(key).ok())
}

fn operations_router(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let stats = Router::new().route("/admin/operations/statistics", get(statistics));
    let limits = Router::new()
        .route("/admin/operations/rate-limits", get(rate_limits))
        .route("/admin/operations/rate-limits.csv", get(rate_export));
    let flags = Router::new().route("/admin/operations/flags", get(effective_flags));
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
        headers: &HeaderMap,
        title: &'static str,
        columns: &'static [&'static str],
        rows: Rows,
    ) -> Self {
        Self {
            lang: if headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("en"))
            {
                "en"
            } else {
                "pl"
            },
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

async fn bounded<T>(future: impl Future<Output = Result<T, sqlx::Error>>) -> Result<T, StatusCode> {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .ok()
        .and_then(Result::ok)
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

async fn statistics(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
) -> ReadResult {
    let days = range(raw.as_deref())?;
    let rows = bounded(sqlx::query_as::<_, (String, i64, i64)>(
        "WITH bounds AS (
            SELECT (statement_timestamp() AT TIME ZONE 'UTC')::date AS today
         ), dates AS (
            SELECT today - n AS day FROM bounds, generate_series(0, $1::int - 1) AS n
         ), registrations AS (
            SELECT (created_at AT TIME ZONE 'UTC')::date AS day, count(*) AS total
            FROM accounts, bounds
            WHERE created_at >= ((today - ($1::int - 1))::timestamp AT TIME ZONE 'UTC')
              AND created_at <= statement_timestamp()
            GROUP BY 1
         ), logins AS (
            SELECT (created_at AT TIME ZONE 'UTC')::date AS day, count(*) AS total
            FROM security_events, bounds
            WHERE kind = 'login_succeeded'
              AND created_at >= ((today - ($1::int - 1))::timestamp AT TIME ZONE 'UTC')
              AND created_at <= statement_timestamp()
            GROUP BY 1
         )
         SELECT to_char(dates.day, 'YYYY-MM-DD'), coalesce(registrations.total, 0), coalesce(logins.total, 0)
         FROM dates LEFT JOIN registrations USING(day) LEFT JOIN logins USING(day)
         ORDER BY dates.day DESC",
    ).bind(i32::from(days)).fetch_all(&pool)).await?;
    let mut page = Page::new(
        &headers,
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
    let policies = NamedPolicy::ALL;
    let names: Vec<_> = policies.iter().map(|p| p.name()).collect();
    let thresholds: Vec<_> = policies.iter().map(|p| p.limit()).collect();
    // The counter saturates at limit + 1. Each saturated window proves at least
    // one denied key charge, but cannot tell us how many later charges it denied.
    let counts = bounded(
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
        .fetch_all(pool),
    )
    .await?;
    Ok(policies
        .iter()
        .zip(counts)
        .map(|(policy, count)| {
            vec![
                policy.name().to_owned(),
                policy.limit().to_string(),
                policy.window_seconds().to_string(),
                count.to_string(),
            ]
        })
        .collect())
}

async fn rate_limits(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
) -> ReadResult {
    no_query(raw.as_deref())?;
    let mut page = Page::new(
        &headers,
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

async fn effective_flags(RawQuery(raw): RawQuery, headers: HeaderMap) -> ReadResult {
    no_query(raw.as_deref())?;
    Page::new(
        &headers,
        "ops-flags",
        &["ops-name", "ops-value"],
        flag_rows(|name| std::env::var(name).as_deref() == Ok("true")),
    )
    .response()
}

async fn system_health(
    State(pool): State<PgPool>,
    RawQuery(raw): RawQuery,
    headers: HeaderMap,
) -> ReadResult {
    no_query(raw.as_deref())?;
    let mut page = Page::new(
        &headers,
        "ops-health",
        &["ops-component", "ops-status"],
        vec![],
    );
    page.rows
        .push(vec![page.text("ops-http"), page.text("ops-available")]);
    let queue = bounded(sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT count(*) FILTER (WHERE state = 'pending' AND run_after <= statement_timestamp()),
                count(*) FILTER (WHERE state = 'running' AND leased_until <= statement_timestamp()),
                count(*) FILTER (WHERE state = 'done' AND updated_at > statement_timestamp() - interval '5 minutes')
         FROM outbox WHERE state IN ('pending', 'running') OR updated_at > statement_timestamp() - interval '5 minutes'",
    ).fetch_one(&pool)).await;
    let database_ready = queue.is_ok();
    let database_status = page.text(if database_ready {
        "ops-available"
    } else {
        "ops-unavailable"
    });
    page.rows.push(vec![page.text("ops-db"), database_status]);
    if let Ok((due, expired, completed)) = queue {
        for (label, value) in [
            ("ops-due", due),
            ("ops-leases", expired),
            ("ops-recent-completions", completed),
        ] {
            page.rows.push(vec![page.text(label), value.to_string()]);
        }
    }
    match bounded(
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT to_char(observed AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS at
         FROM (
           SELECT created_at AS observed FROM audit_log
           WHERE action = 'ops.playerdata_binding_conflict' AND actor IS NULL
           UNION ALL
           SELECT updated_at AS observed FROM outbox WHERE state = 'dead'
         ) AS incidents
         WHERE observed > statement_timestamp() - interval '7 days'
         ORDER BY at DESC LIMIT 50",
        )
        .fetch_all(&pool),
    )
    .await
    {
        Ok(incidents) => page.incidents = incidents.into_iter().map(|at| vec![at]).collect(),
        Err(_) => page.note = "ops-incidents-unavailable",
    }
    page.response()
}

#[cfg(test)]
mod tests;
