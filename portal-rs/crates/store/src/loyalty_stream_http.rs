//! Private, purpose-bound stream route; mount only on verified mutual TLS 1.3.
use super::{Cursor, Error, Stream};
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    middleware,
    response::Response,
    routing::post,
};
use http::{StatusCode, header};
use oteryn_internal_api::{
    canonical::Value,
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const PATH: &str = "/internal/v1/products-entitlements/loyalty-snapshots/stream";
pub const GROUP: &str = "products-entitlements-loyalty-stream";

/// Runtime identity authority; account existence is read and fenced in the same
/// transaction as publication. No credentials or account details are exported.
/// The composition root reuses it for the premium and loyalty snapshot reads.
#[derive(Debug)]
pub struct DatabaseAccounts;
impl super::Accounts for DatabaseAccounts {
    fn state<'a>(
        &'a self,
        account: uuid::Uuid,
        connection: &'a mut sqlx::PgConnection,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<super::AccountState, Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            let known: Option<uuid::Uuid> =
                sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
                    .bind(account)
                    .fetch_optional(connection)
                    .await?;
            Ok(if known.is_some() {
                super::AccountState::Known
            } else {
                super::AccountState::Unknown
            })
        })
    }
}

/// Only register this router with the internal verified-TLS listener.
pub fn configured_router(pool: sqlx::PgPool) -> Router {
    let values: std::collections::BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    configured_with_values(pool, &values)
}

pub(super) fn configured_with_values(
    pool: sqlx::PgPool,
    values: &std::collections::BTreeMap<String, String>,
) -> Router {
    let settings = Settings::from_lookup(|key| values.get(key).cloned());
    let bindings = identity_map(values).unwrap_or_default();
    let stream = Stream::new(pool, Arc::new(DatabaseAccounts), settings.enabled);
    router(stream, settings, Arc::new(bindings))
}

pub(super) fn identity_map(
    values: &std::collections::BTreeMap<String, String>,
) -> Result<IdentityMap, guard::ConfigError> {
    let mut bindings = Vec::new();
    // Match the internal identity configuration forms, including the coins SAN
    // map. Reuse across separately mounted route groups must also fail closed.
    for (key, raw) in values {
        if key.ends_with("_IDENTITIES") {
            let Value::Object(members) =
                oteryn_internal_api::canonical::decode(raw.as_bytes(), 131072, 4)
                    .map_err(|_| guard::ConfigError)?
            else {
                return Err(guard::ConfigError);
            };
            for (san, value) in members {
                if key == "PORTAL_GAME_COINS_IDENTITIES"
                    && !matches!(value, Value::Array(ref values) if values.is_empty())
                {
                    return Err(guard::ConfigError);
                }
                bindings.push(guard::Binding {
                    identity: guard::Identity::san(&san)?,
                    group: key.clone(),
                });
            }
        } else if key.ends_with("_IDENTITY") || key.ends_with("_SAN") {
            bindings.push(guard::Binding {
                identity: guard::Identity::san(raw)?,
                group: if key == "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY" {
                    GROUP.into()
                } else {
                    key.clone()
                },
            });
        }
    }
    IdentityMap::new(bindings)
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub requests_per_minute: u32,
}
impl Settings {
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        Self {
            enabled: get("PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED").as_deref() == Some("true"),
            requests_per_minute: get("PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_REQUESTS_PER_MINUTE")
                .map_or(1200, |value| value.parse().unwrap_or(0)),
        }
    }
    /// Supply every peer binding to IdentityMap together; reuse fails closed.
    pub fn identity_binding(
        mut get: impl FnMut(&str) -> Option<String>,
    ) -> Result<guard::Binding, guard::ConfigError> {
        let identity = get("PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY")
            .ok_or(guard::ConfigError)?;
        Ok(guard::Binding {
            identity: guard::Identity::san(&identity)?,
            group: GROUP.into(),
        })
    }
}

pub fn decode(value: &Value) -> Result<(Option<Cursor>, u16), Error> {
    let Value::Object(members) = value else {
        return Err(Error::Invalid);
    };
    if members.len() != 3
        || value.get("schema") != Some(&Value::String("oteryn.loyalty_stream_request.v1".into()))
    {
        return Err(Error::Invalid);
    }
    let after = match value.get("after") {
        Some(Value::Null) => None,
        Some(Value::String(text)) => Some(Cursor::parse(text)?),
        _ => return Err(Error::Invalid),
    };
    let limit = match value.get("limit") {
        Some(Value::Signed(value)) if (1..=500).contains(value) => *value as u16,
        Some(Value::Unsigned(value)) if (1..=500).contains(value) => *value as u16,
        _ => return Err(Error::Invalid),
    };
    Ok((after, limit))
}

#[derive(Debug)]
struct StateData {
    stream: Stream,
    settings: Settings,
    rate: Mutex<(Instant, u32)>,
}
pub fn router(stream: Stream, settings: Settings, identities: Arc<IdentityMap>) -> Router {
    let enabled =
        stream.enabled && settings.enabled && (1..=6000).contains(&settings.requests_per_minute);
    let state = Arc::new(StateData {
        stream,
        settings,
        rate: Mutex::new((Instant::now(), 0)),
    });
    let routes = Router::new().route(PATH, post(read)).with_state(state);
    guard::protect(
        routes,
        identities,
        RoutePolicy {
            enabled,
            group: GROUP.into(),
            max_body_bytes: 256,
            max_json_depth: 4,
            json: JsonRule::Object,
        },
    )
    .layer(middleware::from_fn(super::super::http::contract_response))
}
async fn read(State(state): State<Arc<StateData>>, request: Request) -> Response {
    let allowed = match state.rate.lock() {
        Ok(mut rate) => {
            if rate.0.elapsed() >= Duration::from_secs(60) {
                *rate = (Instant::now(), 0);
            }
            if rate.1 >= state.settings.requests_per_minute {
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
            .insert(header::RETRY_AFTER, http::HeaderValue::from_static("60"));
        return response;
    }
    let Ok((after, limit)) = request
        .extensions()
        .get::<Value>()
        .ok_or(Error::Invalid)
        .and_then(decode)
    else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    match state
        .stream
        .read(after, limit)
        .await
        .and_then(|page| page.encode())
    {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                http::HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(Error::Invalid) => guard::failure(StatusCode::BAD_REQUEST),
        Err(_) => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
