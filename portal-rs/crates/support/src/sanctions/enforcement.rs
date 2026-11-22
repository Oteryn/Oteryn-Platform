//! Platform producer of the Game enforcement command wire
//! (`oteryn-v2-game-enforcement-wire-v1`, contract Part A): the T34 command
//! journal (outbox) and the four Game-initiated `/internal/v1` routes.
//!
//! Mount only behind the verified TLS 1.3 / HTTP/1.1 internal listener. Every
//! response carries `Cache-Control: no-store, private`; non-200 bodies are empty.
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Shape, Value},
    guard::{self, IdentityMap, JsonRule, RoutePolicy, VerifiedClient},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower::ServiceExt;
use uuid::Uuid;

pub const STREAM_PATH: &str = "/internal/v1/support/enforcement-commands/stream/read";
pub const COMMAND_PATH: &str = "/internal/v1/support/enforcement-commands/commands/read";
pub const RESULTS_PATH: &str = "/internal/v1/support/enforcement-commands/results";
pub const RECONCILE_PATH: &str = "/internal/v1/support/enforcement-commands/reconcile/read";
/// Reader group: A1, A2 and A4.
pub const READER_GROUP: &str = "support-enforcement-reader";
/// Results group: A3.
pub const RESULTS_GROUP: &str = "support-enforcement-results";
pub const READER_IDENTITY: &str = "PORTAL_GAME_ENFORCEMENT_READER_IDENTITY";
pub const RESULTS_IDENTITY: &str = "PORTAL_GAME_ENFORCEMENT_RESULTS_IDENTITY";
const MAX_PAGE: usize = 262144;
const MAX_COMMAND: usize = 1024;
const MAX_RESULT: usize = 1024;

static COMMAND_SHAPE: Shape = Shape::Object(&[
    ("operation_id", Shape::Scalar),
    ("sanction_stream_id", Shape::Scalar),
    ("command_version", Shape::Scalar),
    ("intent", Shape::Scalar),
    ("profile", Shape::Scalar),
    ("decision_revision", Shape::Scalar),
    ("prior_decision_revision", Shape::Scalar),
    ("policy_revision", Shape::Scalar),
    ("account_id", Shape::Scalar),
    ("character_id", Shape::Scalar),
    ("effective_at_ms", Shape::Scalar),
    ("expires_at_ms", Shape::Scalar),
    ("reason_code", Shape::Scalar),
    ("decided_at_ms", Shape::Scalar),
    ("audit_ref", Shape::Scalar),
    ("correlation_id", Shape::Scalar),
]);
static RESULT_SHAPE: Shape = Shape::Object(&[
    ("schema", Shape::Scalar),
    ("operation_id", Shape::Scalar),
    ("sanction_stream_id", Shape::Scalar),
    ("command_digest", Shape::Scalar),
    ("receipt_revision", Shape::Scalar),
    ("state", Shape::Scalar),
    ("result_code", Shape::Scalar),
    ("accepted_decision_revision", Shape::Scalar),
    ("stream_state", Shape::Scalar),
    ("effective_until_ms", Shape::Scalar),
    ("runtime_effect", Shape::Scalar),
    ("capability_revision", Shape::Scalar),
    ("recovery_generation", Shape::Scalar),
    ("decided_at_ms", Shape::Scalar),
]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Mute,
    Jail,
}
impl Profile {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "MUTE" => Some(Self::Mute),
            "JAIL" => Some(Self::Jail),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mute => "MUTE",
            Self::Jail => "JAIL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Apply,
    Replace,
    Revoke,
    Expire,
}
impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "APPLY",
            Self::Replace => "REPLACE",
            Self::Revoke => "REVOKE",
            Self::Expire => "EXPIRE",
        }
    }
    fn parse(value: &str) -> Option<Self> {
        match value {
            "APPLY" => Some(Self::Apply),
            "REPLACE" => Some(Self::Replace),
            "REVOKE" => Some(Self::Revoke),
            "EXPIRE" => Some(Self::Expire),
            _ => None,
        }
    }
}

/// `PORTAL_GAME_ENFORCEMENT_PROFILES`: a profile not listed produces no command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Profiles {
    mute: bool,
    jail: bool,
}
impl Profiles {
    /// Unset is `[]`. Anything but a JSON list of distinct `MUTE` / `JAIL` is refused.
    pub fn parse(raw: Option<&str>) -> Result<Self, Error> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };
        let names: Vec<String> = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
        let mut profiles = Self::default();
        for name in names {
            let slot = match name.as_str() {
                "MUTE" => &mut profiles.mute,
                "JAIL" => &mut profiles.jail,
                _ => return Err(Error::Invalid),
            };
            if *slot {
                return Err(Error::Invalid);
            }
            *slot = true;
        }
        Ok(profiles)
    }
    pub fn enabled(&self, profile: Profile) -> bool {
        match profile {
            Profile::Mute => self.mute,
            Profile::Jail => self.jail,
        }
    }
}

/// Contract §6.6. Both groups default off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub reader_enabled: bool,
    pub results_enabled: bool,
    pub profiles: Profiles,
    pub requests_per_minute: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            reader_enabled: false,
            results_enabled: false,
            profiles: Profiles::default(),
            requests_per_minute: 1200,
        }
    }
}
impl Settings {
    /// An unknown profile or an out-of-range budget disables the whole surface.
    pub fn from_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let flag = get("PORTAL_GAME_ENFORCEMENT_ENABLED").as_deref() == Some("true");
        let profiles = Profiles::parse(get("PORTAL_GAME_ENFORCEMENT_PROFILES").as_deref());
        let budget = match get("PORTAL_GAME_ENFORCEMENT_REQUESTS_PER_MINUTE") {
            None => Some(1200),
            Some(raw) => raw
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=6000).contains(n) && raw == n.to_string()),
        };
        let enabled = flag && profiles.is_ok() && budget.is_some();
        Self {
            reader_enabled: enabled,
            results_enabled: enabled,
            profiles: profiles.unwrap_or_default(),
            requests_per_minute: budget.unwrap_or(1200),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid enforcement wire input")]
    Invalid,
    #[error("enforcement operation not found")]
    NotFound,
    #[error("enforcement revision or fencing conflict")]
    Conflict,
    #[error("enforcement decision revision is stale")]
    Stale,
    #[error("enforcement encoding unavailable")]
    Encoding,
    #[error("enforcement storage unavailable")]
    Storage(#[from] sqlx::Error),
}

/// One command (§3.2). Built server-side from an authorized Platform decision;
/// browser input never supplies its identities or revisions (§6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub operation_id: Uuid,
    pub sanction_stream_id: Uuid,
    pub intent: Intent,
    pub profile: Profile,
    pub decision_revision: i64,
    pub prior_decision_revision: Option<i64>,
    pub policy_revision: i64,
    pub account_id: Uuid,
    pub character_id: Uuid,
    pub effective_at_ms: Option<i64>,
    pub expires_at_ms: Option<i64>,
    pub reason_code: Option<u16>,
    pub decided_at_ms: i64,
    pub audit_ref: Uuid,
    pub correlation_id: Uuid,
}
impl Command {
    fn validate(&self) -> Result<(), Error> {
        for id in [
            self.operation_id,
            self.sanction_stream_id,
            self.account_id,
            self.character_id,
            self.audit_ref,
            self.correlation_id,
        ] {
            super::valid_id(id).map_err(|_| Error::Invalid)?;
        }
        let positive = |n: i64| n >= 1;
        if !positive(self.decision_revision)
            || !positive(self.policy_revision)
            || !positive(self.decided_at_ms)
            || self.reason_code == Some(0)
        {
            return Err(Error::Invalid);
        }
        let timed = matches!(
            (self.effective_at_ms, self.expires_at_ms),
            (Some(start), Some(end)) if start >= 1 && end > start
        );
        let untimed = self.effective_at_ms.is_none() && self.expires_at_ms.is_none();
        let newer = self
            .prior_decision_revision
            .is_some_and(|prior| prior >= 1 && self.decision_revision > prior);
        let valid = match self.intent {
            Intent::Apply => {
                self.prior_decision_revision.is_none() && timed && self.reason_code.is_some()
            }
            Intent::Replace => newer && timed && self.reason_code.is_some(),
            Intent::Revoke => newer && untimed && self.reason_code.is_some(),
            Intent::Expire => {
                self.prior_decision_revision.is_none() && untimed && self.reason_code.is_none()
            }
        };
        if valid { Ok(()) } else { Err(Error::Invalid) }
    }

    /// The exact canonical bytes A1 and A2 serve.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let optional = |n: Option<i64>| n.map_or(Value::Null, Value::Signed);
        let value = object(vec![
            ("operation_id", text(self.operation_id)),
            ("sanction_stream_id", text(self.sanction_stream_id)),
            ("command_version", Value::Signed(1)),
            ("intent", text(self.intent.as_str())),
            ("profile", text(self.profile.as_str())),
            ("decision_revision", Value::Signed(self.decision_revision)),
            (
                "prior_decision_revision",
                optional(self.prior_decision_revision),
            ),
            ("policy_revision", Value::Signed(self.policy_revision)),
            ("account_id", text(self.account_id)),
            ("character_id", text(self.character_id)),
            ("effective_at_ms", optional(self.effective_at_ms)),
            ("expires_at_ms", optional(self.expires_at_ms)),
            ("reason_code", optional(self.reason_code.map(i64::from))),
            ("decided_at_ms", Value::Signed(self.decided_at_ms)),
            ("audit_ref", text(self.audit_ref)),
            ("correlation_id", text(self.correlation_id)),
        ]);
        canonical::encode_canonical(&value, MAX_COMMAND, 4, &COMMAND_SHAPE)
            .map_err(|_| Error::Encoding)
    }
}

/// Lowercase SHA-256 hex: `command_digest` and `result_digest`.
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub position: i64,
    pub command_digest: String,
}

/// Append one command to the journal, in the caller's decision transaction
/// (§6.1). The caller has authorized the decision and owns commit/rollback.
///
/// A profile that is not enabled, and a later decision on a stream that never
/// produced a command (decided while its profile was off, E8), are
/// Platform-local only: `Ok(None)`, nothing written. Ordering per stream is
/// fenced under the publication lock: `APPLY` opens a stream; `REPLACE` and
/// `REVOKE` name the stream's current revision as their precondition; `EXPIRE`
/// reconciles the current timed decision once.
pub async fn record(
    tx: &mut Transaction<'_, Postgres>,
    profiles: &Profiles,
    command: &Command,
) -> Result<Option<Recorded>, Error> {
    let bytes = command.encode()?;
    if !profiles.enabled(command.profile) {
        return Ok(None);
    }
    let command_digest = digest(&bytes);
    // The counter row is the publication lock: a larger position never becomes
    // visible before a smaller one commits, and a rollback removes both.
    let position: i64 = sqlx::query_scalar(
        "SELECT position+1 FROM support_enforcement_counter WHERE singleton FOR UPDATE",
    )
    .fetch_one(&mut **tx)
    .await?;
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM support_enforcement_commands WHERE operation_id=$1)",
    )
    .bind(command.operation_id)
    .fetch_one(&mut **tx)
    .await?;
    if taken {
        return Err(Error::Conflict);
    }
    let latest = sqlx::query("SELECT c.decision_revision,c.intent,c.profile,c.account_id,c.character_id,
            (convert_from(r.result,'UTF8')::jsonb->>'accepted_decision_revision')::bigint AS refused_high_water
        FROM support_enforcement_commands c
        LEFT JOIN support_enforcement_results r
          ON r.operation_id=c.operation_id AND r.terminal AND r.state='REJECTED'
        WHERE c.sanction_stream_id=$1 ORDER BY c.decision_revision DESC,c.position DESC LIMIT 1")
        .bind(command.sanction_stream_id)
        .fetch_optional(&mut **tx)
        .await?;
    match latest {
        None if command.intent != Intent::Apply => return Ok(None),
        None => {}
        Some(row) => {
            if command.intent == Intent::Apply
                || row.try_get::<String, _>("profile")? != command.profile.as_str()
                || row.try_get::<Uuid, _>("account_id")? != command.account_id
                || row.try_get::<Uuid, _>("character_id")? != command.character_id
            {
                return Err(Error::Conflict);
            }
            let current: i64 = row.try_get("decision_revision")?;
            let last =
                Intent::parse(&row.try_get::<String, _>("intent")?).ok_or(Error::Encoding)?;
            let fresh = match command.intent {
                Intent::Apply => false,
                Intent::Replace | Intent::Revoke => {
                    // A rejected request may leave Game on the previous decision.
                    // The next operation advances the journal but fences on Game's receipt.
                    let high_water = row
                        .try_get::<Option<i64>, _>("refused_high_water")?
                        .unwrap_or(current);
                    command.decision_revision > current
                        && command.prior_decision_revision == Some(high_water)
                }
                Intent::Expire => {
                    command.decision_revision == current
                        && matches!(last, Intent::Apply | Intent::Replace)
                }
            };
            if !fresh {
                return Err(Error::Stale);
            }
        }
    }
    sqlx::query("UPDATE support_enforcement_counter SET position=$1 WHERE singleton")
        .bind(position)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO support_enforcement_commands(position,operation_id,sanction_stream_id,decision_revision,intent,profile,account_id,character_id,command,command_digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(position)
        .bind(command.operation_id)
        .bind(command.sanction_stream_id)
        .bind(command.decision_revision)
        .bind(command.intent.as_str())
        .bind(command.profile.as_str())
        .bind(command.account_id)
        .bind(command.character_id)
        .bind(&bytes)
        .bind(&command_digest)
        .execute(&mut **tx)
        .await?;
    Ok(Some(Recorded {
        position,
        command_digest,
    }))
}

fn text(value: impl ToString) -> Value {
    Value::String(value.to_string())
}
fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn uuid7(value: Option<&Value>) -> Result<Uuid, Error> {
    let Some(Value::String(raw)) = value else {
        return Err(Error::Invalid);
    };
    let id = Uuid::parse_str(raw).map_err(|_| Error::Invalid)?;
    if raw.len() != 36 || id.hyphenated().to_string() != *raw {
        return Err(Error::Invalid);
    }
    super::valid_id(id).map_err(|_| Error::Invalid)?;
    Ok(id)
}
fn integer(value: Option<&Value>, min: i64, max: i64) -> Result<i64, Error> {
    let n = match value {
        Some(Value::Signed(n)) => *n,
        Some(Value::Unsigned(n)) => i64::try_from(*n).map_err(|_| Error::Invalid)?,
        _ => return Err(Error::Invalid),
    };
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(Error::Invalid)
    }
}
fn string(value: Option<&Value>) -> Result<&str, Error> {
    match value {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::Invalid),
    }
}
fn exact(value: &Value, members: &[&str]) -> Result<(), Error> {
    match value {
        Value::Object(fields)
            if fields.len() == members.len()
                && members.iter().all(|m| fields.iter().any(|(k, _)| k == m)) =>
        {
            Ok(())
        }
        _ => Err(Error::Invalid),
    }
}

/// A1 / A4 request (§3.1, §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub after: Option<i64>,
    pub limit: i64,
}
impl Page {
    fn decode(value: &Value, schema: &str) -> Result<Self, Error> {
        exact(value, &["schema", "after", "limit"])?;
        if string(value.get("schema"))? != schema {
            return Err(Error::Invalid);
        }
        let after = match value.get("after") {
            Some(Value::Null) => None,
            Some(Value::String(s)) => {
                if !(4..=22).contains(&s.len()) {
                    return Err(Error::Invalid);
                }
                let raw = s.strip_prefix("v1:").ok_or(Error::Invalid)?;
                let n = raw.parse::<i64>().map_err(|_| Error::Invalid)?;
                if n <= 0 || raw != n.to_string() {
                    return Err(Error::Invalid);
                }
                Some(n)
            }
            _ => return Err(Error::Invalid),
        };
        Ok(Self {
            after,
            limit: integer(value.get("limit"), 1, 500)?,
        })
    }
}

/// Page envelope around already-canonical items, within the §1.6 byte budget.
fn encode_page(schema: &str, items: Vec<(i64, Vec<u8>)>) -> Result<Vec<u8>, Error> {
    let mut used = 128_usize;
    let mut body = Vec::new();
    let mut after = None;
    for (position, item) in items {
        if used + item.len() + 1 > MAX_PAGE {
            break;
        }
        used += item.len() + 1;
        if after.is_some() {
            body.push(b',');
        }
        body.extend_from_slice(&item);
        after = Some(position);
    }
    let mut bytes = format!("{{\"schema\":\"{schema}\",\"items\":[").into_bytes();
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(b"],\"next_cursor\":");
    match after {
        Some(n) => bytes.extend_from_slice(format!("\"v1:{n}\"}}").as_bytes()),
        None => bytes.extend_from_slice(b"null}"),
    }
    if bytes.len() > MAX_PAGE {
        return Err(Error::Encoding);
    }
    Ok(bytes)
}

/// A cursor beyond committed history is `Invalid` (400).
async fn beyond_history(tx: &mut Transaction<'_, Postgres>, page: Page) -> Result<(), Error> {
    let committed: i64 =
        sqlx::query_scalar("SELECT position FROM support_enforcement_counter WHERE singleton")
            .fetch_one(&mut **tx)
            .await?;
    if page.after.is_some_and(|n| n > committed) {
        return Err(Error::Invalid);
    }
    Ok(())
}

/// A1: commands in ascending publication position.
pub async fn read_stream(pool: &PgPool, page: Page) -> Result<Vec<u8>, Error> {
    let mut tx = pool.begin().await?;
    beyond_history(&mut tx, page).await?;
    let rows = sqlx::query("SELECT position,command FROM support_enforcement_commands WHERE position>$1 ORDER BY position LIMIT $2")
        .bind(page.after.unwrap_or(0))
        .bind(page.limit)
        .fetch_all(&mut *tx)
        .await?;
    let items = rows
        .iter()
        .map(|row| Ok((row.try_get("position")?, row.try_get("command")?)))
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    tx.commit().await?;
    encode_page("oteryn.enforcement_command_stream_page.v1", items)
}

/// A2: one command's bytes, verbatim.
pub async fn read_command(pool: &PgPool, operation_id: Uuid) -> Result<Option<Vec<u8>>, Error> {
    Ok(
        sqlx::query_scalar(
            "SELECT command FROM support_enforcement_commands WHERE operation_id=$1",
        )
        .bind(operation_id)
        .fetch_optional(pool)
        .await?,
    )
}

/// A4: Platform's view of every operation, in publication order.
pub async fn read_reconcile(pool: &PgPool, page: Page) -> Result<Vec<u8>, Error> {
    let mut tx = pool.begin().await?;
    beyond_history(&mut tx, page).await?;
    let rows = sqlx::query("SELECT c.position,c.operation_id,c.sanction_stream_id,r.state,r.receipt_revision,r.result_digest FROM support_enforcement_commands c LEFT JOIN LATERAL (SELECT state,receipt_revision,result_digest FROM support_enforcement_results WHERE operation_id=c.operation_id ORDER BY receipt_revision DESC LIMIT 1) r ON true WHERE c.position>$1 ORDER BY c.position LIMIT $2")
        .bind(page.after.unwrap_or(0))
        .bind(page.limit)
        .fetch_all(&mut *tx)
        .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let state: Option<String> = row.try_get("state")?;
        let item = object(vec![
            (
                "operation_id",
                text(row.try_get::<Uuid, _>("operation_id")?),
            ),
            (
                "sanction_stream_id",
                text(row.try_get::<Uuid, _>("sanction_stream_id")?),
            ),
            ("state", text(state.as_deref().unwrap_or("PENDING"))),
            (
                "receipt_revision",
                row.try_get::<Option<i32>, _>("receipt_revision")?
                    .map_or(Value::Null, |n| Value::Signed(i64::from(n))),
            ),
            (
                "result_digest",
                row.try_get::<Option<String>, _>("result_digest")?
                    .map_or(Value::Null, text),
            ),
        ]);
        let bytes = canonical::encode(&item).map_err(|_| Error::Encoding)?;
        items.push((row.try_get("position")?, bytes));
    }
    tx.commit().await?;
    encode_page("oteryn.enforcement_reconcile_page.v1", items)
}

/// The members of a decoded A3 body that Platform fences on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub operation_id: Uuid,
    pub sanction_stream_id: Uuid,
    pub command_digest: String,
    pub receipt_revision: i32,
    pub state: String,
    pub result_code: String,
    accepted_decision_revision: i64,
    stream_state: String,
}
impl Receipt {
    pub fn terminal(&self) -> bool {
        self.state != "ACCEPTED_PENDING"
    }
    /// Every body-only §4.1 / §4.2 rule. Rules that need the command run in `ingest`.
    pub fn decode(value: &Value) -> Result<Self, Error> {
        RESULT_SHAPE.validate(value).map_err(|_| Error::Invalid)?;
        if string(value.get("schema"))? != "oteryn.enforcement_result.v1" {
            return Err(Error::Invalid);
        }
        let operation_id = uuid7(value.get("operation_id"))?;
        let sanction_stream_id = uuid7(value.get("sanction_stream_id"))?;
        let command_digest = string(value.get("command_digest"))?;
        if command_digest.len() != 64
            || !command_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Invalid);
        }
        let receipt_revision = integer(value.get("receipt_revision"), 1, 65535)?;
        let state = string(value.get("state"))?;
        let result_code = string(value.get("result_code"))?;
        let code_allowed = match state {
            "ACCEPTED_PENDING" => matches!(result_code, "PENDING_RUNTIME" | "PENDING_EXPIRY"),
            "APPLIED" | "REPLACED" | "REVOKED" | "EXPIRED" => result_code == "OK",
            "REJECTED" => matches!(
                result_code,
                "UNSUPPORTED"
                    | "TARGET_NOT_APPLICABLE"
                    | "TARGET_MISMATCH"
                    | "STALE_REVISION"
                    | "LIFECYCLE_CONFLICT"
                    | "OPERATION_CONFLICT"
                    | "CAPABILITY_CONFLICT"
                    | "INTERNAL_NON_RETRYABLE"
            ),
            _ => false,
        };
        if !code_allowed {
            return Err(Error::Invalid);
        }
        let accepted_decision_revision =
            integer(value.get("accepted_decision_revision"), 0, i64::MAX)?;
        let stream_state = string(value.get("stream_state"))?;
        let open = match stream_state {
            "PENDING" | "ACTIVE" => true,
            "NONE" | "REVOKED" | "EXPIRED" => false,
            _ => return Err(Error::Invalid),
        };
        match (open, value.get("effective_until_ms")) {
            (true, until) => {
                integer(until, 1, i64::MAX)?;
            }
            (false, Some(Value::Null)) => {}
            _ => return Err(Error::Invalid),
        }
        if !matches!(
            string(value.get("runtime_effect"))?,
            "ONLINE_APPLIED" | "ON_NEXT_ADMISSION" | "NONE"
        ) {
            return Err(Error::Invalid);
        }
        let capability = string(value.get("capability_revision"))?;
        if !(1..=64).contains(&capability.len())
            || !capability.bytes().all(|b| {
                b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || matches!(b, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(Error::Invalid);
        }
        integer(value.get("recovery_generation"), 1, i64::MAX)?;
        integer(value.get("decided_at_ms"), 1, i64::MAX)?;
        Ok(Self {
            operation_id,
            sanction_stream_id,
            command_digest: command_digest.into(),
            receipt_revision: i32::try_from(receipt_revision).map_err(|_| Error::Invalid)?,
            state: state.into(),
            result_code: result_code.into(),
            accepted_decision_revision,
            stream_state: stream_state.into(),
        })
    }

    fn confirms_command(&self, decision_revision: i64) -> bool {
        let expected = match self.state.as_str() {
            "APPLIED" | "REPLACED" => "ACTIVE",
            "REVOKED" => "REVOKED",
            "EXPIRED" => "EXPIRED",
            _ => return true,
        };
        self.stream_state == expected && self.accepted_decision_revision == decision_revision
    }

    fn fits(&self, intent: Intent) -> bool {
        match self.state.as_str() {
            "ACCEPTED_PENDING" => self.result_code != "PENDING_EXPIRY" || intent == Intent::Expire,
            "APPLIED" => intent == Intent::Apply,
            "REPLACED" => intent == Intent::Replace,
            "REVOKED" => intent == Intent::Revoke,
            "EXPIRED" => intent == Intent::Expire,
            _ => true,
        }
    }
}

/// A3 (§4.3), in one transaction that locks the operation's command row.
/// `bytes` are the exact canonical body. An identical republication writes
/// nothing; changed bytes under a stored key, a foreign `command_digest`, a
/// result above a stored terminal, or a second terminal are `Conflict`.
pub async fn ingest(pool: &PgPool, receipt: &Receipt, bytes: &[u8]) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let command = sqlx::query("SELECT sanction_stream_id,intent,command_digest,decision_revision FROM support_enforcement_commands WHERE operation_id=$1 FOR UPDATE")
        .bind(receipt.operation_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    let intent = Intent::parse(&command.try_get::<String, _>("intent")?).ok_or(Error::Encoding)?;
    if command.try_get::<Uuid, _>("sanction_stream_id")? != receipt.sanction_stream_id
        || !receipt.fits(intent)
    {
        return Err(Error::Invalid);
    }
    if command.try_get::<String, _>("command_digest")? != receipt.command_digest {
        return Err(Error::Conflict);
    }
    let stored: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT result FROM support_enforcement_results WHERE operation_id=$1 AND receipt_revision=$2",
    )
    .bind(receipt.operation_id)
    .bind(receipt.receipt_revision)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(stored) = stored {
        return if stored == bytes {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    if !receipt.confirms_command(command.try_get("decision_revision")?) {
        return Err(Error::Invalid);
    }
    let terminal: Option<i32> = sqlx::query_scalar(
        "SELECT receipt_revision FROM support_enforcement_results WHERE operation_id=$1 AND terminal",
    )
    .bind(receipt.operation_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(terminal) = terminal
        && (receipt.receipt_revision > terminal || receipt.terminal())
    {
        return Err(Error::Conflict);
    }
    // A terminal result must remain the highest receipt. The command row lock
    // serializes this check with every other receipt for the operation.
    if receipt.terminal()
        && sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM support_enforcement_results WHERE operation_id=$1 AND receipt_revision>$2)",
        )
        .bind(receipt.operation_id)
        .bind(receipt.receipt_revision)
        .fetch_one(&mut *tx)
        .await?
    {
        return Err(Error::Conflict);
    }
    sqlx::query("INSERT INTO support_enforcement_results(operation_id,receipt_revision,state,terminal,result,result_digest) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(receipt.operation_id)
        .bind(receipt.receipt_revision)
        .bind(&receipt.state)
        .bind(receipt.terminal())
        .bind(bytes)
        .bind(digest(bytes))
        .execute(&mut *tx)
        .await?;
    if receipt.terminal() && receipt.state != "REJECTED" {
        let timed = matches!(receipt.state.as_str(), "APPLIED" | "REPLACED");
        sqlx::query(
            "UPDATE support_enforcement_decisions d
             SET reconcile_at_ms=CASE WHEN d.operation_id=$1 AND $2 THEN d.expires_at_ms END
             FROM support_enforcement_commands decided,support_enforcement_commands source
             WHERE decided.operation_id=$1
               AND d.sanction_id=decided.sanction_stream_id
               AND d.decision_revision<=decided.decision_revision
               AND (d.reconcile_at_ms IS NOT NULL OR (d.operation_id=$1 AND $2))
               AND source.operation_id=d.operation_id AND source.position<=decided.position",
        )
        .bind(receipt.operation_id)
        .bind(timed)
        .execute(&mut *tx)
        .await?;
    }
    if receipt.state == "EXPIRED" {
        sqlx::query(
            "UPDATE sanctions s SET status='expired', lock_version=lock_version+1,
                        updated_at=clock_timestamp()
                     WHERE s.id=$1 AND s.enforcement_profile IS NOT NULL AND s.status='active'
                       AND NOT EXISTS (SELECT 1 FROM support_enforcement_commands newer
                         JOIN support_enforcement_commands expired ON expired.operation_id=$2
                         WHERE newer.sanction_stream_id=s.id AND newer.position>expired.position)",
        )
        .bind(receipt.sanction_stream_id)
        .bind(receipt.operation_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

struct Budget(Mutex<(Instant, u32)>);
impl Budget {
    fn new() -> Self {
        Self(Mutex::new((Instant::now(), 0)))
    }
    fn take(&self, limit: u32) -> Option<bool> {
        let mut rate = self.0.lock().ok()?;
        if rate.0.elapsed() >= Duration::from_secs(60) {
            *rate = (Instant::now(), 0);
        }
        if rate.1 >= limit {
            return Some(false);
        }
        rate.1 += 1;
        Some(true)
    }
}

struct Shared {
    pool: PgPool,
    limit: u32,
    reader: Budget,
    results: Budget,
    reader_probe: Router,
    results_probe: Router,
}

fn policy(enabled: bool, group: &str, max: usize, json: JsonRule) -> RoutePolicy {
    RoutePolicy {
        enabled,
        group: group.into(),
        max_body_bytes: max,
        max_json_depth: 4,
        json,
    }
}

/// `identities` is the complete process map: it must hold the dedicated
/// reader and results bindings and every other internal binding (§1.2).
pub fn router(pool: PgPool, settings: Settings, identities: Arc<IdentityMap>) -> Router {
    // A body-independent probe through the shared guard gives the §1.3 order:
    // flag and identity, then the peer budget, then transport and body.
    let probe = |enabled, group| {
        guard::protect(
            Router::new().route("/", post(|| async { StatusCode::NO_CONTENT })),
            identities.clone(),
            policy(enabled, group, 2, JsonRule::Object),
        )
    };
    let shared = Arc::new(Shared {
        pool,
        limit: settings.requests_per_minute,
        reader: Budget::new(),
        results: Budget::new(),
        reader_probe: probe(settings.reader_enabled, READER_GROUP),
        results_probe: probe(settings.results_enabled, RESULTS_GROUP),
    });
    let reader = guard::protect(
        Router::new()
            .route(STREAM_PATH, post(stream))
            .route(COMMAND_PATH, post(command))
            .route(RECONCILE_PATH, post(reconcile)),
        identities.clone(),
        policy(settings.reader_enabled, READER_GROUP, 256, JsonRule::Object),
    );
    let results = guard::protect(
        Router::new().route(RESULTS_PATH, post(result)),
        identities,
        policy(
            settings.results_enabled,
            RESULTS_GROUP,
            MAX_RESULT,
            JsonRule::Canonical(&RESULT_SHAPE),
        ),
    );
    reader
        .merge(results)
        .with_state(shared.clone())
        .route_layer(middleware::from_fn_with_state(shared, peer))
        .layer(middleware::from_fn(response_headers))
}

async fn peer(State(shared): State<Arc<Shared>>, request: Request, next: Next) -> Response {
    let results = request.uri().path() == RESULTS_PATH;
    let mut probe = Request::new(Body::from("{}"));
    *probe.method_mut() = Method::POST;
    probe.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Some(client) = request.extensions().get::<VerifiedClient>() {
        probe.extensions_mut().insert(client.clone());
    }
    let target = if results {
        &shared.results_probe
    } else {
        &shared.reader_probe
    };
    let auth = match target.clone().oneshot(probe).await {
        Ok(response) => response,
        Err(never) => match never {},
    };
    if auth.status() != StatusCode::NO_CONTENT {
        return guard::failure(auth.status());
    }
    let budget = if results {
        &shared.results
    } else {
        &shared.reader
    };
    match budget.take(shared.limit) {
        Some(true) => {}
        Some(false) => {
            let mut response = guard::failure(StatusCode::TOO_MANY_REQUESTS);
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
            return response;
        }
        None => return guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
    if request.uri().query().is_some() {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    let response = next.run(request).await;
    // An oversized body is malformed (§1.4), whether by Content-Length or stream.
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    response
}

async fn response_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() != StatusCode::OK {
        *response.body_mut() = Body::empty();
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}

fn json(bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}
fn answer(outcome: Result<Result<Vec<u8>, Error>, tokio::time::error::Elapsed>) -> Response {
    match outcome {
        Ok(Ok(bytes)) => json(bytes),
        Ok(Err(Error::Invalid)) => guard::failure(StatusCode::BAD_REQUEST),
        Ok(Err(Error::NotFound)) => guard::failure(StatusCode::NOT_FOUND),
        Ok(Err(Error::Conflict | Error::Stale)) => guard::failure(StatusCode::CONFLICT),
        _ => guard::failure(StatusCode::SERVICE_UNAVAILABLE),
    }
}
const TIMEOUT: Duration = Duration::from_secs(3);

async fn stream(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let Some(Ok(page)) = request
        .extensions()
        .get::<Value>()
        .map(|v| Page::decode(v, "oteryn.enforcement_command_stream_request.v1"))
    else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    answer(tokio::time::timeout(TIMEOUT, read_stream(&shared.pool, page)).await)
}
async fn reconcile(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let Some(Ok(page)) = request
        .extensions()
        .get::<Value>()
        .map(|v| Page::decode(v, "oteryn.enforcement_reconcile_request.v1"))
    else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    answer(tokio::time::timeout(TIMEOUT, read_reconcile(&shared.pool, page)).await)
}
async fn command(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let decoded = request.extensions().get::<Value>().map(|value| {
        exact(value, &["schema", "operation_id"])?;
        if string(value.get("schema"))? != "oteryn.enforcement_command_read_request.v1" {
            return Err(Error::Invalid);
        }
        uuid7(value.get("operation_id"))
    });
    let Some(Ok(operation_id)) = decoded else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    let read = async {
        read_command(&shared.pool, operation_id)
            .await?
            .ok_or(Error::NotFound)
    };
    answer(tokio::time::timeout(TIMEOUT, read).await)
}
async fn result(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let Some(value) = request.extensions().get::<Value>() else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    let Ok(receipt) = Receipt::decode(value) else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    // The guard admitted only canonical bytes, so re-encoding yields them exactly.
    let Ok(bytes) = canonical::encode(value) else {
        return guard::failure(StatusCode::BAD_REQUEST);
    };
    let write = async {
        ingest(&shared.pool, &receipt, &bytes).await?;
        Ok(br#"{"schema":"oteryn.enforcement_result_ack.v1","result":"accepted"}"#.to_vec())
    };
    answer(tokio::time::timeout(TIMEOUT, write).await)
}

/// Browser actions use the same profile switches as the internal command
/// stream. The caller also verifies that both mTLS purposes are operational.
pub fn configured_browser(
    pool: PgPool,
    exchange_available: bool,
) -> Result<Router, Box<dyn std::error::Error>> {
    let profiles = Settings::from_lookup(|key| std::env::var(key).ok()).profiles;
    let authority = std::env::var("OTERYN_LCFA_SOURCE_AUTHORITY")
        .ok()
        .filter(|authority| {
            !authority.is_empty() && std::env::var("OTERYN_LCFA_ENABLED").as_deref() == Ok("true")
        });
    oteryn_admin::console::enforcement::configured_router(
        pool,
        BrowserJournal {
            profiles,
            exchange_available,
            authority,
        },
    )
}

use oteryn_admin::console::enforcement::{
    Action, Backend, Decision, Error as BrowserError, Profile as BrowserProfile, Restriction,
    StateLabel, Target,
};

struct BrowserJournal {
    profiles: Profiles,
    exchange_available: bool,
    authority: Option<String>,
}

impl From<BrowserProfile> for Profile {
    fn from(profile: BrowserProfile) -> Self {
        match profile {
            BrowserProfile::Mute => Self::Mute,
            BrowserProfile::Jail => Self::Jail,
        }
    }
}

impl From<Action> for Intent {
    fn from(action: Action) -> Self {
        match action {
            Action::Apply => Self::Apply,
            Action::Replace => Self::Replace,
            Action::Revoke => Self::Revoke,
        }
    }
}

#[derive(sqlx::FromRow)]
struct Latest {
    position: i64,
    operation_id: Uuid,
    sanction_stream_id: Uuid,
    decision_revision: i64,
    state: Option<String>,
    stream_state: Option<String>,
    result_code: Option<String>,
    accepted_revision: Option<i64>,
    received_at: Option<String>,
    effective_until_ms: Option<i64>,
    platform_status: Option<String>,
    confirmed_revision: Option<i64>,
    confirmed: Option<String>,
    confirmed_until_ms: Option<i64>,
    confirmed_received_at: Option<String>,
}

async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
    character: Uuid,
    profile: BrowserProfile,
) -> Result<Option<Latest>, sqlx::Error> {
    sqlx::query_as(
        "SELECT c.position,c.operation_id,c.sanction_stream_id,c.decision_revision,r.state,
            r.body->>'stream_state' AS stream_state,r.body->>'result_code' AS result_code,
            (r.body->>'accepted_decision_revision')::bigint AS accepted_revision,
            to_char(r.received_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS received_at,
            (r.body->>'effective_until_ms')::bigint AS effective_until_ms,
            s.status AS platform_status,
            h.decision_revision AS confirmed_revision,h.state AS confirmed,
            h.effective_until_ms AS confirmed_until_ms,
            to_char(h.received_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS confirmed_received_at
         FROM (
            SELECT * FROM support_enforcement_commands
            WHERE account_id=$1 AND character_id=$2 AND profile=$3
            ORDER BY position DESC LIMIT 1
         ) c
         LEFT JOIN sanctions s ON s.id=c.sanction_stream_id
         LEFT JOIN LATERAL (
            SELECT state,received_at,convert_from(result,'UTF8')::jsonb AS body
            FROM support_enforcement_results WHERE operation_id=c.operation_id
            ORDER BY receipt_revision DESC LIMIT 1
         ) r ON true
         LEFT JOIN LATERAL (
            SELECT p.decision_revision,h.state,h.received_at,
                (convert_from(h.result,'UTF8')::jsonb->>'effective_until_ms')::bigint AS effective_until_ms
            FROM support_enforcement_commands p
            JOIN support_enforcement_results h ON h.operation_id=p.operation_id
            WHERE p.sanction_stream_id=c.sanction_stream_id
              AND p.decision_revision IN (
                CASE WHEN r.state IS DISTINCT FROM 'ACCEPTED_PENDING'
                    THEN (r.body->>'accepted_decision_revision')::bigint END,
                CASE WHEN c.intent='EXPIRE' THEN c.decision_revision
                    ELSE (convert_from(c.command,'UTF8')::jsonb->>'prior_decision_revision')::bigint END
              )
              AND h.terminal AND h.state<>'REJECTED'
            ORDER BY p.decision_revision DESC,p.position DESC LIMIT 1
         ) h ON true",
    )
    .bind(account)
    .bind(character)
    .bind(profile.name())
    .fetch_optional(&mut **tx)
    .await
}

impl Latest {
    fn actions(&self) -> &'static [Action] {
        if matches!(self.state.as_deref(), None | Some("ACCEPTED_PENDING")) {
            return &[];
        }
        if self.accepted_revision == Some(0)
            && self.stream_state.as_deref() == Some("NONE")
            && self.confirmed.is_none()
            && self.state.as_deref() == Some("REJECTED")
        {
            return &[Action::Apply];
        }
        if self.confirmed_revision.is_none() || self.confirmed_revision != self.accepted_revision {
            return &[];
        }
        match (self.stream_state.as_deref(), self.confirmed.as_deref()) {
            (Some("REVOKED"), Some("REVOKED")) | (Some("EXPIRED"), Some("EXPIRED")) => {
                &[Action::Apply]
            }
            (Some("ACTIVE"), Some("APPLIED" | "REPLACED")) => match self.platform_status.as_deref()
            {
                Some("revoked") => &[Action::Revoke],
                Some("active") => &[Action::Replace, Action::Revoke],
                _ => &[],
            },
            _ => &[],
        }
    }

    fn confirmed_state(&self) -> StateLabel {
        self.confirmed
            .as_deref()
            .map_or(StateLabel::Unconfirmed, operation_state)
    }
}

fn operation_state(state: &str) -> StateLabel {
    match state {
        "APPLIED" | "REPLACED" => StateLabel::Applied,
        "REVOKED" => StateLabel::Revoked,
        "EXPIRED" => StateLabel::Expired,
        "REJECTED" => StateLabel::Refused,
        _ => StateLabel::Pending,
    }
}

fn utc(ms: Option<i64>) -> Option<String> {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms?) * 1_000_000)
        .ok()?
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}

impl Backend for BrowserJournal {
    async fn candidate_account(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        character: Uuid,
    ) -> Result<Uuid, BrowserError> {
        let accounts = sqlx::query_scalar::<_, Uuid>(
            "SELECT c.account_id FROM account_characters c
             JOIN accounts a ON a.id=c.account_id AND a.status='active'
             WHERE c.character_id=$1 ORDER BY c.account_id LIMIT 2",
        )
        .bind(character)
        .fetch_all(&mut **tx)
        .await?;
        match accounts.as_slice() {
            [account] => Ok(*account),
            [] => Err(BrowserError::NotFound),
            _ => Err(BrowserError::Unavailable),
        }
    }

    async fn lock_resources(
        &self,
        connection: &mut sqlx::PgConnection,
    ) -> Result<(), BrowserError> {
        sqlx::query("SELECT 1 FROM account_character_feed WHERE id=1 FOR SHARE")
            .execute(connection)
            .await?;
        Ok(())
    }

    async fn target(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        character: Uuid,
    ) -> Result<Target, BrowserError> {
        let authority = self.authority.as_deref().ok_or(BrowserError::Unavailable)?;
        // This is the LCFA character-to-account lookup. Hold the same feed lock
        // as LCFA readers so a snapshot or epoch cannot change during submission.
        let epoch: Option<String> = sqlx::query_scalar(
            "SELECT CASE WHEN highest_epoch=watermark_epoch AND source_authority=$1
                AND complete_through BETWEEN floor(extract(epoch FROM clock_timestamp()))::bigint-29
                    AND floor(extract(epoch FROM clock_timestamp()))::bigint+1 THEN highest_epoch END
             FROM account_character_feed WHERE id=1 FOR SHARE",
        ).bind(authority).fetch_one(&mut **tx).await?;
        let epoch = epoch.ok_or(BrowserError::Unavailable)?;
        // Test uniqueness before using any snapshot. An invalid or missing
        // snapshot is still an ownership claim and cannot grant authority.
        let claims: Vec<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT c.account_id,c.name,
                 COALESCE(c.availability='AVAILABLE' AND NOT s.invalid
                     AND s.projection_epoch=$2 AND s.source_authority=$3,false)
             FROM account_characters c
             JOIN accounts a ON a.id=c.account_id AND a.status='active'
             LEFT JOIN account_character_snapshots s ON s.account_id=c.account_id
             WHERE c.character_id=$1 ORDER BY c.account_id LIMIT 2",
        )
        .bind(character)
        .bind(epoch)
        .bind(authority)
        .fetch_all(&mut **tx)
        .await?;
        match claims.as_slice() {
            [(account, name, true)] if super::valid_id(*account).is_ok() => Ok(Target {
                account: *account,
                character,
                name: name.clone(),
            }),
            [] => Err(BrowserError::NotFound),
            _ => Err(BrowserError::Unavailable),
        }
    }

    async fn restrictions(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target: &Target,
    ) -> Result<Vec<Restriction>, BrowserError> {
        let now = database_now(tx).await?;
        let mut restrictions = Vec::new();
        for profile in BrowserProfile::ALL {
            let mut actions: &'static [Action] = &[];
            if self.exchange_available && self.profiles.enabled(profile.into()) {
                actions = &[Action::Apply];
            }
            let mut row = Restriction {
                profile,
                version: 0,
                operation: None,
                state: StateLabel::None,
                current: StateLabel::Unconfirmed,
                expiry_pending: false,
                received_at: None,
                current_received_at: None,
                effective_until: None,
                result_code: None,
                actions,
            };
            if let Some(last) = latest(tx, target.account, target.character, profile).await? {
                row.current = last.confirmed_state();
                row.expiry_pending = matches!(row.current, StateLabel::Applied)
                    && last.confirmed_until_ms.is_some_and(|until| until <= now);
                row.actions = if actions.is_empty() {
                    &[]
                } else {
                    last.actions()
                };
                row.state = last
                    .state
                    .as_deref()
                    .map_or(StateLabel::Pending, operation_state);
                row.version = last.position;
                row.operation = Some(last.operation_id);
                row.current_received_at = last.confirmed_received_at;
                row.effective_until = utc(last.confirmed_until_ms);
                row.received_at = last.received_at;
                if matches!(row.state, StateLabel::Refused) {
                    row.result_code = last.result_code;
                }
            }
            restrictions.push(row);
        }
        Ok(restrictions)
    }

    async fn submit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target: &Target,
        input: Decision,
    ) -> Result<(), BrowserError> {
        use oteryn_admin::audit::{self, AuditContext, Entry};
        use serde_json::json;

        let profile = input.profile.into();
        if !self.exchange_available || !self.profiles.enabled(profile) {
            return Err(BrowserError::Unavailable);
        }
        lock_publication(tx).await?;
        let intent: Intent = input.action.into();
        let details = json!({
            "operation_id": input.operation,
            "character_id": target.character,
            "profile": input.profile.name(),
            "intent": intent.as_str(),
            "reason_code": input.reason.code(),
            "duration_minutes": input.minutes,
            "form_version": input.version,
        });
        // An identical retry reuses its committed decision. The audit Ref is
        // the opaque link between the private staff decision and its wire item.
        let replay: Option<bool> = sqlx::query_scalar(
            "SELECT COALESCE(a.actor=$2 AND c.account_id=$3 AND
                 a.after @> $4::jsonb,false)
             FROM support_enforcement_commands c
             LEFT JOIN audit_log a ON a.request_id=c.operation_id AND a.action='game.enforcement.requested'
             WHERE c.operation_id=$1",
        ).bind(input.operation).bind(input.actor).bind(target.account).bind(&details)
            .fetch_optional(&mut **tx).await?;
        if let Some(replay) = replay {
            return if replay {
                Ok(())
            } else {
                Err(BrowserError::Conflict)
            };
        }

        let previous = latest(tx, target.account, target.character, input.profile).await?;
        let actions = previous
            .as_ref()
            .map_or(&[Action::Apply][..], Latest::actions);
        if previous.as_ref().map_or(0, |row| row.position) != input.version
            || !actions.contains(&input.action)
        {
            return Err(BrowserError::Conflict);
        }
        let (stream, prior) = match (intent, &previous) {
            (Intent::Apply, _) => (Uuid::now_v7(), None),
            (_, Some(last)) => (last.sanction_stream_id, last.accepted_revision),
            _ => return Err(BrowserError::Conflict),
        };
        let now_ms = database_now(tx).await?;
        let expires = input
            .minutes
            .map(|minutes| now_ms + i64::from(minutes) * 60_000);
        let reason = input.reason.code();
        let mut command = Command {
            operation_id: input.operation,
            sanction_stream_id: stream,
            intent,
            profile,
            decision_revision: 1,
            prior_decision_revision: prior,
            policy_revision: 1,
            account_id: target.account,
            character_id: target.character,
            effective_at_ms: expires.map(|_| now_ms),
            expires_at_ms: expires,
            reason_code: Some(reason),
            decided_at_ms: now_ms,
            audit_ref: Uuid::nil(), // Set by the audit writer before publication.
            correlation_id: input.operation,
        };
        let revision = persist_platform_sanction(tx, &command, input.actor).await?;
        command.decision_revision = revision;
        let target_id = stream.to_string();
        let mut after = details;
        after["decision_revision"] = json!(revision);
        after["sanction_stream_id"] = json!(stream);
        after["state"] = json!("pending");
        after["effective_until_ms"] = json!(expires);
        command.audit_ref = audit::append(
            tx,
            Entry {
                context: AuditContext {
                    actor: Some(input.actor),
                    request_id: input.operation,
                },
                action: "game.enforcement.requested",
                target: &target_id,
                before: previous.as_ref().map_or(serde_json::Value::Null, |last| {
                    json!({
                        "sanction_stream_id": last.sanction_stream_id,
                        "decision_revision": last.decision_revision,
                        "receipt_state": last.state,
                        "effective_until_ms": last.effective_until_ms,
                    })
                }),
                after,
            },
        )
        .await?;
        persist_typed_decision(tx, &command).await?;
        match record(tx, &self.profiles, &command).await {
            Ok(Some(_)) => Ok(()),
            Err(Error::Conflict | Error::Stale) => Err(BrowserError::Conflict),
            _ => Err(BrowserError::Unavailable),
        }
    }
}

async fn lock_publication(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM support_enforcement_counter WHERE singleton FOR UPDATE")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn database_now(tx: &mut Transaction<'_, Postgres>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut **tx)
        .await
}

fn public_reason(code: u16, locale: &str) -> &'static str {
    match (code, locale) {
        (7, "pl") => "Zakłócanie gry",
        (7, _) => "Disruptive behavior",
        (_, "pl") => "Naruszenie zasad",
        _ => "Rule violation",
    }
}

/// The legacy-facing sanction and appeal use the same stable identity as N009
/// and the command. The typed history retains every moderator decision.
async fn persist_platform_sanction(
    tx: &mut Transaction<'_, Postgres>,
    command: &Command,
    actor: Uuid,
) -> Result<i64, BrowserError> {
    let code = command.reason_code.ok_or(BrowserError::BadInput)?;
    let locale: String = sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1")
        .bind(command.account_id)
        .fetch_one(&mut **tx)
        .await?;
    let reason = public_reason(code, &locale);
    let (starts, ends) = if command.intent == Intent::Apply {
        sqlx::query("INSERT INTO sanctions(id,account_id,category,status,public_reason,
                        effective_at,expires_at,created_by,enforcement_profile,enforcement_character_id)
                     VALUES($1,$2,'restriction','active',$3,to_timestamp($4::bigint::float8/1000),
                        to_timestamp($5::bigint::float8/1000),$6,$7,$8)")
            .bind(command.sanction_stream_id).bind(command.account_id).bind(reason)
            .bind(command.effective_at_ms).bind(command.expires_at_ms).bind(actor)
            .bind(command.profile.as_str()).bind(command.character_id)
            .execute(&mut **tx).await?;
        crate::appeals::Appeals { enabled: true }
            .register(tx, command.sanction_stream_id, command.account_id)
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        (command.effective_at_ms, command.expires_at_ms)
    } else {
        let revoke = command.intent == Intent::Revoke;
        sqlx::query_as(
            "UPDATE sanctions
             SET status=CASE WHEN $7 THEN 'revoked' ELSE 'active' END,
                 public_reason=$4, effective_at=COALESCE(to_timestamp($5::bigint::float8/1000),effective_at),
                 expires_at=COALESCE(to_timestamp($6::bigint::float8/1000),expires_at),
                 lock_version=lock_version+1,updated_at=clock_timestamp()
             WHERE id=$1 AND account_id=$2 AND enforcement_profile=$3 AND enforcement_character_id=$8
               AND ($7 OR status='active')
             RETURNING (extract(epoch FROM effective_at)*1000)::bigint,
                       (extract(epoch FROM expires_at)*1000)::bigint",
        ).bind(command.sanction_stream_id).bind(command.account_id).bind(command.profile.as_str())
            .bind(reason).bind(command.effective_at_ms).bind(command.expires_at_ms)
            .bind(revoke).bind(command.character_id)
            .fetch_optional(&mut **tx).await?.ok_or(BrowserError::Conflict)?
    };

    let kind: i16 = match command.profile {
        Profile::Mute => 1,
        Profile::Jail => 2,
    };
    sqlx::query("SELECT support_sanction_export_append($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(command.sanction_stream_id)
        .bind(command.account_id)
        .bind(command.character_id)
        .bind(kind)
        .bind(i32::from(code))
        .bind(reason)
        .bind(starts)
        .bind(ends)
        .bind(command.intent == Intent::Revoke)
        .execute(&mut **tx)
        .await?;
    export_revision(tx, command.sanction_stream_id)
        .await
        .map_err(Into::into)
}

async fn export_revision(
    tx: &mut Transaction<'_, Postgres>,
    stream: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT decision_revision FROM support_sanction_export_revisions
         WHERE sanction_stream_id=$1 ORDER BY decision_revision DESC LIMIT 1",
    )
    .bind(stream)
    .fetch_one(&mut **tx)
    .await
}

async fn persist_typed_decision(
    tx: &mut Transaction<'_, Postgres>,
    command: &Command,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO support_enforcement_decisions(sanction_id,decision_revision,operation_id,
            profile,character_id,intent,reason_code,decided_at_ms,expires_at_ms)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(command.sanction_stream_id)
    .bind(command.decision_revision)
    .bind(command.operation_id)
    .bind(command.profile.as_str())
    .bind(command.character_id)
    .bind(command.intent.as_str())
    .bind(command.reason_code.map(i32::from))
    .bind(command.decided_at_ms)
    .bind(command.expires_at_ms)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Prepare the game effect before the appeal updates N009. The caller already
/// holds the sanction owner's account and workflow locks.
pub(super) async fn prepare_appeal_revoke(
    tx: &mut Transaction<'_, Postgres>,
    stream: Uuid,
    account: Uuid,
    character: Uuid,
    profile: &str,
) -> Result<Option<Command>, oteryn_admin::support::Error> {
    use oteryn_admin::support::Error as StaffError;
    let profile = match profile {
        "MUTE" => BrowserProfile::Mute,
        "JAIL" => BrowserProfile::Jail,
        _ => return Err(StaffError::Unavailable),
    };
    lock_publication(tx)
        .await
        .map_err(|_| StaffError::Unavailable)?;
    let last = latest(tx, account, character, profile)
        .await
        .map_err(|_| StaffError::Unavailable)?
        .ok_or(StaffError::Conflict)?;
    if last.sanction_stream_id != stream || last.actions().contains(&Action::Apply) {
        // Another stream could only open after this one was confirmed closed.
        return Ok(None);
    }
    if !last.actions().contains(&Action::Revoke) {
        return Err(StaffError::Conflict);
    }
    let code: i32 = sqlx::query_scalar(
        "SELECT reason_code FROM support_sanction_export_revisions
         WHERE sanction_stream_id=$1 ORDER BY decision_revision DESC LIMIT 1",
    )
    .bind(stream)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StaffError::Unavailable)?;
    let now = database_now(tx)
        .await
        .map_err(|_| StaffError::Unavailable)?;
    let operation = Uuid::now_v7();
    Ok(Some(Command {
        operation_id: operation,
        sanction_stream_id: stream,
        intent: Intent::Revoke,
        profile: profile.into(),
        decision_revision: 1,
        prior_decision_revision: last.accepted_revision,
        policy_revision: 1,
        account_id: account,
        character_id: character,
        effective_at_ms: None,
        expires_at_ms: None,
        reason_code: Some(u16::try_from(code).map_err(|_| StaffError::Unavailable)?),
        decided_at_ms: now,
        audit_ref: Uuid::nil(),
        correlation_id: operation,
    }))
}

pub(super) async fn publish_appeal_revoke(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    mut command: Command,
    profiles: Profiles,
) -> Result<(), oteryn_admin::support::Error> {
    use oteryn_admin::{audit, support::Error as StaffError};
    use serde_json::json;
    // The existing appeal export trigger already wrote the public revision.
    command.decision_revision = export_revision(tx, command.sanction_stream_id)
        .await
        .map_err(|_| StaffError::Unavailable)?;
    command.audit_ref = audit::append(tx, audit::Entry {
        context: audit::AuditContext { actor: Some(actor), request_id: command.operation_id },
        action: "game.enforcement.appeal_revoke_requested", target: &command.sanction_stream_id.to_string(),
        before: json!({"decision_revision":command.prior_decision_revision}),
        after: json!({"operation_id":command.operation_id,"decision_revision":command.decision_revision,"state":"pending"}),
    }).await.map_err(|_| StaffError::Unavailable)?;
    persist_typed_decision(tx, &command)
        .await
        .map_err(|_| StaffError::Unavailable)?;
    record(tx, &profiles, &command)
        .await
        .map_err(|error| match error {
            Error::Conflict | Error::Stale => StaffError::Conflict,
            _ => StaffError::Unavailable,
        })?
        .ok_or(StaffError::Conflict)?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DueExpiry {
    sanction_id: Uuid,
    decision_revision: i64,
    profile: String,
    account_id: Uuid,
    character_id: Uuid,
    operation_id: Uuid,
    head_operation: Uuid,
    head_revision: i64,
    head_state: String,
}

#[derive(Debug)]
pub struct Expiry {
    pool: PgPool,
    profiles: Profiles,
}
impl Expiry {
    pub fn new(pool: PgPool, profiles: Profiles) -> Self {
        Self { pool, profiles }
    }

    async fn reconcile(&self) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        let locked: Option<bool> = sqlx::query_scalar(
            "SELECT singleton FROM support_enforcement_counter WHERE singleton FOR UPDATE SKIP LOCKED",
        ).fetch_optional(&mut *tx).await?;
        if locked.is_none() {
            return Ok(());
        }
        let now = database_now(&mut tx).await?;
        // A pending successor leaves the confirmed timer in place. When a
        // newer command is refused, A1 still prohibits publishing the older
        // revision. Report the fence without inventing a new staff decision.
        let due = sqlx::query_as::<_, DueExpiry>(
            "SELECT d.sanction_id,d.decision_revision,d.profile,c.account_id,d.character_id,
                    c.operation_id,head.operation_id AS head_operation,head.decision_revision AS head_revision,
                    terminal.state AS head_state
             FROM support_enforcement_decisions d
             JOIN support_enforcement_commands c ON c.operation_id=d.operation_id
             JOIN LATERAL (
                 SELECT operation_id,decision_revision
                 FROM support_enforcement_commands WHERE sanction_stream_id=d.sanction_id
                 ORDER BY decision_revision DESC,position DESC LIMIT 1
             ) head ON true
             JOIN support_enforcement_results terminal ON terminal.operation_id=head.operation_id AND terminal.terminal
             WHERE d.reconcile_at_ms<=$1
               AND ((d.profile='MUTE' AND $2) OR (d.profile='JAIL' AND $3))
             ORDER BY d.reconcile_at_ms,d.sanction_id LIMIT 8",
        ).bind(now).bind(self.profiles.enabled(Profile::Mute)).bind(self.profiles.enabled(Profile::Jail))
            .fetch_all(&mut *tx).await?;
        for item in due {
            sqlx::query(
                "UPDATE support_enforcement_decisions SET reconcile_at_ms=NULL
                 WHERE sanction_id=$1 AND decision_revision=$2",
            )
            .bind(item.sanction_id)
            .bind(item.decision_revision)
            .execute(&mut *tx)
            .await?;
            let stream = item.sanction_id;
            let revision = item.decision_revision;
            if item.head_operation != item.operation_id {
                if item.head_state == "REJECTED" {
                    tracing::warn!(target: "support.enforcement_expiry_revision_blocked",
                        %stream, revision, head_revision = item.head_revision,
                        "Game expiry of the prior decision is unconfirmed after a newer refusal");
                }
                continue;
            }
            let profile = Profile::parse(&item.profile).ok_or(Error::Encoding)?;
            let operation = Uuid::now_v7();
            let audit_ref = oteryn_admin::audit::append(
                &mut tx,
                oteryn_admin::audit::Entry {
                    context: oteryn_admin::audit::AuditContext {
                        actor: None,
                        request_id: operation,
                    },
                    action: "game.enforcement.expiry_requested",
                    target: &stream.to_string(),
                    before: serde_json::json!({"decision_revision":revision}),
                    after: serde_json::json!({"operation_id":operation,"state":"pending"}),
                },
            )
            .await
            .map_err(|_| Error::Encoding)?;
            let command = Command {
                operation_id: operation,
                sanction_stream_id: stream,
                intent: Intent::Expire,
                profile,
                decision_revision: revision,
                prior_decision_revision: None,
                policy_revision: 1,
                account_id: item.account_id,
                character_id: item.character_id,
                effective_at_ms: None,
                expires_at_ms: None,
                reason_code: None,
                decided_at_ms: now,
                audit_ref,
                correlation_id: operation,
            };
            record(&mut tx, &self.profiles, &command)
                .await?
                .ok_or(Error::Stale)?;
        }
        tx.commit().await?;
        Ok(())
    }
}

impl oteryn_db::outbox::Cleanup for Expiry {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move { self.reconcile().await.map_err(|error| error.to_string()) })
    }
}

#[cfg(test)]
#[path = "enforcement_tests.rs"]
mod enforcement_tests;
