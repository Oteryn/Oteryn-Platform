//! Agent-initiated control-plane transport. Only verified TLS extensions authenticate peers.
use crate::control::{canonical, signer, store};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_internal_api::guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

const MIB: usize = 1024 * 1024;
/// Connection adapter evidence, populated only after certificate verification.
/// The adapter must pass the certificate subject CN, never an HTTP header.
#[derive(Debug, Clone)]
pub struct VerifiedAgent(String);
impl VerifiedAgent {
    pub fn from_verified_certificate_cn(cn: &str) -> Result<Self, HttpError> {
        let id = cn
            .strip_prefix("oteryn-admin-agent/")
            .ok_or(HttpError::forbidden())?;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(HttpError::forbidden());
        }
        Ok(Self(id.to_owned()))
    }
}
#[derive(Debug, Clone)]
pub struct AgentBinding {
    pub agent_id: String,
    pub identity: Identity,
    pub result_key: Option<(ed25519_dalek::VerifyingKey, signer::Validity)>,
}
#[derive(Clone)]
struct Context {
    pool: PgPool,
    store: store::Store,
    binding: AgentBinding,
    gate: Arc<tokio::sync::Mutex<()>>,
}
#[derive(Debug)]
pub struct HttpError(StatusCode, &'static str);
impl HttpError {
    fn invalid() -> Self {
        Self(StatusCode::BAD_REQUEST, "ADM_INVALID_PARAMS")
    }
    fn forbidden() -> Self {
        Self(StatusCode::FORBIDDEN, "ADM_FORBIDDEN")
    }
    fn limit() -> Self {
        Self(StatusCode::PAYLOAD_TOO_LARGE, "ADM_LIMIT_EXCEEDED")
    }
}
impl From<sqlx::Error> for HttpError {
    fn from(_: sqlx::Error) -> Self {
        Self(StatusCode::SERVICE_UNAVAILABLE, "ADM_INTERNAL")
    }
}
impl From<store::Error> for HttpError {
    fn from(e: store::Error) -> Self {
        match e {
            store::Error::Disabled => Self(StatusCode::NOT_FOUND, "ADM_FORBIDDEN"),
            store::Error::InvalidInput => Self::invalid(),
            store::Error::Unavailable => Self(StatusCode::CONFLICT, "ADM_AGENT_SUPERSEDED"),
            store::Error::Conflict => Self(StatusCode::CONFLICT, "ADM_IDEMPOTENCY_CONFLICT"),
            store::Error::NotFound => Self(StatusCode::NOT_FOUND, "ADM_TARGET_NOT_FOUND"),
            store::Error::IllegalTransition => {
                Self(StatusCode::CONFLICT, "ADM_PRECONDITION_FAILED")
            }
            store::Error::SelfApproval => Self::forbidden(),
            store::Error::Storage(e) => e.into(),
            store::Error::Audit(_) => Self(StatusCode::SERVICE_UNAVAILABLE, "ADM_INTERNAL"),
        }
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(json!({"error":{"code":self.1,"retryable": self.1 == "ADM_INTERNAL"}})),
        )
            .into_response()
    }
}
/// One router per configured identity. IdentityMap rejects duplicate transport identities.
/// A single dispatcher chooses the peer by the certificate CN extension, then the
/// existing internal guard verifies its SAN/SPKI binding before parsing any body.
pub fn router(
    pool: PgPool,
    enabled: bool,
    bindings: Vec<AgentBinding>,
) -> Result<Router, guard::ConfigError> {
    let identities = Arc::new(IdentityMap::new(
        bindings
            .iter()
            .map(|b| Binding {
                identity: b.identity.clone(),
                group: b.agent_id.clone(),
            })
            .collect(),
    )?);
    if enabled && pool.options().get_max_connections() < 2 {
        return Err(guard::ConfigError);
    }
    let gate = Arc::new(tokio::sync::Mutex::new(()));
    let mut agents = Vec::new();
    for binding in bindings {
        if VerifiedAgent::from_verified_certificate_cn(&format!(
            "oteryn-admin-agent/{}",
            binding.agent_id
        ))
        .is_err()
            || agents.iter().any(|(id, _)| id == &binding.agent_id)
        {
            return Err(guard::ConfigError);
        }
        let context = Context {
            pool: pool.clone(),
            store: store::Store::new(pool.clone(), store::Config { enabled }),
            binding: binding.clone(),
            gate: gate.clone(),
        };
        let small = Router::new()
            .route("/hello", post(hello))
            .route("/channel", post(channel))
            .route("/results", post(results))
            .route("/rollups", post(rollups))
            .route("/heartbeat", post(heartbeat));
        let large = Router::new().route("/stream", post(stream));
        let protect = |r, max| {
            guard::protect(
                r,
                identities.clone(),
                RoutePolicy {
                    enabled,
                    group: binding.agent_id.clone(),
                    max_body_bytes: max,
                    max_json_depth: 32,
                    json: JsonRule::Object,
                },
            )
        };
        let routes = Router::new()
            .nest(
                "/internal/v1/admin-control",
                protect(small, MIB).merge(protect(large, 90 * MIB)),
            )
            .layer(axum::extract::DefaultBodyLimit::disable())
            .with_state(context);
        agents.push((binding.agent_id, routes));
    }
    let dispatch = Dispatch {
        enabled,
        agents: Arc::new(agents),
    };
    Ok(Router::new()
        .route(
            "/internal/v1/admin-control/{endpoint}",
            post(|| async { StatusCode::NOT_FOUND }),
        )
        .layer(middleware::from_fn_with_state(dispatch, dispatch_peer)))
}
#[derive(Clone)]
struct Dispatch {
    enabled: bool,
    agents: Arc<Vec<(String, Router)>>,
}
async fn dispatch_peer(State(state): State<Dispatch>, request: Request, _: Next) -> Response {
    if !state.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(peer) = request.extensions().get::<VerifiedAgent>() else {
        return HttpError::forbidden().into_response();
    };
    let Some((_, routes)) = state.agents.iter().find(|(id, _)| id == &peer.0) else {
        return HttpError::forbidden().into_response();
    };
    use tower::ServiceExt;
    let response = match routes.clone().oneshot(request).await {
        Ok(r) => r,
        Err(e) => match e {},
    };
    match response.status() {
        StatusCode::UNAUTHORIZED => HttpError::forbidden().into_response(),
        StatusCode::BAD_REQUEST if !response.headers().contains_key("content-type") => {
            HttpError::invalid().into_response()
        }
        StatusCode::PAYLOAD_TOO_LARGE if !response.headers().contains_key("content-type") => {
            HttpError::limit().into_response()
        }
        _ => response,
    }
}
fn check(ctx: &Context, agent: &str, epoch: i64) -> Result<(), HttpError> {
    if agent != ctx.binding.agent_id {
        return Err(HttpError::forbidden());
    }
    if !(1..=9_007_199_254_740_992).contains(&epoch) {
        return Err(HttpError::invalid());
    }
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, HttpError> {
    canonical::encode_wire(&value).map_err(|_| HttpError::invalid())?;
    serde_json::from_value(value).map_err(|_| HttpError::invalid())
}
// The existing guard's bounded decoder rejects duplicate JSON members.
fn body<T: serde::de::DeserializeOwned>(
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<T, HttpError> {
    decode(raw.map_err(|_| HttpError::invalid())?.0)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hello {
    agent_id: String,
    build: String,
    catalog_digest: String,
    advertised_operations: Vec<store::AdvertisedOperation>,
    served_worlds: Vec<String>,
    served_nodes: Vec<String>,
    owner_epoch: i64,
    clock: String,
}
async fn hello(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let h: Hello = body(raw)?;
    check(&ctx, &h.agent_id, h.owner_epoch)?;
    time::OffsetDateTime::parse(&h.clock, &time::format_description::well_known::Rfc3339)
        .map_err(|_| HttpError::invalid())?;
    let digest = URL_SAFE_NO_PAD
        .decode(&h.catalog_digest)
        .map_err(|_| HttpError::invalid())?;
    let _permit = ctx.gate.lock().await;
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, false).await?;
    // A competing owner at the same or newer epoch makes this hello ambiguous.
    let conflict:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_agents WHERE agent_id<>$1 AND owner_epoch >= $2 AND (served_worlds && $3 OR served_nodes && $4))")
        .bind(&h.agent_id).bind(h.owner_epoch).bind(&h.served_worlds).bind(&h.served_nodes).fetch_one(&mut *tx).await?;
    if conflict {
        return Err(store::Error::Unavailable.into());
    }
    ctx.store
        .register_agent(&store::AgentHello {
            agent_id: h.agent_id,
            build: h.build,
            catalog_digest: digest,
            advertised_operations: h.advertised_operations,
            served_worlds: h.served_worlds,
            served_nodes: h.served_nodes,
            owner_epoch: h.owner_epoch,
        })
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"status":"registered"})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Poll {
    agent_id: String,
    owner_epoch: i64,
    #[serde(default)]
    wait_seconds: u8,
}
async fn channel(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let p: Poll = body(raw)?;
    check(&ctx, &p.agent_id, p.owner_epoch)?;
    if p.wait_seconds > 25 {
        return Err(HttpError::invalid());
    }
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, true).await?;
    session(&mut tx, &p.agent_id, p.owner_epoch).await?;
    tx.commit().await?;
    ctx.store.heartbeat(&p.agent_id, p.owner_epoch).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(u64::from(p.wait_seconds));
    loop {
        let requests = ctx
            .store
            .claim_pending(&p.agent_id, p.owner_epoch, 32)
            .await?;
        if !requests.is_empty() || tokio::time::Instant::now() >= deadline {
            let mut envelopes = Vec::new();
            for r in requests {
                let mut value: Value = serde_json::from_slice(&r.canonical_envelope)
                    .map_err(|_| HttpError(StatusCode::SERVICE_UNAVAILABLE, "ADM_INTERNAL"))?;
                let object = value.as_object_mut().ok_or(HttpError::invalid())?;
                object.insert(
                    "signature".into(),
                    Value::String(URL_SAFE_NO_PAD.encode(r.signature)),
                );
                envelopes.push(value);
            }
            return Ok(Json(json!({"requests":envelopes})));
        }
        tokio::time::sleep_until(
            deadline.min(tokio::time::Instant::now() + Duration::from_millis(100)),
        )
        .await;
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultPost {
    owner_epoch: i64,
    envelope: signer::ResultEnvelope,
}
async fn results(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let p: ResultPost = body(raw)?;
    let e = &p.envelope;
    check(&ctx, &e.agent_id, p.owner_epoch)?;
    if let Some((key, validity)) = &ctx.binding.result_key {
        signer::verify_result(
            e,
            key,
            *validity,
            time::OffsetDateTime::now_utc().unix_timestamp(),
        )
        .map_err(|error| {
            HttpError(
                StatusCode::FORBIDDEN,
                match error {
                    signer::Error::Untrusted => "ADM_SIGNER_UNTRUSTED",
                    _ => "ADM_BAD_SIGNATURE",
                },
            )
        })?;
    }
    let payload = serde_json::to_value(e).map_err(|_| HttpError::invalid())?;
    let status: store::Status =
        serde_json::from_value(serde_json::to_value(e.status).map_err(|_| HttpError::invalid())?)
            .map_err(|_| HttpError::invalid())?;
    let _permit = ctx.gate.lock().await;
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, true).await?;
    lock_request(&mut tx, e.request_id).await?;
    owns_request(&mut tx, &e.agent_id, p.owner_epoch, e.request_id).await?;
    let operation: Uuid = sqlx::query_scalar(
        "SELECT admin_operation_id FROM admin_control_requests WHERE request_id=$1",
    )
    .bind(e.request_id)
    .fetch_one(&mut *tx)
    .await?;
    if operation != e.admin_operation_id {
        return Err(HttpError::invalid());
    }
    let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_results WHERE request_id=$1 AND agent_id=$2 AND payload=$3)").bind(e.request_id).bind(&e.agent_id).bind(&payload).fetch_one(&mut *tx).await?;
    if !duplicate {
        ctx.store
            .record_result(
                &store::NewResult {
                    request_id: e.request_id,
                    agent_id: e.agent_id.clone(),
                    status,
                    game_ledger_id: e.game_ledger_id.clone(),
                    error_code: e.error.as_ref().map(|e| e.code.clone()),
                    payload,
                },
                p.owner_epoch,
            )
            .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"status":"recorded"})))
}
async fn lock_request(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), HttpError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17))")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn owns_request(
    tx: &mut Transaction<'_, Postgres>,
    agent: &str,
    epoch: i64,
    id: Uuid,
) -> Result<(), HttpError> {
    let owns:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_requests r JOIN admin_control_agents a ON a.agent_id=r.agent_id WHERE r.request_id=$1 AND a.agent_id=$2 AND a.owner_epoch=$3 AND (r.target->>'world_id' IS NULL OR r.target->>'world_id'=ANY(a.served_worlds)) AND (r.target->>'node_id' IS NULL OR r.target->>'node_id'=ANY(a.served_nodes)) AND NOT EXISTS(SELECT 1 FROM admin_control_agents b WHERE b.agent_id<>a.agent_id AND b.owner_epoch>=a.owner_epoch AND (r.target->>'world_id'=ANY(b.served_worlds) OR r.target->>'node_id'=ANY(b.served_nodes))))")
        .bind(id).bind(agent).bind(epoch).fetch_one(&mut **tx).await?;
    if !owns {
        return Err(store::Error::Unavailable.into());
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stream {
    agent_id: String,
    owner_epoch: i64,
    request_id: Uuid,
    chunks: Vec<Chunk>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chunk {
    index: u8,
    data: String,
}
async fn stream(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let p: Stream = body(raw)?;
    check(&ctx, &p.agent_id, p.owner_epoch)?;
    if p.chunks.is_empty() || p.chunks.len() > 64 {
        return Err(HttpError::limit());
    }
    let mut chunks = Vec::new();
    for c in p.chunks {
        if c.index >= 64 || c.data.len() > MIB.div_ceil(3) * 4 {
            return Err(HttpError::limit());
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(c.data)
            .map_err(|_| HttpError::invalid())?;
        if bytes.len() > MIB {
            return Err(HttpError::limit());
        }
        chunks.push((c.index, bytes));
    }
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, true).await?;
    lock_request(&mut tx, p.request_id).await?;
    owns_request(&mut tx, &p.agent_id, p.owner_epoch, p.request_id).await?;
    for (index, bytes) in chunks {
        let existing:Option<Vec<u8>>=sqlx::query_scalar("SELECT payload FROM admin_control_stream_chunks WHERE request_id=$1 AND chunk_index=$2").bind(p.request_id).bind(i32::from(index)).fetch_optional(&mut *tx).await?;
        if let Some(old) = existing {
            if old != bytes {
                return Err(store::Error::Conflict.into());
            }
        } else {
            sqlx::query("INSERT INTO admin_control_stream_chunks(request_id,chunk_index,agent_id,payload) VALUES($1,$2,$3,$4)").bind(p.request_id).bind(i32::from(index)).bind(&p.agent_id).bind(bytes).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(Json(json!({"status":"stored"})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    agent_id: String,
    owner_epoch: i64,
    payload: canonical::Json,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rollup {
    agent_id: String,
    owner_epoch: i64,
    key: String,
    payload: canonical::Json,
}
async fn heartbeat(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let p: Snapshot = body(raw)?;
    check(&ctx, &p.agent_id, p.owner_epoch)?;
    if !p.payload.0.is_object() {
        return Err(HttpError::invalid());
    }
    let _permit = ctx.gate.lock().await;
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, true).await?;
    session(&mut tx, &p.agent_id, p.owner_epoch).await?;
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=clock_timestamp(),updated_at=clock_timestamp() WHERE agent_id=$1 AND owner_epoch=$2").bind(&p.agent_id).bind(p.owner_epoch).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO admin_control_health(agent_id,owner_epoch,payload) VALUES($1,$2,$3) ON CONFLICT(agent_id) DO UPDATE SET owner_epoch=EXCLUDED.owner_epoch,payload=EXCLUDED.payload,updated_at=clock_timestamp() WHERE admin_control_health.owner_epoch<=EXCLUDED.owner_epoch")
        .bind(p.agent_id).bind(p.owner_epoch).bind(p.payload.0).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"status":"recorded"})))
}
async fn rollups(
    State(ctx): State<Context>,
    raw: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, HttpError> {
    let p: Rollup = body(raw)?;
    check(&ctx, &p.agent_id, p.owner_epoch)?;
    if p.key.trim().is_empty() || p.key.len() > 256 || !p.payload.0.is_object() {
        return Err(HttpError::invalid());
    }
    let _permit = ctx.gate.lock().await;
    let mut tx = ctx.pool.begin().await?;
    ingress_lock(&mut tx, true).await?;
    session(&mut tx, &p.agent_id, p.owner_epoch).await?;
    sqlx::query("UPDATE admin_control_agents SET last_seen_at=clock_timestamp(),updated_at=clock_timestamp() WHERE agent_id=$1 AND owner_epoch=$2").bind(&p.agent_id).bind(p.owner_epoch).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO analytics_cache(agent_id,cache_key,owner_epoch,payload) VALUES($1,$2,$3,$4) ON CONFLICT(agent_id,cache_key) DO UPDATE SET owner_epoch=EXCLUDED.owner_epoch,payload=EXCLUDED.payload,updated_at=clock_timestamp() WHERE analytics_cache.owner_epoch<=EXCLUDED.owner_epoch")
        .bind(p.agent_id).bind(p.key).bind(p.owner_epoch).bind(p.payload.0).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"status":"stored"})))
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;

/// Startup configuration contains public identities and public verification keys only.
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = store::Config::from_env().enabled;
    if !enabled {
        return Ok(router(pool, false, Vec::new())?);
    }
    let config = std::env::var("PORTAL_ADMIN_CONTROL_AGENTS_JSON")?;
    let bindings: Vec<ConfiguredAgent> = serde_json::from_str(&config)?;
    if bindings.is_empty() {
        return Err(guard::ConfigError.into());
    }
    let bindings = bindings
        .into_iter()
        .map(|b| {
            let peer = VerifiedAgent::from_verified_certificate_cn(&b.cn)
                .map_err(|_| guard::ConfigError)?;
            let identity = match b.identity {
                ConfiguredIdentity::San(s) => Identity::san(&s)?,
                ConfiguredIdentity::Spki(s) => Identity::spki_sha256(
                    URL_SAFE_NO_PAD
                        .decode(s)
                        .map_err(|_| guard::ConfigError)?
                        .try_into()
                        .map_err(|_| guard::ConfigError)?,
                ),
            };
            let result_key = b
                .result_key
                .map(|k| {
                    let bytes: [u8; 32] = URL_SAFE_NO_PAD
                        .decode(k.public_key)
                        .map_err(|_| guard::ConfigError)?
                        .try_into()
                        .map_err(|_| guard::ConfigError)?;
                    if k.not_after <= k.not_before {
                        return Err(guard::ConfigError);
                    }
                    Ok((
                        ed25519_dalek::VerifyingKey::from_bytes(&bytes)
                            .map_err(|_| guard::ConfigError)?,
                        signer::Validity {
                            not_before: k.not_before,
                            not_after: k.not_after,
                        },
                    ))
                })
                .transpose()?;
            Ok(AgentBinding {
                agent_id: peer.0,
                identity,
                result_key,
            })
        })
        .collect::<Result<Vec<_>, guard::ConfigError>>()?;
    Ok(router(pool, true, bindings)?)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredAgent {
    cn: String,
    identity: ConfiguredIdentity,
    result_key: Option<ConfiguredKey>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConfiguredIdentity {
    San(String),
    Spki(String),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredKey {
    public_key: String,
    not_before: i64,
    not_after: i64,
}

async fn ingress_lock(tx: &mut Transaction<'_, Postgres>, shared: bool) -> Result<(), HttpError> {
    if shared {
        sqlx::query(
            "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-http',0))",
        )
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('admin-control-http',0))")
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
async fn session(
    tx: &mut Transaction<'_, Postgres>,
    agent: &str,
    epoch: i64,
) -> Result<(), HttpError> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_control_agents a WHERE a.agent_id=$1 AND a.owner_epoch=$2 AND NOT EXISTS(SELECT 1 FROM admin_control_agents b WHERE b.agent_id<>a.agent_id AND b.owner_epoch>=a.owner_epoch AND (a.served_worlds && b.served_worlds OR a.served_nodes && b.served_nodes)))")
        .bind(agent).bind(epoch).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(store::Error::Unavailable.into());
    }
    Ok(())
}
