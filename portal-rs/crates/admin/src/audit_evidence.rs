//! Closed UTC days of Platform audit evidence.
mod store;

use crate::{
    rbac::{self, AccountStateReader, CheckContext, MutationAuthorization},
    web::{Panel, StaffSession, language, text},
};
use askama::Template;
use axum::{
    Extension, Form,
    extract::{State, rejection::FormRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use std::sync::Arc;
use time::{Date, format_description::well_known::Iso8601};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid audit evidence date")]
    Invalid,
    #[error("day outside the audit evidence window")]
    OutsideWindow,
    #[error("audit evidence exceeds the daily export limit")]
    Limit,
    #[error("audit evidence export quota reached")]
    Limited(i64),
    #[error("audit evidence cannot be verified")]
    Integrity,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error("audit evidence is unavailable")]
    Unavailable,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}

impl Error {
    fn response(&self, lang: &str) -> Response {
        let (status, key) = match self {
            Self::Invalid => (StatusCode::BAD_REQUEST, "audit-evidence-invalid"),
            Self::OutsideWindow => (StatusCode::NOT_FOUND, "audit-evidence-window"),
            Self::Limit => (StatusCode::PAYLOAD_TOO_LARGE, "audit-evidence-limit"),
            Self::Limited(_) => (StatusCode::TOO_MANY_REQUESTS, "audit-evidence-limited"),
            Self::Integrity => (StatusCode::CONFLICT, "audit-evidence-integrity"),
            Self::Authorization(rbac::Error::Denied) => {
                (StatusCode::FORBIDDEN, "audit-evidence-denied")
            }
            _ => (
                StatusCode::SERVICE_UNAVAILABLE,
                "audit-evidence-unavailable",
            ),
        };
        let mut response = private((status, text(lang, key)).into_response());
        if let Self::Limited(delay) = self {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from(*delay));
        }
        response
    }
}

fn private(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

#[derive(Template)]
#[template(path = "admin/audit_evidence.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    description: String,
    panel: String,
    day: String,
    download: String,
    csrf: String,
    first_day: String,
    last_day: String,
}

pub(super) async fn page<I: StaffSession, A: AccountStateReader>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    let Ok((first_day, last_day)) = store::window(&state.pool).await else {
        return Error::Unavailable.response(lang);
    };
    let Ok(csrf) = session.csrf_token().await else {
        return Error::Unavailable.response(lang);
    };
    private(oteryn_web::render(&Page {
        lang,
        title: text(lang, "audit-evidence-title"),
        description: text(lang, "audit-evidence-description"),
        panel: text(lang, "title"),
        day: text(lang, "audit-evidence-day"),
        download: text(lang, "audit-evidence-download"),
        csrf,
        first_day,
        last_day,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportForm {
    day: String,
    #[serde(default)]
    _csrf: String,
}

fn parse_day(day: &str) -> Result<Date, Error> {
    let result = Date::parse(day, &Iso8601::DATE).map_err(|_| Error::Invalid)?;
    if day.len() != 10 || result.year() < 1 || result.to_string() != day {
        return Err(Error::Invalid);
    }
    Ok(result)
}

pub(super) async fn export<I: StaffSession, A: AccountStateReader>(
    State(state): State<Arc<Panel<I, A>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    headers: HeaderMap,
    form: Result<Form<ExportForm>, FormRejection>,
) -> Response {
    let lang = language(&headers);
    let Ok(Form(form)) = form else {
        return Error::Invalid.response(lang);
    };
    if let Err(error) = parse_day(&form.day) {
        return error.response(lang);
    }
    if let Err(error) = store::charge_export(&state.pool, actor.account).await {
        return error.response(lang);
    }
    match store::export(&state.pool, &authority, actor, &form.day).await {
        Ok(body) => private(
            (
                [
                    (header::CONTENT_TYPE, "application/json".to_owned()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"audit-evidence-{}.json\"", form.day),
                    ),
                ],
                body,
            )
                .into_response(),
        ),
        Err(error) => error.response(lang),
    }
}

#[cfg(test)]
mod tests;
