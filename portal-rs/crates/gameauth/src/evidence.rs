//! Platform-native evidence producer. Requests are observations, not evidence uploads.
//!
//! Every successful read issues a new immutable revision shared by fresh/recovery
//! operations. A retained volume outside the database restore unit is mandatory.
//! Source owners provide accounts(id UUIDv7, status, native_security_generation),
//! and native_game_signing_trust_profiles / native_game_signing_trust_key_versions
//! with UUIDv7 profile ids and immutable positive profile/key revisions.
//! Account and signing-trust mutations must advance their retained state floor
//! before publishing a new generation/revision. Missing source schema fails closed.
use crate::runtime::{
    self, Failure, Throttle, bad, bounded_text, decimal, json, now, object, string, text,
    unavailable, uuid,
};
use axum::{
    Extension, Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Semaphore;

const FRESH_ACCOUNT: &str = "ReadAccountSecurityV1";
const RECOVERY_ACCOUNT: &str = "ReadRecoveryAccountSecurityV2";
const FRESH_TRUST: &str = "ReadFreshSigningTrustV1";
const RECOVERY_TRUST: &str = "ReadRecoverySigningTrustV2";

#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    /// The evidence contract configures exactly one peer for this purpose.
    pub peer_identity: Option<Identity>,
    /// Activation is a one-way deployment decision, distinct from route enablement.
    pub activated: bool,
    pub witness_directory: PathBuf,
    pub source_authority: String,
    pub fresh_account_purpose: String,
    pub fresh_account_scope: String,
    pub fresh_key_purpose: String,
    pub clock_uncertainty_seconds: u8,
    pub requests_per_minute: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            peer_identity: None,
            activated: false,
            witness_directory: PathBuf::new(),
            source_authority: String::new(),
            fresh_account_purpose: String::new(),
            fresh_account_scope: String::new(),
            fresh_key_purpose: String::new(),
            clock_uncertainty_seconds: 0,
            requests_per_minute: 120,
        }
    }
}
impl Settings {
    fn valid(&self) -> bool {
        self.clock_uncertainty_seconds <= 5
            && !self.witness_directory.as_os_str().is_empty()
            && bounded_text(&self.source_authority, 128, |b| {
                b.is_ascii_alphanumeric() || b"._:/-".contains(&b)
            })
            && [
                &self.fresh_account_purpose,
                &self.fresh_account_scope,
                &self.fresh_key_purpose,
            ]
            .iter()
            .all(|s| binding(s))
    }
}
fn binding(s: &str) -> bool {
    bounded_text(s, 256, |b| (0x20..=0x7e).contains(&b))
}
fn hash(s: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(s.as_ref()))
}

#[derive(Debug, Clone)]
pub struct EvidenceRequest {
    version: u64,
    operation: String,
    value: Value,
    namespace: String,
    state_namespace: String,
}
impl EvidenceRequest {
    pub fn decode(value: &Value) -> Result<Self, Failure> {
        let operation = text(value, "operation")?;
        let (version, fields) = match operation {
            FRESH_ACCOUNT => (
                1,
                vec!["version", "operation", "account_id", "purpose", "scope"],
            ),
            RECOVERY_ACCOUNT => (
                2,
                vec!["version", "operation", "account_id", "purpose", "scope"],
            ),
            FRESH_TRUST => (
                1,
                vec![
                    "version",
                    "operation",
                    "issuer",
                    "profile",
                    "key_purpose",
                    "key_id",
                ],
            ),
            RECOVERY_TRUST => (
                2,
                vec![
                    "version",
                    "operation",
                    "issuer",
                    "profile",
                    "key_purpose",
                    "key_id",
                ],
            ),
            _ => return Err(bad()),
        };
        let Value::Object(members) = value else {
            return Err(bad());
        };
        if members.len() != fields.len() || !runtime::number(value, "version", version) {
            return Err(bad());
        }
        for field in fields.into_iter().filter(|f| *f != "version") {
            if !binding(text(value, field)?) {
                return Err(bad());
            }
        }
        let (namespace, state_namespace) = if matches!(operation, FRESH_ACCOUNT | RECOVERY_ACCOUNT)
        {
            let id = text(value, "account_id")?;
            uuid(id, true)?;
            (
                hash(format!("account-security\0{id}")),
                hash(format!("account-security-generation\0{id}")),
            )
        } else {
            if !bounded_text(text(value, "key_id")?, 64, |b| {
                b.is_ascii_alphanumeric() || b"._-".contains(&b)
            }) {
                return Err(bad());
            }
            let suffix = format!(
                "{}\0{}\0{}",
                text(value, "issuer")?,
                text(value, "profile")?,
                text(value, "key_purpose")?
            );
            (
                hash(format!("signing-trust\0{suffix}")),
                hash(format!("signing-trust-state\0{suffix}")),
            )
        };
        Ok(Self {
            version,
            operation: operation.to_owned(),
            value: value.clone(),
            namespace,
            state_namespace,
        })
    }
    fn failure(&self, result: &str) -> Value {
        object([
            ("version", Value::Unsigned(self.version)),
            ("operation", string(&self.operation)),
            ("result", string(result)),
        ])
    }
    fn supported(&self, settings: &Settings) -> Result<bool, Failure> {
        let v = &self.value;
        Ok(match self.operation.as_str() {
            FRESH_ACCOUNT => {
                text(v, "purpose")? == settings.fresh_account_purpose
                    && text(v, "scope")? == settings.fresh_account_scope
            }
            RECOVERY_ACCOUNT => {
                text(v, "purpose")? == "platform_security"
                    && text(v, "scope")? == "existing_actor_recovery"
            }
            FRESH_TRUST => {
                text(v, "issuer")? == "urn:oteryn:platform:game-admission"
                    && text(v, "profile")? == "oteryn-pre-admission-v1"
                    && text(v, "key_purpose")? == settings.fresh_key_purpose
            }
            RECOVERY_TRUST => {
                text(v, "issuer")? == "urn:oteryn:platform:game-recovery"
                    && text(v, "profile")? == "oteryn-reauth-recovery-v1"
                    && text(v, "key_purpose")? == "existing_actor_recovery"
            }
            _ => false,
        })
    }
}

/// Filesystem lock remains held until drop, including across the database commit.
/// The retained directory is trusted operator storage, never a request parameter.
#[derive(Debug)]
pub struct Witness {
    directory: PathBuf,
    namespace: String,
    _lock: File,
}
impl Witness {
    pub fn lock(directory: &Path, namespace: &str) -> Result<Self, Failure> {
        if namespace.len() != 64
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(unavailable());
        }
        let directory = directory.canonicalize().map_err(|_| unavailable())?;
        if !directory.is_dir() {
            return Err(unavailable());
        }
        let lock_path = directory.join(format!("{namespace}.lock"));
        reject_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| unavailable())?;
        lock.try_lock().map_err(|_| unavailable())?;
        Ok(Self {
            directory,
            namespace: namespace.to_owned(),
            _lock: lock,
        })
    }
    fn path(&self, namespace: &str) -> PathBuf {
        self.directory.join(format!("{namespace}.floor"))
    }
    pub fn floor(&self, namespace: &str) -> Result<Option<i64>, Failure> {
        valid_namespace(namespace)?;
        if namespace != self.namespace {
            return Err(unavailable());
        }
        read_small(&self.path(namespace))?
            .map(|s| {
                let n = decimal(s.trim(), true).map_err(|_| unavailable())?;
                n.try_into().map_err(|_| unavailable())
            })
            .transpose()
    }
    /// Domain writers call while holding this namespace lock, before committing state.
    pub fn advance(&self, namespace: &str, revision: i64) -> Result<(), Failure> {
        valid_namespace(namespace)?;
        if namespace != self.namespace {
            return Err(unavailable());
        }
        if revision <= 0 || self.floor(namespace)?.is_some_and(|n| n > revision) {
            return Err(unavailable());
        }
        durable_write(&self.path(namespace), format!("{revision}\n").as_bytes())
    }
    async fn provenance(&self, pool: &PgPool) -> Result<(), Failure> {
        let store_path = self.directory.join("witness-store.id");
        let lock_path = self.directory.join("witness-store.lock");
        reject_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| unavailable())?;
        lock.try_lock().map_err(|_| unavailable())?;
        let marker = read_small(&store_path)?.map(|s| s.trim().to_owned());
        if marker.as_ref().is_some_and(|s| valid_namespace(s).is_err()) {
            return Err(unavailable());
        }
        let db: Option<String> =
            sqlx::query_scalar("SELECT store_id FROM native_evidence_witness_stores WHERE id=1")
                .fetch_optional(pool)
                .await
                .map_err(|_| unavailable())?;
        match (marker, db) {
            (Some(marker), Some(db)) if marker == db => Ok(()),
            (_, Some(_)) => Err(unavailable()),
            (marker, None) => {
                let id = match marker {
                    Some(marker) => {
                        durable_write(&store_path, format!("{marker}\n").as_bytes())?;
                        marker
                    }
                    None => {
                        let history: bool =
                            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_evidence)")
                                .fetch_one(pool)
                                .await
                                .map_err(|_| unavailable())?;
                        if history {
                            return Err(unavailable());
                        }
                        let mut bytes = [0; 32];
                        getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
                        let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                        durable_write(&store_path, format!("{id}\n").as_bytes())?;
                        id
                    }
                };
                sqlx::query(
                    "INSERT INTO native_evidence_witness_stores (id,store_id) VALUES (1,$1)",
                )
                .bind(id)
                .execute(pool)
                .await
                .map_err(|_| unavailable())?;
                Ok(())
            }
        }
    }
}
fn valid_namespace(s: &str) -> Result<(), Failure> {
    if s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(unavailable())
    }
}
fn reject_symlink(path: &Path) -> Result<(), Failure> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => Err(unavailable()),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(unavailable()),
    }
}
fn read_small(path: &Path) -> Result<Option<String>, Failure> {
    reject_symlink(path)?;
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unavailable()),
    };
    let mut value = String::new();
    file.take(129)
        .read_to_string(&mut value)
        .map_err(|_| unavailable())?;
    if value.len() > 128 {
        return Err(unavailable());
    }
    Ok(Some(value))
}
fn durable_write(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
    reject_symlink(path)?;
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| unavailable())?;
    let temporary = path.with_extension(format!("tmp.{}", hash(random)));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| unavailable())?;
        file.write_all(bytes).map_err(|_| unavailable())?;
        file.sync_all().map_err(|_| unavailable())?;
        fs::rename(&temporary, path).map_err(|_| unavailable())?;
        File::open(path.parent().ok_or_else(unavailable)?)
            .map_err(|_| unavailable())?
            .sync_all()
            .map_err(|_| unavailable())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

/// Namespace helpers let account/trust writers advance the same retained state.
pub fn account_state_namespace(account: &str) -> Result<String, Failure> {
    uuid(account, true)?;
    Ok(hash(format!("account-security-generation\0{account}")))
}
pub fn trust_state_namespace(
    issuer: &str,
    profile: &str,
    purpose: &str,
) -> Result<String, Failure> {
    if ![issuer, profile, purpose].into_iter().all(binding) {
        return Err(bad());
    }
    Ok(hash(format!(
        "signing-trust-state\0{issuer}\0{profile}\0{purpose}"
    )))
}

/// Reads canonical account security truth under its owning row lock. A missing
/// account schema, unknown status or absent native generation is unavailable.
/// Trust comes only from the current immutable public trust profile/key versions;
/// OAuth keys and admission private signing material are never consulted.
async fn source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &EvidenceRequest,
    witness: &Witness,
    history: i64,
) -> Result<Option<Vec<(&'static str, Value)>>, Failure> {
    let value = &request.value;
    if matches!(request.operation.as_str(), FRESH_ACCOUNT | RECOVERY_ACCOUNT) {
        let row = sqlx::query(
            "SELECT status, native_security_generation FROM accounts WHERE id=$1 FOR UPDATE",
        )
        .bind(uuid(text(value, "account_id")?, true)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| unavailable())?;
        let Some(row) = row else {
            if history > 0 || witness.floor(&request.state_namespace)?.is_some() {
                return Err(unavailable());
            }
            return Ok(None);
        };
        let generation: i64 = row
            .try_get("native_security_generation")
            .map_err(|_| unavailable())?;
        if generation <= 0 {
            return Err(unavailable());
        }
        match witness.floor(&request.state_namespace)? {
            Some(floor) if floor == generation => {}
            None if history == 0 => witness.advance(&request.state_namespace, generation)?,
            _ => return Err(unavailable()),
        }
        let status: String = row.try_get("status").map_err(|_| unavailable())?;
        let allowed = match status.as_str() {
            "active" => true,
            "disabled" | "suspended" | "terminated" => false,
            _ => return Err(unavailable()),
        };
        Ok(Some(vec![
            ("account_id", string(text(value, "account_id")?)),
            ("purpose", string(text(value, "purpose")?)),
            ("scope", string(text(value, "scope")?)),
            ("allowed", Value::Bool(allowed)),
            ("minimum_valid_generation", string(generation)),
        ]))
    } else {
        let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE").bind(text(value,"issuer")?).bind(text(value,"profile")?).bind(text(value,"key_purpose")?).fetch_optional(&mut **tx).await.map_err(|_| unavailable())?;
        let Some(profile) = profile else {
            if history > 0 || witness.floor(&request.state_namespace)?.is_some() {
                return Err(unavailable());
            }
            return Ok(None);
        };
        let revision: i64 = profile
            .try_get("issuer_revision")
            .map_err(|_| unavailable())?;
        if revision <= 0 || witness.floor(&request.state_namespace)? != Some(revision) {
            return Err(unavailable());
        }
        let id: sqlx::types::Uuid = profile.try_get("id").map_err(|_| unavailable())?;
        let row = sqlx::query("SELECT public_key, trusted, key_revision FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$2 ORDER BY key_revision DESC LIMIT 1 FOR UPDATE").bind(id).bind(text(value,"key_id")?).fetch_optional(&mut **tx).await.map_err(|_| unavailable())?;
        let Some(row) = row else { return Ok(None) };
        let key: String = row.try_get("public_key").map_err(|_| unavailable())?;
        let decoded = URL_SAFE_NO_PAD.decode(&key).map_err(|_| unavailable())?;
        if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(&decoded) != key {
            return Err(unavailable());
        }
        let key_revision: i64 = row.try_get("key_revision").map_err(|_| unavailable())?;
        if key_revision <= 0 {
            return Err(unavailable());
        }
        let revoked: bool = profile.try_get("revoked").map_err(|_| unavailable())?;
        let trusted: bool = row.try_get("trusted").map_err(|_| unavailable())?;
        Ok(Some(vec![
            ("issuer", string(text(value, "issuer")?)),
            ("profile", string(text(value, "profile")?)),
            ("key_purpose", string(text(value, "key_purpose")?)),
            ("key_id", string(text(value, "key_id")?)),
            ("trusted", Value::Bool(!revoked && trusted)),
            ("public_key", string(key)),
        ]))
    }
}

/// Returns exact response bytes only after both relational and retained durability.
/// A database failure after decode becomes the bounded wire result `unavailable`.
pub async fn observe(pool: &PgPool, settings: &Settings, request: &EvidenceRequest) -> Value {
    match observe_inner(pool, settings, request).await {
        Ok(value) => value,
        Err(_) => request.failure("unavailable"),
    }
}
async fn observe_inner(
    pool: &PgPool,
    settings: &Settings,
    request: &EvidenceRequest,
) -> Result<Value, Failure> {
    if !settings.activated || !settings.valid() {
        return Err(unavailable());
    }
    if !request.supported(settings)? {
        return Ok(request.failure("unsupported"));
    }
    let witness = Witness::lock(&settings.witness_directory, &request.namespace)?;
    witness.provenance(pool).await?;
    let state = Witness::lock(&settings.witness_directory, &request.state_namespace)?;
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    let high: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(source_revision),0)::bigint FROM native_evidence WHERE namespace_hash=$1").bind(&request.namespace).fetch_one(&mut *tx).await.map_err(|_| unavailable())?;
    match witness.floor(&request.namespace)? {
        None if high == 0 => {}
        Some(floor) if floor <= high => {
            if floor < high {
                witness.advance(&request.namespace, high)?;
            }
        }
        _ => return Err(unavailable()),
    }
    let Some(facts) = source(&mut tx, request, &state, high).await? else {
        return Ok(request.failure("not_found"));
    };
    let next = high.checked_add(1).ok_or_else(unavailable)?;
    let mut fields = vec![
        ("version", Value::Unsigned(request.version)),
        ("operation", string(&request.operation)),
        ("result", string("observed")),
        ("source_authority", string(&settings.source_authority)),
        ("source_revision", string(next)),
        ("decision_identity", string(next)),
        ("source_observed_at", string(now()?)),
        (
            "clock_uncertainty_seconds",
            string(settings.clock_uncertainty_seconds),
        ),
    ];
    fields.extend(facts);
    let response = object(fields);
    let bytes = canonical::encode(&response).map_err(|_| unavailable())?;
    if bytes.len() > 8192 {
        return Err(unavailable());
    }
    let wire = String::from_utf8(bytes).map_err(|_| unavailable())?;
    sqlx::query("INSERT INTO native_evidence(namespace_hash,source_revision,response_json) VALUES ($1,$2,$3)").bind(&request.namespace).bind(next).bind(wire).execute(&mut *tx).await.map_err(|_| unavailable())?;
    tx.commit().await.map_err(|_| unavailable())?;
    witness.advance(&request.namespace, next)?;
    Ok(response)
}

#[derive(Debug, Clone)]
struct Evidence {
    pool: PgPool,
    settings: Settings,
    throttle: Arc<Throttle>,
    capacity: Arc<Semaphore>,
    peer_guard: Router,
    group_guard: Router,
}
pub fn router<S: Clone + Send + Sync + 'static>(
    pool: PgPool,
    settings: Settings,
    identities: Arc<IdentityMap>,
) -> Router<S> {
    let enabled = settings.enabled;
    let single_peer = settings
        .peer_identity
        .clone()
        .and_then(|identity| {
            IdentityMap::new(vec![Binding {
                identity,
                group: "native-evidence".into(),
            }])
            .ok()
        })
        .unwrap_or_default();
    let policy = RoutePolicy {
        enabled,
        group: "native-evidence".into(),
        max_body_bytes: 1024,
        max_json_depth: 2,
        json: JsonRule::Object,
    };
    let state = Evidence {
        pool,
        settings,
        throttle: Arc::default(),
        capacity: Arc::new(Semaphore::new(2)),
        peer_guard: runtime::authorization_probe(Arc::new(single_peer), "native-evidence"),
        group_guard: runtime::authorization_probe(identities.clone(), "native-evidence"),
    };
    guard::protect(
        Router::new()
            .route("/internal/v1/game-auth/native-evidence", post(handle))
            .route_layer(middleware::from_fn(runtime::http_bounds)),
        identities,
        policy,
    )
    .route_layer(middleware::from_fn_with_state(state, gate))
    .route_layer(middleware::from_fn(runtime::normalize))
    .route_layer(middleware::from_fn(runtime::no_cache))
}

async fn gate(State(state): State<Evidence>, mut request: Request, next: Next) -> Response {
    if !state.settings.enabled || !(1..=600).contains(&state.settings.requests_per_minute) {
        return guard::failure(unavailable());
    }
    for probe in [&state.group_guard, &state.peer_guard] {
        let status =
            runtime::authorize(probe, request.extensions().get::<guard::VerifiedClient>()).await;
        if status != axum::http::StatusCode::NO_CONTENT {
            return guard::failure(status);
        }
    }
    // One evidence peer identity per purpose, enforced by configuration. Every
    // authenticated operation shares this peer's window, including refused input.
    if let Err(response) = state
        .throttle
        .admit("evidence", state.settings.requests_per_minute)
    {
        return *response;
    }
    request.extensions_mut().insert(state);
    next.run(request).await
}
async fn handle(
    Extension(state): Extension<Evidence>,
    Extension(value): Extension<Value>,
) -> Response {
    let request = match EvidenceRequest::decode(&value) {
        Ok(request) => request,
        Err(status) => return guard::failure(status),
    };
    let Ok(_permit) = state.capacity.clone().try_acquire_owned() else {
        return json(&request.failure("unavailable"));
    };
    json(&observe(&state.pool, &state.settings, &request).await)
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
