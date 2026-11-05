//! Privacy-filtered presence; counts include visible characters only because
//! the presence contract grants no independent public aggregate count.
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::{PgPool, Row};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Debug)]
pub struct CharacterRef {
    pub world: String,
    pub name: String,
}
#[derive(Debug)]
pub struct Unavailable;
pub type VisibilityResult<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<bool>, Unavailable>> + Send + 'a>>;
/// Exact world/name references match T24; T45 must resolve these to privacy subjects.
/// Results correspond to input order. Missing evidence must return an error.
pub trait PublicVisibility: Send + Sync {
    fn hidden_batch<'a>(&'a self, characters: &'a [CharacterRef]) -> VisibilityResult<'a>;
}
#[derive(Debug)]
pub struct FailClosed;
impl PublicVisibility for FailClosed {
    fn hidden_batch<'a>(&'a self, _: &'a [CharacterRef]) -> VisibilityResult<'a> {
        Box::pin(async { Err(Unavailable) })
    }
}
#[derive(Clone, Debug)]
pub struct Config {
    pub enabled: bool,
    pub stale_after: Duration,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            stale_after: Duration::from_secs(600),
        }
    }
}
#[derive(Clone)]
struct Online {
    db: PgPool,
    visibility: Arc<dyn PublicVisibility>,
    config: Config,
}
pub fn router<S: Clone + Send + Sync + 'static>(
    db: PgPool,
    visibility: Arc<dyn PublicVisibility>,
    config: Config,
) -> Router<S> {
    Router::new()
        .route("/online", get(page))
        .route("/pl/online", get(page))
        .route("/en/online", get(page))
        .layer(axum::middleware::from_fn(response_policy))
        .with_state(Online {
            db,
            visibility,
            config,
        })
}
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    router(
        db,
        Arc::new(FailClosed),
        Config {
            enabled: std::env::var("PORTAL_ONLINE_ENABLED").as_deref() == Ok("true"),
            stale_after: Duration::from_secs(
                std::env::var("PORTAL_ONLINE_STALE_SECONDS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(600),
            ),
        },
    )
}
#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Option<String>,
    sort: Option<String>,
}
struct Character {
    name: String,
    level: String,
    vocation: u64,
}
struct World {
    id: String,
    as_of: String,
    stale: bool,
    characters: Vec<Character>,
}
#[derive(Template)]
#[template(path = "pages/online.html", config = "src/online.askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    ids: Vec<String>,
    filter: String,
    by_level: bool,
    worlds: Vec<World>,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../web/i18n/en.ftl")
    } else {
        include_str!("../../web/i18n/pl.ftl")
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
        "world",
        "all",
        "sort",
        "name",
        "level",
        "apply",
        "count",
        "as-of",
        "stale",
        "vocation",
    ]
    .iter()
    .map(|k| {
        bundle
            .get_message(&format!("online-{k}"))
            .and_then(|m| m.value())
            .map(|v| bundle.format_pattern(v, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
async fn load(state: &Online, query: &Query) -> Result<(Vec<String>, Vec<World>), Unavailable> {
    let rows = sqlx::query("SELECT subject, watermark, to_char(to_timestamp(watermark) AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS as_of, payload FROM public_snapshots WHERE family='world_online' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) ORDER BY subject").fetch_all(&state.db).await.map_err(|_| Unavailable)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Unavailable)?
        .as_secs();
    let mut ids = vec![];
    let mut worlds = vec![];
    for row in rows {
        let id: String = row.try_get("subject").map_err(|_| Unavailable)?;
        ids.push(id.clone());
        if query
            .world
            .as_ref()
            .is_some_and(|w| !w.is_empty() && *w != id)
        {
            continue;
        }
        let watermark: i64 = row.try_get("watermark").map_err(|_| Unavailable)?;
        let payload: serde_json::Value = row.try_get("payload").map_err(|_| Unavailable)?;
        let entries = payload["characters"].as_array().ok_or(Unavailable)?;
        let refs = entries
            .iter()
            .map(|c| {
                Ok(CharacterRef {
                    world: id.clone(),
                    name: c["name"].as_str().ok_or(Unavailable)?.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, Unavailable>>()?;
        let hidden = state.visibility.hidden_batch(&refs).await?;
        if hidden.len() != entries.len() {
            return Err(Unavailable);
        }
        let mut characters = vec![];
        for (c, hidden) in entries.iter().zip(hidden) {
            if hidden {
                continue;
            }
            characters.push(Character {
                name: c["name"].as_str().ok_or(Unavailable)?.into(),
                level: c["level"].as_str().ok_or(Unavailable)?.into(),
                vocation: c["vocation"].as_u64().ok_or(Unavailable)?,
            });
        }
        characters.sort_by(|a, b| {
            let name = a.name.cmp(&b.name);
            if query.sort.as_deref() == Some("level") {
                b.level
                    .len()
                    .cmp(&a.level.len())
                    .then_with(|| b.level.cmp(&a.level))
                    .then(name)
            } else {
                name
            }
        });
        worlds.push(World {
            id,
            as_of: row.try_get("as_of").map_err(|_| Unavailable)?,
            stale: now.saturating_sub(u64::try_from(watermark).map_err(|_| Unavailable)?)
                > state.config.stale_after.as_secs(),
            characters,
        });
    }
    state.visibility.hidden_batch(&[]).await?;
    Ok((ids, worlds))
}
async fn page(
    State(state): State<Online>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    if !state.config.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or("")) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if query
        .sort
        .as_deref()
        .is_some_and(|s| !["name", "level"].contains(&s))
        || query.world.as_ref().is_some_and(|w| {
            !w.is_empty()
                && (w.len() != 36
                    || !w.bytes().enumerate().all(|(i, b)| {
                        if [8, 13, 18, 23].contains(&i) {
                            b == b'-'
                        } else {
                            b.is_ascii_hexdigit()
                        }
                    }))
        })
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let en = uri.path().starts_with("/en/");
    let result = load(&state, &query).await;
    let unavailable = result.is_err();
    let (ids, worlds) = result.unwrap_or_default();
    let page = Page {
        lang: if en { "en" } else { "pl" },
        labels: labels(en),
        unavailable,
        ids,
        worlds,
        filter: query.world.unwrap_or_default(),
        by_level: query.sort.as_deref() == Some("level"),
    };
    let mut response = match page.render() {
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
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
}

async fn response_policy(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "private, no-store"),
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
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
