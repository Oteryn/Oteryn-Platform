//! Ownership-bound native runtime observations. Assignment writers are owned by T20.
//!
//! The assignment reader uses the shared epoch lock before assignment/report rows.
//! T20 must take that singleton lock exclusively when raising the global epoch.
use axum::{
    Extension, Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use sqlx::{PgPool, Row, types::Uuid};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

pub(crate) type Failure = StatusCode;
pub(crate) fn bad() -> Failure {
    StatusCode::BAD_REQUEST
}
pub(crate) fn unavailable() -> Failure {
    StatusCode::SERVICE_UNAVAILABLE
}
pub(crate) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Failure> {
    match value.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(bad()),
    }
}
pub(crate) fn number(value: &Value, key: &str, expected: u64) -> bool {
    matches!(value.get(key), Some(Value::Unsigned(n)) if *n == expected)
}
pub(crate) fn object(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}
pub(crate) fn string(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
pub(crate) fn bounded_text(s: &str, max: usize, allowed: impl Fn(u8) -> bool) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(allowed)
}
pub(crate) fn uuid(s: &str, version_seven: bool) -> Result<Uuid, Failure> {
    let id = Uuid::parse_str(s).map_err(|_| bad())?;
    if (version_seven && id.is_nil())
        || id.to_string() != s
        || (version_seven
            && (id.get_version_num() != 7
                || id.get_variant() != sqlx::types::uuid::Variant::RFC4122))
    {
        return Err(bad());
    }
    Ok(id)
}
pub(crate) fn decimal(s: &str, positive: bool) -> Result<u64, Failure> {
    let n = s.parse::<u64>().map_err(|_| bad())?;
    if n.to_string() != s || (positive && n == 0) {
        return Err(bad());
    }
    Ok(n)
}
pub(crate) fn now() -> Result<i64, Failure> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| unavailable())?
        .as_secs()
        .try_into()
        .map_err(|_| unavailable())
}
pub(crate) fn json(value: &Value) -> Response {
    match canonical::encode(value) {
        Ok(bytes) => {
            let mut response = Response::new(bytes.into());
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(_) => guard::failure(unavailable()),
    }
}

/// Fixed-window per-authenticated-peer throughput guard, independent of capacity.
/// All workers in this process share it; like Laravel this is a best-effort guard.
#[derive(Debug, Default)]
pub(crate) struct Throttle(Mutex<BTreeMap<String, (Instant, u16)>>);
impl Throttle {
    pub(crate) fn admit(&self, peer: &str, max: u16) -> Result<(), Box<Response>> {
        let mut peers = self
            .0
            .lock()
            .map_err(|_| Box::new(guard::failure(unavailable())))?;
        let (start, used) = peers.entry(peer.to_owned()).or_insert((Instant::now(), 0));
        if start.elapsed() >= Duration::from_secs(60) {
            *start = Instant::now();
            *used = 0;
        }
        if *used >= max {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            if let Ok(value) = (60 - start.elapsed().as_secs().min(59)).to_string().parse() {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
            return Err(Box::new(response));
        }
        *used += 1;
        Ok(())
    }
}

/// Scope bindings are deployment policy, separate from runtime assertions.
#[derive(Debug, Clone)]
pub struct Peer {
    pub name: String,
    pub identity: Identity,
    pub scopes: Vec<(Uuid, Uuid)>,
}
#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub peers: Vec<Peer>,
    pub freshness_seconds: u8,
    pub clock_uncertainty_seconds: u8,
    pub requests_per_minute: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            peers: vec![],
            freshness_seconds: 15,
            clock_uncertainty_seconds: 0,
            requests_per_minute: 600,
        }
    }
}
impl Settings {
    fn valid(&self) -> bool {
        (1..=60).contains(&self.freshness_seconds)
            && self.clock_uncertainty_seconds <= 5
            && self.clock_uncertainty_seconds < self.freshness_seconds
            && (1..=600).contains(&self.requests_per_minute)
            && !self.peers.is_empty()
            && self.peers.len() <= 256
            && self.peers.iter().all(|p| {
                bounded_text(&p.name, 128, |b| (0x20..=0x7e).contains(&b))
                    && !p.scopes.is_empty()
                    && p.scopes.len() <= 256
                    && p.scopes.iter().all(|(w, c)| {
                        uuid(&w.to_string(), true).is_ok() && uuid(&c.to_string(), true).is_ok()
                    })
            })
            && IdentityMap::new(
                self.peers
                    .iter()
                    .map(|p| Binding {
                        identity: p.identity.clone(),
                        group: "runtime".into(),
                    })
                    .collect(),
            )
            .is_ok()
            && self
                .peers
                .iter()
                .enumerate()
                .all(|(i, p)| self.peers[..i].iter().all(|q| p.name != q.name))
    }
}

/// A validated publication. `observed_at` alone is a heartbeat, never a new revision.
#[derive(Debug, Clone)]
pub struct Report {
    pub world_id: Uuid,
    pub channel_id: Uuid,
    pub assignment_epoch: u64,
    pub ownership_generation: u64,
    pub source_revision: u64,
    pub observed_at: i64,
    content: String,
}
impl Report {
    pub fn decode(value: &Value) -> Result<Self, Failure> {
        const FIELDS: &[&str] = &[
            "source_authority",
            "world_id",
            "channel_id",
            "node_id",
            "assignment_epoch",
            "scope_ownership_generation",
            "source_revision",
            "decision_identity",
            "ready",
            "published_at",
            "protocol_major",
            "transport_profile",
            "route_revision",
            "runtime_observation_revision",
            "ruleset_revision",
            "content_revision",
            "map_revision",
            "world_policy_revision",
            "offer_revision",
        ];
        let Value::Object(members) = value else {
            return Err(bad());
        };
        if members.len() != 22
            || !number(value, "contract_version", 1)
            || text(value, "operation")? != "ReportRuntimeStatusV1"
        {
            return Err(bad());
        }
        let mut content = Vec::new();
        for &field in FIELDS {
            let v = value.get(field).ok_or_else(bad)?;
            let valid = match field {
                "world_id" | "channel_id" => uuid(text(value, field)?, true).is_ok(),
                "node_id" => uuid(text(value, field)?, false).is_ok(),
                "assignment_epoch" | "scope_ownership_generation" | "source_revision" => {
                    decimal(text(value, field)?, true).is_ok()
                }
                "published_at" => {
                    decimal(text(value, field)?, false).is_ok_and(|n| n <= i64::MAX as u64)
                }
                "ready" => matches!(v, Value::Bool(_)),
                "protocol_major" | "transport_profile" => number(value, field, 1),
                "source_authority" => bounded_text(text(value, field)?, 128, |b| {
                    b.is_ascii_alphanumeric() || b"._:/-".contains(&b)
                }),
                _ => bounded_text(
                    text(value, field)?,
                    if field == "decision_identity" {
                        128
                    } else {
                        64
                    },
                    |b| b.is_ascii_alphanumeric() || b"._:-".contains(&b),
                ),
            };
            if !valid {
                return Err(bad());
            }
            content.push((field.to_owned(), v.clone()));
        }
        let observed_at: i64 = decimal(text(value, "observed_at")?, false)?
            .try_into()
            .map_err(|_| bad())?;
        let published_at: i64 = decimal(text(value, "published_at")?, false)?
            .try_into()
            .map_err(|_| bad())?;
        if observed_at < published_at {
            return Err(bad());
        }
        Ok(Self {
            world_id: uuid(text(value, "world_id")?, true)?,
            channel_id: uuid(text(value, "channel_id")?, true)?,
            assignment_epoch: decimal(text(value, "assignment_epoch")?, true)?,
            ownership_generation: decimal(text(value, "scope_ownership_generation")?, true)?,
            source_revision: decimal(text(value, "source_revision")?, true)?,
            observed_at,
            content: String::from_utf8(
                canonical::encode(&Value::Object(content)).map_err(|_| bad())?,
            )
            .map_err(|_| bad())?,
        })
    }
    fn key(&self) -> (u64, u64, u64) {
        (
            self.assignment_epoch,
            self.ownership_generation,
            self.source_revision,
        )
    }
}

/// T20 owns these tables and mutations. Missing schema/lock rows fail closed.
/// All readers and writers use epoch -> assignment -> report lock order.
/// Ingestion locks the assignment row exclusively; readers only share it.
async fn assignment_matches(
    tx: &mut sqlx::PgConnection,
    peer: &str,
    report: &Report,
    exclusive: bool,
) -> Result<bool, Failure> {
    let lock: Option<i16> =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id = 1 FOR SHARE")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
    if lock.is_none() {
        return Err(unavailable());
    }
    let highest: Option<String> = sqlx::query_scalar("SELECT assignment_epoch::numeric::text FROM native_scope_assignments ORDER BY assignment_epoch::numeric DESC LIMIT 1").fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    let query = if exclusive {
        "SELECT assignment_epoch::text, ownership_generation::text, node_identity FROM native_scope_assignments WHERE world_id = $1 AND channel_id = $2 FOR UPDATE"
    } else {
        "SELECT assignment_epoch::text, ownership_generation::text, node_identity FROM native_scope_assignments WHERE world_id = $1 AND channel_id = $2 FOR SHARE"
    };
    let assignment: Option<(String, String, String)> = sqlx::query_as(query)
        .bind(report.world_id)
        .bind(report.channel_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| unavailable())?;
    Ok(assignment.is_some_and(|(epoch, generation, identity)| {
        highest.as_deref() == Some(epoch.as_str())
            && epoch == report.assignment_epoch.to_string()
            && generation == report.ownership_generation.to_string()
            && identity == peer
    }))
}

pub async fn ingest(pool: &PgPool, peer: &str, report: &Report) -> Result<&'static str, Failure> {
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    if !assignment_matches(&mut tx, peer, report, true).await? {
        return Err(StatusCode::CONFLICT);
    }
    let row = sqlx::query("SELECT assignment_epoch::text AS epoch, scope_ownership_generation::text AS generation, source_revision::text AS revision, content_json, observed_at, invalid FROM native_runtime_status_reports WHERE world_id=$1 AND channel_id=$2 FOR UPDATE")
        .bind(report.world_id).bind(report.channel_id).fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    let mut result = "accepted";
    if let Some(row) = row {
        let read = |key| -> Result<u64, Failure> {
            decimal(
                row.try_get::<&str, _>(key).map_err(|_| unavailable())?,
                true,
            )
            .map_err(|_| unavailable())
        };
        let key = (read("epoch")?, read("generation")?, read("revision")?);
        if report.key() < key {
            return Ok("superseded");
        }
        if report.key() == key {
            let invalid: bool = row.try_get("invalid").map_err(|_| unavailable())?;
            let content: String = row.try_get("content_json").map_err(|_| unavailable())?;
            if invalid || content != report.content {
                sqlx::query("UPDATE native_runtime_status_reports SET invalid=true, updated_at=now() WHERE world_id=$1 AND channel_id=$2").bind(report.world_id).bind(report.channel_id).execute(&mut *tx).await.map_err(|_| unavailable())?;
                tx.commit().await.map_err(|_| unavailable())?;
                return Err(StatusCode::CONFLICT);
            }
            let observed: i64 = row.try_get("observed_at").map_err(|_| unavailable())?;
            if report.observed_at <= observed {
                return Ok("superseded");
            }
            result = "refreshed";
        }
    }
    sqlx::query("INSERT INTO native_runtime_status_reports (world_id,channel_id,assignment_epoch,scope_ownership_generation,source_revision,node_identity,content_json,observed_at) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5::text::numeric,$6,$7,$8) ON CONFLICT (world_id,channel_id) DO UPDATE SET assignment_epoch=EXCLUDED.assignment_epoch, scope_ownership_generation=EXCLUDED.scope_ownership_generation, source_revision=EXCLUDED.source_revision,node_identity=EXCLUDED.node_identity,content_json=EXCLUDED.content_json,observed_at=EXCLUDED.observed_at,invalid=false,updated_at=now()")
        .bind(report.world_id).bind(report.channel_id).bind(report.assignment_epoch.to_string()).bind(report.ownership_generation.to_string()).bind(report.source_revision.to_string()).bind(peer).bind(&report.content).bind(report.observed_at).execute(&mut *tx).await.map_err(|_| unavailable())?;
    tx.commit().await.map_err(|_| unavailable())?;
    Ok(result)
}

/// A read model never promotes unavailable or stale evidence to offline or ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Fresh,
    Stale,
    Unavailable,
    Invalid,
}
#[derive(Debug)]
pub struct Observation {
    pub state: EvidenceState,
    pub content: Option<Value>,
    pub observed_at: Option<i64>,
}
pub async fn read(
    pool: &PgPool,
    settings: &Settings,
    world: Uuid,
    channel: Uuid,
    at: i64,
) -> Result<Observation, Failure> {
    if !settings.enabled || !settings.valid() {
        return Ok(Observation {
            state: EvidenceState::Unavailable,
            content: None,
            observed_at: None,
        });
    }
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    read_in(&mut tx, settings, world, channel, at).await
}
/// `read` inside the caller's transaction, under the same epoch fence; the
/// native admission issuer selects its route with it (contract §7.4, §17).
pub async fn read_in(
    tx: &mut sqlx::PgConnection,
    settings: &Settings,
    world: Uuid,
    channel: Uuid,
    at: i64,
) -> Result<Observation, Failure> {
    let mut result = Observation {
        state: EvidenceState::Unavailable,
        content: None,
        observed_at: None,
    };
    if !settings.enabled || !settings.valid() {
        return Ok(result);
    }
    // Take the epoch fence before reading a report; retain it throughout the ownership check.
    let lock: Option<i16> =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
    if lock.is_none() {
        return Err(unavailable());
    }
    let row = sqlx::query("SELECT content_json, observed_at, invalid, node_identity FROM native_runtime_status_reports WHERE world_id=$1 AND channel_id=$2").bind(world).bind(channel).fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    let Some(row) = row else { return Ok(result) };
    let content: String = row.try_get("content_json").map_err(|_| unavailable())?;
    let observed: i64 = row.try_get("observed_at").map_err(|_| unavailable())?;
    let mut value = canonical::decode(content.as_bytes(), 2048, 2).map_err(|_| unavailable())?;
    if let Value::Object(members) = &mut value {
        members.insert(0, ("contract_version".into(), Value::Unsigned(1)));
        members.insert(1, ("operation".into(), string("ReportRuntimeStatusV1")));
        members.push(("observed_at".into(), string(observed)));
    }
    let report = Report::decode(&value).map_err(|_| unavailable())?;
    let identity: String = row.try_get("node_identity").map_err(|_| unavailable())?;
    let invalid: bool = row.try_get("invalid").map_err(|_| unavailable())?;
    let authorized = settings
        .peers
        .iter()
        .any(|p| p.name == identity && p.scopes.contains(&(world, channel)));
    result.state = if invalid
        || !authorized
        || !assignment_matches(tx, &identity, &report, false).await?
        || observed > at.saturating_add(i64::from(settings.clock_uncertainty_seconds))
    {
        EvidenceState::Invalid
    } else if at
        .saturating_sub(observed)
        .saturating_add(i64::from(settings.clock_uncertainty_seconds))
        <= i64::from(settings.freshness_seconds)
    {
        EvidenceState::Fresh
    } else {
        EvidenceState::Stale
    };
    result.content =
        Some(canonical::decode(content.as_bytes(), 2048, 2).map_err(|_| unavailable())?);
    result.observed_at = Some(observed);
    Ok(result)
}

#[derive(Debug, Clone)]
struct Runtime {
    pool: PgPool,
    settings: Settings,
    peer_maps: Vec<(Peer, Router)>,
    throttle: Arc<Throttle>,
    group_guard: Router,
}

pub(crate) fn authorization_probe(identities: Arc<IdentityMap>, group: &str) -> Router {
    guard::protect(
        Router::new().route("/", post(|| async { StatusCode::NO_CONTENT })),
        identities,
        RoutePolicy {
            enabled: true,
            group: group.into(),
            max_body_bytes: 2,
            max_json_depth: 2,
            json: JsonRule::Object,
        },
    )
}
pub(crate) async fn authorize(probe: &Router, client: Option<&VerifiedClient>) -> StatusCode {
    let mut check = Request::new(axum::body::Body::from("{}"));
    *check.method_mut() = axum::http::Method::POST;
    check.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    if let Some(client) = client {
        check.extensions_mut().insert(client.clone());
    }
    match probe.clone().oneshot(check).await {
        Ok(response) => response.status(),
        Err(never) => match never {},
    }
}

/// Explicit configuration builder. The default deployment mount remains disabled.
pub fn router<S: Clone + Send + Sync + 'static>(
    pool: PgPool,
    settings: Settings,
    identities: Arc<IdentityMap>,
) -> Router<S> {
    let peer_maps = settings
        .peers
        .iter()
        .filter_map(|peer| {
            let map = IdentityMap::new(vec![Binding {
                identity: peer.identity.clone(),
                group: "runtime".into(),
            }])
            .ok()?;
            // T15 deliberately keeps certificate identity private. Reuse its guard
            // for a bounded in-process authorization check; no network call or
            // independent certificate comparison is introduced here.
            let probe = authorization_probe(Arc::new(map), "runtime");
            Some((peer.clone(), probe))
        })
        .collect();
    let policy = RoutePolicy {
        enabled: settings.enabled && settings.valid(),
        group: "native-runtime-status".into(),
        max_body_bytes: 2048,
        max_json_depth: 2,
        json: JsonRule::Object,
    };
    let state = Runtime {
        pool,
        settings,
        peer_maps,
        throttle: Arc::default(),
        group_guard: authorization_probe(identities.clone(), "native-runtime-status"),
    };
    guard::protect(
        Router::new()
            .route("/internal/v1/game-auth/native-runtime-status", post(handle))
            .route_layer(middleware::from_fn(http_bounds)),
        identities,
        policy,
    )
    .route_layer(middleware::from_fn_with_state(state, gate))
    .route_layer(middleware::from_fn(normalize))
    .route_layer(middleware::from_fn(no_cache))
}

async fn gate(State(state): State<Runtime>, mut request: Request, next: Next) -> Response {
    if !state.settings.enabled || !state.settings.valid() {
        return guard::failure(unavailable());
    }
    let status = authorize(
        &state.group_guard,
        request.extensions().get::<VerifiedClient>(),
    )
    .await;
    if status != StatusCode::NO_CONTENT {
        return guard::failure(status);
    }
    let mut peer = None;
    for (candidate, probe) in &state.peer_maps {
        if authorize(probe, request.extensions().get::<VerifiedClient>()).await
            == StatusCode::NO_CONTENT
        {
            peer = Some(candidate.clone());
        }
    }
    let Some(peer) = peer else {
        return guard::failure(StatusCode::UNAUTHORIZED);
    };
    if let Err(response) = state
        .throttle
        .admit(&peer.name, state.settings.requests_per_minute)
    {
        return *response;
    }
    request.extensions_mut().insert(peer);
    request.extensions_mut().insert(state);
    next.run(request).await
}
async fn handle(
    Extension(state): Extension<Runtime>,
    Extension(peer): Extension<Peer>,
    Extension(value): Extension<Value>,
) -> Response {
    let result = async {
        let report = Report::decode(&value)?;
        if !peer.scopes.contains(&(report.world_id, report.channel_id)) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if report.observed_at
            > now()?.saturating_add(i64::from(state.settings.clock_uncertainty_seconds))
        {
            return Err(bad());
        }
        ingest(&state.pool, &peer.name, &report).await
    }
    .await;
    match result {
        Ok(result) => json(&object([
            ("contract_version", Value::Unsigned(1)),
            ("result", string(result)),
        ])),
        Err(status) => guard::failure(status),
    }
}

/// Mounts the two default-off routes. The TLS adapter alone supplies VerifiedClient.
/// Environment configuration contains purpose-specific certificate SANs, never keys.
pub fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool) -> Router<S> {
    configured_routes(pool, |key| std::env::var(key).ok())
}
pub fn configured_routes<S: Clone + Send + Sync + 'static>(
    pool: PgPool,
    lookup: impl Fn(&str) -> Option<String>,
) -> Router<S> {
    let (runtime, evidence, locator, identities) = configuration(lookup).unwrap_or_default();
    let identities = Arc::new(identities);
    router(pool.clone(), runtime, identities.clone())
        .merge(crate::evidence::router(
            pool.clone(),
            evidence,
            identities.clone(),
        ))
        .merge(crate::locator::router(pool, locator, identities))
}
/// The runtime-status read settings of `configured_routes`, for readers outside
/// the ingestion route (the native admission issuer). Invalid configuration is
/// the disabled default, which reads every scope as unavailable.
pub fn configured_settings(lookup: impl Fn(&str) -> Option<String>) -> Settings {
    configuration(lookup)
        .map(|(runtime, _, _, _)| runtime)
        .unwrap_or_default()
}
/// `[(name, scopes)]` from an identities variable, `{"<SAN>":["<world>/<channel>"]}`.
fn scoped_identities(raw: &str) -> Option<Vec<Peer>> {
    let Value::Object(peers) = canonical::decode(raw.as_bytes(), 131072, 4).ok()? else {
        return None;
    };
    let mut parsed = Vec::new();
    for (name, scopes) in peers {
        let identity = Identity::san(&name).ok()?;
        let Value::Array(scopes) = scopes else {
            return None;
        };
        let mut list = Vec::new();
        for scope in scopes {
            let Value::String(scope) = scope else {
                return None;
            };
            let (world, channel) = scope.split_once('/')?;
            list.push((uuid(world, true).ok()?, uuid(channel, true).ok()?));
        }
        parsed.push(Peer {
            name,
            identity,
            scopes: list,
        });
    }
    Some(parsed)
}
pub(crate) type Configuration = (
    Settings,
    crate::evidence::Settings,
    crate::locator::Settings,
    IdentityMap,
);
pub(crate) fn configuration(lookup: impl Fn(&str) -> Option<String>) -> Option<Configuration> {
    let number = |key: &str, min: u64, max: u64, default: u64| -> Option<u64> {
        let raw = lookup(key).unwrap_or_else(|| default.to_string());
        let n = decimal(&raw, false).ok()?;
        (min..=max).contains(&n).then_some(n)
    };
    let mut bindings = Vec::new();
    let mut runtime = Settings {
        enabled: lookup("GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED").as_deref() == Some("true"),
        freshness_seconds: number(
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_FRESHNESS_SECONDS",
            1,
            60,
            15,
        )? as u8,
        clock_uncertainty_seconds: number(
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_CLOCK_UNCERTAINTY_SECONDS",
            0,
            5,
            0,
        )? as u8,
        requests_per_minute: number(
            "GAME_AUTH_NATIVE_RUNTIME_STATUS_REQUESTS_PER_MINUTE",
            1,
            600,
            600,
        )? as u16,
        ..Settings::default()
    };
    if let Some(raw) = lookup("GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES") {
        for peer in scoped_identities(&raw)? {
            bindings.push(Binding {
                identity: peer.identity.clone(),
                group: "native-runtime-status".into(),
            });
            runtime.peers.push(peer);
        }
    }
    // §19.6.1: one locator certificate per node host, never a runtime-status,
    // evidence or other purpose identity (the shared map refuses an overlap).
    let environment = lookup("OTERYN_ENV").unwrap_or_default();
    let mut locator = crate::locator::Settings {
        enabled: lookup(crate::admission::PROTOCOL3_ENABLED).as_deref() == Some("true")
            && lookup(crate::admission::RECOVERY_ENABLED).as_deref() == Some("true")
            && matches!(environment.as_str(), "testing" | "preprod"),
        projection_authority: if lookup("OTERYN_LCFA_ENABLED").as_deref() == Some("true") {
            lookup("OTERYN_LCFA_SOURCE_AUTHORITY").unwrap_or_default()
        } else {
            String::new()
        },
        clock_uncertainty_seconds: runtime.clock_uncertainty_seconds,
        requests_per_minute: runtime.requests_per_minute,
        ..crate::locator::Settings::default()
    };
    if let Some(raw) = lookup(crate::locator::IDENTITIES) {
        for peer in scoped_identities(&raw)? {
            bindings.push(Binding {
                identity: peer.identity.clone(),
                group: crate::locator::GROUP.into(),
            });
            locator.peers.push(peer);
        }
    }
    let peer_identity = lookup("GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY")
        .map(|s| Identity::san(&s))
        .transpose()
        .ok()?;
    if let Some(identity) = &peer_identity {
        bindings.push(Binding {
            identity: identity.clone(),
            group: "native-evidence".into(),
        });
    }
    let evidence = crate::evidence::Settings {
        enabled: lookup("GAME_AUTH_NATIVE_EVIDENCE_ENABLED").as_deref() == Some("true"),
        peer_identity,
        activated: lookup("GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED").as_deref() == Some("true"),
        witness_directory: lookup("GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY")
            .unwrap_or_default()
            .into(),
        source_authority: lookup("GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY").unwrap_or_default(),
        fresh_account_purpose: lookup("GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE")
            .unwrap_or_default(),
        fresh_account_scope: lookup("GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE")
            .unwrap_or_default(),
        fresh_key_purpose: lookup("GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE")
            .unwrap_or_default(),
        // Evidence requires an explicit uncertainty binding; absence is unavailable.
        clock_uncertainty_seconds: if lookup("GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS")
            .is_some()
        {
            number(
                "GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS",
                0,
                5,
                0,
            )? as u8
        } else {
            u8::MAX
        },
        requests_per_minute: number("GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE", 1, 600, 120)?
            as u16,
    };
    Some((runtime, evidence, locator, IdentityMap::new(bindings).ok()?))
}

pub(crate) async fn no_cache(request: Request, next: Next) -> Response {
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
/// Additional native HTTP envelope rules absent from the shared T15 body guard.
/// Body collection, identity verification and JSON decoding stay in T15.
pub(crate) async fn http_bounds(request: Request, next: Next) -> Response {
    let mut count = 0;
    let mut bytes = request.method().as_str().len() + request.uri().to_string().len() + 14;
    for (name, value) in request.headers() {
        count += 1;
        let line = name.as_str().len() + value.as_bytes().len() + 4;
        bytes += line;
        if line > 2048 || count > 32 || bytes > 8192 {
            return guard::failure(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
        }
    }
    use axum::body::HttpBody;
    if let Some(length) = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.parse::<u64>().ok())
        && request
            .body()
            .size_hint()
            .exact()
            .is_some_and(|actual| actual != length)
    {
        return guard::failure(bad());
    }
    next.run(request).await
}
pub(crate) async fn normalize(mut request: Request, next: Next) -> Response {
    // Laravel accepts media-type parameters and case-insensitive tokens. Normalize
    // only accepted tokens before handing the envelope to the shared guard.
    for (name, expected) in [
        (header::CONTENT_TYPE, "application/json"),
        (header::CONTENT_ENCODING, "identity"),
    ] {
        if request.headers().get_all(&name).iter().count() == 1 {
            let accepted = request
                .headers()
                .get(&name)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| {
                    let token = if name == header::CONTENT_TYPE {
                        v.split(';').next().unwrap_or("")
                    } else {
                        v
                    };
                    token.trim().eq_ignore_ascii_case(expected)
                });
            if accepted {
                request
                    .headers_mut()
                    .insert(name, header::HeaderValue::from_static(expected));
            }
        }
    }
    next.run(request).await
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
pub(crate) mod tests;
