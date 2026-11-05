use super::*;
use askama::Template;
use axum::{
    Router,
    extract::{Path, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use serde::Deserialize;

struct Deny;
impl Current for Deny {
    fn character(&self, _: Uuid) -> CurrentFuture<'_> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
/// T24b must provide a current, fenced public-profile/privacy adapter before activation.
pub fn configured_router(db: PgPool) -> Router {
    router(Characters {
        db,
        enabled: std::env::var("PORTAL_CHARACTER_STATS_ENABLED").as_deref() == Ok("true"),
        current: Arc::new(Deny),
    })
}
pub fn router(state: Characters) -> Router {
    Router::new()
        .route("/{lang}/stats/characters/{id}/{tab}", get(character))
        .route("/{lang}/stats/compare", get(compare))
        .with_state(state)
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    start: Option<String>,
    end: Option<String>,
    #[serde(default, deserialize_with = "target")]
    target: Option<u32>,
    ids: Option<String>,
    metric: Option<String>,
}
fn target<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<u32>, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    raw.filter(|s| !s.is_empty())
        .map(|s| {
            let n = s.parse::<u32>().map_err(serde::de::Error::custom)?;
            if n == 0 || n.to_string() != s {
                return Err(serde::de::Error::custom("invalid target"));
            }
            Ok(n)
        })
        .transpose()
}
fn date(raw: &str) -> Result<Date, Error> {
    let parts: Vec<_> = raw.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::Invalid);
    }
    let d = Date::from_calendar_date(
        parts[0].parse().map_err(|_| Error::Invalid)?,
        parts[1]
            .parse::<u8>()
            .map_err(|_| Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
        parts[2].parse().map_err(|_| Error::Invalid)?,
    )
    .map_err(|_| Error::Invalid)?;
    if d.to_string() != raw {
        return Err(Error::Invalid);
    }
    Ok(d)
}
fn parse(raw: &str) -> Result<(Query, Date, Date), Error> {
    if raw.len() > 2048
        || raw.as_bytes().iter().enumerate().any(|(i, b)| {
            *b == b'%'
                && (i + 2 >= raw.len()
                    || !raw.as_bytes()[i + 1].is_ascii_hexdigit()
                    || !raw.as_bytes()[i + 2].is_ascii_hexdigit())
        })
    {
        return Err(Error::Invalid);
    }
    let q: Query = serde_urlencoded::from_str(raw).map_err(|_| Error::Invalid)?;
    let end = q
        .end
        .as_deref()
        .map(date)
        .transpose()?
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().date());
    let start = q
        .start
        .as_deref()
        .map(date)
        .transpose()?
        .unwrap_or(end.checked_sub(Duration::days(30)).ok_or(Error::Invalid)?);
    if start > end
        || (end - start).whole_days() > 365
        || end > time::OffsetDateTime::now_utc().date()
        || q.target == Some(0)
    {
        return Err(Error::Invalid);
    }
    Ok((q, start, end))
}
fn id(raw: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(raw).map_err(|_| Error::Invalid)?;
    if id.is_nil() || id.to_string() != raw {
        return Err(Error::Invalid);
    }
    Ok(id)
}
struct Text(FluentBundle<FluentResource>);
impl Text {
    fn new(lang: &str) -> Option<Self> {
        let source = match lang {
            "pl" => include_str!("../../../../templates/stats/character/pl.ftl"),
            "en" => include_str!("../../../../templates/stats/character/en.ftl"),
            _ => return None,
        };
        let mut bundle = FluentBundle::new_concurrent(vec![lang.parse().ok()?]);
        bundle
            .add_resource(FluentResource::try_new(source.into()).ok()?)
            .ok()?;
        Some(Self(bundle))
    }
    fn get(&self, key: &str) -> String {
        self.0
            .get_message(key)
            .and_then(|m| m.value())
            .map(|p| self.0.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}
#[derive(Template)]
#[template(path = "stats/character/index.html", config = "../web/askama.toml")]
struct Page {
    lang: String,
    title: String,
    message: String,
    hint: String,
    tabs: Vec<(String, String)>,
    rows: Vec<(String, String, String)>,
    labels: Vec<String>,
    start: String,
    end: String,
    ids: String,
    comparison: bool,
    selected_metric: String,
    charts: Vec<(String, String)>,
    metrics: Vec<(String, String)>,
}
fn page(lang: &str, text: &Text) -> Page {
    Page {
        lang: lang.into(),
        title: text.get("title"),
        message: String::new(),
        hint: text.get("hint"),
        tabs: vec![],
        rows: vec![],
        labels: [
            "date",
            "character",
            "value",
            "start",
            "end",
            "apply",
            "ids",
            "metric",
            "level",
            "experience",
            "skills",
            "target",
        ]
        .map(|k| text.get(k))
        .to_vec(),
        start: String::new(),
        end: String::new(),
        ids: String::new(),
        comparison: false,
        selected_metric: "level".into(),
        charts: vec![],
        metrics: [
            "level",
            "experience",
            "magic_level",
            "fist_level",
            "club_level",
            "sword_level",
            "axe_level",
            "distance_level",
            "shielding_level",
            "fishing_level",
        ]
        .map(|key| (key.into(), text.get(key)))
        .to_vec(),
    }
}
fn finish(mut page: Page, text: &Text, result: Result<(), Error>) -> Response {
    let status = match result {
        Ok(()) => {
            if page.rows.is_empty() {
                page.message = text.get("empty");
            }
            StatusCode::OK
        }
        Err(error) => {
            page.rows.clear();
            page.charts.clear();
            page.message = text.get(match error {
                Error::Disabled | Error::NotFound => "not-found",
                Error::Invalid => "invalid",
                Error::Unavailable => "unavailable",
            });
            match error {
                Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
                Error::Invalid => StatusCode::BAD_REQUEST,
                Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            }
        }
    };
    match page.render() {
        Ok(body) => (status, [(header::CACHE_CONTROL,"no-store, private"),(header::CONTENT_LANGUAGE,page.lang.as_str()),(header::CONTENT_SECURITY_POLICY,"default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"),(header::X_CONTENT_TYPE_OPTIONS,"nosniff"),(header::REFERRER_POLICY,"strict-origin-when-cross-origin"),(header::STRICT_TRANSPORT_SECURITY,"max-age=63072000; includeSubDomains"),(header::HeaderName::from_static("permissions-policy"),"camera=(), microphone=(), geolocation=()")],Html(body)).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn character(
    State(state): State<Characters>,
    Path((lang, raw_id, tab)): Path<(String, String, String)>,
    RawQuery(raw): RawQuery,
) -> Response {
    let Some(text) = Text::new(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut page = page(&lang, &text);
    let result = async {
        if !state.enabled {
            return Err(Error::Disabled);
        }
        if ![
            "history",
            "experience",
            "rankings",
            "deaths",
            "progress",
            "online",
        ]
        .contains(&tab.as_str())
        {
            return Err(Error::NotFound);
        }
        let (q, start, end) = parse(raw.as_deref().unwrap_or(""))?;
        if q.ids.is_some() || q.metric.is_some() {
            return Err(Error::Invalid);
        }
        let id = id(&raw_id)?;
        let report = state.load(id, start, end, q.target).await?;
        page.title = report.name.clone();
        page.start = start.to_string();
        page.end = end.to_string();
        for key in ["history", "experience", "rankings", "deaths", "progress"] {
            page.tabs.push((
                format!("/{lang}/stats/characters/{id}/{key}?start={start}&end={end}"),
                text.get(key),
            ));
        }
        match tab.as_str() {
            "experience" => {
                let maximum = report
                    .history
                    .daily
                    .iter()
                    .filter_map(|d| metric(d, "experience"))
                    .max()
                    .unwrap_or(1);
                page.charts.push((
                    text.get("experience"),
                    chart_points(&report.history.daily, "experience", start, end, maximum, 0),
                ));
                for d in &report.history.daily {
                    page.rows.push((
                        d.day.to_string(),
                        text.get("experience"),
                        d.experience
                            .clone()
                            .unwrap_or_else(|| text.get("unavailable")),
                    ));
                }
                for (i, key) in ["day", "week", "month"].iter().enumerate() {
                    page.rows.push((
                        end.to_string(),
                        text.get(key),
                        report.gains[i]
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| text.get("unavailable")),
                    ));
                }
                page.rows.push((
                    String::new(),
                    text.get("record"),
                    report
                        .record
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| text.get("unavailable")),
                ));
                page.rows.push((
                    String::new(),
                    text.get("prediction"),
                    report
                        .predicted
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| text.get("unavailable")),
                ));
            }
            "history" | "deaths" => {
                let mut events = report.history.events;
                if tab == "deaths" {
                    events.sort_by_key(|e| {
                        std::cmp::Reverse((
                            e.details
                                .get("occurred_at")
                                .and_then(|v| v.as_str())
                                .and_then(|v| v.parse::<i64>().ok())
                                .unwrap_or(0),
                            e.details
                                .get("death_occurrence_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_owned(),
                        ))
                    });
                }
                for event in events {
                    if (tab == "deaths") != (event.kind == "death") {
                        continue;
                    }
                    // Never render arbitrary source JSON, participant payloads or owner data.
                    let value = match event.kind.as_str() {
                        "name_change" => event
                            .details
                            .get("to")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        "level_up" | "death" => event
                            .details
                            .get("level")
                            .or_else(|| event.details.get("to"))
                            .and_then(|v| {
                                v.as_i64()
                                    .map(|v| v.to_string())
                                    .or_else(|| v.as_str().map(str::to_owned))
                            })
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    page.rows
                        .push((event.day.to_string(), text.get(&event.kind), value));
                }
            }
            "rankings" => match report.ranks {
                Some(rows) => {
                    for (day, rank) in rows {
                        page.rows
                            .push((day.to_string(), text.get("rankings"), rank.to_string()));
                    }
                }
                None => {
                    page.rows
                        .push((String::new(), text.get("rankings"), text.get("unavailable")))
                }
            },
            _ => page
                .rows
                .push((String::new(), text.get(&tab), text.get("unavailable"))),
        }
        Ok(())
    }
    .await;
    finish(page, &text, result)
}
fn metric(d: &Daily, metric: &str) -> Option<u64> {
    match metric {
        "level" => u64::try_from(d.level).ok(),
        "experience" => d.experience.as_deref()?.parse().ok(),
        skill => d.skills.as_ref()?.get(skill)?.as_u64(),
    }
}
fn chart_points(
    daily: &[Daily],
    key: &str,
    start: Date,
    end: Date,
    maximum: u64,
    series: usize,
) -> String {
    let mut points = String::new();
    let span = (end - start).whole_days().max(1) as u128;
    for d in daily {
        if let Some(v) = metric(d, key) {
            let x = 10 + ((d.day - start).whole_days() as u128) * 280 / span;
            let y = 140 - u128::from(v) * 130 / u128::from(maximum.max(1));
            let shape = match series % 5 {
                0 => "h3v3h-3z",
                1 => "l3,3m-3,0l3,-3",
                2 => "h4m-2,-2v4",
                3 => "l2,-3l2,3z",
                _ => "l2,-2l2,2l-2,2z",
            };
            points.push_str(&format!("M{x},{y}{shape} "));
        }
    }
    points
}
async fn compare(
    State(state): State<Characters>,
    Path(lang): Path<String>,
    RawQuery(raw): RawQuery,
) -> Response {
    let Some(text) = Text::new(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut page = page(&lang, &text);
    page.comparison = true;
    let result = async {
        if !state.enabled {
            return Err(Error::Disabled);
        }
        let (q, start, end) = parse(raw.as_deref().unwrap_or(""))?;
        if q.target.is_some() {
            return Err(Error::Invalid);
        }
        let metric_name = q.metric.as_deref().unwrap_or("level");
        if ![
            "level",
            "experience",
            "magic_level",
            "fist_level",
            "club_level",
            "sword_level",
            "axe_level",
            "distance_level",
            "shielding_level",
            "fishing_level",
        ]
        .contains(&metric_name)
        {
            return Err(Error::Invalid);
        }
        page.selected_metric = metric_name.into();
        let raw_ids = q.ids.as_deref().ok_or(Error::Invalid)?;
        let ids = raw_ids.split(',').map(id).collect::<Result<Vec<_>, _>>()?;
        let reports = state.compare(&ids, start, end).await?;
        page.start = start.to_string();
        page.end = end.to_string();
        page.ids = raw_ids.into();
        let maximum = reports
            .iter()
            .flat_map(|r| {
                r.history
                    .daily
                    .iter()
                    .filter_map(|d| metric(d, metric_name))
            })
            .max()
            .unwrap_or(1)
            .max(1);
        for (series, report) in reports.into_iter().enumerate() {
            let points = chart_points(
                &report.history.daily,
                metric_name,
                start,
                end,
                maximum,
                series,
            );
            for d in report.history.daily {
                page.rows.push((
                    d.day.to_string(),
                    report.name.clone(),
                    metric(&d, metric_name)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| text.get("unavailable")),
                ));
            }
            page.charts.push((report.name, points));
        }
        Ok(())
    }
    .await;
    finish(page, &text, result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn query_bounds_and_localization() {
        for raw in [
            "start=2026-11-04&end=2026-11-01",
            "start=2026-1-01",
            "target=0",
            "x=1",
            "start=%",
            "end=2026-11-01&end=2026-11-02",
            "start=2024-01-01&end=2026-01-01",
        ] {
            assert!(parse(raw).is_err(), "{raw}");
        }
        assert!(parse("start=2026-11-01&end=2026-11-02&target=").is_ok());
        for lang in ["pl", "en"] {
            let text = Text::new(lang).unwrap();
            for key in [
                "unavailable",
                "history",
                "experience",
                "rankings",
                "deaths",
                "progress",
                "prediction",
                "sword_level",
            ] {
                assert!(!text.get(key).is_empty());
            }
        }
    }
}
