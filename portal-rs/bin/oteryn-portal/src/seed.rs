//! Test-only seed commands for the cross-repository E2E harness
//! (`portal-rs/e2e/harness.mjs`): `oteryn-portal seed <entity> '<json object>'`.
//!
//! Each command is an idempotent ensure: running it again with the same input
//! changes nothing and prints the same single JSON object on stdout. Different
//! input for an existing key is a conflict, never a silent rewrite. The role
//! refuses unless `OTERYN_ENV=testing`, and it never seeds through the browser.
//!
//! | entity | table | output |
//! |---|---|---|
//! | `account` | `accounts` | `accountId`, `email`, `status`, `nativeSecurityGeneration` |
//! | `world` | `world_status` | `worldId`, `name`, `region` (+ `notStored`) |
//! | `oauth-native-client` | `oauth_native_clients` | `clientId` and its four flags |
//! | `signing-trust` | `native_game_signing_trust_*` + retained witness | `issuer`, `profile`, `purpose`, `keyId`, `publicKey`, `issuerRevision` |
//! | `bootstrap-intent` | `character_bootstrap_intents` | `operationId`, `issuerDecisionId`, `sourceRevision`, `issuedAtSource`, `expiresAtSource`, `intentSha256` |
//!
//! `channel` and `route` need the T19c `game_channels` registry (gateway
//! contract §7.3, mode 34a), which is not on this tree; they answer `pending`.
//! `native-evidence` answers `not_applicable`: portal-rs observes evidence live
//! and its first observation bootstraps the retained witness, so no reconcile
//! step exists.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_gameauth::{
    evidence::{Witness, trust_state_namespace},
    keys::{self, PublicTrustRegistry},
};
use oteryn_internal_api::canonical::{self, Value as Wire};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{fmt, path::PathBuf, process::ExitCode};
use uuid::Uuid;

/// Witness directory shared with the native evidence producer.
pub const WITNESS_DIRECTORY: &str = "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY";
/// Fixed client id used when `oauth-native-client` gets none.
pub const DEFAULT_NATIVE_CLIENT: &str = "019a0000-0000-7000-8000-0000000c11e7";
const RECOVERY_ISSUER: &str = "urn:oteryn:platform:game-recovery";
const RECOVERY_PROFILE: &str = "oteryn-reauth-recovery-v1";
const RECOVERY_PURPOSE: &str = "existing_actor_recovery";
const PENDING_T19C: &str = "needs the T19c game_channels registry and route record (gateway contract §7.3, mode 34a), which is not on this tree";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedError {
    BadInput(String),
    Conflict(String),
    Pending(&'static str),
    NotApplicable(&'static str),
    Unavailable(&'static str),
}
impl fmt::Display for SeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, reason) = self.parts();
        write!(f, "{kind}: {reason}")
    }
}
impl std::error::Error for SeedError {}
impl SeedError {
    fn parts(&self) -> (&'static str, &str) {
        match self {
            Self::BadInput(r) => ("bad_input", r),
            Self::Conflict(r) => ("conflict", r),
            Self::Pending(r) => ("pending", r),
            Self::NotApplicable(r) => ("not_applicable", r),
            Self::Unavailable(r) => ("unavailable", r),
        }
    }
    fn exit(&self) -> ExitCode {
        match self {
            Self::BadInput(_) | Self::Conflict(_) => ExitCode::from(2),
            Self::Pending(_) | Self::NotApplicable(_) => ExitCode::from(3),
            Self::Unavailable(_) => ExitCode::FAILURE,
        }
    }
}
impl From<sqlx::Error> for SeedError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable("database")
    }
}
fn bad(reason: impl Into<String>) -> SeedError {
    SeedError::BadInput(reason.into())
}

/// Settings read from the environment once per invocation.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub witness_directory: Option<PathBuf>,
}

/// Entry point for `oteryn-portal seed`; prints exactly one JSON object.
pub async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let result = run(&args).await;
    let (body, code) = match result {
        Ok(value) => (value, ExitCode::SUCCESS),
        Err(error) => {
            let (kind, reason) = error.parts();
            (json!({ "error": kind, "reason": reason }), error.exit())
        }
    };
    println!("{body}");
    code
}

async fn run(args: &[String]) -> Result<Value, SeedError> {
    if std::env::var("OTERYN_ENV").as_deref() != Ok("testing") {
        return Err(SeedError::Unavailable(
            "seed runs only with OTERYN_ENV=testing",
        ));
    }
    let entity = args
        .first()
        .ok_or_else(|| bad("usage: oteryn-portal seed <entity> '<json object>'"))?;
    let input: Value = match args.get(1) {
        Some(raw) => serde_json::from_str(raw).map_err(|_| bad("arguments are not JSON"))?,
        None => json!({}),
    };
    if args.len() > 2 {
        return Err(bad("expected one JSON object argument"));
    }
    let config = oteryn_core::Config::from_env()
        .map_err(|_| SeedError::Unavailable("OTERYN_DATABASE_URL is not configured"))?;
    let pool = oteryn_db::lazy_pool(config.database_url())
        .map_err(|_| SeedError::Unavailable("database"))?;
    let settings = Settings {
        witness_directory: std::env::var_os(WITNESS_DIRECTORY).map(PathBuf::from),
    };
    let result = execute(&pool, &settings, entity, &input).await;
    pool.close().await;
    result
}

/// Dispatch one seed command.
pub async fn execute(
    pool: &PgPool,
    settings: &Settings,
    entity: &str,
    input: &Value,
) -> Result<Value, SeedError> {
    let args = Args::new(input)?;
    match entity {
        "account" => account(pool, &args).await,
        "world" => world(pool, &args).await,
        "oauth-native-client" => oauth_native_client(pool, &args).await,
        "signing-trust" => signing_trust(pool, settings, &args).await,
        "bootstrap-intent" => bootstrap_intent(pool, &args).await,
        "channel" | "route" => Err(SeedError::Pending(PENDING_T19C)),
        "native-evidence" => Err(SeedError::NotApplicable(
            "evidence is observed live; the first observation bootstraps the retained witness",
        )),
        _ => Err(bad(format!("unknown entity {entity:?}"))),
    }
}

/// A JSON object whose keys are checked against each command's allow-list.
struct Args<'a>(&'a Map<String, Value>);
impl<'a> Args<'a> {
    fn new(input: &'a Value) -> Result<Self, SeedError> {
        input
            .as_object()
            .map(Self)
            .ok_or_else(|| bad("arguments must be a JSON object"))
    }
    fn only(&self, allowed: &[&str]) -> Result<(), SeedError> {
        match self.0.keys().find(|key| !allowed.contains(&key.as_str())) {
            Some(key) => Err(bad(format!("unknown argument {key:?}"))),
            None => Ok(()),
        }
    }
    fn text(&self, key: &str) -> Result<Option<&'a str>, SeedError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value)),
            Some(_) => Err(bad(format!("{key} must be a string"))),
        }
    }
    fn required(&self, key: &str) -> Result<&'a str, SeedError> {
        self.text(key)?
            .ok_or_else(|| bad(format!("{key} is required")))
    }
    fn uuid(&self, key: &str, v7: bool) -> Result<Option<Uuid>, SeedError> {
        self.text(key)?
            .map(|raw| {
                let id = Uuid::parse_str(raw).map_err(|_| bad(format!("{key} is not a UUID")))?;
                if id.to_string() != raw
                    || id.get_variant() != uuid::Variant::RFC4122
                    || (v7 && id.get_version_num() != 7)
                    || !(1..=8).contains(&id.get_version_num())
                {
                    return Err(bad(format!(
                        "{key} must be a lower-case RFC 4122 UUID{}",
                        if v7 { "v7" } else { "" }
                    )));
                }
                Ok(id)
            })
            .transpose()
    }
}

fn bounded(key: &str, value: &str, max: usize) -> Result<(), SeedError> {
    if value.trim().is_empty() || value.trim() != value || value.chars().count() > max {
        return Err(bad(format!("{key} must be 1..={max} characters, trimmed")));
    }
    Ok(())
}

async fn account(pool: &PgPool, args: &Args<'_>) -> Result<Value, SeedError> {
    args.only(&["email", "password", "accountId", "locale"])?;
    let email = args.required("email")?.to_ascii_lowercase();
    let local = email
        .strip_suffix("@example.invalid")
        .ok_or_else(|| bad("email must end with @example.invalid"))?;
    if local.is_empty()
        || email.len() > 254
        || !local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
    {
        return Err(bad("email local part must be [a-z0-9._+-]"));
    }
    let password = args.required("password")?;
    if !oteryn_identity::password::valid(password) {
        return Err(bad("password must be 10..=256 bytes"));
    }
    let locale = args.text("locale")?.unwrap_or("en");
    if !matches!(locale, "en" | "pl") {
        return Err(bad("locale must be en or pl"));
    }
    let requested = args.uuid("accountId", true)?;
    let mut tx = pool.begin().await?;
    let existing =
        sqlx::query("SELECT id, password_hash FROM accounts WHERE lower(email)=$1 FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
    let id: Uuid = if let Some(row) = existing {
        let id: Uuid = row.try_get("id")?;
        if requested.is_some_and(|wanted| wanted != id) {
            return Err(SeedError::Conflict(
                "email belongs to another accountId".into(),
            ));
        }
        let hash: String = row.try_get("password_hash")?;
        if !oteryn_identity::password::verify(password, &hash) {
            return Err(SeedError::Conflict(
                "account exists with a different password".into(),
            ));
        }
        id
    } else {
        let id = requested.unwrap_or_else(Uuid::now_v7);
        let hash = oteryn_identity::password::hash(password)
            .map_err(|_| SeedError::Unavailable("password hashing"))?;
        let inserted = sqlx::query("INSERT INTO accounts (id, email, email_verified_at, password_hash, status, locale, native_security_generation) VALUES ($1, $2, now(), $3, 'active', $4, 1) ON CONFLICT (id) DO NOTHING")
            .bind(id)
            .bind(&email)
            .bind(hash)
            .bind(locale)
            .execute(&mut *tx)
            .await?;
        if inserted.rows_affected() != 1 {
            return Err(SeedError::Conflict(
                "accountId belongs to another email".into(),
            ));
        }
        id
    };
    let row = sqlx::query("SELECT status, native_security_generation FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(json!({
        "entity": "account",
        "accountId": id.to_string(),
        "email": email,
        "status": row.try_get::<String, _>("status")?,
        "nativeSecurityGeneration": row.try_get::<i64, _>("native_security_generation")?,
    }))
}

async fn world(pool: &PgPool, args: &Args<'_>) -> Result<Value, SeedError> {
    const UNSTORED: [&str; 3] = ["host", "port", "slug"];
    args.only(&["worldId", "name", "region", "host", "port", "slug"])?;
    let id = args
        .uuid("worldId", true)?
        .ok_or_else(|| bad("worldId is required"))?;
    let name = args.required("name")?;
    bounded("name", name, 96)?;
    let region = args.text("region")?.unwrap_or("local");
    bounded("region", region, 96)?;
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO world_status (id, name, location) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET name=excluded.name, location=excluded.location, updated_at=now() WHERE (world_status.name, world_status.location) IS DISTINCT FROM (excluded.name, excluded.location)")
        .bind(id)
        .bind(name)
        .bind(region)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let not_stored: Vec<&str> = UNSTORED
        .into_iter()
        .filter(|key| args.0.contains_key(*key))
        .collect();
    let mut out = json!({
        "entity": "world",
        "worldId": id.to_string(),
        "name": name,
        "region": region,
    });
    if !not_stored.is_empty()
        && let Some(map) = out.as_object_mut()
    {
        map.insert("notStored".into(), json!(not_stored));
        map.insert("notStoredReason".into(), json!(PENDING_T19C));
    }
    Ok(out)
}

async fn oauth_native_client(pool: &PgPool, args: &Args<'_>) -> Result<Value, SeedError> {
    args.only(&["clientId"])?;
    let id = match args.uuid("clientId", false)? {
        Some(id) => id,
        None => Uuid::parse_str(DEFAULT_NATIVE_CLIENT)
            .map_err(|_| SeedError::Unavailable("default client id"))?,
    };
    sqlx::query("INSERT INTO oauth_native_clients (client_id, enabled, first_party, public_client, native_ticket) VALUES ($1, true, true, true, true) ON CONFLICT (client_id) DO UPDATE SET enabled=true, first_party=true, public_client=true, native_ticket=true, updated_at=now() WHERE NOT (oauth_native_clients.enabled AND oauth_native_clients.first_party AND oauth_native_clients.public_client AND oauth_native_clients.native_ticket)")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(json!({
        "entity": "oauth-native-client",
        "clientId": id.to_string(),
        "enabled": true,
        "firstParty": true,
        "publicClient": true,
        "nativeTicket": true,
    }))
}

fn hex32(raw: &str) -> Result<[u8; 32], SeedError> {
    let digits = raw.as_bytes();
    if digits.len() != 64 {
        return Err(bad("publicKeyHex must be 64 hex digits"));
    }
    let nibble = |b: u8| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(bad("publicKeyHex must be 64 hex digits")),
    };
    let mut out = [0u8; 32];
    for (index, pair) in digits.chunks_exact(2).enumerate() {
        out[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(out)
}

async fn signing_trust(
    pool: &PgPool,
    settings: &Settings,
    args: &Args<'_>,
) -> Result<Value, SeedError> {
    args.only(&["issuer", "profile", "purpose", "keyId", "publicKeyHex"])?;
    let issuer = args.text("issuer")?.unwrap_or(keys::ISSUER);
    let profile = args.text("profile")?.unwrap_or(match issuer {
        RECOVERY_ISSUER => RECOVERY_PROFILE,
        _ => keys::PROFILE,
    });
    let purpose = args.required("purpose")?;
    let key_id = args.required("keyId")?;
    if !(1..=64).contains(&key_id.len())
        || !key_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(bad("keyId must match [A-Za-z0-9._-]{1,64}"));
    }
    let key = hex32(args.required("publicKeyHex")?)?;
    let public_key = URL_SAFE_NO_PAD.encode(key);
    let directory = settings
        .witness_directory
        .clone()
        .filter(|dir| dir.is_absolute())
        .ok_or(SeedError::Unavailable(
            "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY must be an absolute directory",
        ))?;
    match (issuer, profile) {
        (keys::ISSUER, keys::PROFILE) => {
            let registry = PublicTrustRegistry::new(
                pool.clone(),
                keys::Settings {
                    enabled: true,
                    key_purpose: purpose.to_owned(),
                    clock_uncertainty_seconds: 0,
                },
                directory,
            )
            .map_err(trust_error)?;
            registry
                .publish(&keys::PublicKey {
                    key_id: key_id.to_owned(),
                    public_key: public_key.clone(),
                })
                .await
                .map_err(trust_error)?;
        }
        (RECOVERY_ISSUER, RECOVERY_PROFILE) if purpose == RECOVERY_PURPOSE => {
            publish_recovery(pool, &directory, key_id, &public_key).await?;
        }
        (RECOVERY_ISSUER, RECOVERY_PROFILE) => {
            return Err(bad(
                "the recovery profile's purpose is existing_actor_recovery",
            ));
        }
        _ => return Err(bad("unknown issuer/profile pair")),
    }
    let revision: i64 = sqlx::query_scalar("SELECT issuer_revision FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1")
        .bind(issuer)
        .bind(profile)
        .bind(purpose)
        .fetch_one(pool)
        .await?;
    Ok(json!({
        "entity": "signing-trust",
        "issuer": issuer,
        "profile": profile,
        "purpose": purpose,
        "keyId": key_id,
        "publicKey": public_key,
        "issuerRevision": revision,
    }))
}

fn trust_error(error: keys::KeyError) -> SeedError {
    match error {
        keys::KeyError::BadInput => bad("signing trust input refused"),
        keys::KeyError::InvalidState => SeedError::Conflict(
            "key id or public key already bound differently, or two keys already trusted".into(),
        ),
        _ => SeedError::Unavailable("signing trust registry"),
    }
}

/// The recovery scope has no registry type on this tree yet; this applies the
/// same rules as `PublicTrustRegistry::publish` (gateway contract §9.2): witness
/// before commit, a kid bound to one key forever, at most two trusted keys.
async fn publish_recovery(
    pool: &PgPool,
    directory: &std::path::Path,
    key_id: &str,
    public_key: &str,
) -> Result<(), SeedError> {
    let witness_down = |_| SeedError::Unavailable("retained witness");
    let namespace = trust_state_namespace(RECOVERY_ISSUER, RECOVERY_PROFILE, RECOVERY_PURPOSE)
        .map_err(witness_down)?;
    let witness = Witness::lock(directory, &namespace).map_err(witness_down)?;
    let floor = witness.floor(&namespace).map_err(witness_down)?;
    let mut tx = pool.begin().await?;
    let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 AND profile=$2 AND key_purpose=$3 ORDER BY profile_version DESC LIMIT 1 FOR UPDATE")
        .bind(RECOVERY_ISSUER).bind(RECOVERY_PROFILE).bind(RECOVERY_PURPOSE)
        .fetch_optional(&mut *tx).await?;
    let stale = || SeedError::Conflict("recovery trust state is revoked or ambiguous".into());
    let (id, revision) = match profile {
        Some(row) => {
            let revision: i64 = row.try_get("issuer_revision")?;
            if row.try_get::<bool, _>("revoked")? || floor != Some(revision) {
                return Err(stale());
            }
            (row.try_get::<Uuid, _>("id")?, revision)
        }
        None if floor.is_none() => {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO native_game_signing_trust_profiles (id, issuer, profile, key_purpose, profile_version, issuer_revision) VALUES ($1, $2, $3, $4, 1, 1)")
                .bind(id).bind(RECOVERY_ISSUER).bind(RECOVERY_PROFILE).bind(RECOVERY_PURPOSE)
                .execute(&mut *tx).await?;
            (id, 0)
        }
        None => return Err(stale()),
    };
    // A kid is never published in both scopes (contract §19.5).
    let elsewhere: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM native_game_signing_trust_key_versions WHERE key_id=$1 AND profile_id<>$2)")
        .bind(key_id).bind(id).fetch_one(&mut *tx).await?;
    if elsewhere {
        return Err(SeedError::Conflict(
            "key id is used in another trust scope".into(),
        ));
    }
    let rows = sqlx::query("SELECT DISTINCT ON (key_id) key_id, public_key, trusted FROM native_game_signing_trust_key_versions WHERE profile_id=$1 ORDER BY key_id, key_revision DESC")
        .bind(id).fetch_all(&mut *tx).await?;
    let mut trusted = 0;
    for row in rows {
        let kid: String = row.try_get("key_id")?;
        let key: String = row.try_get("public_key")?;
        let live: bool = row.try_get("trusted")?;
        if kid == key_id {
            return if live && key == public_key {
                Ok(())
            } else {
                Err(SeedError::Conflict("key id is bound to another key".into()))
            };
        }
        if key == public_key {
            return Err(SeedError::Conflict(
                "public key is bound to another key id".into(),
            ));
        }
        trusted += usize::from(live);
    }
    if trusted >= 2 {
        return Err(SeedError::Conflict("two keys are already trusted".into()));
    }
    let next = revision + 1;
    sqlx::query("INSERT INTO native_game_signing_trust_key_versions (profile_id, key_id, key_revision, public_key, trusted, trusted_at) VALUES ($1, $2, 1, $3, true, clock_timestamp())")
        .bind(id).bind(key_id).bind(public_key).execute(&mut *tx).await?;
    sqlx::query("UPDATE native_game_signing_trust_profiles SET issuer_revision=$2, updated_at=clock_timestamp() WHERE id=$1")
        .bind(id).bind(next).execute(&mut *tx).await?;
    witness.advance(&namespace, next).map_err(witness_down)?;
    tx.commit().await?;
    Ok(())
}

const CONTEXT: [(&str, &str); 4] = [
    ("profileRevision", "profile_revision"),
    ("rulesetRevision", "ruleset_revision"),
    ("contentRevision", "content_revision"),
    ("starterTemplateRevision", "starter_template_revision"),
];

fn revision(key: &str, value: &str) -> Result<(), SeedError> {
    if value.is_empty()
        || value.len() > 128
        || !value.as_bytes()[0].is_ascii_alphanumeric()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err(bad(format!(
            "{key} must match [A-Za-z0-9][A-Za-z0-9._:-]{{0,127}}"
        )));
    }
    Ok(())
}

/// Issues the v3 intent that `character-bootstrap-intents/read` serves, in the
/// exact stored form its reader verifies (`oteryn_characters::bootstrap`).
async fn bootstrap_intent(pool: &PgPool, args: &Args<'_>) -> Result<Value, SeedError> {
    let mut allowed = vec![
        "accountId",
        "operationId",
        "targetWorldId",
        "requestedName",
        "requestedSex",
        "ttlSeconds",
    ];
    allowed.extend(CONTEXT.iter().map(|(key, _)| *key));
    args.only(&allowed)?;
    let account = args
        .uuid("accountId", true)?
        .ok_or_else(|| bad("accountId is required"))?
        .to_string();
    let operation = args
        .uuid("operationId", false)?
        .ok_or_else(|| bad("operationId is required"))?
        .to_string();
    let world = args
        .uuid("targetWorldId", false)?
        .ok_or_else(|| bad("targetWorldId is required"))?
        .to_string();
    let name = args.required("requestedName")?;
    if !(2..=29).contains(&name.len())
        || !name
            .split(' ')
            .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()))
    {
        return Err(bad(
            "requestedName must be 2..=29 letters in single-spaced words",
        ));
    }
    let sex = args.required("requestedSex")?;
    if !matches!(sex, "Female" | "Male") {
        return Err(bad("requestedSex must be Female or Male"));
    }
    let mut context = Vec::with_capacity(CONTEXT.len());
    for (key, column) in CONTEXT {
        let value = args.required(key)?;
        revision(key, value)?;
        context.push((column, value));
    }
    let ttl = match args.0.get("ttlSeconds") {
        None => 300,
        Some(value) => value
            .as_i64()
            .filter(|ttl| (1..=300).contains(ttl))
            .ok_or_else(|| bad("ttlSeconds must be an integer in 1..=300"))?,
    };

    let mut tx = pool.begin().await?;
    let active: Option<String> = sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1")
        .bind(Uuid::parse_str(&account).map_err(|_| bad("accountId"))?)
        .fetch_optional(&mut *tx)
        .await?;
    if active.as_deref() != Some("active") {
        return Err(bad("accountId is not an active account"));
    }
    sqlx::query("INSERT INTO character_bootstrap_intent_authority (id, last_source_revision) VALUES (1, '0') ON CONFLICT (id) DO NOTHING")
        .execute(&mut *tx)
        .await?;
    let last: String = sqlx::query_scalar(
        "SELECT last_source_revision FROM character_bootstrap_intent_authority WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if let Some(row) =
        sqlx::query("SELECT * FROM character_bootstrap_intents WHERE operation_id=$1")
            .bind(&operation)
            .fetch_optional(&mut *tx)
            .await?
    {
        let mut same = row.try_get::<String, _>("account_id")? == account
            && row.try_get::<String, _>("target_world_id")? == world
            && row.try_get::<String, _>("requested_name")? == name
            && row
                .try_get::<Option<String>, _>("requested_sex")?
                .as_deref()
                == Some(sex);
        for (column, value) in &context {
            same &= row.try_get::<String, _>(*column)? == *value;
        }
        if !same {
            return Err(SeedError::Conflict(
                "operationId was issued with different input".into(),
            ));
        }
        tx.commit().await?;
        return intent_output(&row);
    }
    let next = last
        .parse::<i64>()
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or(SeedError::Unavailable("bootstrap authority revision"))?
        .to_string();
    let decision = Uuid::now_v7().to_string();
    let issued: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx)
            .await?;
    let expires = (issued + ttl).to_string();
    let issued = issued.to_string();
    let s = |value: &str| Wire::String(value.to_owned());
    let intent = Wire::Object(vec![
        ("contract_version".into(), Wire::Unsigned(3)),
        ("variant".into(), s("OPERATOR_CONTROL_PLANE_BOOTSTRAP")),
        (
            "issuer_authority".into(),
            s("OTERYN_PLATFORM_CHARACTER_AUTHORITY"),
        ),
        ("issuer_decision_id".into(), s(&decision)),
        ("source_revision".into(), s(&next)),
        ("operation_id".into(), s(&operation)),
        ("operation".into(), s("INITIAL_CHARACTER_BOOTSTRAP")),
        ("account_id".into(), s(&account)),
        ("target_world_id".into(), s(&world)),
        ("requested_name".into(), s(name)),
        ("requested_sex".into(), s(sex)),
        (
            "interpretation_context".into(),
            Wire::Object(
                context
                    .iter()
                    .map(|(column, value)| ((*column).to_owned(), s(value)))
                    .collect(),
            ),
        ),
        ("issued_at_source".into(), s(&issued)),
        ("expires_at_source".into(), s(&expires)),
        ("audience".into(), s("OTERYN_GAME_CHARACTER_AUTHORITY")),
    ]);
    let raw = canonical::encode(&intent).map_err(|_| SeedError::Unavailable("encoding"))?;
    if raw.len() > 4096 {
        return Err(bad("intent exceeds 4096 bytes"));
    }
    let digest = format!("{:x}", Sha256::digest(&raw));
    let raw = String::from_utf8(raw).map_err(|_| SeedError::Unavailable("encoding"))?;
    let row = sqlx::query("INSERT INTO character_bootstrap_intents (operation_id, issuer_decision_id, source_revision, account_id, target_world_id, requested_name, requested_sex, profile_revision, ruleset_revision, content_revision, starter_template_revision, issued_at_source, expires_at_source, intent_json, intent_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING *")
        .bind(&operation).bind(&decision).bind(&next).bind(&account).bind(&world)
        .bind(name).bind(sex)
        .bind(context[0].1).bind(context[1].1).bind(context[2].1).bind(context[3].1)
        .bind(&issued).bind(&expires).bind(&raw).bind(&digest)
        .fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE character_bootstrap_intent_authority SET last_source_revision=$1, last_issuer_decision_id=$2 WHERE id=1")
        .bind(&next).bind(&decision).execute(&mut *tx).await?;
    tx.commit().await?;
    intent_output(&row)
}

fn intent_output(row: &sqlx::postgres::PgRow) -> Result<Value, SeedError> {
    let get = |key: &str| row.try_get::<String, _>(key).map_err(SeedError::from);
    Ok(json!({
        "entity": "bootstrap-intent",
        "operationId": get("operation_id")?,
        "issuerDecisionId": get("issuer_decision_id")?,
        "sourceRevision": get("source_revision")?,
        "accountId": get("account_id")?,
        "targetWorldId": get("target_world_id")?,
        "issuedAtSource": get("issued_at_source")?,
        "expiresAtSource": get("expires_at_source")?,
        "intentSha256": get("intent_sha256")?,
    }))
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
