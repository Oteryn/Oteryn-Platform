mod data;

use crate::web;
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::{collections::BTreeMap, fmt::Write};
use time::{Date, Duration, OffsetDateTime, format_description::well_known::Iso8601};
use uuid::Uuid;

pub(super) const PAGE: &str = "/admin/charts";

#[derive(Clone)]
struct Charts {
    pool: PgPool,
    extended: bool,
}

pub(super) fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool, extended: bool) -> Router<S> {
    Router::new()
        .route(PAGE, get(page))
        .route("/admin/charts/{file}", get(export))
        .with_state(Charts { pool, extended })
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid chart selection")]
    Invalid,
    #[error("chart unavailable")]
    Missing,
    #[error("statistics storage unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("invalid aggregate")]
    Aggregate(#[from] std::num::ParseIntError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Storage(_) | Self::Aggregate(_) => {
                tracing::warn!(error = %self, "staff statistics unavailable");
                StatusCode::SERVICE_UNAVAILABLE
            }
        }
        .into_response()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chart {
    Registrations,
    Verified,
    LoginsSucceeded,
    LoginsFailed,
    Sessions,
    Opened,
    Resolved,
    Response,
    Sanctions,
    Ledger,
    Online,
}

impl Chart {
    const ALL: [Self; 11] = [
        Self::Registrations,
        Self::Verified,
        Self::LoginsSucceeded,
        Self::LoginsFailed,
        Self::Sessions,
        Self::Opened,
        Self::Resolved,
        Self::Response,
        Self::Sanctions,
        Self::Ledger,
        Self::Online,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Registrations => "registrations",
            Self::Verified => "verified",
            Self::LoginsSucceeded => "logins-succeeded",
            Self::LoginsFailed => "logins-failed",
            Self::Sessions => "web-sessions",
            Self::Opened => "tickets-opened",
            Self::Resolved => "tickets-resolved",
            Self::Response => "first-response",
            Self::Sanctions => "sanctions",
            Self::Ledger => "wallet",
            Self::Online => "game-online",
        }
    }

    fn available(self, extended: bool) -> bool {
        extended || !matches!(self, Self::Ledger | Self::Online)
    }

    fn from_file(file: &str, extended: bool) -> Result<Self, Error> {
        let key = file.strip_suffix(".csv").ok_or(Error::Missing)?;
        Self::ALL
            .into_iter()
            .find(|chart| chart.key() == key && chart.available(extended))
            .ok_or(Error::Missing)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    range: Option<String>,
    from: Option<String>,
    to: Option<String>,
    world: Option<String>,
    lang: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct Range {
    from: Date,
    end: Date,
    previous: Date,
    world: Option<Uuid>,
}

impl Selection {
    fn range(&self, today: Date, extended: bool) -> Result<Range, Error> {
        if self
            .lang
            .as_deref()
            .is_some_and(|lang| !matches!(lang, "pl" | "en"))
        {
            return Err(Error::Invalid);
        }
        let world = self
            .world
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| value.parse::<Uuid>().map_err(|_| Error::Invalid))
            .transpose()?;
        if world.is_some_and(|id| !extended || id.get_version_num() != 7) {
            return Err(Error::Invalid);
        }
        let (from, to) = match self.range.as_deref().unwrap_or("7") {
            "custom" => (parse_date(&self.from)?, parse_date(&self.to)?),
            preset => {
                let days = match preset {
                    "7" => 7,
                    "30" => 30,
                    "90" => 90,
                    _ => return Err(Error::Invalid),
                };
                (
                    today
                        .checked_sub(Duration::days(days - 1))
                        .ok_or(Error::Invalid)?,
                    today,
                )
            }
        };
        let days = (to - from).whole_days() + 1;
        if !(1..=90).contains(&days) || to > today {
            return Err(Error::Invalid);
        }
        let previous = from
            .checked_sub(Duration::days(days))
            .filter(|date| date.year() >= 1)
            .ok_or(Error::Invalid)?;
        Ok(Range {
            from,
            end: to.next_day().ok_or(Error::Invalid)?,
            previous,
            world,
        })
    }
}

fn parse_date(value: &Option<String>) -> Result<Date, Error> {
    let value = value
        .as_deref()
        .filter(|s| s.len() == 10)
        .ok_or(Error::Invalid)?;
    let parsed = Date::parse(value, &Iso8601::DATE).map_err(|_| Error::Invalid)?;
    if parsed.to_string() != value {
        return Err(Error::Invalid);
    }
    Ok(parsed)
}

impl Range {
    fn days(self) -> std::ops::Range<Date> {
        self.from..self.end
    }

    fn labels(self, from: Date, end: Date) -> impl Iterator<Item = (Date, String)> {
        std::iter::successors(Some(from), |date| date.next_day())
            .take_while(move |date| *date < end)
            .map(|date| (date, date.to_string()))
    }

    fn csv_query(self, lang: &str) -> String {
        let world = self
            .world
            .map(|id| format!("&world={id}"))
            .unwrap_or_default();
        format!(
            "range=custom&from={}&to={}&lang={lang}{world}",
            self.from,
            self.end - Duration::days(1)
        )
    }
}

type Labels = fluent_bundle::FluentBundle<fluent_bundle::FluentResource>;

fn labels(lang: &str) -> Option<Labels> {
    let locale = lang.parse().ok()?;
    let source = if lang == "en" {
        include_str!("../../../../templates/admin/charts/en.ftl")
    } else {
        include_str!("../../../../templates/admin/charts/pl.ftl")
    };
    let resource = fluent_bundle::FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

fn text(labels: &Labels, key: &str) -> String {
    labels
        .get_message(key)
        .and_then(|message| message.value())
        .map(|pattern| {
            labels
                .format_pattern(pattern, None, &mut Vec::new())
                .into_owned()
        })
        .unwrap_or_default()
}

pub(super) fn nav_label(lang: &str) -> String {
    labels(lang)
        .map(|labels| text(&labels, "statistics"))
        .unwrap_or_default()
}

#[derive(Template)]
#[template(path = "admin/charts/index.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    labels: Labels,
    preset: String,
    from: Date,
    to: Date,
    previous_from: Date,
    previous_to: Date,
    world: Option<Uuid>,
    extended: bool,
    worlds: Vec<(Uuid, String)>,
    first_failure_day: Date,
    last_online_update: Option<String>,
    export: bool,
    csv_query: String,
    charts: Vec<Card>,
}

impl Page {
    fn text(&self, key: &str) -> String {
        text(&self.labels, key)
    }

    fn series_title(&self, card: &Card, series: &Series) -> String {
        if series.key.is_empty() {
            return self.text(card.chart.key());
        }
        let label = self.text(&format!("kind-{}", series.key.replace('_', "-")));
        if label.is_empty() {
            series.key.clone()
        } else {
            label
        }
    }
}

struct Card {
    chart: Chart,
    series: Vec<Series>,
    available: bool,
}

struct Series {
    key: String,
    rows: Vec<(String, Option<i128>)>,
    marks: Vec<Mark>,
    chart_max: i128,
    value: Option<i128>,
    change: Option<i128>,
}

struct Mark {
    x: u32,
    y: u32,
    height: u32,
}

impl Series {
    fn new(
        chart: Chart,
        key: String,
        values: &BTreeMap<String, i128>,
        range: Range,
        first_failure_day: Date,
    ) -> Self {
        let observed = |date: Date, label: &str| {
            let exhaustive = match chart {
                Chart::LoginsFailed => date >= first_failure_day,
                Chart::Sessions | Chart::Response | Chart::Online => false,
                _ => true,
            };
            values.get(label).copied().or(exhaustive.then_some(0))
        };
        let period = |from, end, median| match chart {
            Chart::Response => values.get(median).copied(),
            Chart::Online | Chart::Sessions => range
                .labels(from, end)
                .filter_map(|(day, label)| observed(day, &label))
                .max(),
            Chart::LoginsFailed if from < first_failure_day => None,
            _ => Some(
                range
                    .labels(from, end)
                    .filter_map(|(day, label)| observed(day, &label))
                    .sum(),
            ),
        };
        let value = period(range.from, range.end, "current");
        let change = value
            .zip(period(range.previous, range.from, "previous"))
            .map(|(current, previous)| current - previous);
        let rows: Vec<_> = range
            .labels(range.from, range.end)
            .map(|(day, label)| {
                let value = observed(day, &label);
                (label, value)
            })
            .collect();
        let chart_max = rows
            .iter()
            .filter_map(|(_, value)| *value)
            .max()
            .unwrap_or_default();
        let step = 540 / rows.len() as u32;
        let marks = rows
            .iter()
            .enumerate()
            .filter_map(|(index, (_, value))| {
                let value = (*value)?;
                let height = if value == 0 {
                    0
                } else {
                    ((110.0 * value as f64 / chart_max.max(1) as f64).round() as u32).max(2)
                };
                Some(Mark {
                    x: 30 + index as u32 * step + step / 2,
                    y: 133 - height,
                    height,
                })
            })
            .collect();
        Self {
            key,
            rows,
            marks,
            chart_max,
            value,
            change,
        }
    }

    fn bar_width(&self) -> u32 {
        (430 / self.rows.len() as u32).min(48)
    }

    fn first_day(&self) -> &str {
        self.rows.first().map_or("", |(day, _)| day.as_str())
    }

    fn last_day(&self) -> &str {
        self.rows.last().map_or("", |(day, _)| day.as_str())
    }

    fn signed_change(&self) -> String {
        self.change
            .map_or_else(String::new, |change| format!("{change:+}"))
    }
}

impl Card {
    fn new(chart: Chart, mut values: data::Values, range: Range, first_failure_day: Date) -> Self {
        if values.is_empty() {
            values.insert(String::new(), BTreeMap::new());
        }
        let series: Vec<_> = values
            .iter()
            .map(|(key, values)| Series::new(chart, key.clone(), values, range, first_failure_day))
            .collect();
        let available = series
            .iter()
            .any(|series| series.rows.iter().any(|(_, value)| value.is_some()));
        Self {
            chart,
            series,
            available,
        }
    }

    fn note(&self, selected: bool) -> &'static str {
        match self.chart {
            Chart::Online if selected => "game-online-world-note",
            Chart::Online => "game-online-note",
            Chart::Ledger => "wallet-note",
            Chart::Sessions => "web-sessions-note",
            Chart::Response => "first-response-note",
            Chart::Verified => "verified-note",
            Chart::Sanctions => "sanctions-note",
            Chart::Resolved => "tickets-resolved-note",
            Chart::LoginsFailed => "logins-failed-note",
            _ => "counts-note",
        }
    }

    fn csv(&self) -> String {
        let mut csv = String::from("date_utc,series,value\r\n");
        for series in &self.series {
            let key = if series.key.is_empty() {
                self.chart.key()
            } else {
                &series.key
            };
            for (day, value) in &series.rows {
                // Dates, metric keys, constrained ledger kinds and integers are machine fields.
                let value = value.map_or_else(String::new, |value| value.to_string());
                let _ = write!(csv, "{day},{key},{value}\r\n");
            }
        }
        csv
    }
}

#[derive(Clone, Copy)]
pub(super) struct CanExport(pub bool);

async fn page(
    State(state): State<Charts>,
    Extension(CanExport(export)): Extension<CanExport>,
    headers: HeaderMap,
    Query(selection): Query<Selection>,
) -> Result<Response, Error> {
    let now = OffsetDateTime::now_utc();
    let range = selection.range(now.date(), state.extended)?;
    let lang = match selection.lang.as_deref() {
        Some("en") => "en",
        Some("pl") => "pl",
        _ => web::language(&headers),
    };
    let mut tx = data::snapshot(&state.pool).await?;
    let (worlds, last_online_update) = if state.extended {
        if let Some(world) = range.world {
            data::world_exists(&mut tx, world).await?;
        }
        (
            data::worlds(&mut tx).await?,
            data::last_online_update(&mut tx, range.world, now).await?,
        )
    } else {
        (Vec::new(), None)
    };
    let first_failure_day = data::first_failure_day(&mut tx).await?;
    let mut charts = Vec::new();
    for chart in Chart::ALL
        .into_iter()
        .filter(|chart| chart.available(state.extended))
    {
        let values = data::load(&mut tx, chart, range, now).await?;
        charts.push(Card::new(chart, values, range, first_failure_day));
    }
    tx.commit().await?;
    let labels = labels(lang).ok_or(Error::Missing)?;
    Ok(oteryn_web::render(&Page {
        lang,
        labels,
        preset: selection.range.unwrap_or_else(|| "7".to_owned()),
        from: range.from,
        to: range.end - Duration::days(1),
        previous_from: range.previous,
        previous_to: range.from - Duration::days(1),
        world: range.world,
        extended: state.extended,
        worlds,
        first_failure_day,
        last_online_update,
        export,
        csv_query: range.csv_query(lang),
        charts,
    }))
}

async fn export(
    State(state): State<Charts>,
    Path(file): Path<String>,
    Query(selection): Query<Selection>,
) -> Result<Response, Error> {
    let chart = Chart::from_file(&file, state.extended)?;
    let now = OffsetDateTime::now_utc();
    let range = selection.range(now.date(), state.extended)?;
    let mut tx = data::snapshot(&state.pool).await?;
    if let Some(world) = range.world {
        data::world_exists(&mut tx, world).await?;
    }
    let first_failure_day = data::first_failure_day(&mut tx).await?;
    let values = data::load(&mut tx, chart, range, now).await?;
    tx.commit().await?;
    let card = Card::new(chart, values, range, first_failure_day);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.csv\"", chart.key()),
            ),
        ],
        card.csv(),
    )
        .into_response())
}

#[cfg(test)]
mod tests;
