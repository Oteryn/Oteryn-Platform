//! Browser-only adapters. Account IDs are resolved from server-side sessions.
use super::*;
use crate::quota::Action;
use axum::{
    Form, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    #[serde(default = "previous_week")]
    week: i64,
    format: Option<String>,
    dataset: Option<String>,
}
/// Header navigation selects the most recent closed UTC week without inventing data.
fn previous_week() -> i64 {
    let at = time::OffsetDateTime::now_utc();
    at.unix_timestamp().div_euclid(86400) * 86400
        - (i64::from(at.weekday().number_days_from_monday()) + 7) * 86400
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteForm {
    #[serde(rename = "_csrf", alias = "csrf")]
    csrf: String,
    body: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreferenceForm {
    #[serde(rename = "_csrf", alias = "csrf")]
    csrf: String,
    enabled: Option<String>,
}

fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled => StatusCode::SERVICE_UNAVAILABLE,
        Error::Denied => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
fn cookie(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for part in header.to_str().ok()?.split(';') {
            if let Some(("__Host-oteryn_session", value)) = part.trim().split_once('=') {
                if found.is_some() {
                    return None;
                }
                found = Some(value);
            }
        }
    }
    found.filter(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
async fn actor(
    service: &Reports,
    headers: &HeaderMap,
) -> std::result::Result<(Uuid, String), StatusCode> {
    service.enabled().map_err(status)?;
    browser_actor(service, headers).await
}

async fn browser_actor(
    service: &Reports,
    headers: &HeaderMap,
) -> std::result::Result<(Uuid, String), StatusCode> {
    let token = cookie(headers).ok_or(StatusCode::FORBIDDEN)?;
    sqlx::query_as("SELECT a.id,s.csrf_token FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.id_hash=$1 AND s.idle_expires_at>now() AND s.absolute_expires_at>now() AND a.status='active' AND a.email_verified_at IS NOT NULL")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).fetch_optional(&service.pool).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.ok_or(StatusCode::FORBIDDEN)
}
fn csrf(expected: &str, received: &str) -> std::result::Result<(), StatusCode> {
    if expected.len() < 32 || !bool::from(expected.as_bytes().ct_eq(received.as_bytes())) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
fn now() -> std::result::Result<i64, StatusCode> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}
async fn report(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(query): Query<Selection>,
) -> std::result::Result<Response, StatusCode> {
    let (account, token) = actor(&service, &headers).await?;
    let locale = Locale::parse(&lang).map_err(status)?;
    let format = query.format.as_deref().unwrap_or("html");
    let dataset = query.dataset.as_deref().unwrap_or("sessions");
    if !matches!(
        (format, dataset),
        ("html", "sessions") | ("json" | "csv", "sessions" | "drops")
    ) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let report = match service.weekly(account, query.week, now()?).await {
        Ok(report) => report,
        Err(Error::Unavailable) if format == "html" => {
            return Ok((
                StatusCode::SERVICE_UNAVAILABLE,
                Html(render::unavailable(locale, query.week).map_err(status)?),
            )
                .into_response());
        }
        Err(error) => return Err(status(error)),
    };
    match (format, dataset) {
        ("html", "sessions") => Ok(Html(
            render::weekly(
                locale,
                &report,
                &token,
                service.email_enabled(account).await.map_err(status)?,
            )
            .map_err(status)?,
        )
        .into_response()),
        ("json", "sessions" | "drops") => Ok((
            [(
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"journal.json\"",
            )],
            Json(report),
        )
            .into_response()),
        ("csv", dataset @ ("sessions" | "drops")) => Ok((
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    header::CONTENT_DISPOSITION,
                    if dataset == "drops" {
                        "attachment; filename=\"drops.csv\""
                    } else {
                        "attachment; filename=\"sessions.csv\""
                    },
                ),
            ],
            csv(&report, dataset == "drops"),
        )
            .into_response()),
        _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
}
async fn note(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(target): Query<Target>,
) -> std::result::Result<Response, StatusCode> {
    let (account, token) = actor(&service, &headers).await?;
    let locale = Locale::parse(&lang).map_err(status)?;
    let body = service
        .note(account, target)
        .await
        .map_err(status)?
        .unwrap_or_default();
    Ok(Html(render::note(locale, &token, &body).map_err(status)?).into_response())
}
async fn save_note(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Form(form): Form<NoteForm>,
) -> std::result::Result<StatusCode, StatusCode> {
    Locale::parse(&lang).map_err(status)?;
    let (account, token) = actor(&service, &headers).await?;
    csrf(&token, &form.csrf)?;
    service
        .save_note(account, target, &form.body)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn preference(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Form(form): Form<PreferenceForm>,
) -> std::result::Result<StatusCode, StatusCode> {
    Locale::parse(&lang).map_err(status)?;
    let (account, token) = actor(&service, &headers).await?;
    csrf(&token, &form.csrf)?;
    if form.enabled.as_deref().is_some_and(|v| v != "true") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    service
        .email_preference(account, form.enabled.is_some())
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn notes_export(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
) -> std::result::Result<Response, StatusCode> {
    Locale::parse(&lang).map_err(status)?;
    let (account, _) = actor(&service, &headers).await?;
    let notes = service.export_notes(account).await.map_err(status)?;
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"journal-notes.json\"",
        )],
        Json(notes),
    )
        .into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteWeek {
    #[serde(default = "previous_week")]
    week: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteExport {
    id: Option<Uuid>,
    format: Option<String>,
}

async fn weekly_note(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(query): Query<NoteWeek>,
) -> std::result::Result<Html<String>, StatusCode> {
    let locale = Locale::parse(&lang).map_err(status)?;
    let (account, token) = browser_actor(&service, &headers).await?;
    let clock = now()?;
    let week = Week::closed(query.week, clock).map_err(status)?;
    let note = service.weekly_note(account, week).await.map_err(status)?;
    Ok(Html(
        render::weekly_note(
            locale,
            week,
            clock,
            &token,
            note.as_ref().map_or("", |note| &note.body),
            false,
        )
        .map_err(status)?,
    ))
}

async fn save_weekly_note(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(query): Query<NoteWeek>,
    Form(form): Form<NoteForm>,
) -> std::result::Result<Response, StatusCode> {
    let locale = Locale::parse(&lang).map_err(status)?;
    let (account, token) = browser_actor(&service, &headers).await?;
    csrf(&token, &form.csrf)?;
    let clock = now()?;
    let week = Week::closed(query.week, clock).map_err(status)?;
    Action::WeeklyNoteSave
        .charge(&service.pool, account)
        .await?;
    let body = form.body.replace("\r\n", "\n");
    if let Err(error) = service.save_weekly_note(account, week, &body).await {
        if error == Error::Invalid {
            let html =
                render::weekly_note(locale, week, clock, &token, &body, true).map_err(status)?;
            return Ok((StatusCode::UNPROCESSABLE_ENTITY, Html(html)).into_response());
        }
        return Err(status(error));
    }
    Ok(Redirect::to(&format!(
        "/{}/account/journal/reports/weekly-note?week={}",
        locale.code(),
        week.start
    ))
    .into_response())
}

async fn weekly_note_export(
    State(service): State<Reports>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(query): Query<NoteExport>,
) -> std::result::Result<Response, StatusCode> {
    Locale::parse(&lang).map_err(status)?;
    let (account, _) = browser_actor(&service, &headers).await?;
    if query
        .format
        .as_deref()
        .is_some_and(|format| format != "json")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Action::WeeklyNoteExport
        .charge(&service.pool, account)
        .await?;
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"weekly-journal-notes.json\"",
        )],
        Json(
            service
                .export_weekly_notes(account, query.id)
                .await
                .map_err(status)?,
        ),
    )
        .into_response())
}

/// Applies even to extractor failures, preventing accidental caching of errors.
async fn private(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY,HeaderValue::from_static("default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response
}
pub fn router(service: Reports) -> Router {
    let mut sessions = oteryn_web::session::SessionLayer::new(service.pool.clone());
    sessions.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    Router::new()
        .route("/{locale}/account/journal/reports", get(report))
        .route(
            "/{locale}/account/journal/reports/notes",
            get(note).post(save_note),
        )
        .route(
            "/{locale}/account/journal/reports/notes/export",
            get(notes_export),
        )
        .route("/{locale}/account/journal/reports/email", post(preference))
        .route(
            "/{locale}/account/journal/reports/weekly-note",
            // URL encoding can use nine bytes per UTF-16 code unit in the textarea.
            get(weekly_note)
                .post(save_weekly_note)
                .layer(DefaultBodyLimit::max(36 * 1024)),
        )
        .route(
            "/{locale}/account/journal/reports/weekly-note/export",
            get(weekly_note_export),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(16384))
        .layer(axum::middleware::from_fn(private))
        .with_state(service)
}
/// No source is inferred from public character projections or unaccepted wire.
/// The dependency owner wires a current, authoritative JournalSource explicitly.
pub fn configured_router(pool: PgPool) -> Router {
    router(Reports {
        pool,
        source: Arc::new(DenySource),
        flags: Flags {
            enabled: std::env::var("OTERYN_JOURNAL_REPORTS_ENABLED").is_ok_and(|v| v == "true"),
            weekly_email: false,
        },
    })
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn missing_week_selects_a_closed_monday_and_bad_input_stays_refused()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let selection =
            Query::<Selection>::try_from_uri(&"/en/account/journal/reports".parse()?)?.0;
        let start = time::OffsetDateTime::from_unix_timestamp(selection.week)?;
        assert_eq!(start.weekday(), time::Weekday::Monday);
        assert_eq!(start.time(), time::Time::MIDNIGHT);
        assert!(selection.week + 7 * 86400 <= time::OffsetDateTime::now_utc().unix_timestamp());
        assert!(
            Query::<Selection>::try_from_uri(&"/en/account/journal/reports?week=invalid".parse()?)
                .is_err()
        );
        assert!(
            Query::<Selection>::try_from_uri(&"/en/account/journal/reports?unknown=value".parse()?)
                .is_err()
        );
        Ok(())
    }
}
