use super::*;
use axum::{
    Router,
    body::Bytes,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::{
    canonical::Value,
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub const PROJECTION_GROUP: &str = "OTERYN_GAME_CHARACTER_SERVICES";
#[derive(Clone)]
struct Http {
    services: Services,
    rate: Arc<Mutex<(Instant, u16)>>,
}
/// Mount only on the private verified TLS listener. Separate publisher identities
/// are mandatory; stopping submissions does not stop this draining exchange.
pub fn router(services: Services, identities: Arc<IdentityMap>) -> Router {
    let state = Http {
        services: services.clone(),
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let mut router = Router::new();
    for (path, handler, limit, projection) in [
        (
            "/internal/v2/game-auth/character-commands/pending",
            post(pending),
            512,
            false,
        ),
        (
            "/internal/v2/game-auth/character-commands/read",
            post(read),
            512,
            false,
        ),
        (
            "/internal/v2/game-auth/character-commands/receipt",
            post(receipt),
            1024,
            false,
        ),
        (
            "/internal/v2/game-auth/character-services/state",
            post(snapshot),
            2048,
            true,
        ),
        (
            "/internal/v2/game-auth/character-services/watermark",
            post(watermark),
            512,
            true,
        ),
    ] {
        let route = Router::new()
            .route(path, handler)
            .route_layer(middleware::from_fn_with_state(state.clone(), rate));
        router = router.merge(guard::protect(
            route,
            identities.clone(),
            RoutePolicy {
                enabled: if projection {
                    services.settings.projection_ready()
                } else {
                    services.settings.exchange_enabled
                        && valid_authority(&services.settings.command_authority)
                },
                group: if projection {
                    PROJECTION_GROUP
                } else {
                    commands::GROUP
                }
                .into(),
                max_body_bytes: limit,
                max_json_depth: 3,
                json: JsonRule::Object,
            },
        ));
    }
    router
        .with_state(state)
        .layer(middleware::from_fn(no_cache))
}
async fn no_cache(request: Request, next: Next) -> Response {
    let mut r = next.run(request).await;
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    r
}
async fn rate(State(state): State<Http>, request: Request, next: Next) -> Response {
    let allowed = if let Ok(mut rate) = state.rate.lock() {
        if rate.0.elapsed() >= Duration::from_secs(60) {
            *rate = (Instant::now(), 0);
        }
        if rate.1 < 120 {
            rate.1 += 1;
            true
        } else {
            false
        }
    } else {
        false
    };
    if !allowed {
        let mut r = guard::failure(StatusCode::TOO_MANY_REQUESTS);
        r.headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
        return r;
    }
    next.run(request).await
}
pub(super) fn status(error: &Error) -> StatusCode {
    match error {
        Error::BadInput => StatusCode::BAD_REQUEST,
        Error::Unavailable | Error::BacklogFull => StatusCode::SERVICE_UNAVAILABLE,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Conflict => StatusCode::CONFLICT,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
    }
}
fn answer(result: Result<String, Error>) -> Response {
    match result {
        Ok(raw) => ([(header::CONTENT_TYPE, "application/json")], raw).into_response(),
        Err(e) => guard::failure(status(&e)),
    }
}
async fn receipt(State(s): State<Http>, raw: Bytes) -> Response {
    answer(s.services.receive(&raw).await.map(|()| ack("accepted")))
}
async fn snapshot(State(s): State<Http>, raw: Bytes) -> Response {
    answer(s.services.ingest(&raw, false).await.map(ack))
}
async fn watermark(State(s): State<Http>, raw: Bytes) -> Response {
    answer(s.services.ingest(&raw, true).await.map(ack))
}
fn ack(result: &str) -> String {
    format!("{{\"contract_version\":2,\"result\":\"{result}\"}}")
}
async fn pending(State(s): State<Http>, raw: Bytes) -> Response {
    answer(s.services.pending(&raw).await)
}
async fn read(State(s): State<Http>, raw: Bytes) -> Response {
    answer(s.services.read(&raw).await)
}
impl Services {
    pub async fn pending(&self, raw: &[u8]) -> Result<String, Error> {
        if !self.settings.exchange_enabled {
            return Err(Error::Unavailable);
        }
        let v = wire::parse(raw, "ListPendingCharacterCommandsV2", 512)?;
        wire::exact_fields(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "after_source_revision",
                "after_operation_id",
                "max_entries",
            ],
        )?;
        source(&v, &self.settings.command_authority)?;
        let revision = wire::number(wire::string(&v, "after_source_revision")?, true)?;
        let after = wire::string(&v, "after_operation_id")?;
        let id = if revision == 0 && after == Uuid::nil().to_string() {
            Uuid::nil()
        } else if revision > 0 {
            wire::identifier(after)?
        } else {
            return Err(Error::BadInput);
        };
        let limit = match v.get("max_entries") {
            Some(Value::Unsigned(n)) if (1..=32).contains(n) => *n as i64,
            _ => return Err(Error::BadInput),
        };
        let mut entries = vec![];
        if let Ok(revision) = i64::try_from(revision) {
            for row in sqlx::query("SELECT id,command,source_revision FROM character_commands WHERE state='PENDING' AND command IN ('ScheduleCharacterDeletion','CancelCharacterDeletion','RenameCharacter') AND (source_revision,id)>($1,$2) ORDER BY source_revision,id LIMIT $3").bind(revision).bind(id).bind(limit).fetch_all(&self.pool).await? {
                entries.push(Value::Object(vec![("command".into(),Value::String(row.try_get("command")?)),("operation_id".into(),Value::String(row.try_get::<Uuid,_>("id")?.to_string())),("source_revision".into(),Value::String(row.try_get::<i64,_>("source_revision")?.to_string()))]));
            }
        }
        wire::encode_value(&Value::Object(vec![
            ("contract_version".into(), Value::Unsigned(2)),
            ("pending".into(), Value::Array(entries)),
        ]))
    }
    pub async fn read(&self, raw: &[u8]) -> Result<String, Error> {
        if !self.settings.exchange_enabled {
            return Err(Error::Unavailable);
        }
        let v = wire::parse(raw, "ReadCharacterCommandIntentV2", 512)?;
        wire::exact_fields(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "issuer_authority",
                "command",
                "operation_id",
            ],
        )?;
        source(&v, &self.settings.command_authority)?;
        if wire::string(&v, "issuer_authority")? != commands::ISSUER {
            return Err(Error::NotFound);
        }
        let id = wire::identifier(wire::string(&v, "operation_id")?)?;
        let row=sqlx::query("SELECT command,intent_json FROM character_commands WHERE id=$1 AND command IN ('ScheduleCharacterDeletion','CancelCharacterDeletion','RenameCharacter')").bind(id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        if row.try_get::<String, _>("command")? != wire::string(&v, "command")? {
            return Err(Error::Conflict);
        }
        Ok(row.try_get("intent_json")?)
    }
}

/// Compose all wire versions with the existing restore notice handler. A peerless
/// readiness probe validates v2 exchange configuration before restore can reopen
/// work. No mutation or paid command is used to negotiate capability.
pub async fn router_with_restore(
    services: Services,
    mut restore_settings: crate::restore::Settings,
    command_settings: commands::Settings,
    identities: Arc<IdentityMap>,
) -> Router {
    use axum::body::Body;
    use tower::ServiceExt;
    let exchange = router(services.clone(), identities.clone());
    let ready = match Request::post("/internal/v2/game-auth/character-commands/pending")
        .body(Body::empty())
    {
        Ok(request) => match exchange.clone().oneshot(request).await {
            Ok(response) => response.status() == StatusCode::UNAUTHORIZED,
            Err(e) => match e {},
        },
        Err(_) => false,
    };
    restore_settings.exchange_available &= ready
        && services.settings.command_authority == command_settings.source_authority
        && services.settings.command_authority == restore_settings.source_authority;
    crate::restore::router(
        crate::restore::Restore::new(services.pool.clone(), restore_settings),
        command_settings,
        identities,
    )
    .merge(exchange)
}
