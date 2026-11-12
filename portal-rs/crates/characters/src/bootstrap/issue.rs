//! Operator issuance of v3 bootstrap intents (producer contract v3). The stored
//! form is exactly what [`super::Reader`] verifies; issuance never serves a read.
use super::wire;
use crate::lcfa::AccountId;
use oteryn_internal_api::canonical::{self, Value as Wire};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

/// Upper bound of the technical expiry interval (contract: 1 through 300 seconds).
pub const MAX_TTL_SECONDS: i64 = 300;
const MAX_INTENT_BYTES: usize = 4096;
/// The staff permission an operator actor must hold (an existing Game-staff permission).
pub const ACTOR_PERMISSION: &str = "game.player.manage";
/// The Platform-local `audit_log` action of a new issuance.
pub const AUDIT_ACTION: &str = "character.bootstrap_intent.issued";

/// Who answers for an issuance. Platform-local only: the actor is never part of the
/// intent, the stored intent row or anything Game reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// A staff account, active and holding [`ACTOR_PERMISSION`], checked inside the
    /// issuance transaction. A new decision writes one `audit_log` row in that same
    /// transaction; an exact retry writes none (the first row already records it).
    Operator(AccountId),
    /// `oteryn-portal seed bootstrap-intent` (testing only): no actor, no audit row.
    TestingSeed,
}

/// The four interpretation revisions bound into the intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub profile_revision: String,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub starter_template_revision: String,
}
impl Context {
    /// Stored column and wire member names, in wire order.
    fn members(&self) -> [(&'static str, &str); 4] {
        [
            ("profile_revision", &self.profile_revision),
            ("ruleset_revision", &self.ruleset_revision),
            ("content_revision", &self.content_revision),
            ("starter_template_revision", &self.starter_template_revision),
        ]
    }
}

/// One issuance request. `identity_id` is the persisted Identity primary key; the
/// AccountId bound into the intent is read from that row, never taken from input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueInput {
    pub identity_id: String,
    pub operation_id: String,
    pub target_world_id: String,
    pub requested_name: String,
    pub requested_sex: String,
    pub context: Context,
    pub ttl_seconds: i64,
}

/// The stored decision; `intent_json` is the exact canonical intent bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issued {
    pub operation_id: String,
    pub issuer_decision_id: String,
    pub source_revision: String,
    pub account_id: String,
    pub target_world_id: String,
    pub issued_at_source: String,
    pub expires_at_source: String,
    pub intent_sha256: String,
    pub intent_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IssueError {
    #[error("bad input: {0}")]
    BadInput(String),
    #[error("operation id was issued with a different binding")]
    Conflict,
    #[error("unavailable: {0}")]
    Unavailable(&'static str),
    #[error("database unavailable")]
    Db,
}
impl From<sqlx::Error> for IssueError {
    fn from(_: sqlx::Error) -> Self {
        Self::Db
    }
}
fn bad(reason: &str) -> IssueError {
    IssueError::BadInput(reason.to_owned())
}

/// Checks every field before any transaction or source-revision allocation.
fn validate(input: &IssueInput) -> Result<AccountId, IssueError> {
    let identity = AccountId::parse(&input.identity_id)
        .map_err(|_| bad("identityId must be a lower-case UUIDv7"))?;
    wire::uuid(&input.operation_id).map_err(|_| bad("operationId must be a lower-case UUID"))?;
    wire::uuid(&input.target_world_id)
        .map_err(|_| bad("targetWorldId must be a lower-case UUID"))?;
    if !wire::name(&input.requested_name) {
        return Err(bad(
            "requestedName must be 2..=29 letters in single-spaced words",
        ));
    }
    if !matches!(input.requested_sex.as_str(), "Female" | "Male") {
        return Err(bad("requestedSex must be Female or Male"));
    }
    for (member, value) in input.context.members() {
        if !wire::revision(value) {
            return Err(IssueError::BadInput(format!(
                "{member} must match [A-Za-z0-9][A-Za-z0-9._:-]{{0,127}}"
            )));
        }
    }
    if !(1..=MAX_TTL_SECONDS).contains(&input.ttl_seconds) {
        return Err(bad("ttl must be an integer in 1..=300 seconds"));
    }
    Ok(identity)
}

/// Issues one intent, or returns the stored decision for an exact retry.
///
/// Issuance is serialized on the singleton authority row. A retained intent above
/// the authority high water, or at it with another decision identity, means the
/// authority was rolled back: issuance fails closed.
pub async fn issue(pool: &PgPool, input: IssueInput, actor: Actor) -> Result<Issued, IssueError> {
    let identity = validate(&input)?;
    let operation = Uuid::parse_str(&input.operation_id)
        .map_err(|_| bad("operationId must be a lower-case UUID"))?;
    let mut tx = pool.begin().await?;
    if let Actor::Operator(actor) = actor {
        // Same rule as the downloads operator: an active account holding the staff
        // permission. The account row stays locked until the decision commits.
        let allowed: Option<bool> = sqlx::query_scalar("SELECT a.status='active' AND EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id WHERE ar.account_id=a.id AND rp.permission=$2) FROM accounts a WHERE a.id=$1 FOR SHARE OF a")
            .bind(actor.as_uuid())
            .bind(ACTOR_PERMISSION)
            .fetch_optional(&mut *tx)
            .await?;
        if allowed != Some(true) {
            return Err(bad(
                "actor is not an active staff account with game.player.manage",
            ));
        }
    }
    // Subject resolution is a persisted primary-key lookup; the row stays locked
    // against a concurrent status change until the decision commits.
    let subject: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(identity.as_uuid())
            .fetch_optional(&mut *tx)
            .await?;
    let account = match subject {
        Some((id, status)) if status == "active" => AccountId::parse(&id.to_string())
            .map_err(|_| bad("identity is not a canonical account"))?
            .to_string(),
        _ => return Err(bad("identityId is not an active account")),
    };
    sqlx::query("INSERT INTO character_bootstrap_intent_authority (id, last_source_revision) VALUES (1, '0') ON CONFLICT (id) DO NOTHING")
        .execute(&mut *tx)
        .await?;
    let (last, last_decision): (String, Option<String>) = sqlx::query_as(
        "SELECT last_source_revision, last_issuer_decision_id FROM character_bootstrap_intent_authority WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    let high = wire::decimal(&last, 0).map_err(|_| IssueError::Unavailable("authority"))?;
    // Canonical decimals order by (length, text).
    let top: Option<(String, String)> = sqlx::query_as("SELECT source_revision, issuer_decision_id FROM character_bootstrap_intents ORDER BY length(source_revision) DESC, source_revision DESC LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?;
    if let Some((revision, decision)) = top {
        let revision =
            wire::decimal(&revision, 1).map_err(|_| IssueError::Unavailable("authority"))?;
        if revision > high
            || (revision == high && last_decision.as_deref() != Some(decision.as_str()))
        {
            return Err(IssueError::Unavailable("authority"));
        }
    }
    if let Some(row) =
        sqlx::query("SELECT * FROM character_bootstrap_intents WHERE operation_id=$1")
            .bind(&input.operation_id)
            .fetch_optional(&mut *tx)
            .await?
    {
        // Retired (v1/v2) or corrupt rows are not current: their retries fail closed.
        let raw: String = row.try_get("intent_json")?;
        wire::stored(&row, raw.as_bytes()).map_err(|_| IssueError::Unavailable("stored intent"))?;
        let mut same = row.try_get::<String, _>("account_id")? == account
            && row.try_get::<String, _>("target_world_id")? == input.target_world_id
            && row.try_get::<String, _>("requested_name")? == input.requested_name
            && row
                .try_get::<Option<String>, _>("requested_sex")?
                .as_deref()
                == Some(input.requested_sex.as_str());
        for (column, value) in input.context.members() {
            same &= row.try_get::<String, _>(column)? == value;
        }
        if !same {
            return Err(IssueError::Conflict);
        }
        tx.commit().await?;
        return issued(&row);
    }
    let next = high
        .checked_add(1)
        .ok_or(IssueError::Unavailable("authority"))?
        .to_string();
    let decision = Uuid::now_v7().to_string();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx)
            .await?;
    let expires = now
        .checked_add(input.ttl_seconds)
        .ok_or(IssueError::Unavailable("clock"))?
        .to_string();
    let now = now.to_string();
    let s = |value: &str| Wire::String(value.to_owned());
    let context = input.context.members();
    let intent = Wire::Object(vec![
        ("contract_version".into(), Wire::Unsigned(3)),
        ("variant".into(), s("OPERATOR_CONTROL_PLANE_BOOTSTRAP")),
        (
            "issuer_authority".into(),
            s("OTERYN_PLATFORM_CHARACTER_AUTHORITY"),
        ),
        ("issuer_decision_id".into(), s(&decision)),
        ("source_revision".into(), s(&next)),
        ("operation_id".into(), s(&input.operation_id)),
        ("operation".into(), s("INITIAL_CHARACTER_BOOTSTRAP")),
        ("account_id".into(), s(&account)),
        ("target_world_id".into(), s(&input.target_world_id)),
        ("requested_name".into(), s(&input.requested_name)),
        ("requested_sex".into(), s(&input.requested_sex)),
        (
            "interpretation_context".into(),
            Wire::Object(
                context
                    .iter()
                    .map(|(member, value)| ((*member).to_owned(), s(value)))
                    .collect(),
            ),
        ),
        ("issued_at_source".into(), s(&now)),
        ("expires_at_source".into(), s(&expires)),
        ("audience".into(), s("OTERYN_GAME_CHARACTER_AUTHORITY")),
    ]);
    let raw = canonical::encode(&intent).map_err(|_| IssueError::Unavailable("encoding"))?;
    if raw.len() > MAX_INTENT_BYTES {
        return Err(bad("intent exceeds 4096 bytes"));
    }
    let digest = format!("{:x}", Sha256::digest(&raw));
    let raw = String::from_utf8(raw).map_err(|_| IssueError::Unavailable("encoding"))?;
    let row = sqlx::query("INSERT INTO character_bootstrap_intents (operation_id, issuer_decision_id, source_revision, account_id, target_world_id, requested_name, requested_sex, profile_revision, ruleset_revision, content_revision, starter_template_revision, issued_at_source, expires_at_source, intent_json, intent_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING *")
        .bind(&input.operation_id).bind(&decision).bind(&next).bind(&account)
        .bind(&input.target_world_id).bind(&input.requested_name).bind(&input.requested_sex)
        .bind(context[0].1).bind(context[1].1).bind(context[2].1).bind(context[3].1)
        .bind(&now).bind(&expires).bind(&raw).bind(&digest)
        .fetch_one(&mut *tx).await?;
    // The reader must accept what was written; never commit a form it would refuse.
    wire::stored(&row, raw.as_bytes()).map_err(|_| IssueError::Unavailable("encoding"))?;
    sqlx::query("UPDATE character_bootstrap_intent_authority SET last_source_revision=$1, last_issuer_decision_id=$2 WHERE id=1")
        .bind(&next).bind(&decision).execute(&mut *tx).await?;
    if let Actor::Operator(actor) = actor {
        // Hash, TTL and subject only: never the intent body or the requested name.
        sqlx::query("INSERT INTO audit_log (actor, action, target, before, after, request_id) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(actor.as_uuid())
            .bind(AUDIT_ACTION)
            .bind(&input.operation_id)
            .bind(json!({}))
            .bind(json!({
                "identityId": input.identity_id,
                "operationId": input.operation_id,
                "sourceRevision": next,
                "ttlSeconds": input.ttl_seconds,
                "intentSha256": digest,
            }))
            .bind(operation)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    issued(&row)
}

fn issued(row: &PgRow) -> Result<Issued, IssueError> {
    let get = |key: &str| row.try_get::<String, _>(key).map_err(IssueError::from);
    Ok(Issued {
        operation_id: get("operation_id")?,
        issuer_decision_id: get("issuer_decision_id")?,
        source_revision: get("source_revision")?,
        account_id: get("account_id")?,
        target_world_id: get("target_world_id")?,
        issued_at_source: get("issued_at_source")?,
        expires_at_source: get("expires_at_source")?,
        intent_sha256: get("intent_sha256")?,
        intent_json: get("intent_json")?,
    })
}

#[cfg(test)]
#[path = "issue_tests.rs"]
mod tests;
