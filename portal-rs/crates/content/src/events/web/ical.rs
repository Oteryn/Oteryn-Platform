use super::{locale, response};
use crate::events::{Error, Events, repository::ExportEvent, valid_slug};
use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use time::{OffsetDateTime, UtcOffset};

pub(super) async fn single(
    State(events): State<Events>,
    Path((lang, slug)): Path<(String, String)>,
) -> Response {
    if !valid_slug(&slug) {
        return response(Error::NotFound);
    }
    export(events, &lang, Some(&slug)).await
}

pub(super) async fn upcoming(State(events): State<Events>, Path(lang): Path<String>) -> Response {
    export(events, &lang, None).await
}

async fn export(events: Events, lang: &str, slug: Option<&str>) -> Response {
    if let Err(error) = locale(lang) {
        return response(error);
    }
    let entries = match events.export_events(lang, slug).await {
        Ok(entries) => entries,
        Err(error) => return response(error),
    };
    if slug.is_some() && entries.is_empty() {
        return response(Error::NotFound);
    }

    let mut body = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Oteryn//Portal Events//EN\r\nCALSCALE:GREGORIAN\r\n",
    );
    for event in entries {
        append_event(&mut body, event);
    }
    body.push_str("END:VCALENDAR\r\n");
    let attachment = if slug.is_some() {
        "attachment; filename=\"oteryn-event.ics\""
    } else {
        "attachment; filename=\"oteryn-upcoming.ics\""
    };
    (
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (header::CONTENT_DISPOSITION, attachment),
        ],
        body,
    )
        .into_response()
}

fn append_event(calendar: &mut String, event: ExportEvent) {
    calendar.push_str("BEGIN:VEVENT\r\n");
    line(calendar, &format!("UID:{}@oteryn.com", event.id));
    line(
        calendar,
        &format!("DTSTAMP:{}", utc_stamp(event.updated_at)),
    );
    line(calendar, &format!("DTSTART:{}", utc_stamp(event.starts_at)));
    line(calendar, &format!("DTEND:{}", utc_stamp(event.ends_at)));
    line(calendar, &format!("SUMMARY:{}", escape_text(&event.title)));
    line(
        calendar,
        &format!("DESCRIPTION:{}", escape_text(&event.summary)),
    );
    calendar.push_str("END:VEVENT\r\n");
}

fn utc_stamp(date: OffsetDateTime) -> String {
    let date = date.to_offset(UtcOffset::UTC);
    let (year, month, day) = date.to_calendar_date();
    let (hour, minute, second) = date.to_hms();
    format!(
        "{year:04}{:02}{day:02}T{hour:02}{minute:02}{second:02}Z",
        u8::from(month)
    )
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.replace("\r\n", "\n").chars() {
        match ch {
            '\r' | '\n' => escaped.push_str("\\n"),
            '\\' | ',' | ';' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            ch if !ch.is_control() => escaped.push(ch),
            _ => continue,
        }
    }
    escaped
}

fn line(output: &mut String, value: &str) {
    // RFC 5545 folds at 75 octets. The continuation space counts toward the limit.
    let mut length = 0;
    for ch in value.chars() {
        if length + ch.len_utf8() > 75 {
            output.push_str("\r\n ");
            length = 1;
        }
        output.push(ch);
        length += ch.len_utf8();
    }
    output.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_utf8_content_lines_and_preserves_the_value() {
        let value = format!("SUMMARY:{}", "Żółć 🗓️ ".repeat(20));
        let mut output = String::new();
        line(&mut output, &value);
        assert!(output.split("\r\n").all(|line| line.len() <= 75));
        assert_eq!(output.replace("\r\n ", ""), format!("{value}\r\n"));
    }
}
