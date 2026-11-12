//! FND-04 fresh-entry grant issuance (gateway login contract §7-§9, §17 mode
//! 34a): route selection over `game_channels` route records and fresh runtime
//! reports, the exact §8.1 header and §8.2 claims, and pure Ed25519 signing
//! through the §9 key custody. Testing and preproduction only, default off;
//! production issuance stays refused (U8).
use super::{AdmissionRequest, AuthorizedAccount, Endpoint, Error, Future};
use crate::keys::{self, KeyStorage, Keyring, PublicTrustRegistry, TrustSnapshot};
use crate::runtime::{self, EvidenceState};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_internal_api::canonical::{self, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::{
    fmt,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Default-off switch for grant issuance on top of `PORTAL_NATIVE_ADMISSIONS_ENABLED`.
pub const GRANTS_ENABLED: &str = "PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED";
/// Grant TTL in seconds, 5..=30, default 20 (contract §8.2, U4).
pub const GRANT_TTL_SECONDS: &str = "PORTAL_NATIVE_ADMISSION_GRANT_TTL_SECONDS";
/// Encrypted key custody snapshot (`keys::EncryptedFileStorage`).
pub const KEY_CUSTODY_FILE: &str = "PORTAL_NATIVE_ADMISSION_KEY_CUSTODY_FILE";
/// 32-byte owner-only wrapping key of the custody snapshot.
pub const KEY_WRAPPING_FILE: &str = "PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE";
/// Platform clock uncertainty for key retirement, 0..=5, default 1.
pub const CLOCK_UNCERTAINTY_SECONDS: &str = "PORTAL_NATIVE_ADMISSION_CLOCK_UNCERTAINTY_SECONDS";
/// Fixed trust scope purpose shared with the native evidence producer (§9.1).
pub const KEY_PURPOSE: &str = "GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE";
/// Trust-registry witness directory shared with the native evidence producer.
pub const WITNESS_DIRECTORY: &str = "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY";

pub const DEFAULT_TTL_SECONDS: u8 = 20;
const ALPN: &str = "oteryn-game/1";
const ISSUER: &str = keys::ISSUER;
const AUDIENCE: &str = "urn:oteryn:game:admission";
const PROFILE: &str = keys::PROFILE;
const TYPE: &str = "oteryn-admission+jwt";
const PURPOSE: &str = "fresh_entry";
/// Bounded read of one World's route records; v1 Worlds have few channels.
const MAX_CHANNELS: i64 = 64;
/// The §9.2 self-check result is cached for less than its 5 s bound.
const TRUST_REFRESH: Duration = Duration::from_secs(2);

/// Issuance policy. `signer` is composed by the binary only when the policy
/// permits issuance, so production never loads the private key.
#[derive(Debug, Clone)]
pub struct Issuance {
    pub enabled: bool,
    /// `OTERYN_ENV`.
    pub environment: String,
    pub ttl_seconds: u8,
    pub runtime: runtime::Settings,
    pub signer: Option<Arc<dyn GrantSigner>>,
}
impl Default for Issuance {
    fn default() -> Self {
        Self {
            enabled: false,
            environment: String::new(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
            runtime: runtime::Settings::default(),
            signer: None,
        }
    }
}
impl Issuance {
    /// A malformed or out-of-range TTL disables issuance rather than defaulting.
    pub fn configured(lookup: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            enabled: lookup(GRANTS_ENABLED).as_deref() == Some("true"),
            environment: lookup("OTERYN_ENV").unwrap_or_default(),
            ttl_seconds: lookup(GRANT_TTL_SECONDS).map_or(DEFAULT_TTL_SECONDS, |raw| {
                raw.parse::<u8>()
                    .ok()
                    .filter(|ttl| raw == ttl.to_string() && (5..=30).contains(ttl))
                    .unwrap_or(0)
            }),
            runtime: runtime::configured_settings(&lookup),
            signer: None,
        }
    }
    /// Contract §17 (N4P-3) allows issuance only in testing and preproduction.
    pub fn permitted(&self) -> bool {
        self.enabled
            && matches!(self.environment.as_str(), "testing" | "preprod")
            && (5..=30).contains(&self.ttl_seconds)
    }
    pub(super) fn signer(&self) -> Result<&dyn GrantSigner, Error> {
        match &self.signer {
            Some(signer) if self.permitted() => Ok(signer.as_ref()),
            _ => Err(Error::Unavailable),
        }
    }
}

/// The issuer's signing boundary. Implementations hold the private key; the
/// returned token is never persisted (contract §6.1).
pub trait GrantSigner: Send + Sync + fmt::Debug {
    /// Refresh the §9.2 trust self-check. Call before reading the issuer clock.
    fn prepare(&self) -> Future<'_, ()>;
    /// Sign `base64url(header) "." base64url(payload)` with the current key and
    /// return the compact token.
    fn sign(&self, payload: &[u8], expires_at: u64, now: u64) -> Result<String, Error>;
    /// Re-sign a committed signing input with the key its header names (§6.2).
    fn resign(&self, input: &str, expires_at: u64, now: u64) -> Result<String, Error>;
}

/// Key custody (`keys::Keyring`) with the public trust registry as self-check.
pub struct KeyringSigner<S> {
    ring: Mutex<Keyring<S>>,
    registry: PublicTrustRegistry,
    trust: Mutex<Option<(Instant, TrustSnapshot)>>,
    refresh: tokio::sync::Mutex<()>,
}
impl<S> fmt::Debug for KeyringSigner<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyringSigner([redacted])")
    }
}
impl<S: KeyStorage> KeyringSigner<S> {
    pub fn new(ring: Keyring<S>, registry: PublicTrustRegistry) -> Self {
        Self {
            ring: Mutex::new(ring),
            registry,
            trust: Mutex::new(None),
            refresh: tokio::sync::Mutex::new(()),
        }
    }
    fn with_trust<T>(
        &self,
        failure: Error,
        f: impl FnOnce(&mut Keyring<S>, &TrustSnapshot) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let trust = self.trust.lock().map_err(|_| failure)?;
        let (at, snapshot) = trust.as_ref().ok_or(failure)?;
        if at.elapsed() > TRUST_REFRESH * 2 {
            return Err(failure);
        }
        let mut ring = self.ring.lock().map_err(|_| failure)?;
        f(&mut ring, snapshot)
    }
}
impl<S: KeyStorage + Send + 'static> GrantSigner for KeyringSigner<S> {
    fn prepare(&self) -> Future<'_, ()> {
        Box::pin(async move {
            let _serial = self.refresh.lock().await;
            let current = self
                .trust
                .lock()
                .map_err(|_| Error::Unavailable)?
                .as_ref()
                .is_some_and(|(at, _)| at.elapsed() < TRUST_REFRESH);
            if !current {
                let snapshot = self
                    .registry
                    .snapshot()
                    .await
                    .map_err(|_| Error::Unavailable)?;
                *self.trust.lock().map_err(|_| Error::Unavailable)? =
                    Some((Instant::now(), snapshot));
            }
            Ok(())
        })
    }
    fn sign(&self, payload: &[u8], expires_at: u64, now: u64) -> Result<String, Error> {
        self.with_trust(Error::Unavailable, |ring, trust| {
            let kid = ring.active_key_id().ok_or(Error::Unavailable)?.to_owned();
            let input = signing_input(&kid, payload)?;
            let signed = ring
                .sign(input.as_bytes(), expires_at, now, trust)
                .map_err(|_| Error::Unavailable)?;
            if signed.key_id() != kid {
                return Err(Error::Unavailable);
            }
            Ok(format!(
                "{input}.{}",
                URL_SAFE_NO_PAD.encode(signed.signature())
            ))
        })
    }
    fn resign(&self, input: &str, expires_at: u64, now: u64) -> Result<String, Error> {
        let kid = header_key_id(input).ok_or(Error::ReconciliationRequired)?;
        self.with_trust(Error::ReconciliationRequired, |ring, trust| {
            let signature = ring
                .resign(&kid, input.as_bytes(), expires_at, now, trust)
                .map_err(|_| Error::ReconciliationRequired)?;
            Ok(format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature)))
        })
    }
}

/// Opens existing custody (never provisions keys) only when `issuance` permits
/// issuance; `Ok(None)` otherwise. Errors carry no path or secret.
#[cfg(unix)]
pub fn configured_signer(
    pool: PgPool,
    issuance: &Issuance,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Option<Arc<dyn GrantSigner>>, keys::KeyError> {
    if !issuance.permitted() {
        return Ok(None);
    }
    let clock = match lookup(CLOCK_UNCERTAINTY_SECONDS) {
        None => 1,
        Some(raw) => raw
            .parse::<u8>()
            .ok()
            .filter(|n| raw == n.to_string() && *n <= 5)
            .ok_or(keys::KeyError::BadInput)?,
    };
    let settings = keys::Settings {
        enabled: true,
        key_purpose: lookup(KEY_PURPOSE).unwrap_or_default(),
        clock_uncertainty_seconds: clock,
    };
    let custody = lookup(KEY_CUSTODY_FILE).ok_or(keys::KeyError::BadInput)?;
    let wrapping = lookup(KEY_WRAPPING_FILE).ok_or(keys::KeyError::BadInput)?;
    let witness = lookup(WITNESS_DIRECTORY).ok_or(keys::KeyError::BadInput)?;
    let storage = keys::EncryptedFileStorage::from_key_file(
        std::path::PathBuf::from(custody),
        std::path::Path::new(&wrapping),
    )?;
    let ring = Keyring::open(settings.clone(), storage)?;
    let registry = PublicTrustRegistry::new(pool, settings, witness.into())?;
    Ok(Some(Arc::new(KeyringSigner::new(ring, registry))))
}
#[cfg(not(unix))]
pub fn configured_signer(
    _: PgPool,
    _: &Issuance,
    _: impl Fn(&str) -> Option<String>,
) -> Result<Option<Arc<dyn GrantSigner>>, keys::KeyError> {
    Ok(None)
}

/// §8.1 header, exactly this member order and no whitespace.
pub fn header(kid: &str) -> Result<String, Error> {
    if !(1..=64).contains(&kid.len())
        || !kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(Error::Unavailable);
    }
    Ok(format!(
        r#"{{"alg":"Ed25519","kid":"{kid}","typ":"{TYPE}"}}"#
    ))
}
fn signing_input(kid: &str, payload: &[u8]) -> Result<String, Error> {
    let header = header(kid)?;
    if header.len() > 512 || payload.len() > 3072 {
        return Err(Error::Unavailable);
    }
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload)
    ))
}
/// The `kid` of a stored signing input whose header is exactly §8.1.
pub(super) fn header_key_id(input: &str) -> Option<String> {
    let (encoded, _) = input.split_once('.')?;
    let raw = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let value = canonical::decode(&raw, 512, 2).ok()?;
    let Some(Value::String(kid)) = value.get("kid") else {
        return None;
    };
    (header(kid).ok()?.as_bytes() == raw.as_slice()).then(|| kid.clone())
}
/// A claim of a stored signing input, for replay revalidation.
pub(super) fn claim(input: &str, name: &str) -> Option<String> {
    let encoded = input.split('.').nth(1)?;
    let raw = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    match canonical::decode(&raw, 3072, 2).ok()?.get(name) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

/// §7.3: `rt.<version>.<first 32 hex of SHA-256(JCS(route descriptor))>`.
pub fn route_revision(
    version: u64,
    world: Uuid,
    channel: Uuid,
    host: &str,
    port: u16,
    tls_server_name: &str,
) -> Result<String, Error> {
    // JCS of this all-ASCII descriptor: members sorted by name, no whitespace.
    let descriptor = Value::Object(vec![
        ("alpn".into(), Value::String(ALPN.into())),
        ("channel_id".into(), Value::String(channel.to_string())),
        ("host".into(), Value::String(host.into())),
        ("port".into(), Value::Unsigned(port.into())),
        ("protocol_major".into(), Value::Unsigned(1)),
        (
            "tls_server_name".into(),
            Value::String(tls_server_name.into()),
        ),
        ("transport_profile".into(), Value::Unsigned(1)),
        ("world_id".into(), Value::String(world.to_string())),
    ]);
    let bytes = canonical::encode(&descriptor).map_err(|_| Error::Unavailable)?;
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(format!("rt.{version}.{}", &digest[..32]))
}

/// The host and TLS server name a route record must carry to route anywhere
/// (§7.3): host is an IP literal or DNS name, server name a non-numeric DNS name.
pub fn route_names_valid(host: &str, tls_server_name: &str) -> bool {
    (host.parse::<std::net::IpAddr>().is_ok() || dns_name(host)) && server_name(tls_server_name)
}

fn dns_name(s: &str) -> bool {
    (1..=253).contains(&s.len())
        && s.split('.').all(|label| {
            let bytes = label.as_bytes();
            (1..=63).contains(&bytes.len())
                && bytes[0].is_ascii_alphanumeric()
                && bytes[bytes.len() - 1].is_ascii_alphanumeric()
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        })
}
fn server_name(s: &str) -> bool {
    dns_name(s)
        && !s
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_digit()))
}

#[derive(Debug, Clone)]
pub(super) struct Route {
    pub channel: Uuid,
    pub endpoint: Endpoint,
    pub revision: String,
}
/// A stored record that is incomplete, malformed or whose revision differs from
/// the one recomputed from its descriptor routes nowhere (§17 mode 34a).
fn record(world: Uuid, row: &sqlx::postgres::PgRow) -> Result<Option<Route>, Error> {
    let get = |_: sqlx::Error| Error::Unavailable;
    let channel: Uuid = row.try_get("channel_id").map_err(get)?;
    let host: Option<String> = row.try_get("native_route_host").map_err(get)?;
    let port: Option<i32> = row.try_get("native_route_port").map_err(get)?;
    let tls: Option<String> = row.try_get("native_route_tls_server_name").map_err(get)?;
    let version: Option<i64> = row.try_get("native_route_version").map_err(get)?;
    let stored: Option<String> = row.try_get("native_route_revision").map_err(get)?;
    let (Some(host), Some(port), Some(tls), Some(version), Some(stored)) =
        (host, port, tls, version, stored)
    else {
        return Ok(None);
    };
    let (Ok(port), Ok(version)) = (u16::try_from(port), u64::try_from(version)) else {
        return Ok(None);
    };
    if port == 0
        || version == 0
        || !route_names_valid(&host, &tls)
        || super::wire::uuid(&channel.to_string()).is_err()
        || route_revision(version, world, channel, &host, port, &tls)? != stored
    {
        return Ok(None);
    }
    Ok(Some(Route {
        channel,
        endpoint: Endpoint {
            host,
            port,
            tls_server_name: tls,
            alpn: ALPN.into(),
            protocol_major: 1,
            transport_profile: 1,
        },
        revision: stored,
    }))
}

/// §7.4 candidate set and selection. The requested channel is filtered before
/// the bound; only the selected route record is share-locked, re-read and kept
/// locked until commit so it cannot change under the grant.
pub(super) async fn select(
    db: &mut PgConnection,
    runtime: &runtime::Settings,
    world: Uuid,
    requested: Option<Uuid>,
    now: i64,
) -> Result<(Route, Value), Error> {
    let rows = sqlx::query("SELECT channel_id, native_route_host, native_route_port, native_route_tls_server_name, native_route_version, native_route_revision FROM game_channels WHERE world_id=$1 AND native_login_enabled AND native_route_revision IS NOT NULL AND ($2::uuid IS NULL OR channel_id=$2) ORDER BY channel_id LIMIT $3")
        .bind(world).bind(requested).bind(MAX_CHANNELS)
        .fetch_all(&mut *db).await.map_err(|_| Error::Unavailable)?;
    for row in &rows {
        let Some(route) = record(world, row)? else {
            continue;
        };
        let observation = runtime::read_in(db, runtime, world, route.channel, now)
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(report) = observation.content else {
            continue;
        };
        if observation.state == EvidenceState::Fresh
            && matches!(report.get("ready"), Some(Value::Bool(true)))
            && matches!(report.get("route_revision"), Some(Value::String(r)) if *r == route.revision)
            && matches!(report.get("protocol_major"), Some(Value::Unsigned(1)))
            && matches!(report.get("transport_profile"), Some(Value::Unsigned(1)))
        {
            match current(db, world, route.channel, &route.endpoint, &route.revision).await {
                Ok(()) => return Ok((route, report)),
                Err(Error::RouteUnavailable) => continue,
                Err(error) => return Err(error),
            }
        }
    }
    Err(Error::RouteUnavailable)
}
/// Replay revalidation (§17): the bound record must still be the one in the
/// grant, with native login still enabled.
pub(super) async fn current(
    db: &mut PgConnection,
    world: Uuid,
    channel: Uuid,
    endpoint: &Endpoint,
    revision: &str,
) -> Result<(), Error> {
    let row = sqlx::query("SELECT channel_id, native_route_host, native_route_port, native_route_tls_server_name, native_route_version, native_route_revision FROM game_channels WHERE world_id=$1 AND channel_id=$2 AND native_login_enabled FOR SHARE")
    .bind(world)
    .bind(channel)
    .fetch_optional(db)
    .await
    .map_err(|_| Error::Unavailable)?;
    match row.map(|row| record(world, &row)).transpose()?.flatten() {
        Some(route) if route.revision == revision && route.endpoint == *endpoint => Ok(()),
        _ => Err(Error::RouteUnavailable),
    }
}

/// Everything §8.2 binds, gathered after every §4-§7 check passed.
pub(super) struct Claims<'a> {
    pub iat: u64,
    pub ttl: u8,
    pub jti: String,
    pub request: &'a AdmissionRequest,
    pub account: AuthorizedAccount,
    pub world: Uuid,
    pub route: &'a Route,
    pub report: &'a Value,
}
/// FND-04 revision grammar as enforced at runtime-status ingestion; the issuer
/// re-checks before signing and fails closed.
fn revision(report: &Value, name: &str) -> Result<Value, Error> {
    match report.get(name) {
        Some(Value::String(s))
            if (1..=64).contains(&s.len())
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)) =>
        {
            Ok(Value::String(s.clone()))
        }
        _ => Err(Error::Unavailable),
    }
}
/// §8.2 claims, exactly in this order and without whitespace.
pub(super) fn payload(claims: &Claims<'_>) -> Result<Vec<u8>, Error> {
    let generation = match claims.report.get("scope_ownership_generation") {
        Some(Value::String(s))
            if s.parse::<u64>()
                .is_ok_and(|n| n != 0 && n.to_string() == *s) =>
        {
            s.clone()
        }
        _ => return Err(Error::Unavailable),
    };
    if claims.account.security_generation == 0
        || claims.jti.len() != 43
        || claims.route.revision.len() > 64
        || !(5..=30).contains(&claims.ttl)
    {
        return Err(Error::Unavailable);
    }
    let text = |s: &str| Value::String(s.into());
    let id = |id: Uuid| Value::String(id.to_string());
    let mut members = vec![
        ("iss", text(ISSUER)),
        ("aud", text(AUDIENCE)),
        ("iat", Value::Unsigned(claims.iat)),
        ("nbf", Value::Unsigned(claims.iat)),
        (
            "exp",
            Value::Unsigned(
                claims
                    .iat
                    .checked_add(claims.ttl.into())
                    .ok_or(Error::Unavailable)?,
            ),
        ),
        ("jti", text(&claims.jti)),
        ("profile", text(PROFILE)),
        ("purpose", text(PURPOSE)),
        ("attempt_ref", id(claims.request.attempt_ref)),
        ("account_id", id(claims.account.id)),
        ("character_id", id(claims.request.character_id)),
        ("world_id", id(claims.world)),
        ("channel_id", id(claims.route.channel)),
        (
            "account_security_generation",
            Value::String(claims.account.security_generation.to_string()),
        ),
        ("route_revision", text(&claims.route.revision)),
        (
            "runtime_observation_revision",
            revision(claims.report, "runtime_observation_revision")?,
        ),
        ("scope_ownership_generation", Value::String(generation)),
        ("protocol_major", Value::Unsigned(1)),
        ("transport_profile", Value::Unsigned(1)),
    ];
    for name in [
        "ruleset_revision",
        "content_revision",
        "map_revision",
        "world_policy_revision",
        "offer_revision",
    ] {
        members.push((name, revision(claims.report, name)?));
    }
    let bytes = canonical::encode(&Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    ))
    .map_err(|_| Error::Unavailable)?;
    if bytes.len() > 3072 {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}
/// 32 CSPRNG bytes, base64url without padding (43 characters).
pub(super) fn jti() -> Result<String, Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
#[path = "grant_tests.rs"]
mod tests;
