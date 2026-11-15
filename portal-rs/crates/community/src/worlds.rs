//! Public registry metadata. Runtime health and population require an accepted
//! runtime-status authority; individual presence supplies neither claim.
//! Enable with PORTAL_WORLDS_ENABLED=true (default off). Freshness is configured
//! by PORTAL_WORLDS_FRESHNESS_SECONDS (default 120); invalid values disable routes.
use askama::Template;
use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_web::Language;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Clone, Debug)]
pub struct Config {
    pub enabled: bool,
    pub freshness_seconds: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            freshness_seconds: 120,
        }
    }
}
#[derive(Clone, Debug)]
struct Worlds {
    db: PgPool,
}
pub fn configured_router(db: PgPool) -> Router {
    let threshold = std::env::var("PORTAL_WORLDS_FRESHNESS_SECONDS")
        .map_or(Some(120), |s| s.parse::<u64>().ok().filter(|n| *n > 0));
    router(
        db,
        Config {
            enabled: std::env::var("PORTAL_WORLDS_ENABLED").as_deref() == Ok("true")
                && threshold.is_some(),
            freshness_seconds: threshold.unwrap_or(120),
        },
    )
}
pub fn router(db: PgPool, config: Config) -> Router {
    if !config.enabled || config.freshness_seconds == 0 {
        return Router::new();
    }
    Router::new()
        .route("/worlds", get(page))
        .route("/pl/worlds", get(page))
        .route("/en/worlds", get(page))
        .route("/api/v1/worlds", get(api))
        .with_state(Worlds { db })
        .layer(middleware::from_fn(response_policy))
}
async fn response_policy(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'none'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    if !response.headers().contains_key(header::CACHE_CONTROL) {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}
#[derive(Debug)]
struct World {
    id: String,
    name: String,
    pvp: Option<String>,
    location: Option<String>,
    status: &'static str,
    count: Option<i32>,
    as_of: Option<String>,
}
async fn read(state: &Worlds) -> Result<Vec<World>, sqlx::Error> {
    // Presence snapshots grant neither runtime health nor aggregate authority.
    let rows = sqlx::query("SELECT id::text, name, pvp_type, location FROM world_status ORDER BY name COLLATE \"C\", id LIMIT 257").fetch_all(&state.db).await?;
    if rows.len() > 256 {
        return Err(sqlx::Error::Protocol(
            "world catalogue exceeds serving bound".into(),
        ));
    }
    rows.into_iter()
        .map(|row| {
            Ok(World {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                pvp: row.try_get("pvp_type")?,
                location: row.try_get("location")?,
                status: "unknown",
                count: None,
                as_of: None,
            })
        })
        .collect()
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
async fn api(State(state): State<Worlds>, headers: HeaderMap) -> Response {
    let Ok(worlds) = read(&state).await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let generated_at = now();
    let worlds: Vec<Value> = worlds
        .iter()
        .map(|w| json!({"id":w.id,"name":w.name,"status":w.status,"players_online":w.count}))
        .collect();
    // Weak validators ignore generation time (Unix seconds), while tracking all
    // public world fields, including registry changes and freshness transitions.
    let etag = format!(
        "W/\"{:x}\"",
        Sha256::digest(json!({"schema":1,"worlds":worlds}).to_string())
    );
    let unchanged = headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(','))
        .any(|tag| {
            let tag = tag.trim();
            tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == &etag[2..]
        });
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        axum::Json(json!({"schema":1,"generated_at":generated_at,"worlds":worlds})).into_response()
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(etag) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, etag);
    }
    response
}
#[derive(Template)]
#[template(path = "pages/worlds.html", config = "src/worlds_askama.toml")]
struct Page {
    lang: &'static str,
    language: Language,
    worlds: Vec<World>,
}
fn language(path: &str, headers: &HeaderMap) -> Language {
    if path.starts_with("/en/") {
        return Language::En;
    }
    if path.starts_with("/pl/") {
        return Language::Pl;
    }
    for value in headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
    {
        for part in value.split(';') {
            match part.trim() {
                "portal_language=en" => return Language::En,
                "portal_language=pl" => return Language::Pl,
                _ => {}
            }
        }
    }
    if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.starts_with("en"))
    {
        Language::En
    } else {
        Language::Pl
    }
}
async fn page(State(state): State<Worlds>, request: Request) -> Response {
    let language = language(request.uri().path(), request.headers());
    let Ok(worlds) = read(&state).await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut response = oteryn_web::render(&Page {
        lang: language.code(),
        language,
        worlds,
    });
    response.headers_mut().insert(
        header::CONTENT_LANGUAGE,
        HeaderValue::from_static(language.code()),
    );
    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static("Accept-Language, Cookie"),
    );
    response
}

/// Status for the home page from the same authority as the
/// worlds page. `None` when the module is off or its source is unavailable.
pub async fn home_status(db: &PgPool) -> Option<oteryn_ui::HomeStatus> {
    std::env::var("PORTAL_WORLDS_FRESHNESS_SECONDS")
        .map_or(Some(120), |s| s.parse::<u64>().ok().filter(|n| *n > 0))?;
    if std::env::var("PORTAL_WORLDS_ENABLED").as_deref() != Ok("true") {
        return None;
    }
    let state = Worlds { db: db.clone() };
    home_status_for(&state).await
}
async fn home_status_for(state: &Worlds) -> Option<oteryn_ui::HomeStatus> {
    let worlds = read(state).await.ok()?;
    if worlds.is_empty() {
        return None;
    }
    let fresh: Vec<&World> = worlds.iter().filter(|w| w.status == "online").collect();
    let online =
        (!fresh.is_empty()).then(|| fresh.iter().filter_map(|w| w.count).map(i64::from).sum());
    let first = worlds[0].pvp.clone();
    let pvp = first.filter(|p| !p.is_empty() && worlds.iter().all(|w| w.pvp.as_deref() == Some(p)));
    Some(oteryn_ui::HomeStatus {
        online,
        worlds_fresh: fresh.len(),
        worlds_total: worlds.len(),
        pvp,
        as_of: fresh.iter().filter_map(|w| w.as_of.clone()).max(),
    })
}

#[cfg(test)]
#[path = "worlds_audit_tests.rs"]
mod audit_tests;
