use super::{Entry, Error, browser_session_expiry, player_entries, submit_player};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{FromRequest, Path, Query, Request, State, rejection::PathRejection},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Keys, RateLimiter},
    session::{BrowserSession, SessionLayer, SessionMutationDeadline, with_post_deadline},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

const APPEAL_DETAIL: &str = "/{lang}/support/appeals/{id}";

/// Apply outside shared chrome so appeal POSTs use one budget from their first lookup.
pub fn with_browser_deadline(app: Router) -> Router {
    with_post_deadline(app, |path| path == APPEAL_DETAIL)
}

pub(crate) fn router(pool: PgPool, origin: &str, submissions: bool) -> Router {
    let mut session = SessionLayer::new(pool.clone()).with_bounded_refresh();
    session.csrf.trusted_origins = vec![origin.to_owned()];
    let sessions = middleware::from_fn_with_state(session, oteryn_web::session::middleware);
    Router::new()
        .route(
            "/{lang}/support/appeals",
            get(index).route_layer(sessions.clone()),
        )
        .route(
            APPEAL_DETAIL,
            (if submissions {
                get(detail).post(submit)
            } else {
                get(detail)
            })
            .route_layer(sessions),
        )
        .layer(Extension(Submissions(submissions)))
        .route_layer(middleware::from_fn(localize_refusal))
        .layer(middleware::from_fn(crate::tickets::private))
        .with_state(pool)
}

#[derive(Clone, Copy)]
struct Submissions(bool);

#[derive(Template)]
#[template(path = "appeals/page.html")]
struct Page<'a> {
    lang: &'a str,
    rows: Vec<Entry>,
    detail: bool,
    csrf: String,
    next: Option<u32>,
}

impl Page<'_> {
    fn label(&self, key: &str) -> &'static str {
        let (pl, en) = match key {
            "none" => ("Brak odwołania", "No appeal"),
            "requested" => ("Oczekuje na rozpatrzenie", "Awaiting review"),
            "reviewing" => ("W trakcie rozpatrywania", "Under review"),
            "accepted" => ("Uwzględnione", "Overturned"),
            "rejected" => ("Odrzucone", "Upheld"),
            "empty" => (
                "Brak sankcji do wyświetlenia.",
                "There are no sanctions to display.",
            ),
            "message" => ("Uzasadnienie odwołania", "Reason for appeal"),
            "outcome" => ("Uzasadnienie decyzji", "Decision reason"),
            "submit" => ("Złóż odwołanie", "Submit appeal"),
            "next" => ("Następna strona", "Next page"),
            _ => ("Odwołania od sankcji", "Sanction appeals"),
        };
        if self.lang == "pl" { pl } else { en }
    }

    fn respond(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}

async fn localize_refusal(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let status = response.status();
    if matches!(
        status,
        StatusCode::FORBIDDEN | StatusCode::SERVICE_UNAVAILABLE
    ) && !response.headers().contains_key(header::CONTENT_TYPE)
        && let Some(lang @ ("en" | "pl")) = path.split('/').nth(1)
    {
        let localized = refusal(lang, &path, status, None);
        let (parts, body) = localized.into_parts();
        response.headers_mut().remove(header::CONTENT_LENGTH);
        response.headers_mut().extend(parts.headers);
        *response.body_mut() = body;
    }
    response
}

fn refusal(lang: &str, path: &str, status: StatusCode, message: Option<&str>) -> Response {
    let chrome = oteryn_ui::Registry::default().chrome(lang, path, oteryn_ui::Account::Anonymous);
    match message {
        Some(message) => {
            let mut response = oteryn_ui::error_with_message(&chrome, status, message);
            response
                .extensions_mut()
                .insert(oteryn_ui::RefusalMessage(message.to_owned()));
            response
        }
        None => oteryn_ui::error(&chrome, status),
    }
}

fn reauthenticate(lang: &str, path: &str) -> Response {
    let message = if lang == "pl" {
        "Zaloguj się ponownie, aby skorzystać z odwołań."
    } else {
        "Sign in again to use appeals."
    };
    refusal(lang, path, StatusCode::FORBIDDEN, Some(message))
}

async fn actor(
    pool: &PgPool,
    session: &BrowserSession,
    lang: &str,
    path: &str,
    deadline: Option<&SessionMutationDeadline>,
) -> Result<(Uuid, Vec<u8>), Response> {
    if !matches!(lang, "en" | "pl") {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    let id = session
        .account_id()
        .await
        .ok_or_else(|| reauthenticate(lang, path))?;
    let hash = session
        .identity_hash()
        .await
        .ok_or_else(|| reauthenticate(lang, path))?;
    let expiry = browser_session_expiry(pool, id, &hash);
    let expiry = match deadline {
        Some(deadline) => deadline.read(expiry).await,
        None => expiry.await,
    }
    .map_err(|_| refusal(lang, path, StatusCode::SERVICE_UNAVAILABLE, None))?;
    if expiry.is_none() {
        return Err(reauthenticate(lang, path));
    }
    Ok((id, hash))
}

fn status(error: Error) -> StatusCode {
    StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
}

#[derive(Deserialize)]
struct Paging {
    #[serde(default)]
    page: u32,
}

async fn index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Query(paging): Query<Paging>,
) -> Result<Response, Response> {
    let path = format!("/{lang}/support/appeals");
    let (actor, _) = actor(&pool, &session, &lang, &path, None).await?;
    let mut rows = player_entries(&pool, actor, None, i64::from(paging.page) * 20)
        .await
        .map_err(|error| status(error).into_response())?;
    let next = if rows.len() > 20 {
        rows.truncate(20);
        paging.page.checked_add(1)
    } else {
        None
    };
    Ok(Page {
        lang: &lang,
        rows,
        detail: false,
        csrf: String::new(),
        next,
    }
    .respond())
}

async fn detail(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Extension(submissions): Extension<Submissions>,
) -> Result<Response, Response> {
    let path = format!("/{lang}/support/appeals/{id}");
    let (actor, _) = actor(&pool, &session, &lang, &path, None).await?;
    let mut rows = player_entries(&pool, actor, Some(id), 0)
        .await
        .map_err(|error| status(error).into_response())?;
    let row = rows
        .first_mut()
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    row.can_submit &= submissions.0;
    let csrf = if row.can_submit {
        session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())?
    } else {
        String::new()
    };
    Ok(Page {
        lang: &lang,
        rows,
        detail: true,
        csrf,
        next: None,
    }
    .respond())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: i64,
    message: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

async fn submit(
    State(pool): State<PgPool>,
    target: Result<Path<(String, Uuid)>, PathRejection>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<ClientSource>,
    deadline: Option<Extension<SessionMutationDeadline>>,
    request: Request,
) -> Result<Redirect, Response> {
    let Path((lang, id)) = target.map_err(|rejection| {
        let path = request.uri().path();
        match path.split('/').nth(1) {
            Some(lang @ ("pl" | "en")) => refusal(lang, path, rejection.status(), None),
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    })?;
    let language = match lang.as_str() {
        "pl" => Language::Pl,
        "en" => Language::En,
        _ => return Err(StatusCode::NOT_FOUND.into_response()),
    };
    let path = format!("/{lang}/support/appeals/{id}");
    let error = |status| refusal(&lang, &path, status, None);
    let Extension(deadline) = deadline.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE))?;
    let Form(input) = deadline
        .read_request(Form::<Input>::from_request(request, &pool))
        .await
        .map_err(error)?
        .map_err(|rejection| error(rejection.status()))?;
    let (actor, session_hash) = actor(&pool, &session, &lang, &path, Some(&deadline)).await?;
    let peer = peer.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE))?;
    let mut keys = Keys::source(peer.0);
    keys.account = Some(actor.to_string());
    RateLimiter::new(pool.clone())
        .check_appeal(&keys, &deadline)
        .await
        .map_err(|denial| denial.response(language, false))?;
    submit_player(
        &pool,
        actor,
        &session_hash,
        id,
        input.version,
        &oteryn_web::normalize_textarea(&input.message),
        &deadline,
    )
    .await
    .map_err(|failure| match failure {
        Error::SessionRevoked => reauthenticate(&lang, &path),
        failure @ (Error::Conflict | Error::SanctionInactive) => {
            let (description, destination, label) = if matches!(failure, Error::SanctionInactive) {
                (
                    "appeals-inactive-description",
                    format!("/{lang}/support/appeals"),
                    "appeals-open-list",
                )
            } else {
                (
                    "appeals-conflict-description",
                    path.clone(),
                    "appeals-open-current",
                )
            };
            let chrome =
                oteryn_ui::Registry::default().chrome(&lang, &path, oteryn_ui::Account::Anonymous);
            oteryn_ui::error_with_link(
                &chrome,
                StatusCode::CONFLICT,
                &chrome.text(description),
                &destination,
                &chrome.text(label),
            )
        }
        other => error(status(other)),
    })?;
    Ok(Redirect::to(&path))
}
