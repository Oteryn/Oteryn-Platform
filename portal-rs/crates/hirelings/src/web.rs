//! Read-only account surface. Mutations require trusted adapters, never browser evidence.
use crate::{Flags, Result};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_db::outbox::{PgPool, Uuid};
use oteryn_web::session::{BrowserSession, SessionLayer};

pub fn router(pool: PgPool, flags: Flags) -> Router {
    if !flags.enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/panel/hirelings", get(page))
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .with_state(pool)
}
pub fn configured_router(pool: PgPool) -> Router {
    router(
        pool,
        Flags {
            enabled: std::env::var("PORTAL_HIRELINGS_ENABLED").as_deref() == Ok("true"),
            command_outbox: false,
        },
    )
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><title>{{ title }}</title><body class=\"op-theme\"><main class=\"op-stack\"><h1>{{ title }}</h1><p>{{ limit }}</p><p>{{ waiting }}</p><h2>{{ helpers_label }}</h2><ul>{% for row in helpers %}<li>{{ row }}</li>{% endfor %}</ul><h2>{{ history_label }}</h2><ul>{% for row in history %}<li>{{ row }}</li>{% endfor %}</ul><h2>{{ rewards_label }}</h2><p>{{ waiting }}</p></main></body></html>",
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    limit: String,
    waiting: String,
    helpers_label: String,
    history_label: String,
    rewards_label: String,
    helpers: Vec<String>,
    history: Vec<String>,
}
async fn page(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(account) = session.account_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match load(&pool, account, &lang).await {
        Ok(page) => {
            let mut response = oteryn_web::render(&page);
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("private, no-store"),
            );
            response
        }
        Err(_) => StatusCode::FORBIDDEN.into_response(),
    }
}
async fn load<'a>(pool: &PgPool, account: Uuid, lang: &'a str) -> Result<Page<'a>> {
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL)").bind(account).fetch_one(pool).await?;
    if !active {
        return Err("inactive account".into());
    }
    let professions: Vec<String> =
        sqlx::query_scalar("SELECT profession FROM hirelings WHERE account_id=$1 ORDER BY slot")
            .bind(account)
            .fetch_all(pool)
            .await?;
    let states:Vec<String>=sqlx::query_scalar("SELECT state FROM hireling_expeditions WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100").bind(account).fetch_all(pool).await?;
    let rows = |values: Vec<String>| -> Result<Vec<String>> {
        if values.is_empty() {
            return Ok(vec![crate::text(lang, "empty")?]);
        }
        values.iter().map(|key| crate::text(lang, key)).collect()
    };
    Ok(Page {
        lang,
        title: crate::text(lang, "title")?,
        limit: crate::text(lang, "limit")?,
        waiting: crate::text(lang, "waiting")?,
        helpers_label: crate::text(lang, "helpers")?,
        history_label: crate::text(lang, "history")?,
        rewards_label: crate::text(lang, "rewards")?,
        helpers: rows(professions)?,
        history: rows(states)?,
    })
}
