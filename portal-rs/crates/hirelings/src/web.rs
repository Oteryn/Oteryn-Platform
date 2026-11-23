//! Private hireling qualification views. Gameplay actions require trusted adapters.
use crate::Result;
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_db::outbox::PgPool;
use oteryn_web::session::{BrowserSession, SessionLayer};

pub fn router(pool: PgPool, flags: crate::Flags) -> Router {
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
        crate::Flags {
            enabled: std::env::var("PORTAL_HIRELINGS_ENABLED").as_deref() == Ok("true"),
            command_outbox: false,
        },
    )
}

#[derive(Template)]
#[template(path = "hirelings/index.html", config = "askama.toml")]
struct Page<'a> {
    lang: &'a str,
    title: String,
    waiting: String,
    unavailable: String,
    sections: [Section; 4],
}

struct Section {
    id: &'static str,
    title: String,
    note: Option<String>,
    labels: Vec<String>,
}

impl<'a> Page<'a> {
    fn unavailable(lang: &'a str) -> Result<Self> {
        let translations = crate::messages(lang)?;
        let text = |key: &str| crate::text(&translations, key);
        let section = |id, title, labels: &[&str], note: Option<&str>| -> Result<Section> {
            Ok(Section {
                id,
                title: text(title)?,
                note: note.map(&text).transpose()?,
                labels: labels.iter().map(|key| text(key)).collect::<Result<_>>()?,
            })
        };
        Ok(Self {
            lang,
            title: text("title")?,
            waiting: text("waiting")?,
            unavailable: oteryn_ui::text(lang, "online-unavailable"),
            sections: [
                section("purchase", "purchase", &["store-offers"], None)?,
                section("hirelings", "helpers", &["roster", "professions"], None)?,
                section(
                    "expeditions",
                    "expeditions",
                    &[
                        "active-expeditions",
                        "weekly-budget",
                        "rewards",
                        "start-expedition",
                        "collect-expedition",
                        "chest",
                    ],
                    Some("limit"),
                )?,
                section("history", "history", &["results"], None)?,
            ],
        })
    }
}

async fn page(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    let mut response = match load(&pool, &session, &lang).await {
        Ok(page) => oteryn_web::render(&page),
        Err(status) => status.into_response(),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}

async fn load<'a>(
    pool: &PgPool,
    session: &BrowserSession,
    lang: &'a str,
) -> std::result::Result<Page<'a>, StatusCode> {
    if !matches!(lang, "pl" | "en") {
        return Err(StatusCode::NOT_FOUND);
    }
    let account = session.account_id().await.ok_or(StatusCode::UNAUTHORIZED)?;
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM accounts WHERE id = $1 AND status = 'active'
                AND email_verified_at IS NOT NULL
        )",
    )
    .bind(account)
    .fetch_one(pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !active {
        return Err(StatusCode::FORBIDDEN);
    }
    Page::unavailable(lang).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
