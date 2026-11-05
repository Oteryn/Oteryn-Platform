use super::{now, wire, wire::Death};
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, Path, RawQuery, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::{PgPool, Row};
use std::{future::Future, pin::Pin, sync::Arc};
#[derive(Debug)]
pub struct Unavailable;
pub type PrivacyFuture<'a> = Pin<Box<dyn Future<Output = Result<bool, Unavailable>> + Send + 'a>>;
/// Resolve stable identity and current monotonic privacy floors before granting
/// public deaths and historical killer labels. Missing evidence must deny.
pub trait Privacy: Send + Sync {
    /// Prove that current privacy dependencies are available, even for empty lists.
    fn ready(&self) -> PrivacyFuture<'_> {
        Box::pin(async { Err(Unavailable) })
    }
    fn allows_deaths<'a>(&'a self, subject: &'a str, epoch: &'a str) -> PrivacyFuture<'a>;
}
#[derive(Debug)]
pub struct FailClosed;
impl Privacy for FailClosed {
    fn allows_deaths<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async { Err(Unavailable) })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub enabled: bool,
    pub source_authority: String,
}
#[derive(Clone)]
struct StateData {
    db: PgPool,
    privacy: Arc<dyn Privacy>,
    config: Config,
}
pub fn router(db: PgPool, privacy: Arc<dyn Privacy>, config: Config) -> Router {
    Router::new()
        .route("/deaths", get(world))
        .route("/pl/deaths", get(world))
        .route("/en/deaths", get(world))
        .route("/characters/{name}/deaths", get(character))
        .route("/pl/characters/{name}/deaths", get(character))
        .route("/en/characters/{name}/deaths", get(character))
        .with_state(StateData {
            db,
            privacy,
            config,
        })
        .layer(axum::middleware::from_fn(policy))
}
#[derive(serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Option<String>,
}
struct Entry {
    name: String,
    world: String,
    occurred: String,
    level: String,
    killers: String,
    pvp: bool,
    href: String,
    id: String,
    timestamp: u64,
}
#[derive(Template)]
#[template(path = "deaths/index.html", config = "src/deaths/askama.toml")]
struct Page {
    lang: &'static str,
    labels: Vec<String>,
    unavailable: bool,
    entries: Vec<Entry>,
    filter: String,
    character: String,
}
fn labels(en: bool) -> Vec<String> {
    let mut bundle = fluent_bundle::FluentBundle::new(vec![if en {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    }]);
    let source = if en {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    [
        "title",
        "unavailable",
        "window",
        "empty",
        "world",
        "apply",
        "level",
        "killers",
        "unrecorded",
        "pvp",
    ]
    .iter()
    .map(|key| {
        bundle
            .get_message(key)
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    })
    .collect()
}
async fn world(
    State(s): State<StateData>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
) -> Response {
    page(s, uri.path(), raw, None).await
}
async fn character(
    State(s): State<StateData>,
    OriginalUri(uri): OriginalUri,
    RawQuery(raw): RawQuery,
    Path(name): Path<String>,
) -> Response {
    if !wire::name(&name) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    page(s, uri.path(), raw, Some(name)).await
}
fn fresh(through: i64, now: u64) -> bool {
    u64::try_from(through)
        .is_ok_and(|t| t <= now.saturating_add(1) && now.saturating_sub(t).saturating_add(1) <= 30)
}
async fn evidence(s: &StateData) -> Result<String, Unavailable> {
    let row = sqlx::query("SELECT d.epoch::text, d.complete_through, p.watermark AS profile_through FROM public_deaths_v2_watermark d JOIN public_projection_state f ON f.id=1 AND f.highest_epoch=d.epoch JOIN public_snapshots p ON p.family='character_profile' AND p.is_watermark AND p.epoch=d.epoch WHERE d.id=1 AND d.authority=$1").bind(&s.config.source_authority).fetch_optional(&s.db).await.map_err(|_| Unavailable)?.ok_or(Unavailable)?;
    let through: i64 = row.try_get("complete_through").map_err(|_| Unavailable)?;
    let profile: i64 = row.try_get("profile_through").map_err(|_| Unavailable)?;
    if !fresh(through, now()) || !fresh(profile, now()) {
        return Err(Unavailable);
    }
    row.try_get("epoch").map_err(|_| Unavailable)
}
async fn load(
    s: &StateData,
    query: &Query,
    name: Option<&str>,
    en: bool,
) -> Result<Option<Vec<Entry>>, Unavailable> {
    if !wire::authority(&s.config.source_authority) {
        return Err(Unavailable);
    }
    // T24 takes this fence exclusively on every publication. Keep a shared lock
    // until presentation has read all profile/suppression evidence. Its public
    // latest API uses the pool, so a second connection must be available.
    if s.db.options().get_max_connections() < 2 {
        return Err(Unavailable);
    }
    let mut fence = s.db.begin().await.map_err(|_| Unavailable)?;
    let locked_epoch: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR SHARE",
    )
    .fetch_one(&mut *fence)
    .await
    .map_err(|_| Unavailable)?;
    if locked_epoch.is_none() {
        return Err(Unavailable);
    }

    if let Some(world) = query.world.as_deref().filter(|w| !w.is_empty()) {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM world_status WHERE name=$1")
            .bind(world)
            .fetch_one(&s.db)
            .await
            .map_err(|_| Unavailable)?;
        if count != 1 {
            return Err(Unavailable);
        }
    }
    let epoch = evidence(s).await?;
    if locked_epoch.as_deref() != Some(epoch.as_str()) {
        return Err(Unavailable);
    }
    if !s.privacy.ready().await? {
        return Err(Unavailable);
    }
    let subjects: Vec<String> = sqlx::query_scalar("SELECT subject FROM public_snapshots WHERE family='character_profile' AND NOT is_watermark AND epoch=$1::numeric ORDER BY subject LIMIT 10001").bind(&epoch).fetch_all(&s.db).await.map_err(|_| Unavailable)?;
    if subjects.len() > 10000 {
        return Err(Unavailable);
    }
    let mut entries = vec![];
    let mut matched = 0;
    let mut names = std::collections::HashSet::new();
    let mut visible = false;
    for subject in subjects {
        let profile = crate::ingest::latest(&s.db, "character_profile", &subject)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Unavailable)?;
        if profile["character_id"].as_str() != Some(subject.as_str()) {
            return Err(Unavailable);
        }
        let profiles = profile["profile"].as_array().ok_or(Unavailable)?;
        let Some(p) = profiles.first() else {
            continue;
        };
        let display = p["name"].as_str().ok_or(Unavailable)?;
        if name.is_none() && !names.insert(display.to_owned()) {
            return Err(Unavailable);
        }
        if name.is_some_and(|n| n != display) {
            continue;
        }
        if name.is_some() {
            matched += 1;
            if matched > 1 {
                return Ok(None);
            }
        }
        let world_id = p["world_id"].as_str().ok_or(Unavailable)?;
        let world: Option<String> =
            sqlx::query_scalar("SELECT name FROM world_status WHERE id::text=$1")
                .bind(world_id)
                .fetch_optional(&s.db)
                .await
                .map_err(|_| Unavailable)?;
        let world = world.ok_or(Unavailable)?;
        if query
            .world
            .as_deref()
            .is_some_and(|w| !w.is_empty() && w != world)
        {
            continue;
        }
        if !s.privacy.allows_deaths(&subject, &epoch).await? {
            continue;
        }
        visible = true;
        let payload: serde_json::Value = sqlx::query_scalar("SELECT deaths FROM public_deaths_v2 WHERE subject=$1 AND epoch=$2::numeric AND authority=$3").bind(&subject).bind(&epoch).bind(&s.config.source_authority).fetch_optional(&s.db).await.map_err(|_| Unavailable)?.ok_or(Unavailable)?;
        let deaths: Vec<Death> = serde_json::from_value(payload).map_err(|_| Unavailable)?;
        for d in deaths {
            let timestamp = wire::time(&d.occurred_at, 1, now()).map_err(|_| Unavailable)?;
            if timestamp < now().saturating_sub(30 * 86400) {
                continue;
            }
            let occurred = time::OffsetDateTime::from_unix_timestamp(
                i64::try_from(timestamp).map_err(|_| Unavailable)?,
            )
            .map_err(|_| Unavailable)?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| Unavailable)?;
            entries.push(Entry {
                name: display.into(),
                world: world.clone(),
                occurred,
                level: d.level,
                killers: d.killers.join(", "),
                pvp: d.pvp,
                href: format!(
                    "/{}/characters/{}/deaths",
                    if en { "en" } else { "pl" },
                    percent_encoding::utf8_percent_encode(
                        display,
                        percent_encoding::NON_ALPHANUMERIC
                    )
                ),
                id: d.death_occurrence_id,
                timestamp,
            });
        }
    }
    if evidence(s).await? != epoch {
        return Err(Unavailable);
    }
    if name.is_some() && (matched != 1 || !visible) {
        return Ok(None);
    }
    entries.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| b.id.cmp(&a.id))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries.truncate(if name.is_some() { 20 } else { 50 });
    Ok(Some(entries))
}
async fn page(
    s: StateData,
    path: &str,
    raw: Option<String>,
    character: Option<String>,
) -> Response {
    if !s.config.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(query) = serde_urlencoded::from_str::<Query>(raw.as_deref().unwrap_or("")) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if query
        .world
        .as_deref()
        .is_some_and(|w| w.len() > 96 || w.chars().any(char::is_control))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let en = path.starts_with("/en/");
    let result = load(&s, &query, character.as_deref(), en).await;
    if matches!(result, Ok(None)) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let unavailable = result.is_err();
    let page = Page {
        lang: if en { "en" } else { "pl" },
        labels: labels(en),
        unavailable,
        entries: result.ok().flatten().unwrap_or_default(),
        filter: query.world.unwrap_or_default(),
        character: character.unwrap_or_default(),
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
async fn policy(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "private, no-store"),
        (
            "content-security-policy",
            "default-src 'none'; style-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
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
