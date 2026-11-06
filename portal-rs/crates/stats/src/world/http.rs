use super::{Config, Day, Error, read};
use crate::store::{Permissions, Visibility};
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use std::sync::Arc;
use time::Date;
use uuid::Uuid;

/// An unavailable privacy service denies the entire representation.
pub trait Policy: Visibility + Send + Sync {
    fn available(&self) -> bool;
}
struct Deny;
impl Visibility for Deny {
    fn permissions(&self, _: Uuid) -> Permissions {
        Permissions::default()
    }
}
impl Policy for Deny {
    fn available(&self) -> bool {
        false
    }
}
#[derive(Clone)]
struct World {
    db: PgPool,
    config: Config,
    policy: Arc<dyn Policy>,
}
pub fn router<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    config: Config,
    policy: Arc<dyn Policy>,
) -> Router<S> {
    Router::new()
        .route("/stats/world", get(page))
        .route("/pl/stats/world", get(page))
        .route("/en/stats/world", get(page))
        .with_state(World { db, config, policy })
}
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    router(
        db,
        Config {
            enabled: std::env::var("PORTAL_WORLD_STATS_ENABLED").as_deref() == Ok("true"),
        },
        Arc::new(Deny),
    )
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Uuid,
    start: String,
    end: String,
}
#[derive(Template)]
#[template(path = "stats/world/index.html", config = "src/world.askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    days: Vec<Day>,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../../../templates/stats/world/en.ftl")
    } else {
        include_str!("../../../../templates/stats/world/pl.ftl")
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    [
        "title",
        "unavailable",
        "census",
        "deaths",
        "online",
        "note",
        "level",
        "day",
        "count",
        "kills",
    ]
    .iter()
    .map(|key| {
        bundle
            .get_message(&format!("world-{key}"))
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
fn date(value: &str) -> Result<Date, Error> {
    let parts = value.split('-').collect::<Vec<_>>();
    if !value.bytes().all(|b| b.is_ascii_digit() || b == b'-')
        || parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
    {
        return Err(Error::BadInput);
    }
    let year = parts[0].parse().map_err(|_| Error::BadInput)?;
    let month = parts[1]
        .parse::<u8>()
        .map_err(|_| Error::BadInput)?
        .try_into()
        .map_err(|_| Error::BadInput)?;
    let day = parts[2].parse().map_err(|_| Error::BadInput)?;
    Date::from_calendar_date(year, month, day).map_err(|_| Error::BadInput)
}
async fn page(
    State(state): State<World>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    let response = async {
        if !state.config.enabled {
            return StatusCode::NOT_FOUND.into_response();
        }
        if raw.as_deref().is_none_or(str::is_empty) {
            let en = uri.path().starts_with("/en/");
            let page = Page {
                lang: if en { "en" } else { "pl" },
                labels: labels(en),
                unavailable: true,
                days: vec![],
            };
            return match page.render() {
                Ok(body) => (StatusCode::SERVICE_UNAVAILABLE, Html(body)).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }
        let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or_default())
        else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let (Ok(start), Ok(end)) = (date(&query.start), date(&query.end)) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if query.world.is_nil() || start > end || (end - start).whole_days() > 30 {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let result = if state.policy.available() {
            read(
                &state.db,
                state.config,
                query.world,
                start,
                end,
                &PolicyRef(state.policy.as_ref()),
            )
            .await
        } else {
            Err(Error::Unavailable)
        };
        let unavailable = result.is_err();
        let en = uri.path().starts_with("/en/");
        let page = Page {
            lang: if en { "en" } else { "pl" },
            labels: labels(en),
            unavailable,
            days: result.map(|r| r.days).unwrap_or_default(),
        };
        match page.render() {
            Ok(body) => (
                if unavailable {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::OK
                },
                Html(body),
            )
                .into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
    .await;
    let mut response = response;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    response
}
struct PolicyRef<'a>(&'a dyn Policy);
impl Visibility for PolicyRef<'_> {
    fn permissions(&self, id: Uuid) -> Permissions {
        self.0.permissions(id)
    }
}
