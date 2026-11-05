//! Ownership-authority publications; revoked and ambiguous scopes authorize nobody.
use crate::runtime::{self, Peer, Throttle, bad, decimal, object, string, text, unavailable, uuid};
use axum::{
    Extension, Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::Value,
    guard::{self, Binding, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use sqlx::{PgConnection, PgPool, Row};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub world: Uuid,
    pub channel: Uuid,
    pub epoch: u64,
    pub generation: u64,
    pub node: Option<String>,
    pub decided_at: i64,
}
impl Publication {
    pub fn decode(value: &Value, revoke: bool) -> Result<Self, StatusCode> {
        let Value::Object(fields) = value else {
            return Err(bad());
        };
        let mut names = vec![
            "contract_version",
            "operation",
            "world_id",
            "channel_id",
            "assignment_epoch",
            "ownership_generation",
        ];
        names.extend(if revoke {
            vec!["revoked_at"]
        } else {
            vec!["node_identity", "assigned_at"]
        });
        if fields.len() != names.len()
            || fields.iter().any(|(k, _)| !names.contains(&k.as_str()))
            || !runtime::number(value, "contract_version", 1)
            || text(value, "operation")?
                != if revoke {
                    "ReportScopeRevocationV1"
                } else {
                    "ReportScopeAssignmentV1"
                }
        {
            return Err(bad());
        }
        let node = if revoke {
            None
        } else {
            let node = text(value, "node_identity")?;
            if !runtime::bounded_text(node, 128, |b| (0x20..=0x7e).contains(&b)) {
                return Err(bad());
            }
            Some(node.to_owned())
        };
        let result = Self {
            world: uuid(text(value, "world_id")?, true)?,
            channel: uuid(text(value, "channel_id")?, true)?,
            epoch: decimal(text(value, "assignment_epoch")?, true)?,
            generation: decimal(text(value, "ownership_generation")?, true)?,
            node,
            decided_at: decimal(
                text(value, if revoke { "revoked_at" } else { "assigned_at" })?,
                false,
            )?
            .try_into()
            .map_err(|_| bad())?,
        };
        if result.world == result.channel {
            return Err(bad());
        }
        Ok(result)
    }
}
/// Called inside the ownership transaction only for an actual state change.
/// Implementations must use this connection and must not publish external effects.
pub trait AuditHook: Send + Sync + std::fmt::Debug {
    fn changed<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        authority: &'a str,
        publication: &'a Publication,
    ) -> Pin<Box<dyn Future<Output = Result<(), StatusCode>> + Send + 'a>>;
}
#[derive(Debug)]
pub struct NoAudit;
impl AuditHook for NoAudit {
    fn changed<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a str,
        _: &'a Publication,
    ) -> Pin<Box<dyn Future<Output = Result<(), StatusCode>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}
#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub authorities: Vec<Peer>,
    pub nodes: Vec<Peer>,
    pub requests_per_minute: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            authorities: vec![],
            nodes: vec![],
            requests_per_minute: 120,
        }
    }
}
impl Settings {
    fn valid(&self) -> bool {
        if self.authorities.is_empty()
            || self.authorities.len() > 256
            || self.nodes.is_empty()
            || self.nodes.len() > 256
            || !(1..=600).contains(&self.requests_per_minute)
        {
            return false;
        }
        let mut bindings = Vec::new();
        for (peers, group) in [(&self.authorities, "authority"), (&self.nodes, "runtime")] {
            for (i, peer) in peers.iter().enumerate() {
                if !runtime::bounded_text(&peer.name, 128, |b| (0x20..=0x7e).contains(&b))
                    || peers[..i].iter().any(|p| p.name == peer.name)
                    || peer.scopes.is_empty()
                    || peer.scopes.len() > 256
                    || peer.scopes.iter().any(|(w, c)| {
                        w == c
                            || uuid(&w.to_string(), true).is_err()
                            || uuid(&c.to_string(), true).is_err()
                    })
                {
                    return false;
                }
                bindings.push(Binding {
                    identity: peer.identity.clone(),
                    group: group.into(),
                });
            }
        }
        IdentityMap::new(bindings).is_ok()
    }
}
/// Every mutation takes the exclusive epoch lock before reading assignment rows.
pub async fn publish(
    pool: &PgPool,
    settings: &Settings,
    authority: &str,
    publication: &Publication,
    audit: &dyn AuditHook,
) -> Result<&'static str, StatusCode> {
    if !settings.enabled || !settings.valid() {
        return Err(unavailable());
    }
    let scope = (publication.world, publication.channel);
    if !settings
        .authorities
        .iter()
        .any(|p| p.name == authority && p.scopes.contains(&scope))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if publication.epoch == 0
        || publication.generation == 0
        || publication.decided_at < 0
        || uuid(&publication.world.to_string(), true).is_err()
        || uuid(&publication.channel.to_string(), true).is_err()
        || publication.world == publication.channel
    {
        return Err(bad());
    }
    if let Some(node) = &publication.node
        && !settings
            .nodes
            .iter()
            .any(|p| &p.name == node && p.scopes.contains(&scope))
    {
        return Err(StatusCode::CONFLICT);
    }
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| unavailable())?;
    let _: i16 =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
    let highest: Option<String> =
        sqlx::query_scalar("SELECT max(assignment_epoch)::text FROM native_scope_assignments")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
    if let Some(highest) = highest
        && publication.epoch < decimal(&highest, true).map_err(|_| unavailable())?
    {
        return Ok("superseded");
    }
    let row = sqlx::query("SELECT assignment_epoch::text AS epoch, ownership_generation::text AS generation, node_identity, assigned_at, revoked_at FROM native_scope_assignments WHERE world_id=$1 AND channel_id=$2 FOR UPDATE").bind(publication.world).bind(publication.channel).fetch_optional(&mut *tx).await.map_err(|_|unavailable())?;
    if let Some(row) = row {
        let epoch: String = row.try_get("epoch").map_err(|_| unavailable())?;
        let generation: String = row.try_get("generation").map_err(|_| unavailable())?;
        let key = (
            decimal(&epoch, true).map_err(|_| unavailable())?,
            decimal(&generation, true).map_err(|_| unavailable())?,
        );
        match (publication.epoch, publication.generation).cmp(&key) {
            std::cmp::Ordering::Less => return Ok("superseded"),
            std::cmp::Ordering::Equal => {
                let node: Option<String> =
                    row.try_get("node_identity").map_err(|_| unavailable())?;
                let time: Option<i64> = row
                    .try_get(if publication.node.is_some() {
                        "assigned_at"
                    } else {
                        "revoked_at"
                    })
                    .map_err(|_| unavailable())?;
                return if node == publication.node && time == Some(publication.decided_at) {
                    Ok("accepted")
                } else {
                    Err(StatusCode::CONFLICT)
                };
            }
            std::cmp::Ordering::Greater => {}
        }
    }
    sqlx::query("INSERT INTO native_scope_assignments(world_id,channel_id,assignment_epoch,ownership_generation,node_identity,assigned_at,revoked_at) VALUES($1,$2,$3::text::numeric,$4::text::numeric,$5,$6,$7) ON CONFLICT(world_id,channel_id) DO UPDATE SET assignment_epoch=EXCLUDED.assignment_epoch,ownership_generation=EXCLUDED.ownership_generation,node_identity=EXCLUDED.node_identity,assigned_at=EXCLUDED.assigned_at,revoked_at=EXCLUDED.revoked_at,updated_at=now()")
        .bind(publication.world).bind(publication.channel).bind(publication.epoch.to_string()).bind(publication.generation.to_string()).bind(&publication.node).bind(publication.node.as_ref().map(|_|publication.decided_at)).bind(publication.node.is_none().then_some(publication.decided_at)).execute(&mut *tx).await.map_err(|_|unavailable())?;
    sqlx::query("UPDATE native_runtime_status_reports SET invalid=true,updated_at=now() WHERE assignment_epoch < $1::text::numeric OR (world_id=$2 AND channel_id=$3)").bind(publication.epoch.to_string()).bind(publication.world).bind(publication.channel).execute(&mut *tx).await.map_err(|_|unavailable())?;
    audit.changed(&mut tx, authority, publication).await?;
    tx.commit().await.map_err(|_| unavailable())?;
    Ok("accepted")
}
/// Locks authoritative ownership until the caller's admission transaction ends.
pub async fn allows(
    connection: &mut PgConnection,
    world: Uuid,
    channel: Uuid,
    node: &str,
    epoch: u64,
    generation: u64,
) -> Result<bool, StatusCode> {
    let _: i16 =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE")
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| unavailable())?;
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_scope_assignments WHERE world_id=$1 AND channel_id=$2 AND node_identity=$3 AND revoked_at IS NULL AND assignment_epoch=$4::text::numeric AND ownership_generation=$5::text::numeric AND assignment_epoch=(SELECT max(assignment_epoch) FROM native_scope_assignments))")
        .bind(world).bind(channel).bind(node).bind(epoch.to_string()).bind(generation.to_string()).fetch_one(connection).await.map_err(|_|unavailable())
}

#[derive(Clone)]
struct HttpState {
    pool: PgPool,
    settings: Settings,
    audit: Arc<dyn AuditHook>,
    probes: Vec<(Peer, Router)>,
    throttle: Arc<Throttle>,
}
pub fn router<S: Clone + Send + Sync + 'static>(
    pool: PgPool,
    settings: Settings,
    identities: Arc<IdentityMap>,
    audit: Arc<dyn AuditHook>,
) -> Router<S> {
    let probes = settings
        .authorities
        .iter()
        .filter_map(|p| {
            let map = IdentityMap::new(vec![Binding {
                identity: p.identity.clone(),
                group: "authority".into(),
            }])
            .ok()?;
            Some((
                p.clone(),
                runtime::authorization_probe(Arc::new(map), "authority"),
            ))
        })
        .collect();
    let enabled = settings.enabled && settings.valid();
    let state = HttpState {
        pool,
        settings,
        audit,
        probes,
        throttle: Arc::default(),
    };
    guard::protect(
        Router::new()
            .route(
                "/internal/v1/game-auth/native-scope-assignments",
                post(assign),
            )
            .route(
                "/internal/v1/game-auth/native-scope-revocations",
                post(revoke),
            )
            .route_layer(middleware::from_fn_with_state(state, gate))
            .route_layer(middleware::from_fn(runtime::http_bounds)),
        identities,
        RoutePolicy {
            enabled,
            group: "native-scope-assignments".into(),
            max_body_bytes: 2048,
            max_json_depth: 2,
            json: JsonRule::Object,
        },
    )
    .route_layer(middleware::from_fn(runtime::normalize))
    .route_layer(middleware::from_fn(runtime::no_cache))
}
async fn gate(State(state): State<HttpState>, mut request: Request, next: Next) -> Response {
    let mut peer = None;
    for (candidate, probe) in &state.probes {
        if runtime::authorize(probe, request.extensions().get::<VerifiedClient>()).await
            == StatusCode::NO_CONTENT
        {
            if peer.is_some() {
                return guard::failure(unavailable());
            }
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
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
async fn assign(
    Extension(state): Extension<HttpState>,
    Extension(peer): Extension<Peer>,
    Extension(value): Extension<Value>,
) -> Response {
    handle(state, peer, value, false).await
}
async fn revoke(
    Extension(state): Extension<HttpState>,
    Extension(peer): Extension<Peer>,
    Extension(value): Extension<Value>,
) -> Response {
    handle(state, peer, value, true).await
}
async fn handle(state: HttpState, peer: Peer, value: Value, revoke: bool) -> Response {
    let result = async {
        let publication = Publication::decode(&value, revoke)?;
        publish(
            &state.pool,
            &state.settings,
            &peer.name,
            &publication,
            state.audit.as_ref(),
        )
        .await
    }
    .await;
    match result {
        Ok(result) => runtime::json(&object([
            ("contract_version", Value::Unsigned(1)),
            ("result", string(result)),
        ])),
        Err(status) => guard::failure(status),
    }
}
#[cfg(test)]
#[path = "scopes_tests.rs"]
mod tests;

/// Default-off deployment mount. Purpose identity overlap makes configuration unavailable.
pub fn routes<S: Clone + Send + Sync + 'static>(pool: PgPool) -> Router<S> {
    configured_routes(pool, |key| std::env::var(key).ok())
}
pub fn configured_routes<S: Clone + Send + Sync + 'static>(
    pool: PgPool,
    lookup: impl Fn(&str) -> Option<String>,
) -> Router<S> {
    let (settings, identities) = configuration(lookup).unwrap_or_default();
    router(pool, settings, Arc::new(identities), Arc::new(NoAudit))
}
fn configuration(lookup: impl Fn(&str) -> Option<String>) -> Option<(Settings, IdentityMap)> {
    use oteryn_internal_api::{canonical, guard::Identity};
    let parse = |key: &str| -> Option<Vec<Peer>> {
        let raw = lookup(key)?;
        let Value::Object(entries) = canonical::decode(raw.as_bytes(), 131072, 4).ok()? else {
            return None;
        };
        if entries.is_empty() || entries.len() > 256 {
            return None;
        }
        entries
            .into_iter()
            .map(|(name, value)| {
                let identity = Identity::san(&name).ok()?;
                let Value::Array(scopes) = value else {
                    return None;
                };
                if scopes.is_empty() || scopes.len() > 256 {
                    return None;
                }
                let scopes = scopes
                    .into_iter()
                    .map(|v| {
                        let Value::String(v) = v else { return None };
                        let (world, channel) = v.split_once('/')?;
                        let pair = (uuid(world, true).ok()?, uuid(channel, true).ok()?);
                        (pair.0 != pair.1).then_some(pair)
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(Peer {
                    name,
                    identity,
                    scopes,
                })
            })
            .collect()
    };
    let settings = Settings {
        enabled: lookup("GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED").as_deref() == Some("true"),
        authorities: parse("GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES")?,
        nodes: parse("GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES")?,
        requests_per_minute: decimal(
            &lookup("GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_REQUESTS_PER_MINUTE")
                .unwrap_or_else(|| "120".into()),
            true,
        )
        .ok()?
        .try_into()
        .ok()?,
    };
    let mut bindings = Vec::new();
    for (peers, group) in [
        (&settings.authorities, "native-scope-assignments"),
        (&settings.nodes, "native-runtime-status"),
    ] {
        bindings.extend(peers.iter().map(|p| Binding {
            identity: p.identity.clone(),
            group: group.into(),
        }));
    }
    if let Some(identity) = lookup("GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY") {
        bindings.push(Binding {
            identity: Identity::san(&identity).ok()?,
            group: "native-evidence".into(),
        });
    }
    Some((settings, IdentityMap::new(bindings).ok()?))
}
