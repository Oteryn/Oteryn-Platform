//! PostgreSQL serializes each publisher/route window across portal processes.
use axum::{
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use oteryn_internal_api::guard::{self, PublisherKey};
use sqlx::PgPool;
pub(crate) fn configured_limit() -> u16 {
    std::env::var("PORTAL_PUBLIC_PROJECTIONS_REQUESTS_PER_MINUTE")
        .map_or(6000, |value| value.parse().unwrap_or(0))
}

pub(crate) enum Denied {
    Limited,
    Unavailable,
}

impl Denied {
    pub(crate) fn response(self) -> Response {
        match self {
            Self::Unavailable => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
            Self::Limited => {
                let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
                response
            }
        }
    }
}

pub(crate) async fn check(
    db: &PgPool,
    publisher: &PublisherKey,
    route: &str,
    limit: u16,
) -> Result<(), Denied> {
    if !(1..=6000).contains(&limit) {
        return Err(Denied::Unavailable);
    }
    if route.starts_with("v2/") && route.ends_with("/watermark") && route != "v2/watermark" {
        if !(1..=6000).contains(&configured_limit()) {
            return Err(Denied::Unavailable);
        }
        match oteryn_db::ingest_budget::admit(
            db,
            publisher.as_bytes(),
            "v2/watermark",
            configured_limit(),
        )
        .await
        {
            Ok(true) => (),
            Ok(false) => return Err(Denied::Limited),
            Err(_) => return Err(Denied::Unavailable),
        }
    }
    match oteryn_db::ingest_budget::admit(db, publisher.as_bytes(), route, limit).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(Denied::Limited),
        Err(_) => Err(Denied::Unavailable),
    }
}

#[cfg(test)]
#[path = "ingest_budget_tests.rs"]
mod tests;
