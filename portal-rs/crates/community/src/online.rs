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

const FRESHNESS_SECONDS: u64 = 30;
const CLOCK_UNCERTAINTY_SECONDS: u64 = 1;

#[derive(Clone, Debug)]
pub struct CharacterRef {
    pub world: String,
    pub name: String,
    pub epoch: String,
    pub observed_at: i64,
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
    /// Age at which to label an individual snapshot while the family watermark is fresh.
    pub stale_after: Duration,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            stale_after: Duration::from_secs(FRESHNESS_SECONDS),
        }
    }
}
#[derive(Clone)]
struct Online {
    db: PgPool,
    fences: PgPool,
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
            // Fence transactions have separate bounded capacity. Privacy adapters
            // can use every ordinary pool connection without deadlocking readers.
            fences: sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .min_connections(0)
                .acquire_timeout(std::time::Duration::from_secs(2))
                .connect_lazy_with(db.connect_options().as_ref().clone()),
            db,
            visibility,
            config,
        })
}
pub fn configured_router<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    router(
        db.clone(),
        Arc::new(crate::privacy::Authority::configured(db)),
        Config {
            enabled: std::env::var("PORTAL_ONLINE_ENABLED").as_deref() == Ok("true"),
            stale_after: Duration::from_secs(
                std::env::var("PORTAL_ONLINE_STALE_SECONDS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(FRESHNESS_SECONDS),
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
    observed: u64,
    id: String,
    as_of: String,
    stale: bool,
    characters: Vec<Character>,
}
#[derive(Default)]
struct Catalogue {
    ids: Vec<String>,
    worlds: Vec<World>,
}
#[derive(Template)]
#[template(path = "pages/online.html", config = "src/online.askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    stale: bool,
    ids: Vec<String>,
    filter: String,
    by_level: bool,
    worlds: Vec<World>,
}
fn labels(en: bool) -> Vec<String> {
    let source = if en {
        include_str!("../../ui/i18n/en.ftl")
    } else {
        include_str!("../../ui/i18n/pl.ftl")
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
fn now() -> Result<u64, Unavailable> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Unavailable)?
        .as_secs())
}

fn watermark_is_fresh(complete_through: u64, now: u64) -> bool {
    (now.saturating_sub(FRESHNESS_SECONDS - CLOCK_UNCERTAINTY_SECONDS)
        ..=now.saturating_add(CLOCK_UNCERTAINTY_SECONDS))
        .contains(&complete_through)
}

async fn load_inner(state: &Online, query: &Query) -> Result<Option<Catalogue>, Unavailable> {
    let privacy_cut = crate::privacy::Cut::read(&state.db)
        .await
        .map_err(|_| Unavailable)?;
    // The existing publisher takes this fence exclusively, including suppression.
    let mut fence = state.fences.begin().await.map_err(|_| Unavailable)?;
    sqlx::query("SELECT set_config('statement_timeout','2500ms',true),set_config('idle_in_transaction_session_timeout','3s',true)")
        .execute(&mut *fence).await.map_err(|_| Unavailable)?;
    sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR SHARE")
        .fetch_one(&mut *fence)
        .await
        .map_err(|_| Unavailable)?;
    if sqlx::query_scalar::<_, bool>("SELECT public_projection_conflicted('world_online')")
        .fetch_one(&mut *fence)
        .await
        .map_err(|_| Unavailable)?
    {
        return Err(Unavailable);
    }
    // The family cut advances after a complete scan. Unchanged worlds can retain older snapshots.
    // A missing or stale cut withholds names, but must not mask the fail-closed
    // checks below (visibility authority, privacy cut), which answer 503.
    let complete_through = sqlx::query_scalar::<_, i64>(
        "SELECT watermark FROM public_snapshots
         WHERE family='world_online' AND subject='' AND is_watermark
           AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)",
    )
    .fetch_optional(&mut *fence)
    .await
    .map_err(|_| Unavailable)?
    .map(u64::try_from)
    .transpose()
    .map_err(|_| Unavailable)?;
    let fresh_cut = match complete_through {
        Some(cut) if watermark_is_fresh(cut, now()?) => Some(cut),
        _ => None,
    };
    let mut visible_refs = Vec::new();
    let rows = if fresh_cut.is_none() {
        Vec::new()
    } else {
        sqlx::query("SELECT subject, epoch::text, watermark, to_char(to_timestamp(watermark) AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS as_of, payload FROM public_snapshots WHERE family='world_online' AND NOT public_projection_conflicted('world_online') AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) ORDER BY subject LIMIT 257").fetch_all(&mut *fence).await.map_err(|_| Unavailable)?
    };
    if rows.len() > 256 {
        return Err(Unavailable);
    }
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
                    epoch: row.try_get("epoch").map_err(|_| Unavailable)?,
                    observed_at: watermark,
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
        for ((c, reference), hidden) in entries.iter().zip(refs).zip(hidden) {
            if hidden {
                continue;
            }
            visible_refs.push(reference);
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
        let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='world_online' AND NOT public_projection_conflicted('world_online') AND NOT is_watermark AND subject=$1 AND payload=$2 AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1))").bind(&id).bind(&payload).fetch_one(&state.db).await.map_err(|_|Unavailable)?;
        if !current {
            return Err(Unavailable);
        }
        worlds.push(World {
            observed: u64::try_from(watermark).map_err(|_| Unavailable)?,
            id,
            as_of: row.try_get("as_of").map_err(|_| Unavailable)?,
            stale: false,
            characters,
        });
    }
    state.visibility.hidden_batch(&[]).await?;
    let final_hidden = state.visibility.hidden_batch(&visible_refs).await?;
    if final_hidden.len() != visible_refs.len() || final_hidden.into_iter().any(|hidden| hidden) {
        return Err(Unavailable);
    }
    if !privacy_cut
        .validate(&mut fence)
        .await
        .map_err(|_| Unavailable)?
    {
        return Err(Unavailable);
    }
    fence.commit().await.map_err(|_| Unavailable)?;
    let serving = now()?;
    let Some(complete_through) = fresh_cut else {
        return Ok(None);
    };
    if !watermark_is_fresh(complete_through, serving) {
        return Ok(None);
    }
    for world in &mut worlds {
        world.stale = serving.saturating_sub(world.observed) > state.config.stale_after.as_secs();
    }
    Ok(Some(Catalogue { ids, worlds }))
}
async fn page(
    State(state): State<Online>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
    headers: axum::http::HeaderMap,
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
    let language = oteryn_web::Language::for_path(uri.path(), &headers);
    let en = language == oteryn_web::Language::En;
    let result = load(&state, &query).await;
    let unavailable = result.is_err();
    let stale = matches!(result, Ok(None));
    let Catalogue { ids, worlds } = result.ok().flatten().unwrap_or_default();
    let page = Page {
        lang: language.code(),
        labels: labels(en),
        unavailable,
        stale,
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

async fn load(state: &Online, query: &Query) -> Result<Option<Catalogue>, Unavailable> {
    tokio::time::timeout(Duration::from_secs(3), load_inner(state, query))
        .await
        .map_err(|_| Unavailable)?
}
