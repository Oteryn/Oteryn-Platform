//! Authenticated private journal and read-only producer status.
use super::{
    ProgressLanguage,
    planner::{self, Action, NewGoal},
    presentation::text,
};
use crate::journal::{PROGRESS_IDENTITY, Settings};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, Redirect, Response},
    routing::{get, post},
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_identity::mfa::{Mfa, SESSION_GRANT_KEY};
use oteryn_web::session::{self, BrowserSession, SessionKey, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::BTreeSet;
use uuid::Uuid;

mod observations;
use observations::{Section, sections};

#[derive(Clone)]
struct Journal {
    pool: PgPool,
    settings: Settings,
    read_enabled: bool,
    login_enabled: bool,
    peer_configured: bool,
    mfa: Option<Mfa>,
}

impl Journal {
    fn source(&self) -> Option<&str> {
        let value = self.settings.sources.progress.as_str();
        ((1..=128).contains(&value.len())
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b)))
        .then_some(value)
    }

    fn read_family(&self, family: &str) -> bool {
        self.read_enabled && self.settings.families.contains(family)
    }
}

pub fn router(pool: PgPool) -> Router {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    let state = Journal {
        settings: Settings::from_lookup(|key| std::env::var(key).ok()),
        read_enabled: on("OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED"),
        login_enabled: on("OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED"),
        peer_configured: std::env::var(PROGRESS_IDENTITY).is_ok_and(|s| !s.trim().is_empty()),
        mfa: Mfa::configured(pool.clone()).ok(),
        pool: pool.clone(),
    };
    let mut layer = SessionLayer::new(pool);
    layer.csrf.trusted_origins = std::env::var("OTERYN_PUBLIC_ORIGIN")
        .ok()
        .into_iter()
        .collect();
    let routes = Router::new()
        .route("/{lang}/account/journal", get(show).post(create))
        .route("/{lang}/account/journal/goals/{goal}", post(change))
        .route("/{lang}/account/journal/status", get(status))
        .layer(DefaultBodyLimit::max(4096))
        .route_layer(middleware::from_fn_with_state(layer, session::middleware))
        .with_state(state);
    oteryn_web::ratelimit::finish_routes(routes).layer(middleware::from_fn(private))
}

async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}

struct Locale {
    code: String,
    messages: FluentBundle<FluentResource>,
}

impl Locale {
    fn new(code: String) -> Result<Self, StatusCode> {
        let language = match code.as_str() {
            "en" => ProgressLanguage::English,
            "pl" => ProgressLanguage::Polish,
            _ => return Err(StatusCode::NOT_FOUND),
        };
        let messages = language
            .bundle()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(Self { code, messages })
    }

    fn label(&self, key: &str) -> String {
        text(&self.messages, key).unwrap_or_else(|_| key.to_owned())
    }

    fn toggle(&self, enabled: &bool) -> String {
        self.label(if *enabled {
            "journal-on"
        } else {
            "journal-off"
        })
    }

    fn url(&self) -> String {
        format!("/{}/account/journal", self.code)
    }
}

#[derive(Template)]
#[template(path = "playerdata/journal.html", config = "src/hunts.askama.toml")]
struct Page {
    locale: Locale,
    csrf: String,
    request_id: Uuid,
    kinds: &'static [&'static str],
    goals: Vec<planner::Goal>,
    selected: Option<Uuid>,
    characters: Vec<(Uuid, String)>,
    sections: Vec<Section>,
    operator: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    character: Option<String>,
}

async fn show(
    State(journal): State<Journal>,
    Path(lang): Path<String>,
    Query(selection): Query<Selection>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Html<String>, StatusCode> {
    let locale = Locale::new(lang)?;
    let account = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let goals = planner::list(&journal.pool, account)
        .await
        .map_err(planner_status)?;
    let selected = selection.character.as_deref().map(parse_id).transpose()?;
    let (characters, sections) = sections(&journal, &locale, account, selected).await?;
    let operator = operator_allowed(&journal, &session, account).await?;
    let page = Page {
        locale,
        csrf: session.csrf_token().await.map_err(unavailable)?,
        request_id: Uuid::now_v7(),
        kinds: planner::KINDS,
        goals,
        selected,
        characters,
        sections,
        operator,
    };
    page.render().map(Html).map_err(unavailable)
}

fn parse_id(id: &str) -> Result<Uuid, StatusCode> {
    crate::deaths::id(id).map_err(|_| StatusCode::BAD_REQUEST)
}

async fn create(
    State(journal): State<Journal>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<NewGoal>,
) -> Result<Redirect, StatusCode> {
    let locale = Locale::new(lang)?;
    let account = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    planner::create(&journal.pool, account, form)
        .await
        .map_err(planner_status)?;
    Ok(Redirect::to(&locale.url()))
}

#[derive(Deserialize)]
struct Change {
    action: Action,
}

async fn change(
    State(journal): State<Journal>,
    Path((lang, goal)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<Change>,
) -> Result<Redirect, StatusCode> {
    let locale = Locale::new(lang)?;
    let account = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    planner::change(&journal.pool, account, parse_id(&goal)?, form.action)
        .await
        .map_err(planner_status)?;
    Ok(Redirect::to(&locale.url()))
}

fn planner_status(error: planner::Error) -> StatusCode {
    match error {
        planner::Error::Invalid => StatusCode::BAD_REQUEST,
        planner::Error::Limit => StatusCode::CONFLICT,
        planner::Error::Denied => StatusCode::FORBIDDEN,
        planner::Error::Missing => StatusCode::NOT_FOUND,
        planner::Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

async fn operator_allowed(
    journal: &Journal,
    session: &BrowserSession,
    actor: Uuid,
) -> Result<bool, StatusCode> {
    let Some(mfa) = &journal.mfa else {
        return Ok(false);
    };
    let Some(grant) = session
        .get(SessionKey::<String>::new(SESSION_GRANT_KEY))
        .await
        .map_err(unavailable)?
    else {
        return Ok(false);
    };
    let allowed: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM accounts a
         JOIN account_roles r ON r.account_id=a.id
         JOIN role_permissions p ON p.role_id=r.role_id
         WHERE a.id=$1 AND a.status='active' AND a.email_verified_at IS NOT NULL
         AND p.permission='admin.access')",
    )
    .bind(actor)
    .fetch_one(&journal.pool)
    .await
    .map_err(unavailable)?;
    Ok(allowed
        && mfa.enrolled(actor).await.map_err(unavailable)?
        && mfa.satisfied(actor, &grant).await.map_err(unavailable)?)
}

#[derive(Template)]
#[template(
    path = "playerdata/journal-status.html",
    config = "src/hunts.askama.toml"
)]
struct StatusPage {
    locale: Locale,
    ingest: bool,
    read: bool,
    source_configured: bool,
    peer_configured: bool,
    families: Vec<(&'static str, &'static str)>,
}

async fn status(
    State(journal): State<Journal>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Html<String>, StatusCode> {
    let locale = Locale::new(lang)?;
    let account = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    if !operator_allowed(&journal, &session, account).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    let published = if let Some(source) = journal.source() {
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT family FROM pd_streams
             WHERE authority=$1 AND progress AND baseline_complete AND NOT erased
             AND watermark_recovery >= recovery_high_water",
        )
        .bind(source)
        .fetch_all(&journal.pool)
        .await
        .map_err(unavailable)?
        .into_iter()
        .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let families = oteryn_internal_api::playerdata::PROGRESS_FAMILIES
        .split_whitespace()
        .chain(["house_rent"])
        .map(|family| {
            let state = if crate::authority::progress_read::mapping_pending(family) {
                "journal-mapping-pending"
            } else if family == "premium_end" {
                "journal-commercial-pending"
            } else if !journal.read_family(family)
                || (family == "game_login" && !journal.login_enabled)
            {
                "journal-read-disabled"
            } else if published.contains(family) {
                "journal-baseline-recorded"
            } else {
                "journal-baseline-missing"
            };
            (family, state)
        })
        .collect();
    StatusPage {
        locale,
        families,
        ingest: journal.settings.progress_ingest,
        read: journal.read_enabled,
        source_configured: journal.source().is_some(),
        peer_configured: journal.peer_configured,
    }
    .render()
    .map(Html)
    .map_err(unavailable)
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn journal_requires_an_account() -> Result<(), Box<dyn std::error::Error>> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@localhost/unused")?;
        let response = router(pool)
            .oneshot(Request::get("/en/account/journal").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
