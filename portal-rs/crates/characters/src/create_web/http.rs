use super::{page, service::*};
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{future::Future, marker::PhantomData};
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Adapter to the owning browser session crate, avoiding a web/domain dependency cycle.
/// The router must be mounted inside the web session/CSRF middleware.
#[derive(Clone, Default)]
pub struct BrowserContext {
    pub account_id: Option<Uuid>,
    pub csrf_token: String,
    pub flash: Option<Flash>,
}
impl std::fmt::Debug for BrowserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserContext")
            .field("account_id", &self.account_id)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flash {
    pub name: String,
    pub world: String,
    pub submission: String,
    pub error: String,
}
#[derive(Clone, Debug)]
pub enum SessionAction {
    Peek,
    Take,
    Store(Flash),
}
#[derive(Clone)]
struct Web<S, F> {
    creation: Creation,
    session: F,
    marker: PhantomData<S>,
}

/// Uses exact true flags; missing configuration keeps both account and game features off.
pub fn configured_router<S, F, Fut, A>(pool: PgPool, session: F) -> Router<A>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send + 'static,
    A: Clone + Send + Sync + 'static,
{
    let flag = |key| std::env::var(key).is_ok_and(|v| v == "true");
    router(
        pool,
        Settings {
            enabled: flag("OTERYN_CHARACTER_CREATE_WEB_ENABLED"),
            commands: crate::commands::Settings {
                enabled: flag("OTERYN_CHARACTER_COMMANDS_ENABLED"),
                create_enabled: flag("OTERYN_CHARACTER_COMMAND_CREATE_ENABLED"),
                source_authority: std::env::var("OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY")
                    .unwrap_or_default(),
                ..Default::default()
            },
        },
        session,
    )
}
pub fn router<S, F, Fut, A>(pool: PgPool, settings: Settings, session: F) -> Router<A>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send + 'static,
    A: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/{lang}/account/characters/new",
            get(form::<S, F, Fut>).post(submit::<S, F, Fut>),
        )
        .route(
            "/{lang}/account/characters/requests/{operation}",
            get(status::<S, F, Fut>),
        )
        .route(
            "/{lang}/account/characters/suggest",
            get(suggest::<S, F, Fut>),
        )
        .layer(DefaultBodyLimit::max(4096))
        .with_state(Web {
            creation: Creation::new(pool, settings),
            session,
            marker: PhantomData,
        })
}
fn locale(lang: &str) -> Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
fn redirect(location: String) -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, location),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}
fn account(context: &BrowserContext) -> Result<Uuid, StatusCode> {
    context.account_id.ok_or(StatusCode::FORBIDDEN)
}
async fn context<S, F, Fut>(
    state: &Web<S, F>,
    session: S,
    action: SessionAction,
) -> Result<BrowserContext, StatusCode>
where
    F: Fn(S, SessionAction) -> Fut,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>>,
{
    (state.session)(session, action)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct WorldFilters {
    pub pvp: String,
    pub location: String,
}
async fn form<S, F, Fut>(
    State(state): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path(lang): Path<String>,
    filters: Result<Query<WorldFilters>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, StatusCode>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send,
{
    locale(&lang)?;
    let context = context(&state, session, SessionAction::Take).await?;
    let account = account(&context)?;
    if !state.creation.available() {
        return Ok(page::notice(&lang, StatusCode::OK, "unavailable"));
    }
    if let Err(error) = state.creation.eligible(account).await {
        return Ok(failure(&lang, error));
    }
    let worlds = match state.creation.worlds().await {
        Ok(worlds) if !worlds.is_empty() => worlds,
        _ => return Ok(failure(&lang, Error::Unavailable)),
    };
    let filters = match filters {
        Ok(Query(filters)) => filters,
        Err(_) => {
            return Ok(page::notice(
                &lang,
                StatusCode::BAD_REQUEST,
                "filter-invalid",
            ));
        }
    };
    if (!filters.pvp.is_empty() && !worlds.iter().any(|world| world.pvp == filters.pvp))
        || (!filters.location.is_empty()
            && !worlds
                .iter()
                .any(|world| world.location == filters.location))
    {
        return Ok(page::notice(
            &lang,
            StatusCode::BAD_REQUEST,
            "filter-invalid",
        ));
    }
    Ok(page::form(
        &lang,
        &context.csrf_token,
        context.flash,
        worlds,
        filters,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(rename = "_csrf")]
    csrf: String,
    name: String,
    world: String,
    submission: String,
}
async fn submit<S, F, Fut>(
    State(state): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path(lang): Path<String>,
    input: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Result<Response, StatusCode>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send,
{
    locale(&lang)?;
    let ctx = context(&state, session.clone(), SessionAction::Peek).await?;
    let account = account(&ctx)?;
    let input = match input {
        Ok(Form(input)) => input,
        Err(rejection) => return Ok(page::notice(&lang, rejection.status(), "name-invalid")),
    };
    if input.csrf.len() != 64 || !bool::from(ctx.csrf_token.as_bytes().ct_eq(input.csrf.as_bytes()))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let parsed = canonical_uuid(&input.submission).and_then(|id| {
        Ok(Submission {
            id,
            world: canonical_uuid(&input.world)?,
            name: input.name.clone(),
        })
    });
    let result = match parsed {
        Ok(input) => state.creation.submit(account, input).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(operation) => Ok(redirect(format!(
            "/{lang}/account/characters/requests/{operation}"
        ))),
        Err(Error::Limited) => Ok(failure(&lang, Error::Limited)),
        Err(error) => {
            let flash = Flash {
                name: input.name.chars().take(255).collect(),
                world: input.world.chars().take(36).collect(),
                submission: input.submission.chars().take(36).collect(),
                error: error_key(&error).into(),
            };
            context(&state, session, SessionAction::Store(flash)).await?;
            Ok(redirect(format!("/{lang}/account/characters/new")))
        }
    }
}
async fn suggest<S, F, Fut>(
    State(state): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path(lang): Path<String>,
) -> Result<Response, StatusCode>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send,
{
    locale(&lang)?;
    let ctx = context(&state, session.clone(), SessionAction::Peek).await?;
    let account = account(&ctx)?;
    if !state.creation.available() {
        return Ok(failure(&lang, Error::Unavailable));
    }
    let result = async {
        state.creation.eligible(account).await?;
        state.creation.limit(account, "read").await?;
        state.creation.suggest().await
    }
    .await;
    match result {
        Ok(name) => {
            context(
                &state,
                session,
                SessionAction::Store(Flash {
                    name,
                    world: String::new(),
                    submission: Uuid::now_v7().to_string(),
                    error: "suggestion-note".into(),
                }),
            )
            .await?;
            Ok(redirect(format!("/{lang}/account/characters/new")))
        }
        Err(error) => Ok(failure(&lang, error)),
    }
}
async fn status<S, F, Fut>(
    State(state): State<Web<S, F>>,
    Extension(session): Extension<S>,
    Path((lang, operation)): Path<(String, String)>,
) -> Result<Response, StatusCode>
where
    S: Clone + Send + Sync + 'static,
    F: Fn(S, SessionAction) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<BrowserContext, sqlx::Error>> + Send,
{
    locale(&lang)?;
    let ctx = context(&state, session, SessionAction::Peek).await?;
    let account = account(&ctx)?;
    let operation = canonical_uuid(&operation).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(match state.creation.status(account, operation).await {
        Ok(outcome) => page::notice(
            &lang,
            StatusCode::OK,
            match outcome {
                Outcome::Pending => "pending",
                Outcome::Created => "created",
                Outcome::Rejected(key) => key,
            },
        ),
        Err(error) => failure(&lang, error),
    })
}
fn canonical_uuid(raw: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(raw).map_err(|_| Error::Invalid)?;
    if id.get_version_num() != 7 || id.to_string() != raw {
        return Err(Error::Invalid);
    }
    Ok(id)
}
fn error_key(error: &Error) -> &'static str {
    match error {
        Error::Invalid => "name-invalid",
        Error::Reserved => "reserved",
        Error::Unavailable => "unavailable",
        Error::Denied => "denied",
        Error::NotFound => "missing",
        Error::Limited => "rate",
    }
}
fn failure(lang: &str, error: Error) -> Response {
    let status = match error {
        Error::Invalid | Error::Reserved => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Limited => StatusCode::TOO_MANY_REQUESTS,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    let mut response = page::notice(lang, status, error_key(&error));
    if matches!(error, Error::Limited) {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("60"),
        );
    }
    response
}
pub(super) fn valid_submission(raw: &str) -> bool {
    canonical_uuid(raw).is_ok()
}
