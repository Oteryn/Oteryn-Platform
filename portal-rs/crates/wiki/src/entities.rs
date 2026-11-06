//! Public world wiki. Editorial bindings use `kind:key` in T13's entity_key.
//! Only the world's public API may supply entity facts; source JSON is never rendered.
use askama::Template;
use axum::{
    Router,
    extract::{OriginalUri, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_world::{Entity, WorldOptions, WorldStore};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{domain, repository::PublicRepository};

const KINDS: [&str; 8] = [
    "items",
    "creatures",
    "npcs",
    "spells",
    "quests",
    "loot",
    "achievements",
    "areas",
];
const PAGE_SIZE: usize = 24;
fn world_kind(route: &str) -> &'static str {
    match route {
        "items" => "Item",
        "creatures" => "Creature",
        "npcs" => "NPC",
        "spells" => "Spell",
        "quests" => "Quest",
        "loot" => "Loot",
        "achievements" => "Achievement",
        "areas" => "Area",
        _ => "",
    }
}
fn route_kind(family: &str) -> &'static str {
    KINDS
        .into_iter()
        .find(|route| world_kind(route) == family)
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct EntityState {
    pool: PgPool,
    world: WorldStore,
    enabled: bool,
}
impl EntityState {
    pub fn new(pool: PgPool, enabled: bool) -> Self {
        Self {
            world: WorldStore::new(pool.clone(), WorldOptions { enabled }),
            pool,
            enabled,
        }
    }
    pub fn configured(pool: PgPool) -> Self {
        Self::new(
            pool,
            std::env::var("PORTAL_WIKI_ENTITIES_ENABLED").as_deref() == Ok("true"),
        )
    }
}
/// Mounts entity routes and extends the existing home/search without replacing T13 routes.
pub fn mount(app: Router, state: EntityState) -> Router {
    let entities = Router::new()
        .route("/wiki/{kind}", get(list))
        .route("/wiki/{kind}/{key}", get(detail))
        .route("/{locale}/wiki/{kind}", get(list))
        .route("/{locale}/wiki/{kind}/{key}", get(detail))
        .with_state(state.clone());
    app.merge(entities)
        .layer(from_fn_with_state(state, dispatch))
        .layer(axum::middleware::from_fn(policy))
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Filters {
    #[serde(default)]
    q: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    classification: String,
    #[serde(default)]
    kind: String,
    page: Option<i64>,
}
impl Filters {
    fn validate(&mut self) -> Result<(), StatusCode> {
        if self.q.len() > 1024 || self.q.chars().any(char::is_control) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        self.q = domain::search(&self.q, self.page.unwrap_or(1))
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if [&self.category, &self.classification]
            .iter()
            .any(|v| v.len() > 80 || v.chars().any(char::is_control))
            || (!self.kind.is_empty() && !KINDS.contains(&self.kind.as_str()))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(())
    }
}
fn language(uri: &axum::http::Uri, headers: &HeaderMap) -> Result<&'static str, StatusCode> {
    match uri.path().split('/').nth(1) {
        Some("pl") => Ok("pl"),
        Some("en") => Ok("en"),
        Some("wiki") => Ok(
            if headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("pl"))
            {
                "pl"
            } else {
                "en"
            },
        ),
        _ => Err(StatusCode::NOT_FOUND),
    }
}
fn t(lang: &str, key: &str) -> String {
    let source = if lang == "pl" {
        include_str!("entities/pl.ftl")
    } else {
        include_str!("entities/en.ftl")
    };
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.into()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|v| bundle.format_pattern(v, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn error(status: StatusCode, lang: &str) -> Response {
    let key = match status {
        StatusCode::NOT_FOUND => "missing",
        StatusCode::UNPROCESSABLE_ENTITY => "invalid",
        _ => "unavailable",
    };
    crate::web::notice(status, lang, t(lang, key))
}
fn render<T: Template>(page: T) -> Response {
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "en"),
    }
}
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn body(entity: &Entity) -> &serde_json::Value {
    entity
        .data
        .get("definition")
        .or_else(|| entity.data.get("declaration"))
        .or_else(|| entity.data.pointer("/bundle/spell"))
        .unwrap_or(&entity.data)
}
fn scalar<'a>(data: &'a serde_json::Value, field: &str) -> &'a str {
    data.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| v.len() <= 256 && !v.chars().any(char::is_control))
        .unwrap_or_default()
}
fn name(entity: &Entity, lang: &str) -> String {
    let data = body(entity);
    if let Some(details) = entity.data.pointer("/authoring/profile/details") {
        let value = scalar(details, "display_name");
        if !value.is_empty() {
            return value.into();
        }
    }
    let localized = data
        .get("names")
        .and_then(|v| v.get(lang))
        .and_then(|v| v.as_str());
    let value = localized
        .or_else(|| {
            ["display_name", "name"]
                .iter()
                .map(|key| scalar(data, key))
                .find(|v| !v.is_empty())
        })
        .unwrap_or_default();
    if !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control) {
        return value.into();
    }
    if let Some(presentation) = data.pointer("/semantics/presentation")
        && presentation.get("state").and_then(|v| v.as_str()) == Some("KNOWN")
        && let Some(name) = presentation.pointer("/value/name")
        && name.get("state").and_then(|v| v.as_str()) == Some("KNOWN")
    {
        let value = scalar(name, "value");
        if !value.is_empty() {
            return value.into();
        }
    }
    entity.key.clone()
}
struct Row {
    title: String,
    href: String,
    category: String,
    classification: String,
}
fn row(entity: &Entity, lang: &str) -> Row {
    Row {
        title: name(entity, lang),
        href: format!(
            "/{lang}/wiki/{}/{}",
            route_kind(&entity.kind),
            segment(&entity.key)
        ),
        category: scalar(body(entity), "category").into(),
        classification: scalar(body(entity), "classification").into(),
    }
}
#[derive(Template)]
#[template(path = "wiki/entities_list.html", config = "wiki_askama.toml")]
struct Listing {
    lang: &'static str,
    filters: Filters,
    rows: Vec<Row>,
    guides: Vec<domain::Translation>,
    version: String,
    next: i64,
    title: String,
    route: String,
    per_type: bool,
}
impl Listing {
    fn selected(&self, kind: &str) -> bool {
        self.filters.kind == kind
    }
    fn t(&self, key: &str) -> String {
        t(self.lang, key)
    }
}
async fn collect(
    state: &EntityState,
    filters: &Filters,
    lang: &str,
) -> Result<(String, Vec<Row>), StatusCode> {
    let version = state
        .world
        .active_version_id()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let mut rows = Vec::new();
    let kinds = if filters.kind.is_empty() {
        KINDS.to_vec()
    } else {
        vec![filters.kind.as_str()]
    };
    let needed = filters.page.unwrap_or(1) as usize * PAGE_SIZE + 1;
    'kinds: for kind in kinds {
        let mut after = None;
        // Bounded scan through the public keyset API. Never silently truncate results.
        for batch in 0..100 {
            let entities = state
                .world
                .list_by_kind(world_kind(kind), after.as_deref(), 1000)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            for entity in &entities {
                if entity.version_id != version {
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
                let row = row(entity, lang);
                if (filters.category.is_empty() || row.category == filters.category)
                    && (filters.classification.is_empty()
                        || row.classification == filters.classification)
                    && (filters.q.is_empty()
                        || row.title.to_lowercase().contains(&filters.q.to_lowercase())
                        || entity
                            .key
                            .to_lowercase()
                            .contains(&filters.q.to_lowercase()))
                {
                    rows.push(row);
                    if rows.len() == needed {
                        break 'kinds;
                    }
                }
            }
            after = entities.last().map(|e| e.key.clone());
            if entities.len() < 1000 {
                break;
            }
            if batch == 99 {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }
    if state
        .world
        .active_version_id()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        != Some(version)
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok((version.to_string(), rows))
}
async fn listing(
    state: &EntityState,
    lang: &'static str,
    mut filters: Filters,
    search: bool,
) -> Response {
    if !state.enabled {
        return error(StatusCode::NOT_FOUND, lang);
    }
    if let Err(status) = filters.validate() {
        return error(status, lang);
    }
    let empty_search = search
        && filters.q.is_empty()
        && filters.kind.is_empty()
        && filters.category.is_empty()
        && filters.classification.is_empty();
    let result = if empty_search {
        state
            .world
            .active_version_id()
            .await
            .ok()
            .flatten()
            .map(|v| (v.to_string(), Vec::new()))
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)
    } else {
        collect(state, &filters, lang).await
    };
    let (version, rows) = match result {
        Ok(v) => v,
        Err(s) => return error(s, lang),
    };
    let number = filters.page.unwrap_or(1);
    let mut rows = if empty_search { vec![] } else { rows };
    let start = (number as usize - 1) * PAGE_SIZE;
    let next = if rows.len() > start + PAGE_SIZE {
        number + 1
    } else {
        0
    };
    rows = rows.into_iter().skip(start).take(PAGE_SIZE).collect();
    let guides = if search || filters.kind.is_empty() {
        match PublicRepository::new(state.pool.clone())
            .search(lang, &filters.q, number, !search)
            .await
        {
            Ok(v) => v,
            Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, lang),
        }
    } else {
        vec![]
    };
    let next = if guides.len() == 12 && number < 1000 {
        number + 1
    } else {
        next
    };
    let title = if filters.kind.is_empty() {
        t(lang, "wiki")
    } else {
        t(lang, &filters.kind)
    };
    let route = if search {
        "/wiki/search".into()
    } else if filters.kind.is_empty() {
        "/wiki".into()
    } else {
        format!("/wiki/{}", filters.kind)
    };
    let next = if number < 1000 { next } else { 0 };
    let per_type = !search && !filters.kind.is_empty();
    render(Listing {
        per_type,
        route,
        lang,
        filters,
        rows,
        guides,
        version,
        next,
        title,
    })
}
async fn list(
    State(state): State<EntityState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    query: Result<Query<Filters>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let lang = match language(&uri, &headers) {
        Ok(v) => v,
        Err(s) => return error(s, "en"),
    };
    let kind = uri
        .path()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    if !KINDS.contains(&kind) {
        return error(StatusCode::NOT_FOUND, lang);
    }
    let mut filters = match query {
        Ok(Query(v)) => v,
        Err(_) => return error(StatusCode::UNPROCESSABLE_ENTITY, lang),
    };
    if !filters.kind.is_empty() && filters.kind != kind {
        return error(StatusCode::UNPROCESSABLE_ENTITY, lang);
    }
    filters.kind = kind.into();
    listing(&state, lang, filters, false).await
}
async fn dispatch(State(state): State<EntityState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let home = matches!(path, "/wiki" | "/en/wiki" | "/pl/wiki");
    let search = matches!(path, "/wiki/search" | "/en/wiki/search" | "/pl/wiki/search");
    let entity = path
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair[0] == "wiki" && KINDS.contains(&pair[1]));
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) || !state.enabled
        || (!home && !search && !entity)
    {
        return next.run(request).await;
    }
    let lang = match language(request.uri(), request.headers()) {
        Ok(v) => v,
        Err(s) => return error(s, "en"),
    };
    // Preserve the shared search abuse boundary when extending T13's route.
    {
        use axum::extract::FromRequestParts;
        let (mut parts, body) = request.into_parts();
        parts
            .extensions
            .insert(oteryn_web::ratelimit::RateLimiter::new(state.pool.clone()));
        parts.extensions.insert(if lang == "pl" {
            oteryn_web::Language::Pl
        } else {
            oteryn_web::Language::En
        });
        if let Err(rejection) = oteryn_web::ratelimit::RateLimit::<oteryn_web::ratelimit::WikiSearch>::from_request_parts(&mut parts, &()).await { return rejection.into_response() }
        let request = Request::from_parts(parts, body);
        if entity {
            return next.run(request).await;
        }
        dispatch_listing(state, request, lang, search).await
    }
}
async fn policy(request: Request, next: Next) -> Response {
    let wiki = request.uri().path().split('/').any(|part| part == "wiki");
    let head = request.method() == axum::http::Method::HEAD;
    let mut response = next.run(request).await;
    if wiki {
        for (name, value) in [
            ("cache-control", "no-store"),
            ("vary", "Accept-Language"),
            (
                "content-security-policy",
                "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
            ),
            ("x-content-type-options", "nosniff"),
            ("referrer-policy", "strict-origin-when-cross-origin"),
            (
                "permissions-policy",
                "camera=(), microphone=(), geolocation=()",
            ),
            (
                "strict-transport-security",
                "max-age=63072000; includeSubDomains",
            ),
        ] {
            response
                .headers_mut()
                .insert(name, header::HeaderValue::from_static(value));
        }
        if head {
            *response.body_mut() = axum::body::Body::empty();
        }
    }
    response
}
async fn dispatch_listing(
    state: EntityState,
    request: Request,
    lang: &'static str,
    search: bool,
) -> Response {
    let filters = match Query::<Filters>::try_from_uri(request.uri()) {
        Ok(Query(v)) => v,
        Err(_) => return error(StatusCode::UNPROCESSABLE_ENTITY, lang),
    };
    let mut response = listing(&state, lang, filters, search).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::VARY,
        header::HeaderValue::from_static("Accept-Language"),
    );
    response
}
#[derive(Template)]
#[template(path = "wiki/entity.html", config = "wiki_askama.toml")]
struct Detail {
    lang: &'static str,
    title: String,
    kind: String,
    key: String,
    version: String,
    classification: String,
    facts: Vec<(String, String)>,
    editorial: String,
    route: String,
}
impl Detail {
    fn t(&self, key: &str) -> String {
        t(self.lang, key)
    }
}
async fn detail(
    State(state): State<EntityState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<std::collections::HashMap<String, String>>,
) -> Response {
    let lang = match language(&uri, &headers) {
        Ok(v) => v,
        Err(s) => return error(s, "en"),
    };
    if !state.enabled {
        return error(StatusCode::NOT_FOUND, lang);
    }
    let kind = params.get("kind").map(String::as_str).unwrap_or_default();
    let key = params.get("key").map(String::as_str).unwrap_or_default();
    if !KINDS.contains(&kind)
        || key.is_empty()
        || key.len() > 512
        || key.chars().any(char::is_control)
    {
        return error(StatusCode::NOT_FOUND, lang);
    }
    let entity = match state.world.get_entity(world_kind(kind), key).await {
        Ok(Some(v)) => v,
        Ok(None) => return error(StatusCode::NOT_FOUND, lang),
        Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, lang),
    };
    let mut facts = Vec::new();
    for field in [
        "health",
        "experience",
        "speed",
        "armor",
        "defense",
        "attack",
        "weight",
        "grade",
        "points",
        "cooldown_ms",
    ] {
        let value = body(&entity)
            .get(field)
            .or_else(|| {
                entity
                    .data
                    .pointer("/authoring/profile")
                    .and_then(|v| v.get(field))
            })
            .or_else(|| {
                entity
                    .data
                    .pointer("/authoring/profile/details")
                    .and_then(|v| v.get(field))
            });
        let hidden_health = field == "health"
            && entity
                .data
                .pointer("/authoring/profile/details/flags/health_hidden")
                .is_some_and(|v| v != &serde_json::Value::Bool(false));
        if let Some(value) = value.filter(|v| v.is_number() && !hidden_health) {
            facts.push((t(lang, field), value.to_string()));
        }
    }
    // Resolve only the current published revision; multiple bindings are ambiguous.
    let slugs: Result<Vec<String>, _> = sqlx::query_scalar("SELECT r.slug FROM wiki_guides g JOIN wiki_revisions r ON r.guide_id=g.id AND r.version=g.revision_version WHERE g.entity_key=$1 AND g.status='published' AND g.published_at<=now() AND r.locale=$2 LIMIT 2")
        .bind(format!("{kind}:{key}")).bind(lang).fetch_all(&state.pool).await;
    let slugs = match slugs {
        Ok(v) if v.len() <= 1 => v,
        _ => return error(StatusCode::SERVICE_UNAVAILABLE, lang),
    };
    let editorial = if let Some(slug) = slugs.first() {
        match PublicRepository::new(state.pool.clone())
            .article(lang, slug)
            .await
        {
            Ok(v) => domain::render(&v.markdown),
            Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, lang),
        }
    } else {
        String::new()
    };
    // Recheck after editorial I/O, so a world switch or visibility denial cannot mix versions.
    match state.world.get_entity(world_kind(kind), key).await {
        Ok(Some(current)) if current.version_id == entity.version_id => {}
        Ok(None) => return error(StatusCode::NOT_FOUND, lang),
        _ => return error(StatusCode::SERVICE_UNAVAILABLE, lang),
    }
    render(Detail {
        route: format!("/wiki/{kind}/{}", segment(key)),
        lang,
        title: name(&entity, lang),
        kind: kind.into(),
        key: key.into(),
        version: entity.version_id.to_string(),
        classification: scalar(body(&entity), "classification").into(),
        facts,
        editorial,
    })
}

#[cfg(test)]
mod tests;
