use super::{private, text};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::FormRejection},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::staff_account::{self, Edit, Field};
use serde::Deserialize;
use sqlx::PgPool;
use std::{future::Future, marker::PhantomData};
use uuid::Uuid;

pub struct DisplaySession {
    pub account: Uuid,
    pub hash: Vec<u8>,
    pub csrf: String,
}

impl std::fmt::Debug for DisplaySession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DisplaySession")
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct Web<S, F> {
    pool: PgPool,
    session: F,
    marker: PhantomData<S>,
}

/// Mount inside the standard browser session and CSRF layers.
pub fn display_routes<S, F, Fut, A>(
    pool: PgPool,
    session: F,
    rendered_page: impl Clone + Send + Sync + 'static,
) -> Router<A>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<Option<DisplaySession>, sqlx::Error>> + Send + 'static,
    A: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/{lang}/account/display",
            get(edit::<S, F, Fut>).post(save::<S, F, Fut>),
        )
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::map_response(
            move |mut response: Response| {
                if response.status() == StatusCode::UNPROCESSABLE_ENTITY {
                    response.extensions_mut().insert(rendered_page.clone());
                }
                std::future::ready(response)
            },
        ))
        .with_state(Web {
            pool,
            session,
            marker: PhantomData,
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(rename = "_csrf")]
    _csrf: String,
    display_name: String,
    locale: String,
}

impl Input {
    fn edits(&self) -> Result<[Edit; 2], staff_account::Error> {
        Ok([
            Edit::parse(Field::DisplayName, &self.display_name)?,
            Edit::parse(Field::Locale, &self.locale)?,
        ])
    }
}

#[derive(Template)]
#[template(path = "account/display.html", config = "src/overview/askama.toml")]
struct Page<'a> {
    lang: &'a str,
    csrf: &'a str,
    name: &'a str,
    locale: &'a str,
    invalid: bool,
    saved: bool,
    back_to_overview: bool,
}

impl Page<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("account display access refused")]
    Denied,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
    #[error(transparent)]
    Edit(#[from] staff_account::Error),
}

async fn settings(
    pool: &PgPool,
    session: &DisplaySession,
    edits: Option<[Edit; 2]>,
) -> Result<staff_account::Details, Error> {
    let mut tx = pool.begin().await?;
    // Identity changes take the account lock before touching sessions.
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE id=$1 AND status='active'
         AND email_verified_at IS NOT NULL FOR UPDATE",
    )
    .bind(session.account)
    .fetch_optional(&mut *tx)
    .await?;
    let id = id.ok_or(Error::Denied)?;
    let session_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT session_id FROM web_sessions WHERE account_id=$1 AND id_hash=$2
         AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()
         FOR SHARE",
    )
    .bind(id)
    .bind(&session.hash)
    .fetch_optional(&mut *tx)
    .await?;
    if session_id.is_none() {
        return Err(Error::Denied);
    }
    let mut current = staff_account::details(&mut tx, id)
        .await?
        .ok_or(Error::Denied)?;
    if let Some(edits) = edits {
        let mut changed = false;
        for edit in edits {
            changed |= edit.apply(&mut tx, id, &current).await?;
        }
        if changed {
            current = staff_account::details(&mut tx, id)
                .await?
                .ok_or(Error::Denied)?;
        }
    }
    tx.commit().await?;
    Ok(current)
}

fn failure(lang: &str, error: Error) -> Response {
    match error {
        Error::Denied => (StatusCode::FORBIDDEN, text(lang, "display-denied")).into_response(),
        _ => (StatusCode::SERVICE_UNAVAILABLE, text(lang, "unavailable")).into_response(),
    }
}

fn render(
    lang: &str,
    csrf: &str,
    current: &staff_account::Details,
    invalid: bool,
    saved: bool,
) -> Response {
    let page = Page {
        lang,
        csrf,
        name: current.value(Field::DisplayName),
        locale: current.value(Field::Locale),
        invalid,
        saved,
        back_to_overview: oteryn_identity::navigation::overview_enabled(),
    };
    match page.render() {
        Ok(html) => (
            if invalid {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            },
            Html(html),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn handle<S, F, Fut>(
    web: Web<S, F>,
    session: S,
    lang: String,
    input: Option<Result<Form<Input>, FormRejection>>,
) -> Response
where
    F: Fn(S) -> Fut,
    Fut: Future<Output = Result<Option<DisplaySession>, sqlx::Error>>,
{
    if !matches!(lang.as_str(), "en" | "pl") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let session = match (web.session)(session).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to(&format!("/{lang}/login")).into_response(),
        Err(error) => return failure(&lang, Error::from(error)),
    };
    let updating = input.is_some();
    let edits = match input {
        Some(Ok(Form(input))) => input.edits().map(Some).map_err(|_| ()),
        Some(Err(_)) => Err(()),
        None => Ok(None),
    };
    let invalid = edits.is_err();
    match settings(&web.pool, &session, edits.ok().flatten()).await {
        Ok(current) => render(
            &lang,
            &session.csrf,
            &current,
            invalid,
            updating && !invalid,
        ),
        Err(error) => failure(&lang, error),
    }
}

async fn edit<S, F, Fut>(
    State(web): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path(lang): Path<String>,
) -> Response
where
    F: Fn(S) -> Fut,
    Fut: Future<Output = Result<Option<DisplaySession>, sqlx::Error>>,
{
    private(handle(web, session, lang, None).await)
}

async fn save<S, F, Fut>(
    State(web): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path(lang): Path<String>,
    input: Result<Form<Input>, FormRejection>,
) -> Response
where
    F: Fn(S) -> Fut,
    Fut: Future<Output = Result<Option<DisplaySession>, sqlx::Error>>,
{
    private(handle(web, session, lang, Some(input)).await)
}
