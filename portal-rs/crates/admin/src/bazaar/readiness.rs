use super::Locale;
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_bazaar::{
    characters::http::text,
    game::runtime::Config,
    qualification::{ListingStatus, listing_status},
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
struct ReadinessState {
    pool: PgPool,
    runtime: Option<Arc<Config>>,
}

pub(super) fn routes(pool: PgPool, runtime: Option<Arc<Config>>) -> Router {
    Router::new()
        .route("/admin/{lang}/bazaar/readiness", get(readiness))
        .with_state(ReadinessState { pool, runtime })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListingQuery {
    listing: Option<String>,
}

fn listing_id(raw: &str) -> Option<Uuid> {
    if raw.len() != 36 {
        return None;
    }
    Uuid::try_parse(raw)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == raw)
}

#[derive(Template)]
#[template(path = "bazaar/readiness_staff.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    runtime_key: &'static str,
    listing: Option<Uuid>,
    status: Option<ListingStatus>,
    message: Option<&'static str>,
}

impl Page {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }

    fn listing_value(&self) -> String {
        self.listing.map(|id| id.to_string()).unwrap_or_default()
    }

    fn response(self, status: StatusCode) -> Response {
        match self.render() {
            Ok(body) => (status, Html(body)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    fn message(mut self, status: StatusCode, key: &'static str) -> Response {
        self.message = Some(key);
        self.response(status)
    }
}

async fn readiness(
    State(state): State<ReadinessState>,
    Path(lang): Path<String>,
    query: Result<Query<ListingQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, StatusCode> {
    let lang = Locale::parse(&lang)?.0;
    let mut page = Page {
        lang,
        runtime_key: match &state.runtime {
            None => "readiness-runtime-inactive",
            Some(config) if config.current().is_ok() => "readiness-runtime-configured",
            Some(_) => "readiness-unavailable",
        },
        listing: None,
        status: None,
        message: None,
    };
    let invalid =
        |page: Page| page.message(StatusCode::UNPROCESSABLE_ENTITY, "readiness-invalid-id");
    let Ok(Query(query)) = query else {
        return Ok(invalid(page));
    };
    let Some(raw) = query.listing else {
        return Ok(page.response(StatusCode::OK));
    };
    let Some(id) = listing_id(&raw) else {
        return Ok(invalid(page));
    };
    page.listing = Some(id);
    Ok(match listing_status(&state.pool, id).await {
        Ok(Some(status)) => {
            page.status = Some(status);
            page.response(StatusCode::OK)
        }
        Ok(None) => page.message(StatusCode::NOT_FOUND, "readiness-missing-listing"),
        Err(_) => page.message(StatusCode::SERVICE_UNAVAILABLE, "readiness-unavailable"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_lookup_requires_canonical_nonzero_uuid() {
        let id = "019a0000-0000-7000-8000-0000000c11e7";
        assert_eq!(listing_id(id).map(|id| id.to_string()).as_deref(), Some(id));
        for invalid in [
            "019A0000-0000-7000-8000-0000000C11E7",
            "019a00000000700080000000000c11e7",
            "00000000-0000-0000-0000-000000000000",
            "<script>",
            "",
        ] {
            assert_eq!(listing_id(invalid), None);
        }
    }
}
