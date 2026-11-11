//! Game-initiated, player-visible sanction history; no game database or outbound I/O.
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
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const PATH: &str = "/internal/v1/support/sanctions/stream/read";
pub const GROUP: &str = "support-sanction-export";
const MAX_RESPONSE: usize = 262144;

#[derive(Debug, Default, Clone, Copy)]
pub struct Settings {
    pub enabled: bool,
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            enabled: get("PORTAL_SANCTION_EXPORT_ENABLED").as_deref() == Some("true"),
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
            let dedicated = key == "PORTAL_SANCTION_EXPORT_IDENTITY";
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
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid sanction export input")]
    Invalid,
    #[error("sanction export storage unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("sanction export encoding unavailable")]
    Encoding,
}
impl Page {
    pub fn decode(value: &Value) -> Result<Self, Error> {
        let Value::Object(members) = value else {
            return Err(Error::Invalid);
        };
        if members.len() != 3
            || value.get("schema") != Some(&text("oteryn.sanction_stream_request.v1"))
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

/// Public reason metadata for typed T34 decisions. The caller must authorize staff
/// and include the decision, reason, audit and appeal effects in one transaction.
/// No generated reason or fallback text is exported when this metadata is absent.
pub async fn set_public_reason(
    tx: &mut Transaction<'_, Postgres>,
    sanction: Uuid,
    code: i32,
    reason: &str,
) -> Result<(), Error> {
    if super::valid_id(sanction).is_err() || !(1..=65535).contains(&code) || !valid_reason(reason) {
        return Err(Error::Invalid);
    }
    sqlx::query("INSERT INTO support_sanction_export_reasons(sanction_id,reason_code,reason_text) VALUES ($1,$2,$3) ON CONFLICT(sanction_id) DO UPDATE SET reason_code=$2,reason_text=$3,updated_at=clock_timestamp()")
        .bind(sanction).bind(code).bind(reason).execute(&mut **tx).await?;
    Ok(())
}
fn valid_reason(reason: &str) -> bool {
    (1..=120).contains(&reason.len()) && !reason.chars().any(char::is_control)
}
fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn encode_page(items: Vec<(i64, Value)>) -> Result<Vec<u8>, Error> {
    // Reserve more than the maximum envelope and decimal cursor require. A limit
    // is an upper bound; heavily escaped text can shorten a page by byte budget.
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
        ("schema", text("oteryn.sanction_stream_page.v1")),
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
    sqlx::query("SELECT support_sanction_export_expire_namelocks()")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query("SELECT position,sanction_stream_id,decision_revision,account_id,character_id,kind,reason_code,reason_text,starts_at_ms,ends_at_ms,revoked FROM support_sanction_export_revisions WHERE position>$1 ORDER BY position LIMIT $2")
        .bind(page.after.unwrap_or(0)).bind(page.limit).fetch_all(&mut *tx).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let position = row.try_get("position")?;
        let reason: String = row.try_get("reason_text")?;
        if !valid_reason(&reason) {
            return Err(Error::Encoding);
        }
        items.push((
            position,
            object(vec![
                (
                    "sanction_stream_id",
                    text(row.try_get::<Uuid, _>("sanction_stream_id")?),
                ),
                (
                    "decision_revision",
                    Value::Signed(row.try_get("decision_revision")?),
                ),
                ("account_id", text(row.try_get::<Uuid, _>("account_id")?)),
                (
                    "character_id",
                    row.try_get::<Option<Uuid>, _>("character_id")?
                        .map_or(Value::Null, text),
                ),
                (
                    "kind",
                    Value::Signed(i64::from(row.try_get::<i16, _>("kind")?)),
                ),
                (
                    "reason_code",
                    Value::Signed(i64::from(row.try_get::<i32, _>("reason_code")?)),
                ),
                ("reason_text", text(reason)),
                ("starts_at_ms", Value::Signed(row.try_get("starts_at_ms")?)),
                (
                    "ends_at_ms",
                    row.try_get::<Option<i64>, _>("ends_at_ms")?
                        .map_or(Value::Null, Value::Signed),
                ),
                ("revoked", Value::Bool(row.try_get("revoked")?)),
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
#[path = "export_tests.rs"]
mod export_tests;
