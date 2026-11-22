//! Session-owned Bazaar read model. Recorded matches and transfers are historical facts.
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::PgPool;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const PAGE_SIZE: usize = 20;

pub fn enabled() -> bool {
    std::env::var("OTERYN_BAZAAR_ACTIVITY_ENABLED").as_deref() == Ok("true")
}

pub fn configured_router(pool: PgPool) -> Router {
    router(pool, enabled())
}

fn router(pool: PgPool, enabled: bool) -> Router {
    if !enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/account/bazaar/activity", get(index))
        .layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .with_state(pool)
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filter {
    listing: Option<Uuid>,
    #[serde(default)]
    page: u16,
}

#[derive(sqlx::FromRow)]
struct Record {
    section: i32,
    listing: Uuid,
    kind: String,
    currency: String,
    amount: i64,
    state: String,
    updated_at: OffsetDateTime,
}

struct Section {
    key: &'static str,
    amount: &'static str,
    records: Vec<Record>,
}

#[derive(Template)]
#[template(path = "activity.html", config = "src/activity_askama.toml")]
struct Page {
    lang: String,
    sections: [Section; 3],
    latest: Option<OffsetDateTime>,
    previous: Option<String>,
    next: Option<String>,
}

impl Page {
    fn timestamp(&self, value: &OffsetDateTime) -> String {
        value.format(&Rfc3339).unwrap_or_default()
    }

    fn text(&self, key: &str) -> String {
        oteryn_ui::text(&self.lang, &format!("bazaar-activity-{key}"))
    }
}

async fn index(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    filter: Result<Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, StatusCode> {
    if !matches!(lang.as_str(), "pl" | "en") {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some(account) = session.account_id().await else {
        return Ok(Redirect::to(&format!("/{lang}/login")).into_response());
    };
    let Query(filter) = filter.map_err(|_| StatusCode::BAD_REQUEST)?;
    render_activity(&pool, account, lang, filter).await
}

async fn render_activity(
    pool: &PgPool,
    account: Uuid,
    lang: String,
    filter: Filter,
) -> Result<Response, StatusCode> {
    let offset = i64::from(filter.page) * PAGE_SIZE as i64;
    // Read one overlapping row per section to decide whether another page exists.
    let rows = sqlx::query_as::<_, Record>(include_str!("activity.sql"))
        .bind(account)
        .bind(filter.listing)
        .bind(offset)
        .bind((PAGE_SIZE + 1) as i64)
        .fetch_all(pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if rows.is_empty() && (filter.listing.is_some() || filter.page > 0) {
        return Err(StatusCode::NOT_FOUND);
    }
    let page_url = |page| match filter.listing {
        Some(listing) => format!("?listing={listing}&page={page}"),
        None => format!("?page={page}"),
    };
    let latest = rows.iter().map(|row| row.updated_at).max();
    let mut sections = [
        Section {
            key: "listings",
            amount: "price",
            records: Vec::new(),
        },
        Section {
            key: "bids",
            amount: "maximum",
            records: Vec::new(),
        },
        Section {
            key: "settlements",
            amount: "gross",
            records: Vec::new(),
        },
    ];
    for row in rows {
        let section = sections
            .get_mut(row.section as usize)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        section.records.push(row);
    }
    let has_next = sections.iter_mut().fold(false, |has_next, section| {
        let more = section.records.len() > PAGE_SIZE;
        section.records.truncate(PAGE_SIZE);
        has_next || more
    });
    let page = Page {
        lang,
        sections,
        latest,
        previous: filter.page.checked_sub(1).map(page_url),
        next: filter
            .page
            .checked_add(1)
            .filter(|_| has_next)
            .map(page_url),
    };
    let html = page
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(([(header::CACHE_CONTROL, "private, no-store")], Html(html)).into_response())
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
