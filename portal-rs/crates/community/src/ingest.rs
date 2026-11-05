//! Bounded, authenticated public projection ingestion. Reads return game facts only;
//! callers must apply Platform privacy policy before public presentation.
use axum::{
    Extension, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use sqlx::{PgPool, Row};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
#[path = "ingest/ingest_schema.rs"]
mod ingest_schema;
use ingest_schema::publication;

#[derive(Debug, Clone, Copy)]
pub struct Family {
    pub name: &'static str,
    pub path: &'static str,
    pub operation: &'static str,
    pub max_bytes: usize,
}
pub const FAMILIES: &[Family] = &[
    Family {
        name: "highscores",
        path: "highscores",
        operation: "PublishWorldHighscoresV1",
        max_bytes: 262144,
    },
    Family {
        name: "character_profile",
        path: "character-profile",
        operation: "PublishCharacterProfileV1",
        max_bytes: 2048,
    },
    Family {
        name: "character_deaths",
        path: "character-deaths",
        operation: "PublishCharacterDeathsV1",
        max_bytes: 32768,
    },
    Family {
        name: "guild",
        path: "guild",
        operation: "PublishGuildV1",
        max_bytes: 262144,
    },
    Family {
        name: "house",
        path: "house",
        operation: "PublishHouseV1",
        max_bytes: 2048,
    },
    Family {
        name: "world_online",
        path: "world-online",
        operation: "PublishWorldOnlineV1",
        max_bytes: 524288,
    },
];
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub enabled: bool,
    pub source_authority: String,
}
#[derive(Debug, Clone)]
struct Ingest {
    db: PgPool,
    config: Config,
    family: Option<Family>,
}
/// The TLS adapter supplies VerifiedClient; headers cannot establish identity.
pub fn router(db: PgPool, identities: Arc<IdentityMap>, config: Config) -> Router {
    let mut router = Router::new();
    for family in FAMILIES
        .iter()
        .copied()
        .map(Some)
        .chain(std::iter::once(None))
    {
        let path = family.map_or("watermark", |f| f.path);
        let policy = RoutePolicy {
            enabled: config.enabled && ingest_schema::authority(&config.source_authority),
            group: "game-public".into(),
            max_body_bytes: family.map_or(512, |f| f.max_bytes),
            max_json_depth: 6,
            json: JsonRule::Object,
        };
        let route = Router::new()
            .route(&format!("/internal/v1/game-public/{path}"), post(handle))
            .with_state(Ingest {
                db: db.clone(),
                config: config.clone(),
                family,
            });
        router = router.merge(guard::protect(route, identities.clone(), policy));
    }
    router.layer(axum::middleware::from_fn(cache_headers))
}
async fn cache_headers(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    // Laravel permits media type parameters and case-insensitive identity encoding.
    for name in ["content-type", "content-encoding"] {
        if request.headers().get_all(name).iter().count() == 1 {
            let normalized = request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    if name == "content-type" {
                        v.split(';')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_ascii_lowercase()
                    } else {
                        v.trim().to_ascii_lowercase()
                    }
                });
            if let Some(value) = normalized.and_then(|v| axum::http::HeaderValue::from_str(&v).ok())
            {
                request
                    .headers_mut()
                    .insert(axum::http::HeaderName::from_static(name), value);
            }
        }
    }
    let mut response = next.run(request).await;
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
async fn handle(
    State(state): State<Ingest>,
    Extension(value): Extension<Value>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if headers.get("content-length").is_some_and(|length| {
        length.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len())
    }) {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    if canonical::encode(&value).ok().as_deref() != Some(body.as_ref()) {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let publication = match publication(state.family, &value, now) {
        Ok(p) => p,
        Err(()) => return guard::failure(StatusCode::BAD_REQUEST),
    };
    if publication.authority != state.config.source_authority {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    match apply(&state.db, &publication).await {
        Ok(result) => {
            axum::Json(serde_json::json!({"contract_version":1,"result":result})).into_response()
        }
        Err(ApplyError::Conflict) => guard::failure(StatusCode::CONFLICT),
        Err(ApplyError::Database) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
#[derive(Debug)]
pub(super) struct Publication {
    family: String,
    subject: String,
    authority: String,
    epoch: String,
    revision: String,
    observed: i64,
    watermark: bool,
    payload: serde_json::Value,
}
#[derive(Debug)]
enum ApplyError {
    Conflict,
    Database,
}
impl From<sqlx::Error> for ApplyError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}
async fn apply(db: &PgPool, p: &Publication) -> Result<&'static str, ApplyError> {
    let mut tx = db.begin().await?;
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if highest
        .as_ref()
        .is_some_and(|h| compare(&p.epoch, h).is_lt())
    {
        return Ok("superseded");
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(&p.epoch)
        .execute(&mut *tx)
        .await?;
    let current = sqlx::query("SELECT version::text, epoch::text, watermark, payload FROM public_snapshots WHERE family=$1 AND subject=$2 AND is_watermark=$3").bind(&p.family).bind(&p.subject).bind(p.watermark).fetch_optional(&mut *tx).await?;
    if let Some(row) = current {
        let epoch: String = row.try_get("epoch")?;
        let version: String = row.try_get("version")?;
        let payload: serde_json::Value = row.try_get("payload")?;
        if p.watermark {
            let watermark: i64 = row.try_get("watermark")?;
            if epoch == p.epoch && p.observed <= watermark {
                return Ok("superseded");
            }
        } else {
            let order = compare(&p.epoch, &epoch).then_with(|| compare(&p.revision, &version));
            if order.is_lt() {
                return Ok("superseded");
            }
            if order.is_eq() {
                return if payload == p.payload {
                    Ok("accepted")
                } else {
                    Err(ApplyError::Conflict)
                };
            }
        }
    }
    sqlx::query("INSERT INTO public_snapshots (family,subject,is_watermark,epoch,version,watermark,payload) VALUES ($1,$2,$3,$4::numeric,$5::numeric,$6,$7) ON CONFLICT (family,subject,is_watermark) DO UPDATE SET previous_snapshot=jsonb_build_object('epoch',public_snapshots.epoch::text,'version',public_snapshots.version::text,'watermark',public_snapshots.watermark,'payload',public_snapshots.payload),epoch=EXCLUDED.epoch,version=EXCLUDED.version,watermark=EXCLUDED.watermark,payload=EXCLUDED.payload")
 .bind(&p.family).bind(&p.subject).bind(p.watermark).bind(&p.epoch).bind(&p.revision).bind(p.observed).bind(&p.payload).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok("accepted")
}
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
/// Latest eligible subject snapshot. Old epochs never become current after an epoch raise.
/// This API does not grant permission to expose privacy-controlled fields.
pub async fn latest(
    db: &PgPool,
    family: &str,
    subject: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    sqlx::query_scalar("SELECT payload FROM public_snapshots WHERE family=$1 AND subject=$2 AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1)").bind(family).bind(subject).fetch_optional(db).await
}
#[cfg(test)]
#[path = "ingest/ingest_tests.rs"]
mod ingest_tests;

/// Environment activation is limited to testing and preproduction, matching Laravel.
/// Missing or invalid configuration leaves every endpoint unavailable.
pub fn configured_router(db: PgPool) -> Router {
    let enabled = std::env::var("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Ok("true")
        && matches!(
            std::env::var("APP_ENV").as_deref(),
            Ok("testing" | "preproduction")
        );
    let source_authority = std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default();
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
    router(
        db,
        Arc::new(identities),
        Config {
            enabled,
            source_authority,
        },
    )
}
