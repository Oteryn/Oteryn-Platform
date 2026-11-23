//! Publication windows shared by editorial news and pages.
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, thiserror::Error)]
#[error("invalid publication window")]
pub(crate) struct InvalidWindow;

#[derive(Debug)]
pub(crate) struct Window {
    pub(crate) start: OffsetDateTime,
    pub(crate) end: Option<OffsetDateTime>,
}

impl Window {
    pub(crate) fn parse(start: &str, end: &str) -> Result<Self, InvalidWindow> {
        let start = timestamp(start)?;
        let end = (!end.is_empty()).then(|| timestamp(end)).transpose()?;
        if end.is_some_and(|end| start >= end) {
            return Err(InvalidWindow);
        }
        Ok(Self { start, end })
    }
}

fn timestamp(value: &str) -> Result<OffsetDateTime, InvalidWindow> {
    // Use explicit offsets so an operator's time is independent of server time zone
    // and daylight-saving changes. RFC 3339 uses -00:00 for an unknown offset.
    if value.ends_with("-00:00") || value.len() > 35 {
        return Err(InvalidWindow);
    }
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| InvalidWindow)?;
    // PostgreSQL stores microseconds. Compare the same precision we persist.
    parsed
        .replace_nanosecond(parsed.nanosecond() / 1_000 * 1_000)
        .map_err(|_| InvalidWindow)
}

pub(crate) fn format_timestamp(value: Option<OffsetDateTime>) -> String {
    value
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_default()
}

pub(crate) fn status(
    now: OffsetDateTime,
    start: Option<OffsetDateTime>,
    end: Option<OffsetDateTime>,
    current: bool,
) -> &'static str {
    match start {
        None => "publication-draft",
        Some(_) if end.is_some_and(|end| end <= now) => "publication-expired",
        Some(_) if !current => "publication-review",
        Some(start) if now < start => "publication-queued",
        Some(_) => "publication-live",
    }
}

pub(crate) fn staff_language(headers: &axum::http::HeaderMap) -> &'static str {
    if headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|lang| lang.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    }
}

#[derive(askama::Template)]
#[template(
    path = "admin/pages/publication-error.html",
    config = "src/pages/askama.toml"
)]
struct PublicationError<'a> {
    lang: &'a str,
    message: String,
    back: String,
    url: &'a str,
}

pub(crate) fn form_error(
    lang: &str,
    status: axum::http::StatusCode,
    key: &str,
    url: &str,
) -> axum::response::Response {
    let mut response = oteryn_web::render(&PublicationError {
        lang,
        message: crate::pages::text(lang, key),
        back: crate::pages::text(lang, "publication-back"),
        url,
    });
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn paired_offsets_resolve_to_a_single_window() -> Result<(), InvalidWindow> {
        let window = Window::parse("2030-10-27T02:30:00+02:00", "2030-10-27T02:30:00+01:00")?;
        assert_eq!(window.start, datetime!(2030-10-27 00:30 UTC));
        assert_eq!(window.end, Some(datetime!(2030-10-27 01:30 UTC)));
        assert!(Window::parse("2030-10-27T02:30:00Z", "")?.end.is_none());
        Ok(())
    }

    #[test]
    fn publication_requires_an_ordered_window_with_known_offsets() {
        for (start, end) in [
            ("", ""),
            ("2030-10-27T02:30:00", ""),
            ("2030-10-27T02:30:00 Europe/Warsaw", ""),
            ("2030-10-27T02:30:00-00:00", ""),
            ("2030-10-27T02:30:00+25:00", ""),
            ("2030-10-27T02:30:00+02:00", "2030-10-27T00:30:00Z"),
            ("2030-10-27T02:30:00+02:00", "2030-10-27T00:29:00Z"),
            (
                "2030-10-27T02:30:00.0000001Z",
                "2030-10-27T02:30:00.0000009Z",
            ),
        ] {
            assert!(Window::parse(start, end).is_err(), "{start}, {end}");
        }
    }

    #[test]
    fn queue_status_observes_inclusive_start_and_exclusive_end() {
        let start = datetime!(2030-10-27 00:30 UTC);
        let end = datetime!(2030-10-27 01:30 UTC);
        assert_eq!(
            status(
                datetime!(2030-10-27 00:29 UTC),
                Some(start),
                Some(end),
                true
            ),
            "publication-queued"
        );
        assert_eq!(
            status(start, Some(start), Some(end), true),
            "publication-live"
        );
        assert_eq!(
            status(start, Some(start), Some(end), false),
            "publication-review"
        );
        assert_eq!(
            status(end, Some(start), Some(end), true),
            "publication-expired"
        );
        assert_eq!(status(start, None, None, true), "publication-draft");
    }
}
