//! Gateway login contract §19.6: the recovery locator feed of Game G1
//! (`ReportRecoveryLocatorV1`) on the internal mTLS surface. A locator entry is
//! routing evidence for §19.4 step 5 only and never authority. The route is
//! default off: it needs protocol 3 and recovery on in `testing` or `preprod`,
//! locator identities (`GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES`) and the
//! runtime-status transport bounds it shares (§19.6.1). The assignment epoch
//! and ownership generation are stored, never logged (§19.10).
use crate::{
    admission,
    runtime::{self, Failure, Peer, Throttle, bad, unavailable},
};
use axum::{
    Extension, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::Value,
    guard::{self, Binding, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use sqlx::{PgPool, Row, types::Uuid};
use std::sync::Arc;

pub const PATH: &str = "/internal/v1/game-auth/native-recovery-locators";
/// Locator identities and their scopes, the runtime-status JSON shape (§19.6.1).
pub const IDENTITIES: &str = "GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES";
pub(crate) const GROUP: &str = "native-recovery-locator";
/// Largest report (§19.6.2).
pub const MAX_REPORT_BYTES: usize = 1024;
const OPERATION: &str = "ReportRecoveryLocatorV1";
const FIELDS: [&str; 12] = [
    "contract_version",
    "operation",
    "source_authority",
    "assignment_epoch",
    "scope_ownership_generation",
    "world_id",
    "channel_id",
    "account_id",
    "character_id",
    "present",
    "locator_revision",
    "committed_at",
];

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    pub peers: Vec<Peer>,
    /// `source_authority` of the §5 read model; empty while it is disabled, which
    /// refuses every report (§19.6.3 rule 5).
    pub projection_authority: String,
    /// Shared with runtime status (§19.6.1), 0..=5.
    pub clock_uncertainty_seconds: u8,
    /// Shared with runtime status (§19.6.1), 1..=600.
    pub requests_per_minute: u16,
}
impl Settings {
    fn valid(&self) -> bool {
        self.clock_uncertainty_seconds <= 5
            && (1..=600).contains(&self.requests_per_minute)
            && !self.peers.is_empty()
            && self.peers.len() <= 256
            && self.peers.iter().enumerate().all(|(i, p)| {
                runtime::bounded_text(&p.name, 128, |b| (0x20..=0x7e).contains(&b))
                    && !p.scopes.is_empty()
                    && p.scopes.len() <= 256
                    && self.peers[..i].iter().all(|q| q.name != p.name)
            })
    }
}

/// A validated §19.6.2 report. `Debug` shows the scope only (§19.10).
#[derive(Clone, PartialEq, Eq)]
pub struct Report {
    source_authority: String,
    assignment_epoch: u64,
    ownership_generation: u64,
    pub world_id: Uuid,
    pub channel_id: Uuid,
    account_id: Uuid,
    character_id: Uuid,
    present: bool,
    locator_revision: u64,
    committed_at: i64,
}
impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Report")
            .field("world_id", &self.world_id)
            .field("channel_id", &self.channel_id)
            .finish_non_exhaustive()
    }
}
fn positive(value: &Value, key: &str) -> Result<u64, Failure> {
    match value.get(key) {
        Some(Value::Unsigned(n)) if *n > 0 => Ok(*n),
        _ => Err(bad()),
    }
}
/// RFC 3339 UTC in whole seconds, exactly `YYYY-MM-DDTHH:MM:SSZ`.
fn utc_seconds(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 20
        || [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ]
        .iter()
        .any(|&(i, c)| b[i] != c)
    {
        return None;
    }
    let part = |from: usize, to: usize| -> Option<i64> {
        let digits = text.get(from..to)?;
        digits
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| digits.parse().ok())
            .flatten()
    };
    let (year, month, day) = (part(0, 4)?, part(5, 7)?, part(8, 10)?);
    let (hour, minute, second) = (part(11, 13)?, part(14, 16)?, part(17, 19)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    // Days from 1970-01-01 in the proleptic Gregorian calendar.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}
impl Report {
    /// The exact §19.6.2 member set: no unknown, missing or `null` member.
    pub fn decode(value: &Value) -> Result<Self, Failure> {
        let Value::Object(members) = value else {
            return Err(bad());
        };
        if members.len() != FIELDS.len()
            || FIELDS.iter().any(|field| value.get(field).is_none())
            || !runtime::number(value, "contract_version", 1)
            || runtime::text(value, "operation")? != OPERATION
        {
            return Err(bad());
        }
        let source_authority = runtime::text(value, "source_authority")?;
        if !runtime::bounded_text(source_authority, 128, |b| {
            b.is_ascii_alphanumeric() || b"._:/-".contains(&b)
        }) {
            return Err(bad());
        }
        let id = |key: &str| runtime::uuid(runtime::text(value, key)?, true);
        let Some(Value::Bool(present)) = value.get("present") else {
            return Err(bad());
        };
        Ok(Self {
            source_authority: source_authority.to_owned(),
            assignment_epoch: positive(value, "assignment_epoch")?,
            ownership_generation: positive(value, "scope_ownership_generation")?,
            world_id: id("world_id")?,
            channel_id: id("channel_id")?,
            account_id: id("account_id")?,
            character_id: id("character_id")?,
            present: *present,
            locator_revision: positive(value, "locator_revision")?,
            committed_at: utc_seconds(runtime::text(value, "committed_at")?).ok_or_else(bad)?,
        })
    }
}

/// §19.6.3 under the epoch fence, then the per-Character ordering. Lock order:
/// epoch, assignment, read model, Character entry.
pub async fn ingest(
    pool: &PgPool,
    projection_authority: &str,
    peer: &str,
    report: &Report,
) -> Result<&'static str, Failure> {
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    let lock: Option<i16> =
        sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id = 1 FOR SHARE")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| unavailable())?;
    if lock.is_none() {
        return Err(unavailable());
    }
    let highest: Option<String> = sqlx::query_scalar("SELECT assignment_epoch::numeric::text FROM native_scope_assignments ORDER BY assignment_epoch::numeric DESC LIMIT 1").fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    let assignment = sqlx::query("SELECT assignment_epoch::text AS epoch, ownership_generation::text AS generation, node_identity FROM native_scope_assignments WHERE world_id = $1 AND channel_id = $2 FOR SHARE")
        .bind(report.world_id).bind(report.channel_id)
        .fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    // Rule 1: only the node the latest assignment names; a deposed owner is 401.
    let Some(assignment) = assignment else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let node: Option<String> = assignment
        .try_get("node_identity")
        .map_err(|_| unavailable())?;
    if node.as_deref() != Some(peer) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // Rule 2: the latest accepted assignment, in the highest epoch.
    let epoch: String = assignment.try_get("epoch").map_err(|_| unavailable())?;
    let generation: String = assignment
        .try_get("generation")
        .map_err(|_| unavailable())?;
    if highest.as_deref() != Some(epoch.as_str())
        || epoch != report.assignment_epoch.to_string()
        || generation != report.ownership_generation.to_string()
    {
        return Err(StatusCode::CONFLICT);
    }
    // Rule 5: ownership must be verifiable, and the reported account must own it.
    match admission::listing(
        &mut tx,
        projection_authority,
        report.account_id,
        report.character_id,
    )
    .await
    {
        Ok(Some((world, _))) if world == report.world_id => {}
        Ok(_) => return Err(StatusCode::CONFLICT),
        // Disabled, stale or invalid read model: refused and not stored.
        Err(_) => return Err(unavailable()),
    }
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('native-recovery-locator:' || $1::text, 0))",
    )
    .bind(report.character_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| unavailable())?;
    let entry = sqlx::query("SELECT assignment_epoch::text AS epoch, locator_revision::text AS revision, scope_ownership_generation::text AS generation, source_authority, world_id, channel_id, account_id, present, committed_at FROM native_recovery_locators WHERE character_id = $1 FOR UPDATE")
        .bind(report.character_id).fetch_optional(&mut *tx).await.map_err(|_| unavailable())?;
    if let Some(entry) = entry {
        let number = |key: &str| -> Result<u64, Failure> {
            runtime::decimal(
                entry.try_get::<&str, _>(key).map_err(|_| unavailable())?,
                true,
            )
            .map_err(|_| unavailable())
        };
        let stored = (number("epoch")?, number("revision")?);
        let key = (report.assignment_epoch, report.locator_revision);
        if key < stored {
            return Ok("superseded");
        }
        if key == stored {
            let get = |_: sqlx::Error| unavailable();
            let identical = number("generation")? == report.ownership_generation
                && entry.try_get::<&str, _>("source_authority").map_err(get)?
                    == report.source_authority
                && entry.try_get::<Uuid, _>("world_id").map_err(get)? == report.world_id
                && entry.try_get::<Uuid, _>("channel_id").map_err(get)? == report.channel_id
                && entry.try_get::<Uuid, _>("account_id").map_err(get)? == report.account_id
                && entry.try_get::<bool, _>("present").map_err(get)? == report.present
                && entry.try_get::<i64, _>("committed_at").map_err(get)? == report.committed_at;
            if identical {
                return Ok("duplicate");
            }
            // Conflicted until a higher revision arrives.
            sqlx::query("UPDATE native_recovery_locators SET conflicted = true, updated_at = clock_timestamp() WHERE character_id = $1")
                .bind(report.character_id).execute(&mut *tx).await.map_err(|_| unavailable())?;
            tx.commit().await.map_err(|_| unavailable())?;
            return Err(StatusCode::CONFLICT);
        }
    }
    sqlx::query("INSERT INTO native_recovery_locators (character_id, account_id, world_id, channel_id, present, assignment_epoch, scope_ownership_generation, locator_revision, committed_at, source_authority) VALUES ($1,$2,$3,$4,$5,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9,$10) ON CONFLICT (character_id) DO UPDATE SET account_id = EXCLUDED.account_id, world_id = EXCLUDED.world_id, channel_id = EXCLUDED.channel_id, present = EXCLUDED.present, assignment_epoch = EXCLUDED.assignment_epoch, scope_ownership_generation = EXCLUDED.scope_ownership_generation, locator_revision = EXCLUDED.locator_revision, committed_at = EXCLUDED.committed_at, source_authority = EXCLUDED.source_authority, conflicted = false, updated_at = clock_timestamp()")
        .bind(report.character_id).bind(report.account_id).bind(report.world_id).bind(report.channel_id)
        .bind(report.present).bind(report.assignment_epoch.to_string())
        .bind(report.ownership_generation.to_string()).bind(report.locator_revision.to_string())
        .bind(report.committed_at).bind(&report.source_authority)
        .execute(&mut *tx).await.map_err(|_| unavailable())?;
    tx.commit().await.map_err(|_| unavailable())?;
    Ok("accepted")
}

#[derive(Debug, Clone)]
struct Locator {
    pool: PgPool,
    settings: Settings,
    peer_maps: Vec<(Peer, Router)>,
    throttle: Arc<Throttle>,
    group_guard: Router,
}

/// Explicit configuration builder; the default deployment mount stays off.
/// `identities` is the internal identity map shared with runtime status and
/// native evidence, so a locator identity is refused on their routes and their
/// identities here (§19.6.1).
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
                group: GROUP.into(),
            }])
            .ok()?;
            Some((
                peer.clone(),
                runtime::authorization_probe(Arc::new(map), GROUP),
            ))
        })
        .collect();
    let policy = RoutePolicy {
        enabled: settings.enabled && settings.valid(),
        group: GROUP.into(),
        max_body_bytes: MAX_REPORT_BYTES,
        max_json_depth: 2,
        json: JsonRule::Object,
    };
    let state = Locator {
        pool,
        settings,
        peer_maps,
        throttle: Arc::default(),
        group_guard: runtime::authorization_probe(identities.clone(), GROUP),
    };
    guard::protect(
        Router::new()
            .route(PATH, post(handle))
            .route_layer(middleware::from_fn(runtime::http_bounds)),
        identities,
        policy,
    )
    .route_layer(middleware::from_fn_with_state(state, gate))
    .route_layer(middleware::from_fn(runtime::normalize))
    .route_layer(middleware::from_fn(runtime::no_cache))
}

async fn gate(State(state): State<Locator>, mut request: Request, next: Next) -> Response {
    if !state.settings.enabled || !state.settings.valid() {
        return guard::failure(unavailable());
    }
    let client = request.extensions().get::<VerifiedClient>();
    let status = runtime::authorize(&state.group_guard, client).await;
    if status != StatusCode::NO_CONTENT {
        return guard::failure(status);
    }
    let mut peer = None;
    for (candidate, probe) in &state.peer_maps {
        if runtime::authorize(probe, request.extensions().get::<VerifiedClient>()).await
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
    Extension(state): Extension<Locator>,
    Extension(peer): Extension<Peer>,
    Extension(value): Extension<Value>,
) -> Response {
    let result = async {
        let report = Report::decode(&value)?;
        if !peer.scopes.contains(&(report.world_id, report.channel_id)) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        // Rule 4: never later than Platform now + clock uncertainty.
        if report.committed_at
            > runtime::now()?.saturating_add(i64::from(state.settings.clock_uncertainty_seconds))
        {
            return Err(bad());
        }
        ingest(
            &state.pool,
            &state.settings.projection_authority,
            &peer.name,
            &report,
        )
        .await
    }
    .await;
    match result {
        Ok(result) => runtime::json(&runtime::object([
            ("contract_version", Value::Unsigned(1)),
            ("result", runtime::string(result)),
        ])),
        Err(status) => guard::failure(status),
    }
}

#[cfg(test)]
#[path = "locator_tests.rs"]
mod tests;
