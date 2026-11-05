//! Spawn overlay and owner-only annotations, disabled by default.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use oteryn_world::{WorldOptions, WorldStore};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Uuid};
use std::collections::HashMap;
fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
pub fn configured_router(pool: PgPool) -> Router {
    let flag = |name| std::env::var(name).is_ok_and(|v| v == "true");
    router(
        pool,
        flag("OTERYN_ATLAS_SPAWNS_ENABLED"),
        flag("OTERYN_ATLAS_ENABLED") && flag("OTERYN_WORLD_ENABLED"),
    )
    .layer(Extension(oteryn_web::csrf::CsrfConfig {
        trusted_origins: std::env::var("OTERYN_PUBLIC_ORIGIN")
            .ok()
            .into_iter()
            .collect(),
        ..Default::default()
    }))
}
pub fn router(pool: PgPool, enabled: bool, world_enabled: bool) -> Router {
    if !enabled || !world_enabled {
        return Router::new();
    }
    Router::new()
        .route("/atlas/spawns-text/{lang}", get(texts))
        .route("/atlas/spawns", get(spawn_layer))
        .route("/atlas/private-markers", get(markers).post(add_marker))
        .route(
            "/atlas/private-markers/{id}",
            axum::routing::delete(remove_marker),
        )
        .route("/atlas/map-reports", post(report))
        .layer(DefaultBodyLimit::max(8192))
        .with_state(pool.clone())
        .layer(middleware::from_fn_with_state(
            SessionLayer::new(pool),
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn(no_cache))
}
async fn no_cache(request: axum::extract::Request, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
    pub floor: u8,
}
#[derive(Deserialize)]
struct SpawnDocument {
    declaration: Declaration,
}
#[derive(Deserialize)]
struct Declaration {
    points: Vec<Point>,
}
// SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
// content/world/spawns/spawns-00000-01999.json; one point represents one monster.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Point {
    pub cell: Cell,
    pub creature: String,
    pub respawn_ms: u32,
}
#[derive(Debug, Serialize)]
pub struct SpawnPoint {
    pub spawn: String,
    #[serde(flatten)]
    pub point: Point,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Viewport {
    version: String,
    floor: u8,
    min_x: u16,
    min_y: u16,
    max_x: u16,
    max_y: u16,
}
impl Viewport {
    fn contains(&self, c: Cell) -> bool {
        c.floor == self.floor
            && c.x >= self.min_x
            && c.x <= self.max_x
            && c.y >= self.min_y
            && c.y <= self.max_y
    }
}
async fn spawn_layer(
    State(pool): State<PgPool>,
    Query(view): Query<Viewport>,
) -> Result<Json<Vec<SpawnPoint>>, StatusCode> {
    if view.min_x > view.max_x || view.min_y > view.max_y {
        return Err(StatusCode::BAD_REQUEST);
    }
    let world = WorldStore::new(pool, WorldOptions { enabled: true });
    let version = world
        .active_version_id()
        .await
        .map_err(unavailable)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if !world
        .import_report(version)
        .await
        .map_err(unavailable)?
        .is_some_and(|r| {
            r.status == oteryn_world::VersionStatus::Ready && r.source_commit == view.version
        })
    {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut after = None;
    let mut creatures = HashMap::new();
    let mut output = Vec::new();
    loop {
        let entities = world
            .list_by_kind("spawns", after.as_deref(), 1000)
            .await
            .map_err(unavailable)?;
        let count = entities.len();
        for entity in entities {
            after = Some(entity.key.clone());
            if entity.version_id != version {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
            let document: SpawnDocument =
                serde_json::from_value(entity.data).map_err(unavailable)?;
            for point in document.declaration.points {
                if !view.contains(point.cell) {
                    continue;
                }
                if point.creature.is_empty() || point.respawn_ms == 0 {
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
                if !creatures.contains_key(&point.creature) {
                    let allowed = world
                        .get_entity("creatures", &point.creature)
                        .await
                        .map_err(unavailable)?
                        .is_some_and(|e| e.version_id == version);
                    creatures.insert(point.creature.clone(), allowed);
                }
                if creatures[&point.creature] {
                    output.push(SpawnPoint {
                        spawn: entity.key.clone(),
                        point,
                    });
                    if output.len() > 100_000 {
                        return Err(StatusCode::SERVICE_UNAVAILABLE);
                    }
                }
            }
        }
        if count < 1000 {
            break;
        }
    }
    if world.active_version_id().await.map_err(unavailable)? != Some(version) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(Json(output))
}
async fn owner(pool: &PgPool, session: &BrowserSession) -> Result<Uuid, StatusCode> {
    let id = session.account_id().await.ok_or(StatusCode::UNAUTHORIZED)?;
    let active: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active')")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(unavailable)?;
    if !active {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(id)
}
#[derive(Serialize, sqlx::FromRow)]
struct Marker {
    id: Uuid,
    x: i32,
    y: i32,
    floor: i16,
    label: String,
}
#[derive(Serialize)]
struct PrivateMarkers {
    csrf_token: String,
    markers: Vec<Marker>,
}
async fn markers(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Json<PrivateMarkers>, StatusCode> {
    let account = owner(&pool, &session).await?;
    let markers = sqlx::query_as("SELECT id,x,y,floor,label FROM atlas_private_markers WHERE account_id=$1 ORDER BY created_at,id").bind(account).fetch_all(&pool).await.map_err(unavailable)?;
    Ok(Json(PrivateMarkers {
        csrf_token: session.csrf_token().await.map_err(unavailable)?,
        markers,
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Annotation {
    x: u16,
    y: u16,
    floor: u8,
    text: String,
}
fn valid_text(text: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&text.chars().count())
        && text.trim() == text
        && !text.chars().any(char::is_control)
}
async fn add_marker(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Json(input): Json<Annotation>,
) -> Result<Response, StatusCode> {
    let account = owner(&pool, &session).await?;
    if !valid_text(&input.text, 1, 80) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let active: Option<bool> =
        sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    if active != Some(true) {
        return Err(StatusCode::FORBIDDEN);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM atlas_private_markers WHERE account_id=$1")
            .bind(account)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
    if count >= 100 {
        return Err(StatusCode::CONFLICT);
    }
    let marker: Marker = sqlx::query_as("INSERT INTO atlas_private_markers(account_id,x,y,floor,label) VALUES ($1,$2,$3,$4,$5) RETURNING id,x,y,floor,label").bind(account).bind(i32::from(input.x)).bind(i32::from(input.y)).bind(i16::from(input.floor)).bind(input.text).fetch_one(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok((StatusCode::CREATED, Json(marker)).into_response())
}
async fn remove_marker(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let account = owner(&pool, &session).await?;
    let result = sqlx::query("DELETE FROM atlas_private_markers WHERE id=$1 AND account_id=$2")
        .bind(id)
        .bind(account)
        .execute(&pool)
        .await
        .map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
/// Pending local queue is the narrow seam for the absent T33c support API.
async fn report(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Json(input): Json<Annotation>,
) -> Result<StatusCode, StatusCode> {
    let account = owner(&pool, &session).await?;
    if !valid_text(&input.text, 10, 2000) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut tx = pool.begin().await.map_err(unavailable)?;
    let active: Option<bool> =
        sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    if active != Some(true) {
        return Err(StatusCode::FORBIDDEN);
    }
    let rejected: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM atlas_map_reports WHERE account_id=$1 AND status='rejected'",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await
    .map_err(unavailable)?;
    if rejected >= 5 {
        return Err(StatusCode::FORBIDDEN);
    }
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM atlas_map_reports WHERE account_id=$1 AND status='pending'",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await
    .map_err(unavailable)?;
    if pending >= 5 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM atlas_map_reports WHERE account_id=$1 AND created_at>now()-interval '1 hour'").bind(account).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if count >= 5 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    sqlx::query(
        "INSERT INTO atlas_map_reports(account_id,x,y,floor,description) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(account)
    .bind(i32::from(input.x))
    .bind(i32::from(input.y))
    .bind(i16::from(input.floor))
    .bind(input.text)
    .execute(&mut *tx)
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(StatusCode::CREATED)
}
#[cfg(test)]
#[path = "spawns_tests.rs"]
mod tests;

async fn texts(Path(lang): Path<String>) -> Result<Json<HashMap<String, String>>, StatusCode> {
    let (source, locale) = match lang.as_str() {
        "en" => (include_str!("spawns_en.ftl"), unic_langid::langid!("en")),
        "pl" => (include_str!("spawns_pl.ftl"), unic_langid::langid!("pl")),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let resource = fluent_bundle::FluentResource::try_new(source.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    bundle
        .add_resource(resource)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut result = HashMap::new();
    for key in [
        "spawns",
        "monsters",
        "sites",
        "respawn",
        "place",
        "fullscreen",
        "private",
        "label",
        "save",
        "remove",
        "report",
        "description",
        "send",
        "saved",
        "failed",
        "sign-in",
        "loading",
    ] {
        let pattern = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let value = bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned();
        result.insert(key.into(), value);
    }
    Ok(Json(result))
}

/// The existing viewer uses a restrictive CSP; the overlay needs same-origin fetch.
/// Apply after mounting the viewer, preserving all of its other directives.
pub async fn viewer_policy(request: axum::extract::Request, next: middleware::Next) -> Response {
    let viewer = matches!(request.uri().path(), "/pl/atlas" | "/en/atlas");
    let mut response = next.run(request).await;
    if viewer
        && let Some(value) = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok())
    {
        let policy = format!("{value}; connect-src 'self'");
        if let Ok(value) = policy.parse() {
            response
                .headers_mut()
                .insert(header::CONTENT_SECURITY_POLICY, value);
        }
    }
    response
}
