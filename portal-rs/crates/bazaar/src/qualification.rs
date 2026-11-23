//! Read-only Platform views for the Bazaar production qualification.
use crate::{Kind, State, characters::http::text};
use askama::Template;
use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use uuid::Uuid;

pub fn public_router() -> Router {
    Router::new().route("/{locale}/bazaar/readiness", get(readiness))
}

#[derive(Template)]
#[template(path = "bazaar/readiness.html", config = "../web/askama.toml")]
struct Readiness<'a> {
    lang: &'a str,
}

impl Readiness<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }

    fn item_unavailable(&self) -> String {
        crate::items::web::text(self.lang, "unavailable")
    }
}

async fn readiness(Path(lang): Path<String>) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    match (Readiness { lang: &lang }).render() {
        Ok(body) => ([(header::CACHE_CONTROL, "no-store")], Html(body)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Persisted Platform records for one listing, read together in one snapshot.
#[derive(Debug, sqlx::FromRow)]
pub struct ListingStatus {
    pub id: Uuid,
    kind: Kind,
    state: State,
    suspended: bool,
    settled: Option<bool>,
}

impl ListingStatus {
    pub fn kind_key(&self) -> &'static str {
        match self.kind {
            Kind::Character => "character",
            Kind::Item => "readiness-item",
        }
    }

    pub fn state_key(&self) -> &'static str {
        match self.state {
            State::Draft => "readiness-draft",
            State::Active => "active",
            State::Matched => "matched",
            State::AwaitingDelivery => "awaiting_delivery",
            State::Completed => "completed",
            State::Cancelled => "cancelled",
            State::Expired => "expired",
        }
    }

    pub fn moderation_key(&self) -> &'static str {
        if self.suspended {
            "readiness-suspended"
        } else {
            "readiness-no-record"
        }
    }

    pub fn settlement_key(&self) -> &'static str {
        match self.settled {
            Some(true) => "completed",
            Some(false) => "readiness-open",
            None => "readiness-no-record",
        }
    }
}

pub async fn listing_status(pool: &PgPool, id: Uuid) -> Result<Option<ListingStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT l.id, l.kind, l.state, h.listing_id IS NOT NULL AS suspended,
                CASE
                    WHEN l.currency = 'gold' AND g.id IS NOT NULL THEN g.completed_at IS NOT NULL
                    WHEN s.id IS NOT NULL THEN s.completed_at IS NOT NULL
                END AS settled
         FROM bazaar_listings l
         LEFT JOIN bazaar_moderation_holds h ON h.listing_id = l.id
         LEFT JOIN bazaar_settlements s ON s.listing_id = l.id
         LEFT JOIN bazaar_authority_gold_sales g ON g.listing_id = l.id
         WHERE l.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
