//! G-4 v2 character families (accepted extension contract rev 3): `character_skills`
//! is public; `character_achievements`, `character_equipment` and `character_outfit`
//! are owner-only. Ingest is mounted on the internal mTLS listener only
//! (`bin/oteryn-portal/src/game_routes.rs`); every flag defaults off.
use axum::{
    Extension, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, IdentityMap, JsonRule, PublisherKey, RoutePolicy},
};
use sqlx::PgPool;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
mod storage;
mod web;
mod wire;
pub use web::{
    DenySkills, PrivacyFuture, ServingConfig, SkillsPrivacy, Unavailable, configured_router, router,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Skills,
    Achievements,
    Equipment,
    Outfit,
}
impl Family {
    pub const ALL: [Family; 4] = [
        Family::Skills,
        Family::Achievements,
        Family::Equipment,
        Family::Outfit,
    ];
    /// The shared V2 watermark `family` value.
    pub fn name(self) -> &'static str {
        match self {
            Family::Skills => "character_skills",
            Family::Achievements => "character_achievements",
            Family::Equipment => "character_equipment",
            Family::Outfit => "character_outfit",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }
    /// Path under `/internal/v2/game-public/`.
    pub fn path(self) -> &'static str {
        match self {
            Family::Skills => "character-skills",
            Family::Achievements => "character-achievements",
            Family::Equipment => "character-equipment",
            Family::Outfit => "character-outfit",
        }
    }
    fn operation(self) -> &'static str {
        match self {
            Family::Skills => "PublishCharacterSkillsV2",
            Family::Achievements => "PublishCharacterAchievementsV2",
            Family::Equipment => "PublishCharacterEquipmentV2",
            Family::Outfit => "PublishCharacterOutfitV2",
        }
    }
    /// Maximum request bytes from the contract's route table.
    pub fn max_bytes(self) -> usize {
        match self {
            Family::Skills => 4096,
            Family::Achievements => 262144,
            Family::Equipment => 8192,
            Family::Outfit => 2048,
        }
    }
}

pub const WATERMARK_PATH: &str = "/internal/v2/game-public/watermark";
const WATERMARK_BYTES: usize = 512;
/// Five nested containers below the root object, as in the guard's depth count.
const DEPTH: usize = 6;

/// Per-family ingest flags; a missing flag is off.
#[derive(Debug, Clone)]
pub struct IngestConfig {
    pub skills: bool,
    pub achievements: bool,
    pub equipment: bool,
    pub outfit: bool,
    pub source_authority: String,
    pub requests_per_minute: u16,
}
impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            skills: false,
            achievements: false,
            equipment: false,
            outfit: false,
            source_authority: String::new(),
            requests_per_minute: 6000,
        }
    }
}
impl IngestConfig {
    fn usable(&self) -> bool {
        wire::authority(&self.source_authority) && (1..=6000).contains(&self.requests_per_minute)
    }
    pub fn permits(&self, family: Family) -> bool {
        self.usable()
            && match family {
                Family::Skills => self.skills,
                Family::Achievements => self.achievements,
                Family::Equipment => self.equipment,
                Family::Outfit => self.outfit,
            }
    }
}

#[derive(Clone)]
struct Ingest {
    db: PgPool,
    config: IngestConfig,
    family: Option<Family>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Mount on the internal listener only; `VerifiedClient` must come from the TLS adapter.
/// The watermark route answers only this module's four families.
pub fn ingest_router(db: PgPool, identities: Arc<IdentityMap>, config: IngestConfig) -> Router {
    let mut app = Router::new();
    for family in Family::ALL.into_iter().map(Some).chain([None]) {
        let path = family.map_or("watermark", Family::path);
        let enabled = family.map_or_else(
            || Family::ALL.iter().any(|f| config.permits(*f)),
            |f| config.permits(f),
        );
        let route = Router::new()
            .route(&format!("/internal/v2/game-public/{path}"), post(handle))
            .with_state(Ingest {
                db: db.clone(),
                config: config.clone(),
                family,
            });
        app = app.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled,
                group: "game-public".into(),
                max_body_bytes: family.map_or(WATERMARK_BYTES, Family::max_bytes),
                max_json_depth: DEPTH,
                json: JsonRule::Object,
            },
        ));
    }
    app.layer(axum::middleware::from_fn(headers))
}
async fn headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    // Every oversize request is 400 (contract *Exact encoding*).
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert("pragma", axum::http::HeaderValue::from_static("no-cache"));
    response
}
/// Canonical bytes, exact member order at every level and no string escapes.
fn decode<T: serde::de::DeserializeOwned + serde::Serialize>(
    value: &Value,
    body: &[u8],
    max: usize,
) -> Result<T, ()> {
    if body.contains(&b'\\') || canonical::encode(value).ok().as_deref() != Some(body) {
        return Err(());
    }
    let decoded: T = serde_json::from_slice(body).map_err(|_| ())?;
    let bytes = serde_json::to_vec(&decoded).map_err(|_| ())?;
    let ordered = canonical::decode(&bytes, max, DEPTH).map_err(|_| ())?;
    if canonical::encode(&ordered).ok().as_deref() != Some(body) {
        return Err(());
    }
    Ok(decoded)
}
async fn handle(
    State(state): State<Ingest>,
    Extension(value): Extension<Value>,
    Extension(publisher): Extension<PublisherKey>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if headers
        .get("content-length")
        .is_some_and(|h| h.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len()))
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let now = now();
    let result = match state.family {
        Some(family) => {
            let Ok(snapshot) = decode::<wire::Snapshot>(&value, &body, family.max_bytes()) else {
                return guard::failure(StatusCode::BAD_REQUEST);
            };
            if snapshot.validate(family, now).is_err() {
                return guard::failure(StatusCode::BAD_REQUEST);
            }
            if snapshot.source_authority != state.config.source_authority {
                return guard::failure(StatusCode::UNAUTHORIZED);
            }
            if let Err(denial) = crate::ingest::budget::check(
                &state.db,
                &publisher,
                &format!("v2/{}", family.path()),
                state.config.requests_per_minute,
            )
            .await
            {
                return denial.response();
            }
            let body = match family {
                Family::Skills => serde_json::to_value(&snapshot.skills),
                Family::Achievements => serde_json::to_value(&snapshot.achievements),
                Family::Equipment => serde_json::to_value(&snapshot.equipment),
                Family::Outfit => serde_json::to_value(&snapshot.outfit),
            };
            let (Ok(body), Ok(character), Ok(observed)) = (
                body,
                uuid::Uuid::parse_str(&snapshot.character_id),
                i64::try_from(wire::time(&snapshot.source_observed_at, 0, now).unwrap_or(0)),
            ) else {
                return guard::failure(StatusCode::BAD_REQUEST);
            };
            storage::snapshot(
                &state.db,
                &storage::Publication {
                    family,
                    character,
                    authority: &snapshot.source_authority,
                    epoch: &snapshot.projection_epoch,
                    revision: &snapshot.projection_revision,
                    observed,
                    body,
                },
            )
            .await
        }
        None => {
            let Ok(mark) = decode::<wire::Watermark>(&value, &body, WATERMARK_BYTES) else {
                return guard::failure(StatusCode::BAD_REQUEST);
            };
            let Ok(family) = mark.validate(now) else {
                return guard::failure(StatusCode::BAD_REQUEST);
            };
            if !state.config.permits(family) {
                return guard::failure(StatusCode::SERVICE_UNAVAILABLE);
            }
            if mark.source_authority != state.config.source_authority {
                return guard::failure(StatusCode::UNAUTHORIZED);
            }
            if let Err(denial) = crate::ingest::budget::check(
                &state.db,
                &publisher,
                "v2/watermark",
                state.config.requests_per_minute,
            )
            .await
            {
                return denial.response();
            }
            let Ok(through) =
                wire::decimal(&mark.complete_through, 0, i64::MAX as u64).map(|n| n as i64)
            else {
                return guard::failure(StatusCode::BAD_REQUEST);
            };
            storage::watermark(
                &state.db,
                family,
                &mark.source_authority,
                &mark.projection_epoch,
                through,
            )
            .await
        }
    };
    match result {
        Ok(result) => {
            axum::Json(serde_json::json!({"contract_version":2,"result":result})).into_response()
        }
        Err(storage::Error::Conflict) => guard::failure(StatusCode::CONFLICT),
        Err(storage::Error::Database) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Production ingest flags from the environment: the T24 prerequisite in testing or
/// preproduction, plus one flag per family.
pub fn ingest_config(get: impl Fn(&str) -> Option<String>) -> IngestConfig {
    let on = |key: &str| get(key).as_deref() == Some("true");
    let base = on("PORTAL_PUBLIC_PROJECTIONS_ENABLED") && oteryn_core::restricted_environment(&get);
    IngestConfig {
        skills: base && on("PORTAL_G4_SKILLS_INGEST_ENABLED"),
        achievements: base && on("PORTAL_G4_ACHIEVEMENTS_INGEST_ENABLED"),
        equipment: base && on("PORTAL_G4_EQUIPMENT_INGEST_ENABLED"),
        outfit: base && on("PORTAL_G4_OUTFIT_INGEST_ENABLED"),
        source_authority: get("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default(),
        requests_per_minute: 6000,
    }
}

#[cfg(test)]
mod tests;
