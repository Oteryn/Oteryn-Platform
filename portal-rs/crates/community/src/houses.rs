//! G-4 house catalogue. Ingest evidence never grants public owner visibility.
use crate::v2_watermark::Part;
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy};
use sqlx::{PgPool, Row};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
#[path = "houses_ingest.rs"]
mod houses_ingest;
#[path = "houses_schema.rs"]
mod houses_schema;
/// Implementations must resolve current identities without name reuse and prove
/// current profile/house visibility, including guild-member privacy where needed.
/// A displayed owner name is never an ownership or authorization join.
pub trait OwnerVisibility: Send + Sync {
    fn visible<'a>(
        &'a self,
        world: &'a str,
        kind: &'a str,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
}
#[derive(Debug)]
pub struct DenyOwners;
impl OwnerVisibility for DenyOwners {
    fn visible<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async { false })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub ingest: crate::ingest::Config,
    pub serving_enabled: bool,
}
#[derive(Clone)]
struct Houses {
    contract_accepted: bool,
    db: PgPool,
    config: Config,
    privacy: Arc<dyn OwnerVisibility>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
pub fn router<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    identities: Arc<IdentityMap>,
    privacy: Arc<dyn OwnerVisibility>,
    config: Config,
) -> Router<S> {
    routes(db, identities, privacy, config, Part::All, true)
}
fn routes<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    identities: Arc<IdentityMap>,
    privacy: Arc<dyn OwnerVisibility>,
    config: Config,
    part: Part,
    contract_accepted: bool,
) -> Router<S> {
    let state = Houses {
        contract_accepted,
        db,
        config: config.clone(),
        privacy,
    };
    let mut app = Router::new();
    for (path, bound) in [("house", 2048), ("watermark", 512)] {
        if !part.includes(path) {
            continue;
        }
        let route = Router::new()
            .route(
                &format!("/internal/v2/game-public/{path}"),
                post(houses_ingest::handle),
            )
            .layer(axum::Extension(path == "watermark"));
        app = app.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: config.ingest.enabled
                    && houses_schema::authority(&config.ingest.source_authority),
                group: "game-public".into(),
                max_body_bytes: bound,
                max_json_depth: 5,
                json: JsonRule::Object,
            },
        ));
    }
    for prefix in ["", "/pl", "/en"]
        .into_iter()
        .filter(|_| part != Part::Watermark)
    {
        app = app
            .route(&format!("{prefix}/houses"), get(page))
            .route(&format!("{prefix}/houses/{{key}}"), get(page));
    }
    app.with_state(state)
        .layer(axum::middleware::from_fn(headers))
}
/// The shared V2 watermark is mounted once by [`crate::v2_watermark`].
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    configured(db, Part::Payload)
}
/// Only the guarded watermark route, for [`crate::v2_watermark::router`].
pub fn configured_watermark_router(db: PgPool) -> Router {
    configured(db, Part::Watermark)
}
fn configured<S: Clone + Send + Sync + 'static>(db: PgPool, part: Part) -> Router<S> {
    let prerequisite = std::env::var("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Ok("true")
        && matches!(
            std::env::var("APP_ENV").as_deref(),
            Ok("testing" | "preproduction")
        );
    let identities = std::env::var("PORTAL_PUBLIC_PROJECTIONS_SAN")
        .ok()
        .and_then(|s| guard::Identity::san(&s).ok())
        .and_then(|identity| {
            IdentityMap::new(vec![guard::Binding {
                identity,
                group: "game-public".into(),
            }])
            .ok()
        })
        .unwrap_or_default();
    routes(
        db,
        Arc::new(identities),
        Arc::new(DenyOwners),
        Config {
            ingest: crate::ingest::Config {
                enabled: prerequisite
                    && std::env::var("PORTAL_HOUSES_INGEST_ENABLED").as_deref() == Ok("true"),
                source_authority: std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY")
                    .unwrap_or_default(),
            },
            serving_enabled: prerequisite
                && std::env::var("PORTAL_HOUSES_SERVING_ENABLED").as_deref() == Ok("true"),
        },
        part,
        false, // G-4 house rev 1 is CANDIDATE, without a Game producer.
    )
}
#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Option<String>,
}
struct House {
    name: String,
    world: String,
    key: String,
    kind: String,
    town: String,
    size: String,
    beds: String,
    rent: String,
    status: String,
    extras: Vec<(String, String)>,
}
#[derive(Template)]
#[template(path = "houses/page.html", config = "src/houses.askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    houses: Vec<House>,
    unavailable: bool,
}
fn label(en: bool, key: &str) -> String {
    let mut bundle = fluent_bundle::FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    let source = if en {
        include_str!("../../../templates/houses/en.ftl")
    } else {
        include_str!("../../../templates/houses/pl.ftl")
    };
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn timestamp(value: &str) -> Option<String> {
    time::OffsetDateTime::from_unix_timestamp(value.parse().ok()?)
        .ok()?
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}
async fn load(
    state: &Houses,
    world: Option<&str>,
    key: Option<&str>,
    en: bool,
) -> Result<Vec<House>, StatusCode> {
    let unavailable = StatusCode::SERVICE_UNAVAILABLE;
    if !state.contract_accepted {
        return Err(unavailable);
    }
    // One committed cut covers the watermark, shared epoch and all catalogue rows.
    let rows=sqlx::query("SELECT m.epoch::text AS generation,m.complete_through,h.payload,h.observed FROM public_houses_v2_watermark m JOIN public_projection_state s ON s.id=1 AND s.highest_epoch=m.epoch LEFT JOIN public_houses_v2 h ON h.epoch=m.epoch AND h.source_authority=m.source_authority WHERE m.id=1 AND m.source_authority=$1 ORDER BY h.subject") .bind(&state.config.ingest.source_authority).fetch_all(&state.db).await.map_err(|_|unavailable)?;
    if rows.is_empty() {
        return Err(unavailable);
    }
    let generation: String = rows[0].try_get("generation").map_err(|_| unavailable)?;
    let complete: i64 = rows[0]
        .try_get("complete_through")
        .map_err(|_| unavailable)?;
    let mut houses = vec![];
    for row in rows {
        let complete: i64 = row.try_get("complete_through").map_err(|_| unavailable)?;
        if complete < 0
            || complete as u64 > now().saturating_add(1)
            || now().saturating_sub(complete as u64).saturating_add(1) > 30
        {
            return Err(unavailable);
        }
        let Some(v) = row
            .try_get::<Option<serde_json::Value>, _>("payload")
            .map_err(|_| unavailable)?
        else {
            return Err(unavailable);
        };
        let observed: i64 = row.try_get("observed").map_err(|_| unavailable)?;
        if observed > complete {
            return Err(unavailable);
        }
        let w = v["world_id"].as_str().ok_or(unavailable)?;
        let k = v["house_key"].as_str().ok_or(unavailable)?;
        if world.is_some_and(|wanted| wanted != w) || key.is_some_and(|wanted| wanted != k) {
            continue;
        }
        let records = v["house"].as_array().ok_or(unavailable)?;
        let Some(h) = records.first() else {
            continue;
        };
        let value = |field: &str| h[field].as_str().map(str::to_owned).ok_or(unavailable);
        let mut status = value("status")?;
        let mut extras = vec![];
        if ["OWNED", "MOVE_OUT_PENDING"].contains(&status.as_str()) {
            let owner = value("owner_name")?;
            if state
                .privacy
                .visible(w, &value("owner_kind")?, &owner)
                .await
            {
                extras.push((label(en, "owner"), owner));
                extras.push((
                    label(en, "paid"),
                    timestamp(&value("paid_until")?).ok_or(unavailable)?,
                ));
                if status == "MOVE_OUT_PENDING" {
                    extras.push((
                        label(en, "move"),
                        timestamp(&value("move_out_at")?).ok_or(unavailable)?,
                    ));
                }
            } else {
                status = "UNAVAILABLE".into();
            }
        }
        if status == "AUCTION" {
            extras.push((label(en, "bid"), value("current_bid")?));
            extras.push((
                label(en, "ends"),
                timestamp(&value("auction_ends_at")?).ok_or(unavailable)?,
            ));
        }
        houses.push(House {
            name: value("name")?,
            world: w.into(),
            key: k.into(),
            kind: label(en, &value("kind")?),
            town: value("town")?,
            size: value("size_sqm")?,
            beds: value("beds")?,
            rent: value("rent_gold")?,
            status: label(en, &status),
            extras,
        });
    }
    // Recheck after asynchronous privacy resolution; an epoch raise or elapsed
    // freshness budget must not publish the previously loaded cut.
    let current: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM public_projection_state WHERE id=1 AND highest_epoch=$1::numeric)").bind(&generation).fetch_one(&state.db).await.map_err(|_| unavailable)?;
    if !current || now().saturating_sub(complete as u64).saturating_add(1) > 30 {
        return Err(unavailable);
    }
    houses.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.world.cmp(&b.world))
            .then_with(|| a.key.cmp(&b.key))
    });
    Ok(houses)
}
async fn page(
    State(state): State<Houses>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    if !state.config.serving_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or("")) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let key = uri.path().split_once("/houses/").map(|(_, s)| s);
    if query
        .world
        .as_deref()
        .is_some_and(|s| !houses_schema::world(s))
        || key.is_some_and(|s| !houses_schema::house_key(s))
        || (key.is_some() && query.world.is_none())
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let en = uri.path().starts_with("/en/");
    let result = load(&state, query.world.as_deref(), key, en).await;
    let unavailable = result.is_err();
    let houses = result.unwrap_or_default();
    if !unavailable && key.is_some() && houses.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let page = Page {
        lang: if en { "en" } else { "pl" },
        labels: [
            "title",
            "unavailable",
            "empty",
            "kind",
            "town",
            "size",
            "beds",
            "rent",
            "status",
            "details",
        ]
        .iter()
        .map(|k| label(en, k))
        .collect(),
        houses,
        unavailable,
    };
    match page.render() {
        Ok(html) => (
            if unavailable {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::OK
            },
            Html(html),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
async fn headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    for (name, value) in [
        (header::CACHE_CONTROL, "private, no-store"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains",
        ),
        (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
        (
            header::HeaderName::from_static("permissions-policy"),
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
#[cfg(test)]
#[path = "houses_tests.rs"]
mod houses_tests;
