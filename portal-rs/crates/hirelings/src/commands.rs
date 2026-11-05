//! Inert G-14 issuance boundary. Only Game can decide or credit an expedition.
//! No HTTP route, worker, Game client or production authority adapter is registered.
use oteryn_db::outbox::{self, OffsetDateTime, PgPool, Uuid};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;

/// Positive canonical decimal-string integer required by v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Positive(u64);
impl Positive {
    pub fn new(value: u64) -> std::result::Result<Self, Reject> {
        if value == 0 {
            Err(Reject::InvalidInput)
        } else {
            Ok(Self(value))
        }
    }
    pub fn get(self) -> u64 {
        self.0
    }
}
impl Serialize for Positive {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Positive {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(serde::de::Error::custom("invalid positive decimal string"));
        }
        Self::new(value.parse::<u64>().map_err(serde::de::Error::custom)?)
            .map_err(serde::de::Error::custom)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    Disabled,
    InvalidInput,
    Unavailable,
    NotEligible,
    StalePrecondition,
    CharacterInSession,
    Busy,
    WeeklyLimit,
    NotReturned,
    AlreadyCollected,
    ChestFull,
    StateConflict,
    TableUnavailable,
    PurchaseBlocked,
    LegacyBoundaryUnavailable,
    IdempotencyConflict,
    RateLimited,
    BacklogFull,
}
impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hireling command refused: {self:?}")
    }
}
impl std::error::Error for Reject {}
#[derive(Debug)]
pub enum Error {
    Rejected(Reject),
    Storage(sqlx::Error),
    Encoding(serde_json::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never expose database messages, IDs or private bodies.
        match self {
            Self::Rejected(reason) => reason.fmt(f),
            Self::Storage(_) => f.write_str("hireling command storage unavailable"),
            Self::Encoding(_) => f.write_str("hireling command encoding unavailable"),
        }
    }
}
impl std::error::Error for Error {}
impl From<Reject> for Error {
    fn from(value: Reject) -> Self {
        Self::Rejected(value)
    }
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Storage(value)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Encoding(value)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// No environment mapping or runtime registration is provided.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandFlags {
    pub projection_enabled: bool,
    pub commands_enabled: bool,
    pub start_enabled: bool,
    pub collect_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartBinding {
    pub reward_table_key: String,
    pub reward_table_revision: Positive,
    pub reward_row_key: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectBinding {
    pub expedition_id: Uuid,
    pub expected_expedition_revision: Positive,
}
/// Closed command set; no purchase, cancel, reward, price or inventory selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "binding", deny_unknown_fields)]
pub enum Action {
    StartHirelingExpedition(StartBinding),
    CollectHirelingExpedition(CollectBinding),
}
impl Action {
    fn name(&self) -> &'static str {
        match self {
            Self::StartHirelingExpedition(_) => "StartHirelingExpedition",
            Self::CollectHirelingExpedition(_) => "CollectHirelingExpedition",
        }
    }
}
/// AccountId comes from authenticated server context, never request data.
/// A future browser adapter must enforce session and CSRF before calling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequest {
    pub idempotency_key: Uuid,
    pub character_id: Uuid,
    pub world_id: Uuid,
    pub hireling_id: Uuid,
    pub expected_character_revision: Positive,
    pub expected_hireling_revision: Positive,
    pub action: Action,
}
fn uuid7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == sqlx::types::uuid::Variant::RFC4122
}
fn key(value: &str) -> bool {
    let alpha = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    (1..=64).contains(&value.len())
        && alpha(value.as_bytes()[0])
        && value.bytes().all(|b| alpha(b) || b"._-".contains(&b))
}
fn authority(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
impl CommandRequest {
    pub fn validate(&self, account: Uuid) -> std::result::Result<(), Reject> {
        if ![
            account,
            self.idempotency_key,
            self.character_id,
            self.world_id,
            self.hireling_id,
        ]
        .into_iter()
        .all(uuid7)
        {
            return Err(Reject::InvalidInput);
        }
        match &self.action {
            Action::StartHirelingExpedition(b)
                if !key(&b.reward_table_key) || !key(&b.reward_row_key) =>
            {
                Err(Reject::InvalidInput)
            }
            Action::CollectHirelingExpedition(b) if !uuid7(b.expedition_id) => {
                Err(Reject::InvalidInput)
            }
            _ => Ok(()),
        }
    }
}
/// Unknown and ambiguous evidence cannot accidentally become true.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    Confirmed,
    Denied,
    #[default]
    Unknown,
    Ambiguous,
}
impl Evidence {
    fn require(self) -> std::result::Result<(), Reject> {
        match self {
            Self::Confirmed => Ok(()),
            Self::Denied => Err(Reject::NotEligible),
            Self::Unknown | Self::Ambiguous => Err(Reject::Unavailable),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedState {
    Ready(StartBinding),
    ReadyToCollect(CollectBinding),
    Running,
    Collected,
    Blocked,
    Unknown,
    Ambiguous,
}
/// Server-only evidence from independent current ownership/security and accepted
/// private Game projections. The adapter must reject hidden, poisoned, contradictory
/// or mismatched epochs and recovery fences. The minimum watermark must cover all
/// causal dependencies, including reward policy and expedition. Quote approval
/// includes the approved content allowlist: every currency reward is forbidden.
#[derive(Debug, Clone)]
pub struct Preflight {
    pub source_authority: String,
    pub account_id: Uuid,
    pub character_id: Uuid,
    pub world_id: Uuid,
    pub hireling_id: Uuid,
    pub character_revision: Positive,
    pub hireling_revision: Positive,
    pub complete_through: i64,
    pub baseline_and_recovery: Evidence,
    pub active_account_and_security: Evidence,
    pub current_ownership_and_privacy: Evidence,
    pub placed_in_owned_house: Evidence,
    pub character_offline: Evidence,
    /// Confirms consistent pinned content/policy and every non-currency reward.
    /// Start also requires an active table and unlocked profession/level fit;
    /// collect requires the retained original table, never a substitute.
    pub approved_non_currency_quote: Evidence,
    pub chest_capacity_available: Evidence,
    pub weekly_budget_available: Evidence,
    pub unfinished_account_expeditions: Option<u8>,
    pub state: ObservedState,
}
impl Preflight {
    fn authorize(
        &self,
        source: &str,
        account: Uuid,
        r: &CommandRequest,
        now: i64,
        replay: bool,
    ) -> std::result::Result<(), Reject> {
        if self.source_authority != source {
            return Err(Reject::Unavailable);
        }
        if (
            self.account_id,
            self.character_id,
            self.world_id,
            self.hireling_id,
        ) != (account, r.character_id, r.world_id, r.hireling_id)
        {
            return Err(Reject::NotEligible);
        }
        let age = now
            .checked_sub(self.complete_through)
            .ok_or(Reject::Unavailable)?;
        if !(-1..=29).contains(&age) {
            return Err(Reject::Unavailable);
        }
        self.baseline_and_recovery.require()?;
        self.active_account_and_security.require()?;
        self.current_ownership_and_privacy.require()?;
        if matches!(
            self.state,
            ObservedState::Unknown | ObservedState::Ambiguous
        ) {
            return Err(Reject::Unavailable);
        }
        // Replay never creates authority/effects. Still hide it from former owners.
        if replay {
            return Ok(());
        }
        self.placed_in_owned_house.require()?;
        match self.character_offline {
            Evidence::Denied => return Err(Reject::CharacterInSession),
            other => other.require()?,
        }
        if self.character_revision != r.expected_character_revision
            || self.hireling_revision != r.expected_hireling_revision
        {
            return Err(Reject::StalePrecondition);
        }
        self.approved_non_currency_quote.require()?;
        match (&r.action, &self.state) {
            (_, ObservedState::Unknown | ObservedState::Ambiguous) => Err(Reject::Unavailable),
            (Action::StartHirelingExpedition(b), ObservedState::Ready(observed)) => {
                if b != observed {
                    return Err(Reject::TableUnavailable);
                }
                match self.weekly_budget_available {
                    Evidence::Denied => return Err(Reject::WeeklyLimit),
                    other => other.require()?,
                }
                match self.unfinished_account_expeditions {
                    Some(0..=2) => Ok(()),
                    Some(3) => Err(Reject::Busy),
                    _ => Err(Reject::Unavailable),
                }
            }
            (Action::CollectHirelingExpedition(b), ObservedState::ReadyToCollect(observed)) => {
                if b != observed {
                    Err(Reject::StalePrecondition)
                } else {
                    match self.chest_capacity_available {
                        Evidence::Denied => Err(Reject::ChestFull),
                        other => other.require(),
                    }
                }
            }
            (
                Action::StartHirelingExpedition(_),
                ObservedState::Running | ObservedState::ReadyToCollect(_),
            ) => Err(Reject::Busy),
            (
                Action::CollectHirelingExpedition(_),
                ObservedState::Running | ObservedState::Ready(_),
            ) => Err(Reject::NotReturned),
            (Action::CollectHirelingExpedition(_), ObservedState::Collected) => {
                Err(Reject::AlreadyCollected)
            }
            _ => Err(Reject::StateConflict),
        }
    }
}
/// Not wired into the application. Never accept browser evidence or call Game
/// during rendering. This preflight cannot replace Game's atomic revalidation.
pub trait CommandAuthority: fmt::Debug + Send + Sync {
    fn inspect(
        &self,
        account: Uuid,
        request: &CommandRequest,
    ) -> std::result::Result<Preflight, Reject>;
}
#[derive(Debug)]
pub struct DenyCommands;
impl CommandAuthority for DenyCommands {
    fn inspect(&self, _: Uuid, _: &CommandRequest) -> std::result::Result<Preflight, Reject> {
        Err(Reject::Unavailable)
    }
}
/// Declaration order is the exact wire order, including the flattened binding.
#[derive(Debug, Serialize)]
struct Envelope<'a, B> {
    contract_version: u8,
    command: &'static str,
    command_semantic_version: u8,
    issuer_authority: &'static str,
    issuer_decision_id: Uuid,
    source_revision: Positive,
    operation_id: Uuid,
    account_id: Uuid,
    character_id: Uuid,
    world_id: Uuid,
    hireling_id: Uuid,
    expected_character_revision: Positive,
    expected_hireling_revision: Positive,
    #[serde(flatten)]
    binding: &'a B,
    issued_at_source: Positive,
    expires_at_source: Positive,
    audience: &'static str,
}
#[derive(Debug, Clone, Copy)]
struct Allocation {
    operation: Uuid,
    decision: Uuid,
    revision: Positive,
    now: i64,
}
fn encode<B: Serialize>(
    account: Uuid,
    r: &CommandRequest,
    binding: &B,
    allocation: Allocation,
) -> Result<String> {
    let issued = u64::try_from(allocation.now).map_err(|_| Reject::Unavailable)?;
    let expires = issued.checked_add(300).ok_or(Reject::Unavailable)?;
    Ok(serde_json::to_string(&Envelope {
        contract_version: 1,
        command: r.action.name(),
        command_semantic_version: 1,
        issuer_authority: "OTERYN_PLATFORM_HIRELING_AUTHORITY",
        issuer_decision_id: allocation.decision,
        source_revision: allocation.revision,
        operation_id: allocation.operation,
        account_id: account,
        character_id: r.character_id,
        world_id: r.world_id,
        hireling_id: r.hireling_id,
        expected_character_revision: r.expected_character_revision,
        expected_hireling_revision: r.expected_hireling_revision,
        binding,
        issued_at_source: Positive::new(issued)?,
        expires_at_source: Positive::new(expires)?,
        audience: "OTERYN_GAME_HIRELINGS",
    })?)
}
fn digest(bytes: &str) -> String {
    format!("{:x}", Sha256::digest(bytes.as_bytes()))
}
/// Issuance means pending, never gameplay success. Without an authenticated Game
/// receipt this original result remains pending, including after TTL/timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedCommand {
    pub operation_id: Uuid,
    pub source_revision: Positive,
    pub intent_bytes: String,
    pub binding_digest: String,
}
#[derive(Debug)]
pub struct Commands<A> {
    pub pool: PgPool,
    pub flags: CommandFlags,
    /// Exact configured Game source; empty/malformed configuration denies.
    pub source_authority: String,
    pub authority: A,
}
impl<A: CommandAuthority> Commands<A> {
    pub async fn enqueue(&self, account: Uuid, r: &CommandRequest) -> Result<QueuedCommand> {
        if !self.flags.commands_enabled
            || !self.flags.projection_enabled
            || !authority(&self.source_authority)
        {
            return Err(Reject::Disabled.into());
        }
        r.validate(account)?;
        let action_bytes = serde_json::to_string(r)?;
        let mut tx = self.pool.begin().await?;
        // Issuer-wide allocation lock is held through commit. A fresh intent can
        // never commit behind the discovery cursor. Also serializes quota/key races.
        let revision: String = sqlx::query_scalar(
            "SELECT revision::text FROM hireling_command_sequence WHERE singleton=true FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let old: Option<(Uuid, String, String, String, String, String)> = sqlx::query_as(
            "SELECT operation_id,action_bytes,source_revision::text,intent_bytes,binding_digest,source_authority FROM hireling_command_intents WHERE account_id=$1 AND idempotency_key=$2")
            .bind(account).bind(r.idempotency_key).fetch_optional(&mut *tx).await?;
        self.authority.inspect(account, r)?.authorize(
            &self.source_authority,
            account,
            r,
            now,
            old.is_some(),
        )?;
        if let Some((
            operation_id,
            original_action,
            source_revision,
            intent_bytes,
            binding_digest,
            original_source,
        )) = old
        {
            if original_source != self.source_authority {
                return Err(Reject::Unavailable.into());
            }
            if action_bytes != original_action {
                return Err(Reject::IdempotencyConflict.into());
            }
            let source_revision =
                Positive::new(source_revision.parse().map_err(|_| Reject::Unavailable)?)?;
            return Ok(QueuedCommand {
                operation_id,
                source_revision,
                intent_bytes,
                binding_digest,
            });
        }
        let specific_enabled = match r.action {
            Action::StartHirelingExpedition(_) => self.flags.start_enabled,
            Action::CollectHirelingExpedition(_) => self.flags.collect_enabled,
        };
        if !specific_enabled {
            return Err(Reject::Disabled.into());
        }
        let (backlog, recent, same_target): (i64, i64, bool) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE source_authority=$1), count(*) FILTER (WHERE account_id=$2 AND created_at > clock_timestamp()-interval '1 minute'), coalesce(bool_or(account_id=$2 AND hireling_id=$3),false) FROM hireling_command_intents")
            .bind(&self.source_authority).bind(account).bind(r.hireling_id).fetch_one(&mut *tx).await?;
        if backlog >= 40 {
            return Err(Reject::BacklogFull.into());
        }
        if recent >= 10 {
            return Err(Reject::RateLimited.into());
        }
        // No blind replacement or second collection while a helper is unresolved.
        if same_target {
            return Err(Reject::Busy.into());
        }
        let next = revision
            .parse::<u64>()
            .ok()
            .and_then(|v| v.checked_add(1))
            .ok_or(Reject::Unavailable)?;
        let source_revision = Positive::new(next)?;
        let operation_id = Uuid::now_v7();
        let allocation = Allocation {
            operation: operation_id,
            decision: Uuid::now_v7(),
            revision: source_revision,
            now,
        };
        let intent_bytes = match &r.action {
            Action::StartHirelingExpedition(b) => encode(account, r, b, allocation)?,
            Action::CollectHirelingExpedition(b) => encode(account, r, b, allocation)?,
        };
        let binding_digest = digest(&intent_bytes);
        sqlx::query(
            "UPDATE hireling_command_sequence SET revision=$1::text::numeric WHERE singleton=true",
        )
        .bind(next.to_string())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO hireling_command_intents(operation_id,account_id,idempotency_key,hireling_id,source_authority,source_revision,action_bytes,intent_bytes,binding_digest) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9)")
            .bind(operation_id).bind(account).bind(r.idempotency_key).bind(r.hireling_id).bind(&self.source_authority)
            .bind(next.to_string()).bind(action_bytes).bind(&intent_bytes).bind(&binding_digest).execute(&mut *tx).await?;
        outbox::enqueue(
            &mut tx,
            "hirelings.command.v1",
            &serde_json::json!({"operation_id": operation_id}).to_string(),
            &operation_id.to_string(),
            OffsetDateTime::now_utc(),
            1,
        )
        .await?;
        tx.commit().await?;
        Ok(QueuedCommand {
            operation_id,
            source_revision,
            intent_bytes,
            binding_digest,
        })
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
