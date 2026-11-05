use super::{Error, Outcome, Projection, Snapshot, Watermark, wire};
use axum::{
    Router,
    body::Bytes,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_core::Environment;
use oteryn_internal_api::guard::{self, IdentityMap, JsonRule, RoutePolicy, VerifiedClient};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const GROUP: &str = "native-account-characters";
pub const SNAPSHOT_PATH: &str = "/internal/v1/game-auth/native-account-characters";
pub const WATERMARK_PATH: &str = "/internal/v1/game-auth/native-account-characters/watermark";

/// Explicit configuration; Default keeps projection reads and ingestion off.
#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub environment: Environment,
    pub source_authority: String,
    pub freshness_seconds: u8,
    pub clock_uncertainty_seconds: u8,
    pub requests_per_minute: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            environment: Environment::Local,
            source_authority: String::new(),
            freshness_seconds: 30,
            clock_uncertainty_seconds: 1,
            requests_per_minute: 600,
        }
    }
}
impl Settings {
    pub(super) fn available(&self) -> bool {
        self.enabled
            && matches!(
                self.environment,
                Environment::Testing | Environment::Preprod
            )
            && wire::authority_valid(&self.source_authority)
            && (1..=120).contains(&self.freshness_seconds)
            && self.clock_uncertainty_seconds <= 5
            && self.clock_uncertainty_seconds < self.freshness_seconds
            && (1..=600).contains(&self.requests_per_minute)
    }
}
/// Mount on a dedicated internal listener. Its trusted TLS adapter must insert
/// VerifiedClient; headers cannot establish identity. OAuth callers use the
/// internal read API after binding the account to their authenticated principal.
pub fn router(projection: Projection, identities: Arc<IdentityMap>) -> Router {
    let settings = projection.settings.clone();
    let mount =
        |path, max| {
            let routes = Router::new().route(path, post(publish)).route_layer(
                middleware::from_fn_with_state(projection.clone(), bounds_and_rate),
            );
            guard::protect(
                routes,
                identities.clone(),
                RoutePolicy {
                    enabled: settings.available(),
                    group: GROUP.into(),
                    max_body_bytes: max,
                    max_json_depth: 4,
                    json: JsonRule::Object,
                },
            )
        };
    mount(SNAPSHOT_PATH, wire::SNAPSHOT_MAX_BYTES)
        .merge(mount(WATERMARK_PATH, wire::WATERMARK_MAX_BYTES))
        .with_state(projection)
        .layer(middleware::from_fn(normalize_transport))
        .layer(middleware::from_fn(no_cache))
}
async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(header::EXPIRES, HeaderValue::from_static("0"));
    response
}
// Normalize Laravel's accepted media type and encoding before T15 checks them.
async fn normalize_transport(mut request: Request, next: Next) -> Response {
    if request
        .headers()
        .get_all(header::CONTENT_TYPE)
        .iter()
        .count()
        == 1
        && request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
            })
    {
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    if request
        .headers()
        .get_all(header::CONTENT_ENCODING)
        .iter()
        .count()
        == 1
        && request
            .headers()
            .get(header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("identity"))
    {
        request.headers_mut().insert(
            header::CONTENT_ENCODING,
            HeaderValue::from_static("identity"),
        );
    }
    next.run(request).await
}
fn now() -> Result<i64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .ok_or(Error::Unavailable)
}
async fn bounds_and_rate(
    State(projection): State<Projection>,
    request: Request,
    next: Next,
) -> Response {
    let (parts, body) = request.into_parts();
    let result = async {
        if let Some(length) = parts.headers.get(header::CONTENT_LENGTH) {
            let length = length.to_str().map_err(|_| Error::BadInput)?;
            if length.len() > 5 { return Ok(Some(guard::failure(StatusCode::PAYLOAD_TOO_LARGE))); }
        }
        let client = parts.extensions.get::<VerifiedClient>().ok_or(Error::Unauthorized)?;
        // This opaque bucket key grants no authority: T15 already authenticated
        // the complete verified client. Never log the representation or digest.
        let key = Sha256::digest(format!("{client:?}").as_bytes()).to_vec();
        let now = now()?;
        let row = sqlx::query("INSERT INTO account_character_publisher_limits (peer_key, reset_at, requests) VALUES ($1,$2 + 60,1) ON CONFLICT (peer_key) DO UPDATE SET reset_at = CASE WHEN account_character_publisher_limits.reset_at <= $2 THEN $2 + 60 ELSE account_character_publisher_limits.reset_at END, requests = CASE WHEN account_character_publisher_limits.reset_at <= $2 THEN 1 ELSE LEAST(account_character_publisher_limits.requests + 1, 601) END RETURNING requests, reset_at")
            .bind(key).bind(now).fetch_one(&projection.pool).await?;
        if row.try_get::<i32, _>("requests")? > i32::from(projection.settings.requests_per_minute) {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            let seconds = row.try_get::<i64, _>("reset_at")?.saturating_sub(now).max(1);
            if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) { response.headers_mut().insert(header::RETRY_AFTER, value); }
            return Ok(Some(response));
        }
        Ok::<_, Error>(None)
    }.await;
    match result {
        Ok(None) => next.run(Request::from_parts(parts, body)).await,
        Ok(Some(response)) => response,
        Err(error) => refusal(error),
    }
}
async fn publish(State(projection): State<Projection>, request: Request) -> Response {
    let watermark = request.uri().path() == WATERMARK_PATH;
    let length = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    let result = async {
        let bytes = axum::body::to_bytes(
            request.into_body(),
            if watermark {
                wire::WATERMARK_MAX_BYTES
            } else {
                wire::SNAPSHOT_MAX_BYTES
            },
        )
        .await
        .map_err(|_| Error::BadInput)?;
        if length.is_some_and(|length| length != bytes.len()) {
            return Err(Error::BadInput);
        }
        let now = now()?;
        if watermark {
            projection.watermark(Watermark::decode(&bytes)?, now).await
        } else {
            projection.snapshot(Snapshot::decode(&bytes)?, now).await
        }
    }
    .await;
    match result {
        Ok(outcome) => {
            let bytes = match outcome {
                Outcome::Accepted => {
                    Bytes::from_static(b"{\"contract_version\":1,\"result\":\"accepted\"}")
                }
                Outcome::Superseded => {
                    Bytes::from_static(b"{\"contract_version\":1,\"result\":\"superseded\"}")
                }
            };
            let mut response = Response::new(bytes.into());
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(error) => refusal(error),
    }
}
fn refusal(error: Error) -> Response {
    guard::failure(match error {
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    })
}
