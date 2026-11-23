use super::{access, http};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_web::session::{self, BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Copy, Default)]
pub(super) struct Readiness {
    guilds: bool,
    growth: bool,
    world: bool,
    characters: bool,
    pub watches: bool,
}

impl Readiness {
    pub fn configured() -> Self {
        let on = |key| std::env::var(key).as_deref() == Ok("true");
        Self {
            guilds: on("PORTAL_GUILD_STATS_ENABLED"),
            growth: on("PORTAL_GROWTH_RANKINGS_ENABLED"),
            world: on("PORTAL_WORLD_STATS_ENABLED"),
            characters: on("PORTAL_CHARACTER_STATS_ENABLED"),
            watches: on("PORTAL_STATS_WATCHES_ENABLED"),
        }
    }
}

#[derive(Clone)]
struct Browser {
    pool: PgPool,
    config: Readiness,
}

pub(super) fn router(pool: PgPool, config: Readiness) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    if let Ok(origin) = std::env::var("OTERYN_PUBLIC_ORIGIN") {
        sessions.csrf.trusted_origins.push(origin);
    }
    Router::new()
        .route("/{lang}/account/stats-watches", get(settings).post(update))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            session::middleware,
        ))
        .layer(DefaultBodyLimit::max(1024))
        .route("/{lang}/stats", get(readiness))
        .layer(middleware::from_fn(http::private))
        .with_state(Browser { pool, config })
}

struct Text(FluentBundle<FluentResource>);
impl Text {
    fn new(lang: &str) -> Result<Self, StatusCode> {
        let source = match lang {
            "en" => include_str!("../../../../templates/stats/readiness/en.ftl"),
            "pl" => include_str!("../../../../templates/stats/readiness/pl.ftl"),
            _ => return Err(StatusCode::NOT_FOUND),
        };
        let mut bundle =
            FluentBundle::new_concurrent(vec![lang.parse().map_err(http::unavailable)?]);
        bundle
            .add_resource(FluentResource::try_new(source.into()).map_err(http::unavailable)?)
            .map_err(http::unavailable)?;
        Ok(Self(bundle))
    }

    fn label(&self, key: &str) -> String {
        self.0
            .get_message(key)
            .and_then(|m| m.value())
            .map_or_else(String::new, |p| {
                self.0.format_pattern(p, None, &mut Vec::new()).into_owned()
            })
    }
}

#[derive(Template)]
#[template(path = "stats/readiness/index.html", config = "../web/askama.toml")]
struct Overview {
    lang: String,
    text: Text,
    reports: [Report; 6],
    watches: bool,
}

struct Report {
    title: &'static str,
    state: &'static str,
    reason: &'static str,
    path: Option<&'static str>,
}

impl Report {
    fn new(
        title: &'static str,
        reason: &'static str,
        enabled: bool,
        path: Option<&'static str>,
    ) -> Self {
        Self {
            title,
            state: if enabled { "unavailable" } else { "off" },
            reason,
            path: path.filter(|_| enabled),
        }
    }
}

async fn readiness(
    State(state): State<Browser>,
    Path(lang): Path<String>,
) -> Result<Response, StatusCode> {
    let config = state.config;
    render(Overview {
        text: Text::new(&lang)?,
        lang,
        reports: [
            Report::new(
                "guilds",
                "guilds-source",
                config.guilds,
                Some("stats/guilds"),
            ),
            Report::new(
                "growth",
                "growth-source",
                config.growth,
                Some("stats/rankings/yesterday"),
            ),
            Report::new("online", "online-source", true, None),
            Report::new("census", "census-source", config.world, Some("stats/world")),
            Report::new("kills", "kills-source", true, None),
            Report::new(
                "comparison",
                "growth-source",
                config.characters,
                Some("stats/compare"),
            ),
        ],
        watches: config.watches,
    })
}

#[derive(Template)]
#[template(path = "stats/readiness/watches.html", config = "../web/askama.toml")]
struct Settings {
    lang: String,
    text: Text,
    csrf: String,
    active: bool,
    has_watches: bool,
    token: Option<String>,
}

async fn actor(session: &BrowserSession) -> Result<(Uuid, Vec<u8>), StatusCode> {
    let account = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let hash = session.identity_hash().await.ok_or(StatusCode::FORBIDDEN)?;
    Ok((account, hash))
}

async fn settings(
    State(state): State<Browser>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    check(&state, &lang)?;
    let (account, hash) = actor(&session).await?;
    let (active, has_watches) = access::status(&state.pool, account, &hash).await?;
    settings_page(lang, &session, active, has_watches, None).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    action: access::Action,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

async fn update(
    State(state): State<Browser>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Form(change): Form<Change>,
) -> Result<Response, StatusCode> {
    check(&state, &lang)?;
    let (account, hash) = actor(&session).await?;
    if let Some((token, has_watches)) =
        access::update(&state.pool, account, &hash, change.action).await?
    {
        settings_page(lang, &session, true, has_watches, Some(token)).await
    } else {
        Ok(Redirect::to(&format!("/{lang}/account/stats-watches")).into_response())
    }
}

fn check(state: &Browser, lang: &str) -> Result<(), StatusCode> {
    if state.config.watches && matches!(lang, "en" | "pl") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn settings_page(
    lang: String,
    session: &BrowserSession,
    active: bool,
    has_watches: bool,
    token: Option<String>,
) -> Result<Response, StatusCode> {
    render(Settings {
        text: Text::new(&lang)?,
        lang,
        csrf: session.csrf_token().await.map_err(http::unavailable)?,
        active,
        has_watches,
        token,
    })
}

fn render(page: impl Template) -> Result<Response, StatusCode> {
    let mut response = Html(page.render().map_err(http::unavailable)?).into_response();
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(oteryn_web::chrome::PORTAL_CONTENT_SECURITY_POLICY),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}
