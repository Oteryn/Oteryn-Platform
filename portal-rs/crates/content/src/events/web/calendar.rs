use super::{TimeLabel, display_date, locale, response, text};
use crate::{
    Pagination,
    events::{Error, Events, repository::CalendarEvent},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Redirect, Response},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use time::{Date, Month};

const ZONE_COOKIE: &str = "__Host-oteryn_events_zone";
const DEFAULT_ZONE: &str = "UTC";
const MAX_YEAR: i32 = 9998;
const PAGE_SIZE: i64 = 200;

pub(super) async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../../../templates/events/calendar.css"),
    )
}

pub(super) fn cookie_zone(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(name, value)| (name == ZONE_COOKIE && value.len() <= 64).then_some(value))
}

#[derive(Deserialize)]
pub(super) struct MonthQuery {
    month: Option<String>,
    #[serde(default)]
    page: u32,
}

fn parse_month(value: &str) -> Result<Date, Error> {
    let (year, month) = value.split_once('-').ok_or(Error::Invalid)?;
    if year.len() != 4
        || month.len() != 2
        || !year
            .bytes()
            .chain(month.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(Error::Invalid);
    }
    let year = year.parse().map_err(|_| Error::Invalid)?;
    if !(1..=MAX_YEAR).contains(&year) {
        return Err(Error::Invalid);
    }
    let month = Month::try_from(month.parse::<u8>().map_err(|_| Error::Invalid)?)
        .map_err(|_| Error::Invalid)?;
    Date::from_calendar_date(year, month, 1).map_err(|_| Error::Invalid)
}

fn month_key(first: Date) -> String {
    format!("{:04}-{:02}", first.year(), u8::from(first.month()))
}

fn month_start(year: i32, month: Month) -> Result<Date, Error> {
    Date::from_calendar_date(year, month, 1).map_err(|_| Error::Invalid)
}

struct CalendarItem {
    slug: String,
    title: String,
    first_day: Date,
    last_day: Date,
    start: TimeLabel,
    end: TimeLabel,
}

impl CalendarItem {
    fn new(lang: &str, event: CalendarEvent) -> Self {
        let first_day = event.local_start.date();
        Self {
            slug: event.slug,
            title: event.title,
            first_day: first_day.min(event.last_day),
            last_day: event.last_day.max(first_day),
            start: TimeLabel::new(lang, event.starts_at, event.local_start),
            end: TimeLabel::new(lang, event.ends_at, event.local_end),
        }
    }
}

struct CalendarDay<'a> {
    date: Date,
    label: String,
    items: Vec<&'a CalendarItem>,
}

#[derive(Template)]
#[template(path = "events/calendar.html", config = "src/events/askama.toml")]
struct CalendarPage<'a> {
    lang: &'a str,
    heading: String,
    title: String,
    list_label: String,
    empty_label: String,
    export_label: String,
    previous_label: String,
    next_label: String,
    month_label: String,
    show_month_label: String,
    zone_label: String,
    save_zone_label: String,
    month: String,
    previous: Option<String>,
    next: Option<String>,
    previous_page: Option<u32>,
    next_page: Option<u64>,
    previous_page_label: String,
    next_page_label: String,
    page: u32,
    weekdays: [String; 7],
    weeks: Vec<&'a [Option<&'a CalendarDay<'a>>]>,
    days: &'a [CalendarDay<'a>],
    is_empty: bool,
    csrf: String,
    zone: &'a str,
    zones: &'a [String],
}

pub(super) async fn show(
    State(events): State<Events>,
    Path(lang): Path<String>,
    Query(query): Query<MonthQuery>,
    headers: HeaderMap,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    match render_month(events, &lang, query, headers, session).await {
        Ok(page) => page,
        Err(error) => response(error),
    }
}

async fn render_month(
    events: Events,
    lang: &str,
    query: MonthQuery,
    headers: HeaderMap,
    session: BrowserSession,
) -> Result<Response, Error> {
    locale(lang)?;
    let first = query.month.as_deref().map(parse_month).transpose()?;
    let zones = events.time_zones(None).await?;
    let zone = cookie_zone(&headers)
        .filter(|name| zones.iter().any(|zone| zone == name))
        .unwrap_or(DEFAULT_ZONE);
    let first = match first {
        Some(first) => first,
        None => {
            let today = events.local_today(zone).await?;
            month_start(today.year(), today.month())?
        }
    };
    let month = first.month();
    let year = first.year();
    let next = month_start(year + i32::from(month == Month::December), month.next())?;
    let previous = month_start(year - i32::from(month == Month::January), month.previous())?;
    let last = next.previous_day().ok_or(Error::Invalid)?;
    let month_name = text(
        lang,
        &format!("month-{}", month.to_string().to_ascii_lowercase()),
    );

    let pagination = Pagination { page: query.page };
    let mut rows = events
        .calendar_events(
            lang,
            zone,
            first,
            next,
            pagination.offset(PAGE_SIZE),
            PAGE_SIZE + 1,
        )
        .await?;
    if query.page > 0 && rows.is_empty() {
        return Err(Error::NotFound);
    }
    let more = rows.len() as i64 > PAGE_SIZE;
    rows.truncate(PAGE_SIZE as usize);
    let items: Vec<_> = rows
        .into_iter()
        .map(|row| CalendarItem::new(lang, row))
        .collect();
    let mut days: Vec<_> = (first.day()..=month.length(year))
        .map(|day| {
            let date = Date::from_calendar_date(year, month, day).map_err(|_| Error::Invalid)?;
            Ok(CalendarDay {
                date,
                label: display_date(lang, date),
                items: Vec::new(),
            })
        })
        .collect::<Result<_, Error>>()?;
    for item in &items {
        let start = item.first_day.max(first);
        let end = item.last_day.min(last);
        if start <= end {
            for day in &mut days[usize::from(start.day() - 1)..usize::from(end.day())] {
                day.items.push(item);
            }
        }
    }
    let mut cells = vec![None; usize::from(first.weekday().number_days_from_monday())];
    cells.extend(days.iter().map(Some));
    cells.resize(cells.len().next_multiple_of(7), None);

    Ok(oteryn_web::render(&CalendarPage {
        lang,
        heading: text(lang, "calendar"),
        title: format!("{month_name} {year}"),
        list_label: text(lang, "events"),
        empty_label: text(lang, "empty-month"),
        export_label: text(lang, "export-upcoming"),
        previous_label: text(lang, "previous-month"),
        next_label: text(lang, "next-month"),
        month_label: text(lang, "month"),
        show_month_label: text(lang, "show-month"),
        zone_label: text(lang, "time-zone"),
        save_zone_label: text(lang, "save-time-zone"),
        month: month_key(first),
        previous: (previous.year() >= 1).then(|| month_key(previous)),
        next: (next.year() <= MAX_YEAR).then(|| month_key(next)),
        previous_page: query.page.checked_sub(1),
        next_page: more.then(|| pagination.next()),
        previous_page_label: text(lang, "previous-events"),
        next_page_label: text(lang, "next-events"),
        page: query.page,
        weekdays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
            .map(|day| text(lang, &format!("weekday-{day}"))),
        weeks: cells.chunks(7).collect(),
        days: &days,
        is_empty: days.iter().all(|day| day.items.is_empty()),
        csrf: session.csrf_token().await?,
        zone,
        zones: &zones,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ZoneForm {
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
    zone: String,
}

pub(super) async fn set_zone(
    State(events): State<Events>,
    Path(lang): Path<String>,
    Query(query): Query<MonthQuery>,
    Form(form): Form<ZoneForm>,
) -> Response {
    match save_zone(events, &lang, query, form).await {
        Ok(saved) => saved,
        Err(error) => response(error),
    }
}

async fn save_zone(
    events: Events,
    lang: &str,
    query: MonthQuery,
    form: ZoneForm,
) -> Result<Response, Error> {
    locale(lang)?;
    let month = query.month.as_deref().map(parse_month).transpose()?;
    if form.zone.len() > 64 || events.time_zones(Some(&form.zone)).await?.is_empty() {
        return Err(Error::Invalid);
    }

    // Every viewer can update the preference on their own browser.
    let cookie = HeaderValue::from_str(&format!(
        "{ZONE_COOKIE}={}; Path=/; Max-Age=31536000; Secure; HttpOnly; SameSite=Lax",
        form.zone
    ))
    .map_err(|_| Error::Invalid)?;
    let mut target = format!("/{lang}/events/calendar/month");
    if let Some(first) = month {
        target.push_str("?month=");
        target.push_str(&month_key(first));
    }
    if query.page > 0 {
        target.push(if month.is_some() { '&' } else { '?' });
        target.push_str(&format!("page={}", query.page));
    }
    Ok(([(header::SET_COOKIE, cookie)], Redirect::to(&target)).into_response())
}
