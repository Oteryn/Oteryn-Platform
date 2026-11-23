use askama::Template;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use oteryn_web::Language;
use std::sync::LazyLock;
use time::{OffsetDateTime, UtcOffset};

type Bundle = FluentBundle<FluentResource>;

fn bundle(lang: &str, source: &str) -> Option<Bundle> {
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = Bundle::new_concurrent(vec![lang.parse().ok()?]);
    bundle.add_resource(resource).ok()?;
    bundle.set_use_isolating(false);
    Some(bundle)
}

pub(super) fn message(
    lang: &str,
    key: &str,
    args: Option<&FluentArgs<'_>>,
    access: bool,
) -> Option<String> {
    static PL: LazyLock<Option<Bundle>> = LazyLock::new(|| bundle("pl", include_str!("pl.ftl")));
    static EN: LazyLock<Option<Bundle>> = LazyLock::new(|| bundle("en", include_str!("en.ftl")));
    static ACCESS_PL: LazyLock<Option<Bundle>> =
        LazyLock::new(|| bundle("pl", include_str!("access.pl.ftl")));
    static ACCESS_EN: LazyLock<Option<Bundle>> =
        LazyLock::new(|| bundle("en", include_str!("access.en.ftl")));

    let bundle = match (lang, access) {
        ("pl", false) => PL.as_ref(),
        ("en", false) => EN.as_ref(),
        ("pl", true) => ACCESS_PL.as_ref(),
        ("en", true) => ACCESS_EN.as_ref(),
        _ => None,
    }?;
    let pattern = bundle.get_message(key)?.value()?;
    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, args, &mut errors);
    (errors.is_empty() && !value.trim().is_empty()).then(|| value.into_owned())
}

pub(super) fn select(value: Option<&str>, headers: &HeaderMap) -> Option<&'static str> {
    match value {
        Some("pl") => Some("pl"),
        Some("en") => Some("en"),
        None => Some(Language::negotiate(headers).code()),
        _ => None,
    }
}

pub(super) fn number(lang: &str, value: impl Into<i128>) -> String {
    let value = value.into().to_string();
    let (sign, digits) = value
        .strip_prefix('-')
        .map_or(("", value.as_str()), |digits| ("-", digits));
    let separator = if lang == "pl" { '\u{a0}' } else { ',' };
    let mut formatted = sign.to_owned();
    for (index, digit) in digits.bytes().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(separator);
        }
        formatted.push(char::from(digit));
    }
    formatted
}

pub(super) fn date(lang: &str, timestamp: OffsetDateTime) -> String {
    let timestamp = timestamp.to_offset(UtcOffset::UTC);
    let (year, month, day) = timestamp.to_calendar_date();
    let month = u8::from(month);
    let day = if lang == "pl" {
        format!("{day:02}.{month:02}.{year:04}")
    } else {
        format!("{year:04}-{month:02}-{day:02}")
    };
    let (hour, minute, second) = timestamp.to_hms();
    format!("{day} {hour:02}:{minute:02}:{second:02} UTC")
}

pub(super) fn seconds_ago(lang: &str, seconds: u64) -> String {
    let mut args = FluentArgs::new();
    args.set("count", seconds);
    args.set("number", number(lang, seconds));
    message(lang, "seconds-ago", Some(&args), false).unwrap_or_default()
}

#[derive(serde::Deserialize)]
struct LocaleQuery {
    lang: Option<String>,
}

#[derive(Template)]
#[template(path = "error.html", config = "src/console/templates/askama.toml")]
struct ErrorPage {
    lang: &'static str,
    message: String,
}

pub(super) async fn response_language(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let query = request
        .uri()
        .query()
        .and_then(|query| serde_urlencoded::from_str::<LocaleQuery>(query).ok());
    let lang = select(query.as_ref().and_then(|q| q.lang.as_deref()), headers)
        .unwrap_or_else(|| Language::negotiate(headers).code());
    let mut response = next.run(request).await;
    let key = match response.status() {
        StatusCode::BAD_REQUEST
        | StatusCode::PAYLOAD_TOO_LARGE
        | StatusCode::UNSUPPORTED_MEDIA_TYPE
        | StatusCode::UNPROCESSABLE_ENTITY => "invalid-form",
        StatusCode::FORBIDDEN => "access-denied",
        StatusCode::SERVICE_UNAVAILABLE => "unavailable",
        _ => "",
    };
    if !key.is_empty()
        && let Some(message) = message(lang, key, None, false)
        && let Ok(html) = (ErrorPage { lang, message }).render()
    {
        *response.body_mut() = Body::from(html);
        let headers = response.headers_mut();
        headers.remove(header::CONTENT_LENGTH);
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
    }
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_LANGUAGE, HeaderValue::from_static(lang));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.append(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn locale_override_negotiation_and_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("pl;q=0.3,en-GB;q=0.8"),
        );
        assert_eq!(select(None, &headers), Some("en"));
        assert_eq!(select(Some("pl"), &headers), Some("pl"));
        assert_eq!(select(Some("fr"), &headers), None);
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("portal_language=pl"),
        );
        assert_eq!(select(None, &headers), Some("pl"));
        assert_eq!(select(None, &HeaderMap::new()), Some("pl"));
    }

    #[tokio::test]
    async fn denied_and_invalid_responses_keep_status_and_use_requested_locale() {
        use axum::{Router, body::to_bytes, middleware, response::IntoResponse, routing::get};
        use tower::ServiceExt;

        let router = Router::new()
            .route(
                "/denied",
                get(|| async { StatusCode::FORBIDDEN.into_response() }),
            )
            .route(
                "/invalid",
                get(|| async { StatusCode::BAD_REQUEST.into_response() }),
            )
            .route_layer(middleware::from_fn(response_language));
        for (path, status, lang, message) in [
            (
                "/denied?lang=pl",
                StatusCode::FORBIDDEN,
                "pl",
                "Brak dostępu",
            ),
            (
                "/invalid?lang=en",
                StatusCode::BAD_REQUEST,
                "en",
                "Check the values in the form.",
            ),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::get(path)
                        .header(header::ACCEPT_LANGUAGE, "en")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
            let body = to_bytes(response.into_body(), 100_000).await.unwrap();
            assert!(String::from_utf8_lossy(&body).contains(message));
        }
    }

    #[test]
    fn time_and_quantity_use_locale_and_utc() {
        let time = OffsetDateTime::parse(
            "2026-02-10T23:45:11-05:00",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        assert_eq!(date("pl", time), "11.02.2026 04:45:11 UTC");
        assert_eq!(date("en", time), "2026-02-11 04:45:11 UTC");
        assert_eq!(number("en", -1234), "-1,234");
        assert_eq!(number("pl", 1234), "1\u{a0}234");
        assert_eq!(seconds_ago("en", 1), "One second ago");
        assert_eq!(seconds_ago("pl", 1), "Sekundę temu");
        assert_eq!(seconds_ago("pl", 2), "2 sekundy temu");
        assert_eq!(seconds_ago("pl", 12), "12 sekund temu");
        assert_eq!(message("pl", "absent-key", None, true), None);
    }
}
