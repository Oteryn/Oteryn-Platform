//! Account security revocation stream V1: Game pulls ordered revocation records
//! over the internal mTLS listener; no game database or outbound I/O. Wire, guard,
//! identity overlap, cursor and byte budget mirror the N009 sanction export.
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy},
};
use sqlx::{PgConnection, PgPool, Row};
use std::{
    fmt,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const PATH: &str = "/internal/v1/identity/security-revocations/stream/read";
pub const GROUP: &str = "identity-security-revocation-export";
const IDENTITY_KEY: &str = "PORTAL_SECURITY_REVOCATION_EXPORT_IDENTITY";
const MAX_RESPONSE: usize = 262144;

#[derive(Debug, Default, Clone, Copy)]
pub struct Settings {
    pub enabled: bool,
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            enabled: get("PORTAL_SECURITY_REVOCATION_EXPORT_ENABLED").as_deref() == Some("true"),
        }
    }
}

/// Include all internal identity variables, including maps, in overlap checks.
/// This function is also usable by a composition root supplying explicit bindings.
pub fn identities(vars: Vec<(String, String)>) -> Result<IdentityMap, guard::ConfigError> {
    let mut bindings = Vec::new();
    let mut own = false;
    for (key, raw) in vars {
        if key.ends_with("_IDENTITIES") {
            let Value::Object(members) =
                canonical::decode(raw.as_bytes(), 131072, 4).map_err(|_| guard::ConfigError)?
            else {
                return Err(guard::ConfigError);
            };
            for (san, _) in members {
                bindings.push(Binding {
                    identity: Identity::san(&san)?,
                    group: key.clone(),
                });
            }
        } else if key.ends_with("_IDENTITY") || key.ends_with("_SAN") {
            let dedicated = key == IDENTITY_KEY;
            own |= dedicated;
            // The trusted adapter extracts DNS SANs; require a DNS name here.
            if dedicated && !dns_name(&raw) {
                return Err(guard::ConfigError);
            }
            bindings.push(Binding {
                identity: Identity::san(&raw)?,
                group: if dedicated { GROUP.into() } else { key },
            });
        }
    }
    if !own {
        return Err(guard::ConfigError);
    }
    IdentityMap::new(bindings)
}
fn dns_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

pub fn configured_router(pool: PgPool) -> Router {
    let identities = identities(std::env::vars().collect());
    let mut settings = Settings::from_lookup(|key| std::env::var(key).ok());
    settings.enabled &= identities.is_ok();
    router(pool, settings, Arc::new(identities.unwrap_or_default()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub after: Option<i64>,
    pub limit: i64,
}
#[derive(Debug)]
pub enum Error {
    Invalid,
    Storage(sqlx::Error),
    Encoding,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Invalid => "invalid security revocation export input",
            Self::Storage(_) => "security revocation export storage unavailable",
            Self::Encoding => "security revocation export encoding unavailable",
        })
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error)
    }
}
impl Page {
    pub fn decode(value: &Value) -> Result<Self, Error> {
        let Value::Object(members) = value else {
            return Err(Error::Invalid);
        };
        if members.len() != 3
            || value.get("schema") != Some(&text("oteryn.security_revocation_stream_request.v1"))
        {
            return Err(Error::Invalid);
        }
        let after = match value.get("after") {
            Some(Value::Null) => None,
            Some(Value::String(s)) => {
                let raw = s.strip_prefix("v1:").ok_or(Error::Invalid)?;
                let n = raw.parse::<i64>().map_err(|_| Error::Invalid)?;
                if n <= 0 || raw != n.to_string() {
                    return Err(Error::Invalid);
                }
                Some(n)
            }
            _ => return Err(Error::Invalid),
        };
        let limit = match value.get("limit") {
            Some(Value::Unsigned(n)) => i64::try_from(*n).map_err(|_| Error::Invalid)?,
            Some(Value::Signed(n)) => *n,
            _ => return Err(Error::Invalid),
        };
        if !(1..=500).contains(&limit) {
            return Err(Error::Invalid);
        }
        Ok(Self { after, limit })
    }
}

/// Staff kick seam. The caller authorizes staff (dedicated permission, confirmed
/// MFA) and writes its audit row in this same transaction. Only the native
/// security generation advances; the accounts trigger publishes cause 4 and the
/// existing native-device trigger revokes device sessions. Returns the new
/// generation, or `None` when the account does not exist.
pub async fn staff_kick(
    connection: &mut PgConnection,
    account: Uuid,
) -> Result<Option<i64>, Error> {
    if account.is_nil() {
        return Err(Error::Invalid);
    }
    sqlx::query("SELECT set_config('oteryn.security_revocation_cause','staff_kick',true)")
        .execute(&mut *connection)
        .await?;
    let generation: Option<i64> = sqlx::query_scalar("UPDATE accounts SET native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1 RETURNING native_security_generation")
        .bind(account)
        .fetch_optional(&mut *connection)
        .await?;
    // Later generation advances in the same transaction keep their own cause.
    sqlx::query("SELECT set_config('oteryn.security_revocation_cause','',true)")
        .execute(&mut *connection)
        .await?;
    Ok(generation)
}

fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn encode_page(items: Vec<(i64, Value)>) -> Result<Vec<u8>, Error> {
    // Reserve more than the maximum envelope and decimal cursor require. A limit
    // is an upper bound; the byte budget can shorten a page.
    let mut used = 128_usize;
    let mut values = Vec::new();
    let mut after = None;
    for (position, item) in items {
        let size = canonical::encode(&item).map_err(|_| Error::Encoding)?.len();
        if used + size + 1 > MAX_RESPONSE {
            break;
        }
        used += size + 1;
        values.push(item);
        after = Some(position);
    }
    let bytes = canonical::encode(&object(vec![
        ("schema", text("oteryn.security_revocation_stream_page.v1")),
        ("items", Value::Array(values)),
        (
            "next_cursor",
            after.map_or(Value::Null, |n| text(format!("v1:{n}"))),
        ),
    ]))
    .map_err(|_| Error::Encoding)?;
    if bytes.len() > MAX_RESPONSE {
        return Err(Error::Encoding);
    }
    Ok(bytes)
}
/// Immutable journal pages ordered by committed publication position.
pub async fn read_page(pool: &PgPool, page: Page) -> Result<Vec<u8>, Error> {
    if !(1..=500).contains(&page.limit) || page.after.is_some_and(|n| n <= 0) {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT identity_security_revocation_reconcile()")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query("SELECT position,revocation_id,account_id,native_security_generation,cause,effective_at_ms FROM identity_security_revocations WHERE position>$1 ORDER BY position LIMIT $2")
        .bind(page.after.unwrap_or(0)).bind(page.limit).fetch_all(&mut *tx).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let position = row.try_get("position")?;
        items.push((
            position,
            object(vec![
                (
                    "revocation_id",
                    text(row.try_get::<Uuid, _>("revocation_id")?),
                ),
                ("account_id", text(row.try_get::<Uuid, _>("account_id")?)),
                (
                    "native_security_generation",
                    Value::Signed(row.try_get("native_security_generation")?),
                ),
                (
                    "cause",
                    Value::Signed(i64::from(row.try_get::<i16, _>("cause")?)),
                ),
                (
                    "effective_at_ms",
                    Value::Signed(row.try_get("effective_at_ms")?),
                ),
            ]),
        ));
    }
    let encoded = encode_page(items)?;
    tx.commit().await?;
    Ok(encoded)
}

#[derive(Debug)]
struct ExportState {
    pool: PgPool,
    rate: Mutex<(Instant, u32)>,
}
/// Mount only behind the verified TLS 1.3 / HTTP/1.1 listener. The complete
/// identity map must contain the dedicated binding and all other peer bindings.
pub fn router(pool: PgPool, settings: Settings, identities: Arc<IdentityMap>) -> Router {
    let state = Arc::new(ExportState {
        pool,
        rate: Mutex::new((Instant::now(), 0)),
    });
    guard::protect(
        Router::new().route(PATH, post(read)).with_state(state),
        identities,
        RoutePolicy {
            enabled: settings.enabled,
            group: GROUP.into(),
            max_body_bytes: 256,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    )
    .layer(middleware::from_fn(response_headers))
}
async fn response_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        *response.status_mut() = StatusCode::BAD_REQUEST;
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    response
}
async fn read(State(state): State<Arc<ExportState>>, request: Request) -> Response {
    let allowed = match state.rate.lock() {
        Ok(mut rate) => {
            if rate.0.elapsed() >= Duration::from_secs(60) {
                *rate = (Instant::now(), 0);
            }
            if rate.1 >= 1200 {
                false
            } else {
                rate.1 += 1;
                true
            }
        }
        Err(_) => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    if !allowed {
        let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
        return response;
    }
    let Some(value) = request.extensions().get::<Value>() else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    let Ok(page) = Page::decode(value) else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    if request.uri().query().is_some() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    match tokio::time::timeout(Duration::from_secs(3), read_page(&state.pool, page)).await {
        Ok(Ok(bytes)) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );
            response
        }
        _ => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}

#[cfg(test)]
#[path = "security_revocations_tests.rs"]
mod security_revocations_tests;
