use super::*;
use axum::{
    Router,
    body::to_bytes,
    extract::{ConnectInfo, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::canonical::{self, Value};
use std::net::SocketAddr;

const BASE: &str = "/api/v1/game-auth/device-sessions";
fn failure(error: Failure) -> Response {
    let (status, message) = match error {
        Failure::Denied => (StatusCode::UNAUTHORIZED, "device_authorization_unavailable"),
        Failure::Unavailable | Failure::Retryable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "device_authorization_unavailable",
        ),
        Failure::Invalid => (StatusCode::BAD_REQUEST, "invalid_request"),
        Failure::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "invalid_request"),
        Failure::Throttled => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
    };
    let mut response = (status, axum::Json(serde_json::json!({"error":message}))).into_response();
    if error == Failure::Throttled {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
    }
    response
}
async fn no_cache(request: Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}
/// Transport evidence supplied only by the trusted TLS listener or a separately
/// qualified TLS termination adapter. Request targets and forwarding headers do
/// not establish transport security.
#[derive(Debug, Clone, Copy)]
pub struct VerifiedHttps(());
impl VerifiedHttps {
    /// Call only after verifying the actual transport, never from request headers.
    pub fn from_verified_tls() -> Self {
        Self(())
    }
}
/// No insecure-loopback exception is enabled by this adapter.
pub fn router<S: Clone + Send + Sync + 'static>(sessions: Sessions) -> Router<S> {
    Router::new()
        .route(BASE, post(handle))
        .route(&format!("{BASE}/native-characters"), post(handle))
        .route(&format!("{BASE}/tickets"), post(handle))
        .route(&format!("{BASE}/revoke"), post(handle))
        .route(
            &format!("{BASE}/{{*rest}}"),
            post(|| async { failure(Failure::Unavailable) }),
        )
        .with_state(sessions)
        .layer(axum::middleware::from_fn(no_cache))
}
/// Unwired T17 authority is intentionally unavailable, even if the flag is set.
pub fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool) -> Router<S> {
    let settings = Settings {
        enabled: std::env::var("GAME_AUTH_DEVICE_SESSIONS_ENABLED").as_deref() == Ok("true"),
        environment: std::env::var("APP_ENV").unwrap_or_default(),
        ..Settings::default()
    };
    router(Sessions::new(
        pool,
        settings,
        Arc::new(UnavailableAuthority),
    ))
}
async fn handle(State(sessions): State<Sessions>, request: Request) -> Response {
    match dispatch(sessions, request).await {
        Ok(value) => axum::Json(value).into_response(),
        Err(error) => failure(error),
    }
}
async fn dispatch(sessions: Sessions, request: Request) -> Result<serde_json::Value> {
    sessions.settings.check()?;
    if request.extensions().get::<VerifiedHttps>().is_none() {
        return Err(Failure::Unavailable);
    }
    // Use the socket source only. Missing peer information denies rather than
    // accepting caller-supplied forwarding headers or a credential-keyed limit.
    let source = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or(Failure::Unavailable)?
        .0
        .ip()
        .to_string();
    throttle(&sessions, &source).await?;
    let enroll = request.uri().path() == BASE;
    let path = request.uri().path().to_string();
    let (parts, body) = request.into_parts();
    let headers = &parts.headers;
    let length = if let Some(value) = headers.get(header::CONTENT_LENGTH) {
        let raw = value.to_str().map_err(|_| Failure::TooLarge)?;
        if raw.is_empty()
            || !raw.bytes().all(|b| b.is_ascii_digit())
            || (raw.len() > 1 && raw.starts_with('0'))
            || raw.len() > 7
        {
            return Err(Failure::TooLarge);
        }
        let n = raw.parse::<usize>().map_err(|_| Failure::TooLarge)?;
        if n > 1024 {
            return Err(Failure::TooLarge);
        }
        Some(n)
    } else {
        None
    };
    if headers.get_all(header::CONTENT_LENGTH).iter().count() > 1
        || headers.get_all(header::CONTENT_TYPE).iter().count() != 1
        || !headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
            })
        || headers.contains_key(header::TRANSFER_ENCODING)
        || headers.get_all(header::CONTENT_ENCODING).iter().count() > 1
        || headers
            .get(header::CONTENT_ENCODING)
            .is_some_and(|v| v != "identity")
    {
        return Err(Failure::Invalid);
    }
    let body = to_bytes(body, 1024).await.map_err(|_| Failure::TooLarge)?;
    if length.is_some_and(|n| n != body.len()) {
        return Err(Failure::Invalid);
    }
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(Failure::Denied);
    }
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= 8192)
        .ok_or(Failure::Denied)?;
    let secret = if enroll {
        // OAuth tokens in the new portal are opaque. Verification belongs solely
        // to the owning OAuth implementation, not a second JWT authenticator.
        authorization
            .strip_prefix("Bearer ")
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_graphic()))
            .ok_or(Failure::Denied)?
    } else {
        authorization
            .strip_prefix("OterynDevice ")
            .filter(|s| valid_secret(s))
            .ok_or(Failure::Denied)?
    };
    let client = decode(&body, enroll, parts.uri.query().is_some())?;
    if enroll {
        return sessions.enroll(secret, true).await.map(|c| c.payload());
    }
    let client = client.ok_or(Failure::Invalid)?;
    if path == format!("{BASE}/revoke") {
        sessions.revoke(secret, client).await?;
        return Ok(serde_json::json!({"protocol_version":1,"revoked":true}));
    }
    let purpose = if path == format!("{BASE}/tickets") {
        Purpose::Ticket
    } else {
        Purpose::Characters
    };
    sessions.rotate(secret, client, purpose).await
}
fn decode(body: &[u8], enroll: bool, query: bool) -> Result<Option<Uuid>> {
    if query {
        return Err(Failure::Invalid);
    }
    let value = canonical::decode(body, 1024, 2).map_err(|_| Failure::Invalid)?;
    let Value::Object(fields) = &value else {
        return Err(Failure::Invalid);
    };
    if fields.len() != 2 || value.get("protocol_version") != Some(&Value::Unsigned(1)) {
        return Err(Failure::Invalid);
    }
    if enroll {
        if value.get("remember_device") != Some(&Value::Bool(true)) {
            return Err(Failure::Invalid);
        }
        Ok(None)
    } else {
        let Some(Value::String(raw)) = value.get("client_id") else {
            return Err(Failure::Invalid);
        };
        let id = Uuid::parse_str(raw).map_err(|_| Failure::Invalid)?;
        if id.to_string() != *raw
            || !(1..=8).contains(&id.get_version_num())
            || id.get_variant() != uuid::Variant::RFC4122
        {
            return Err(Failure::Invalid);
        }
        Ok(Some(id))
    }
}
async fn throttle(sessions: &Sessions, source: &str) -> Result<()> {
    let attempts: i32 = sqlx::query_scalar("INSERT INTO device_session_source_limits(source_hash,window_start,attempts) VALUES($1,floor(extract(epoch FROM clock_timestamp()))::bigint,1) ON CONFLICT(source_hash) DO UPDATE SET window_start=CASE WHEN device_session_source_limits.window_start <= EXCLUDED.window_start-60 THEN EXCLUDED.window_start ELSE device_session_source_limits.window_start END, attempts=CASE WHEN device_session_source_limits.window_start <= EXCLUDED.window_start-60 THEN 1 ELSE least(device_session_source_limits.attempts+1,61) END RETURNING attempts")
        .bind(hash(source)).fetch_one(&sessions.pool).await.map_err(unavailable)?;
    if attempts > sessions.settings.requests_per_minute {
        Err(Failure::Throttled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_requests() {
        assert!(
            decode(
                br#"{"protocol_version":1,"remember_device":true}"#,
                true,
                false
            )
            .is_ok()
        );
        for body in [
            r#"{"protocol_version":1,"remember_device":false}"#,
            r#"{"protocol_version":"1","remember_device":true}"#,
            r#"{"protocol_version":1,"remember_device":true,"account_id":"x"}"#,
            r#"{"protocol_version":1,"protocol_vers\u0069on":1}"#,
            r#"[]"#,
        ] {
            assert_eq!(decode(body.as_bytes(), true, false), Err(Failure::Invalid));
        }
        assert_eq!(
            decode(
                br#"{"protocol_version":1,"remember_device":true}"#,
                true,
                true
            ),
            Err(Failure::Invalid)
        );
    }
}
