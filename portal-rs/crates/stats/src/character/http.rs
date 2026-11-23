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

use oteryn_community::profiles::{self, Directory};

struct ProfileSource(Option<Directory>);
impl Current for ProfileSource {
    fn character(&self, id: Uuid) -> CurrentFuture<'_> {
        Box::pin(async move {
            let directory = self.0.as_ref().ok_or(Error::Unavailable)?;
            let profile = directory
                .identity_by_id(id)
                .await
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::NotFound)?;
            Ok(PublicCharacter {
                name: profile.name,
                permissions: Permissions {
                    visible: true,
                    guild: profile.fields.guild,
                    skills: profile.fields.skills,
                    deaths: profile.fields.deaths,
                    online: false,
                },
                ranks: None,
            })
        })
    }

    fn by_name<'a>(&'a self, name: &'a str) -> NamedFuture<'a> {
        Box::pin(async move {
            let directory = self.0.as_ref().ok_or(Error::Unavailable)?;
            let identity = directory
                .identity_by_name(name)
                .await
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::NotFound)?;
            Ok((identity.id, identity.name))
        })
    }
}

pub fn configured_router(db: PgPool) -> Router {
    let directory = profiles::enabled().then(|| Directory::configured(db.clone()));
    router(Characters {
        db,
        enabled: std::env::var("PORTAL_CHARACTER_STATS_ENABLED").as_deref() == Ok("true"),
        current: Arc::new(ProfileSource(directory)),
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
    first: Option<String>,
    second: Option<String>,
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
    invalid: bool,
    first: String,
    second: String,
    compare_hint: String,
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
            "chart-title",
            "data-table",
            "view-data",
            "table-scroll",
            "detail",
            "first-character",
            "second-character",
        ]
        .map(|k| text.get(k))
        .to_vec(),
        start: String::new(),
        end: String::new(),
        ids: String::new(),
        comparison: false,
        invalid: false,
        first: String::new(),
        second: String::new(),
        compare_hint: text.get("compare-hint"),
        selected_metric: "level".into(),
        charts: vec![],
        metrics: METRICS.map(|key| (key.into(), text.get(key))).to_vec(),
    }
}
fn finish(mut page: Page, text: &Text, result: Result<(), Error>) -> Response {
    let status = match result {
        Ok(()) => {
            if page.rows.is_empty() && !page.comparison {
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
    page.invalid = status == StatusCode::BAD_REQUEST;
    page.charts.retain(|(_, points)| !points.is_empty());
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
        if q.ids.is_some() || q.metric.is_some() || q.first.is_some() || q.second.is_some() {
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
                                v.as_u64()
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
const METRICS: [&str; 10] = [
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
];

enum Subjects<'a> {
    Picker(Option<&'a str>),
    Names([&'a str; 2]),
    Ids(&'a str),
}

fn subjects(query: &Query) -> Result<Subjects<'_>, Error> {
    if query
        .first
        .iter()
        .chain(&query.second)
        .any(|name| !profiles::valid_name(name) || !profiles::valid_name(name.trim()))
    {
        return Err(Error::Invalid);
    }
    let first = query.first.as_deref().map(str::trim);
    let second = query.second.as_deref().map(str::trim);
    match (query.ids.as_deref(), first, second) {
        (Some(ids), None, None) => Ok(Subjects::Ids(ids)),
        (None, Some(first), Some(second)) => Ok(Subjects::Names([first, second])),
        (None, first, None)
            if query.start.is_none() && query.end.is_none() && query.metric.is_none() =>
        {
            Ok(Subjects::Picker(first))
        }
        _ => Err(Error::Invalid),
    }
}

fn metric(d: &Daily, metric: &str) -> Option<u64> {
    match metric {
        "level" => Some(d.level),
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
    page.title = text.get("compare-title");
    let result = async {
        if !state.enabled {
            return Err(Error::Disabled);
        }
        let (q, start, end) = parse(raw.as_deref().unwrap_or(""))?;
        if q.target.is_some() {
            return Err(Error::Invalid);
        }
        page.start = start.to_string();
        page.end = end.to_string();
        let metric_name = q.metric.as_deref().unwrap_or("level");
        if !METRICS.contains(&metric_name) {
            return Err(Error::Invalid);
        }
        page.selected_metric = metric_name.into();
        let selection = match subjects(&q)? {
            Subjects::Picker(first) => {
                page.first = first.unwrap_or_default().into();
                return Ok(());
            }
            Subjects::Ids(raw_ids) => {
                let ids = raw_ids.split(',').map(id).collect::<Result<Vec<_>, _>>()?;
                page.ids = raw_ids.into();
                (ids, None)
            }
            Subjects::Names([first, second]) => {
                let (first_id, first_name) = state.current.by_name(first).await?;
                let (second_id, second_name) = state.current.by_name(second).await?;
                (
                    vec![first_id, second_id],
                    Some([(first_id, first_name), (second_id, second_name)]),
                )
            }
        };
        let (ids, names) = selection;
        let reports = state.compare(&ids, start, end).await?;
        if let Some([(first_id, first_name), (second_id, second_name)]) = names {
            for (id, name) in [(first_id, &first_name), (second_id, &second_name)] {
                if !reports
                    .iter()
                    .any(|report| report.id == id && report.name == *name)
                {
                    return Err(Error::Unavailable);
                }
            }
            page.first = first_name;
            page.second = second_name;
        }
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
            if report.history.daily.is_empty() {
                page.rows
                    .push((String::new(), report.name.clone(), text.get("unavailable")));
            }
            for d in report.history.daily {
                page.rows.push((
                    d.day.to_string(),
                    report.name.clone(),
                    metric(&d, metric_name)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| text.get("unavailable")),
                ));
            }
            if !points.is_empty() {
                page.charts.push((report.name, points));
            }
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
    #[tokio::test]
    async fn chart_data_and_validation_have_accessible_markup() {
        use axum::body::to_bytes;

        for lang in ["pl", "en"] {
            let text = Text::new(lang).unwrap();
            let mut rendered = page(lang, &text);
            rendered
                .rows
                .push(("2026-11-01".into(), "<Name>".into(), "42".into()));
            rendered
                .charts
                .push(("<Name>".into(), "M10,20h3v3h-3z".into()));
            let response = finish(rendered, &text, Ok(()));
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = to_bytes(response.into_body(), 65_536).await.unwrap();
            let html = std::str::from_utf8(&bytes).unwrap();
            assert!(html.contains("href=\"#character-data\""));
            assert!(html.contains(
                "role=\"region\" tabindex=\"0\" aria-labelledby=\"character-data-caption\""
            ));
            assert!(html.contains("id=\"character-data-caption\""));
            assert!(html.contains("scope=\"col\""));
            assert!(html.contains("&#60;Name&#62;"));
            assert!(!html.contains("<Name>"));
            assert!(html.contains(">42</td>"));
        }

        let text = Text::new("en").unwrap();
        let response = finish(page("en", &text), &text, Err(Error::Invalid));
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(response.into_body(), 65_536).await.unwrap();
        let html = std::str::from_utf8(&bytes).unwrap();
        assert!(html.contains("role=\"alert\" tabindex=\"-1\" autofocus"));
        assert!(!html.contains("<table"));
        assert!(!html.contains("<svg"));
    }

    #[test]
    fn comparison_accepts_two_names_or_existing_id_format() {
        let selection = |raw: &str| parse(raw).map(|(query, _, _)| query);
        let picker = selection("first=Aldric").unwrap();
        assert!(matches!(
            subjects(&picker),
            Ok(Subjects::Picker(Some("Aldric")))
        ));
        let names = selection("first=Aldric&second=Carina&metric=level").unwrap();
        assert!(matches!(
            subjects(&names),
            Ok(Subjects::Names(["Aldric", "Carina"]))
        ));
        let ids = selection("ids=a,b").unwrap();
        assert!(matches!(subjects(&ids), Ok(Subjects::Ids("a,b"))));
        for malformed in [
            "first=Aldric&metric=level".to_owned(),
            "first=Aldric&second=Carina&ids=a,b".to_owned(),
            "first=Aldric&second=%2F".to_owned(),
            "first=Aldric&second=%0ACarina".to_owned(),
            format!("first=Aldric&second={}", "a".repeat(65)),
        ] {
            let query = selection(&malformed).unwrap();
            assert!(subjects(&query).is_err(), "{malformed}");
        }
    }

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
