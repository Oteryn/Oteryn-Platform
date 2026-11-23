use super::{
    Error,
    player::{self, LAST_PAGE, PAGE_SIZE, PlayerAction, Sanction},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        DefaultBodyLimit, Path, Query, State,
        rejection::{FormRejection, QueryRejection},
    },
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::LazyLock;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub fn router(pool: PgPool, origin: &str) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![origin.to_owned()];
    Router::new()
        .route("/{lang}/account/sanctions", get(history))
        .route("/{lang}/account/sanctions/{id}", get(detail))
        .route(
            "/{lang}/account/sanctions/{id}/acknowledge",
            post(acknowledge),
        )
        .route("/{lang}/account/sanctions/{id}/appeal", post(appeal))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn(crate::tickets::private_headers))
        .with_state(pool)
}

#[derive(Clone, Copy)]
struct Locale(&'static str);

impl Locale {
    fn parse(lang: &str) -> Result<Self, Failure> {
        match lang {
            "pl" => Ok(Self("pl")),
            "en" => Ok(Self("en")),
            _ => Err(Failure::Language),
        }
    }

    fn text(self, key: &str) -> String {
        type Catalog = FluentBundle<FluentResource>;
        fn catalog(lang: &str, source: &str) -> Option<Catalog> {
            let mut result = Catalog::new_concurrent(vec![lang.parse().ok()?]);
            result
                .add_resource(FluentResource::try_new(source.to_owned()).ok()?)
                .ok()?;
            Some(result)
        }
        static PL: LazyLock<Option<Catalog>> =
            LazyLock::new(|| catalog("pl", include_str!("pl.ftl")));
        static EN: LazyLock<Option<Catalog>> =
            LazyLock::new(|| catalog("en", include_str!("en.ftl")));
        let bundle = if self.0 == "pl" { &PL } else { &EN };
        bundle
            .as_ref()
            .and_then(|bundle| {
                let pattern = bundle.get_message(key)?.value()?;
                Some(
                    bundle
                        .format_pattern(pattern, None, &mut vec![])
                        .into_owned(),
                )
            })
            .unwrap_or_default()
    }

    fn base(self) -> String {
        format!("/{}/account/sanctions", self.0)
    }
}

#[derive(Template)]
#[template(path = "support/sanctions.html", config = "src/appeals/askama.toml")]
struct Page {
    locale: Locale,
    rows: Vec<Sanction>,
    csrf: String,
    detail: bool,
    previous: Option<u32>,
    next: Option<u32>,
    error: Option<&'static str>,
}

impl Page {
    fn new(locale: Locale) -> Self {
        Self {
            locale,
            rows: vec![],
            csrf: String::new(),
            detail: false,
            previous: None,
            next: None,
            error: None,
        }
    }

    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }

    fn url(&self, id: &Uuid) -> String {
        format!("{}/{id}", self.locale.base())
    }

    fn datetime(&self, value: &OffsetDateTime) -> String {
        value.format(&Rfc3339).unwrap_or_default()
    }

    fn date(&self, value: &OffsetDateTime) -> String {
        format!(
            "{} {:02}:{:02} UTC",
            value.date(),
            value.hour(),
            value.minute()
        )
    }

    fn response(&self, status: StatusCode) -> Response {
        match self.render() {
            Ok(html) => (status, [("content-language", self.locale.0)], Html(html)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

enum Failure {
    Language,
    Login(Locale),
    Sanction(Locale, Error),
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        match self {
            Self::Language => StatusCode::NOT_FOUND.into_response(),
            Self::Login(locale) => Redirect::to(&format!("/{}/login", locale.0)).into_response(),
            Self::Sanction(locale, error) => {
                let message = match error {
                    Error::Invalid => "invalid",
                    Error::Conflict => "conflict",
                    Error::Disabled | Error::NotFound => "missing",
                    Error::Database(_) => "unavailable",
                };
                Page {
                    error: Some(message),
                    ..Page::new(locale)
                }
                .response(
                    StatusCode::from_u16(error.status_code())
                        .unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
                )
            }
        }
    }
}

async fn owner(session: &BrowserSession, locale: Locale) -> Result<Uuid, Failure> {
    session.account_id().await.ok_or(Failure::Login(locale))
}

#[derive(Deserialize)]
struct Paging {
    page: Option<u32>,
}

async fn history(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    paging: Result<Query<Paging>, QueryRejection>,
) -> Result<Response, Failure> {
    let locale = Locale::parse(&lang)?;
    let owner = owner(&session, locale).await?;
    let Query(paging) = paging.map_err(|_| Failure::Sanction(locale, Error::Invalid))?;
    let page = paging.page.unwrap_or(1);
    let mut rows = player::history(&pool, owner, page)
        .await
        .map_err(|e| Failure::Sanction(locale, e))?;
    let has_next = rows.len() > PAGE_SIZE as usize && page < LAST_PAGE;
    rows.truncate(PAGE_SIZE as usize);
    Ok(Page {
        rows,
        previous: (page > 1).then_some(page - 1),
        next: has_next.then_some(page + 1),
        ..Page::new(locale)
    }
    .response(StatusCode::OK))
}

async fn target(
    lang: &str,
    raw_id: &str,
    session: &BrowserSession,
) -> Result<(Locale, Uuid, Uuid), Failure> {
    let locale = Locale::parse(lang)?;
    let owner = owner(session, locale).await?;
    let id = raw_id
        .parse()
        .map_err(|_| Failure::Sanction(locale, Error::Invalid))?;
    Ok((locale, owner, id))
}

async fn detail(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, Failure> {
    let (locale, owner, id) = target(&lang, &id, &session).await?;
    let row = player::detail(&pool, owner, id)
        .await
        .map_err(|e| Failure::Sanction(locale, e))?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|e| Failure::Sanction(locale, e.into()))?;
    Ok(Page {
        detail: true,
        rows: vec![row],
        csrf,
        ..Page::new(locale)
    }
    .response(StatusCode::OK))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Acknowledgment {
    version: i64,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Appeal {
    version: i64,
    message: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

async fn acknowledge(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
    form: Result<Form<Acknowledgment>, FormRejection>,
) -> Result<Redirect, Failure> {
    let (locale, owner, id) = target(&lang, &id, &session).await?;
    let Form(form) = form.map_err(|_| Failure::Sanction(locale, Error::Invalid))?;
    change(
        &pool,
        locale,
        owner,
        id,
        form.version,
        PlayerAction::Acknowledge,
    )
    .await
}

async fn appeal(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
    form: Result<Form<Appeal>, FormRejection>,
) -> Result<Redirect, Failure> {
    let (locale, owner, id) = target(&lang, &id, &session).await?;
    let Form(form) = form.map_err(|_| Failure::Sanction(locale, Error::Invalid))?;
    change(
        &pool,
        locale,
        owner,
        id,
        form.version,
        PlayerAction::Appeal(&form.message),
    )
    .await
}

async fn change(
    pool: &PgPool,
    locale: Locale,
    owner: Uuid,
    id: Uuid,
    version: i64,
    action: PlayerAction<'_>,
) -> Result<Redirect, Failure> {
    player::change(pool, owner, id, version, action)
        .await
        .map_err(|e| Failure::Sanction(locale, e))?;
    Ok(Redirect::to(&format!("{}/{id}", locale.base())))
}
