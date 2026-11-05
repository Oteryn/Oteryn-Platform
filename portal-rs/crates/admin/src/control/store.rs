use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row, types::time::OffsetDateTime};
use uuid::Uuid;

macro_rules! text_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
        #[sqlx(type_name = "text")]
        pub enum $name {
            $(#[serde(rename = $wire)] #[sqlx(rename = $wire)] $variant),+
        }
        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }
        }
    };
}
text_enum!(Kind { Command => "command", Query => "query" });
text_enum!(RiskClass { R0 => "R0", R1 => "R1", R2 => "R2", R3 => "R3", R4 => "R4" });
text_enum!(Status {
    Queued => "queued", Delivered => "delivered", Accepted => "accepted",
    InProgress => "in_progress", Succeeded => "succeeded", Failed => "failed",
    Rejected => "rejected", Expired => "expired", Cancelled => "cancelled"
});
text_enum!(Decision { Approved => "approved", Rejected => "rejected" });

impl Status {
    /// Short operations may return a final result directly after delivery.
    /// Only progress may repeat; terminal states never transition again.
    pub const fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Delivered,
                Self::Accepted
                    | Self::Succeeded
                    | Self::Failed
                    | Self::Rejected
                    | Self::Expired
                    | Self::Cancelled
            ) | (
                Self::Accepted | Self::InProgress,
                Self::InProgress | Self::Succeeded | Self::Failed | Self::Cancelled
            )
        )
    }
}

/// This flag is off unless explicitly enabled. Unknown environment values deny.
#[derive(Debug, Clone, Copy, Default)]
pub struct Config {
    pub enabled: bool,
}
impl Config {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("PORTAL_ADMIN_CONTROL_ENABLED").as_deref() == Ok("true"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("admin control is disabled")]
    Disabled,
    #[error("invalid admin control input")]
    InvalidInput,
    #[error("admin operation idempotency conflict")]
    Conflict,
    #[error("admin control request not found")]
    NotFound,
    #[error("agent unavailable, superseded or ownership ambiguous")]
    Unavailable,
    #[error("admin control transition refused")]
    IllegalTransition,
    #[error("requester cannot approve their own request")]
    SelfApproval,
    #[error("admin control storage unavailable")]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub world_id: Option<String>,
    pub node_id: Option<String>,
    pub character_id: Option<String>,
    pub account_ref: Option<String>,
}
impl Target {
    fn valid(&self) -> bool {
        (self.world_id.is_some() || self.node_id.is_some())
            && [
                &self.world_id,
                &self.node_id,
                &self.character_id,
                &self.account_ref,
            ]
            .into_iter()
            .flatten()
            .all(|value| bounded(value, 256))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvertisedOperation {
    pub operation: String,
    pub operation_version: i32,
    pub kind: Kind,
}
#[derive(Debug, Clone)]
pub struct AgentHello {
    pub agent_id: String,
    pub build: String,
    pub catalog_digest: Vec<u8>,
    pub advertised_operations: Vec<AdvertisedOperation>,
    pub served_worlds: Vec<String>,
    pub served_nodes: Vec<String>,
    pub owner_epoch: i64,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Agent {
    pub agent_id: String,
    pub build: String,
    pub catalog_digest: Vec<u8>,
    pub advertised_operations: sqlx::types::Json<Vec<AdvertisedOperation>>,
    pub served_worlds: Vec<String>,
    pub served_nodes: Vec<String>,
    pub owner_epoch: i64,
    pub last_seen_at: OffsetDateTime,
    pub connected: bool,
}

/// A signed, schema-checked request supplied by the trusted signing layer.
/// Digests and canonical bytes are preserved verbatim, never recomputed from JSONB.
#[derive(Debug, Clone)]
pub struct NewRequest {
    pub request_id: Uuid,
    pub admin_operation_id: Uuid,
    pub operation: String,
    pub operation_version: i32,
    pub kind: Kind,
    pub target: Target,
    pub params: Value,
    pub params_digest: Vec<u8>,
    pub dry_run: bool,
    pub risk_class: RiskClass,
    pub actor: Uuid,
    pub reason: Value,
    pub approvals: Vec<Approval>,
    pub issued_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub nonce: Vec<u8>,
    pub signer_key_id: String,
    pub canonical_envelope: Vec<u8>,
    pub signature: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub approval_id: Uuid,
    pub approver: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub approved_at: OffsetDateTime,
    pub preview_digest: Vec<u8>,
    pub decision: Decision,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Request {
    pub request_id: Uuid,
    pub admin_operation_id: Uuid,
    pub operation: String,
    pub operation_version: i32,
    pub kind: Kind,
    pub target: sqlx::types::Json<Target>,
    pub params: Value,
    pub params_digest: Vec<u8>,
    pub dry_run: bool,
    pub risk_class: RiskClass,
    pub actor: Uuid,
    pub reason: Value,
    pub approvals: Value,
    pub issued_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub nonce: Vec<u8>,
    pub signer_key_id: String,
    pub canonical_envelope: Vec<u8>,
    pub signature: Vec<u8>,
    pub status: Status,
    pub agent_id: Option<String>,
    pub delivered_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
#[derive(Debug, Clone)]
pub struct NewResult {
    pub request_id: Uuid,
    pub agent_id: String,
    pub status: Status,
    pub game_ledger_id: Option<String>,
    pub error_code: Option<String>,
    /// Full verified result envelope, including progress or preview where present.
    pub payload: Value,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ResultRow {
    pub result_id: Uuid,
    pub request_id: Uuid,
    pub agent_id: String,
    pub status: Status,
    pub game_ledger_id: Option<String>,
    pub error_code: Option<String>,
    pub payload: Value,
    pub received_at: OffsetDateTime,
}
#[derive(Debug, Default)]
pub struct Filter {
    pub actor: Option<Uuid>,
    pub operation: Option<String>,
    pub status: Option<Status>,
    pub before: Option<(OffsetDateTime, Uuid)>,
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
    config: Config,
}
impl Store {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.config.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }

    /// Only a transport-authenticated hello may call this method. Epoch rollback
    /// and same-epoch scope changes are refused; competing owners deny at claim.
    pub async fn register_agent(&self, hello: &AgentHello) -> Result<(), Error> {
        self.enabled()?;
        if !bounded(&hello.agent_id, 128)
            || !bounded(&hello.build, 256)
            || hello.catalog_digest.len() != 32
            || hello.owner_epoch <= 0
            || hello.advertised_operations.len() > 1024
            || hello.served_worlds.len() + hello.served_nodes.len() == 0
            || hello.served_worlds.len() + hello.served_nodes.len() > 1024
            || !hello
                .served_worlds
                .iter()
                .chain(&hello.served_nodes)
                .all(|s| bounded(s, 256))
            || !hello
                .advertised_operations
                .iter()
                .all(|op| bounded(&op.operation, 128) && op.operation_version > 0)
            || hello
                .advertised_operations
                .iter()
                .enumerate()
                .any(|(i, op)| {
                    hello.advertised_operations[..i].iter().any(|other| {
                        other.operation == op.operation
                            && other.operation_version == op.operation_version
                    })
                })
        {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        ownership_lock(&mut tx, false).await?;
        let changed = sqlx::query(
            "INSERT INTO admin_control_agents (agent_id, build, catalog_digest, advertised_operations,
                served_worlds, served_nodes, owner_epoch, last_seen_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,clock_timestamp())
             ON CONFLICT (agent_id) DO UPDATE SET build=EXCLUDED.build,
                catalog_digest=EXCLUDED.catalog_digest, advertised_operations=EXCLUDED.advertised_operations,
                served_worlds=EXCLUDED.served_worlds, served_nodes=EXCLUDED.served_nodes,
                owner_epoch=EXCLUDED.owner_epoch, last_seen_at=clock_timestamp(), updated_at=clock_timestamp()
             WHERE EXCLUDED.owner_epoch > admin_control_agents.owner_epoch OR
                (EXCLUDED.owner_epoch = admin_control_agents.owner_epoch
                 AND EXCLUDED.served_worlds = admin_control_agents.served_worlds
                 AND EXCLUDED.served_nodes = admin_control_agents.served_nodes)",
        ).bind(&hello.agent_id).bind(&hello.build).bind(&hello.catalog_digest)
            .bind(sqlx::types::Json(&hello.advertised_operations)).bind(&hello.served_worlds)
            .bind(&hello.served_nodes).bind(hello.owner_epoch).execute(&mut *tx).await?.rows_affected();
        if changed == 0 {
            return Err(Error::Unavailable);
        }
        tx.commit().await?;
        Ok(())
    }

    /// The epoch comes from the authenticated agent session, not browser input.
    pub async fn heartbeat(&self, agent: &str, epoch: i64) -> Result<(), Error> {
        self.enabled()?;
        if !bounded(agent, 128) || epoch <= 0 {
            return Err(Error::InvalidInput);
        }
        let changed = sqlx::query(
            "UPDATE admin_control_agents SET last_seen_at=clock_timestamp(), updated_at=clock_timestamp()
            WHERE agent_id=$1 AND owner_epoch=$2",
        )
        .bind(agent)
        .bind(epoch)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(Error::Unavailable);
        }
        Ok(())
    }

    pub async fn enqueue(&self, input: &NewRequest) -> Result<Request, Error> {
        self.enabled()?;
        validate_request(input)?;
        let mut tx = self.pool.begin().await?;
        let inserted: Option<Request> = sqlx::query_as(
            "INSERT INTO admin_control_requests (request_id, admin_operation_id, operation, operation_version,
                kind, target, params, params_digest, dry_run, risk_class, actor, reason, approvals,
                issued_at, expires_at, nonce, signer_key_id, canonical_envelope, signature)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
             ON CONFLICT DO NOTHING RETURNING *",
        ).bind(input.request_id).bind(input.admin_operation_id).bind(&input.operation)
            .bind(input.operation_version).bind(input.kind).bind(sqlx::types::Json(&input.target))
            .bind(&input.params).bind(&input.params_digest).bind(input.dry_run).bind(input.risk_class)
            .bind(input.actor).bind(&input.reason).bind(sqlx::types::Json(&input.approvals))
            .bind(input.issued_at).bind(input.expires_at).bind(&input.nonce).bind(&input.signer_key_id)
            .bind(&input.canonical_envelope).bind(&input.signature).fetch_optional(&mut *tx).await?;
        let request = if let Some(request) = inserted {
            for approval in &input.approvals {
                sqlx::query("INSERT INTO admin_approvals (approval_id,request_id,approver,approved_at,preview_digest,decision)
                    VALUES ($1,$2,$3,$4,$5,$6)")
                    .bind(approval.approval_id).bind(input.request_id).bind(approval.approver)
                    .bind(approval.approved_at).bind(&approval.preview_digest).bind(approval.decision)
                    .execute(&mut *tx).await?;
            }
            audit(
                &mut tx,
                input.request_id,
                Some(input.actor),
                None,
                Status::Queued,
            )
            .await?;
            request
        } else {
            let existing: Request =
                sqlx::query_as("SELECT * FROM admin_control_requests WHERE admin_operation_id=$1")
                    .bind(input.admin_operation_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or(Error::Conflict)?;
            if existing.params_digest != input.params_digest
                || existing.params != input.params
                || existing.operation != input.operation
                || existing.operation_version != input.operation_version
                || existing.kind != input.kind
                || existing.target.0 != input.target
                || existing.actor != input.actor
                || existing.dry_run != input.dry_run
                || existing.risk_class != input.risk_class
            {
                return Err(Error::Conflict);
            }
            existing
        };
        tx.commit().await?;
        Ok(request)
    }

    /// Claims at most 32 once. Locks are held through delivery and audit commit;
    /// another channel skips locked rows, with no lease or silent re-enqueue.
    pub async fn claim_pending(
        &self,
        agent: &str,
        epoch: i64,
        limit: u32,
    ) -> Result<Vec<Request>, Error> {
        self.enabled()?;
        if !bounded(agent, 128) || epoch <= 0 || !(1..=32).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        ownership_lock(&mut tx, true).await?;
        let available: Option<String> = sqlx::query_scalar(
            "SELECT agent_id FROM admin_control_agents
            WHERE agent_id=$1 AND owner_epoch=$2 AND last_seen_at > clock_timestamp()-interval '30 seconds'
            AND last_seen_at <= clock_timestamp() FOR SHARE",
        )
        .bind(agent)
        .bind(epoch)
        .fetch_optional(&mut *tx)
        .await?;
        if available.is_none() {
            return Err(Error::Unavailable);
        }
        let requests: Vec<Request> = sqlx::query_as(
            "WITH pending AS (
                SELECT r.request_id FROM admin_control_requests r, admin_control_agents a
                WHERE a.agent_id=$1 AND a.owner_epoch=$2 AND r.status='queued'
                AND r.issued_at <= clock_timestamp() AND r.expires_at > clock_timestamp()
                AND a.advertised_operations @> jsonb_build_array(jsonb_build_object(
                    'operation',r.operation,'operation_version',r.operation_version,'kind',r.kind))
                AND ((r.target->>'world_id') IS NOT NULL OR (r.target->>'node_id') IS NOT NULL)
                AND ((r.target->>'world_id') IS NULL OR r.target->>'world_id'=ANY(a.served_worlds))
                AND ((r.target->>'node_id') IS NULL OR r.target->>'node_id'=ANY(a.served_nodes))
                AND NOT EXISTS (SELECT 1 FROM admin_control_agents other
                    WHERE other.agent_id<>a.agent_id AND other.owner_epoch>=a.owner_epoch
                    AND (r.target->>'world_id'=ANY(other.served_worlds)
                         OR r.target->>'node_id'=ANY(other.served_nodes)))
                ORDER BY r.issued_at,r.request_id LIMIT $3 FOR UPDATE OF r SKIP LOCKED
             ) UPDATE admin_control_requests r SET status='delivered',agent_id=$1,
                 delivered_at=clock_timestamp(),updated_at=clock_timestamp() FROM pending
                 WHERE r.request_id=pending.request_id RETURNING r.*",
        )
        .bind(agent)
        .bind(epoch)
        .bind(i64::from(limit))
        .fetch_all(&mut *tx)
        .await?;
        for request in &requests {
            audit(
                &mut tx,
                request.request_id,
                None,
                Some(Status::Queued),
                Status::Delivered,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(requests)
    }

    pub async fn record_result(&self, input: &NewResult, epoch: i64) -> Result<ResultRow, Error> {
        self.enabled()?;
        if input.request_id.is_nil()
            || !bounded(&input.agent_id, 128)
            || epoch <= 0
            || !input.payload.is_object()
            || input.payload.to_string().len() > 1048576
            || input
                .game_ledger_id
                .as_ref()
                .is_some_and(|id| !bounded(id, 256))
            || input
                .error_code
                .as_ref()
                .is_some_and(|code| !valid_error(code))
            || matches!(input.status, Status::Queued | Status::Delivered)
        {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        ownership_lock(&mut tx, true).await?;
        let request: Request =
            sqlx::query_as("SELECT * FROM admin_control_requests WHERE request_id=$1 FOR UPDATE")
                .bind(input.request_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        if request.agent_id.as_deref() != Some(&input.agent_id) {
            return Err(Error::Unavailable);
        }
        let owns: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM admin_control_agents a
            WHERE a.agent_id=$1 AND a.owner_epoch=$2
            AND ($3::text IS NULL OR $3=ANY(a.served_worlds))
            AND ($4::text IS NULL OR $4=ANY(a.served_nodes))
            AND NOT EXISTS (SELECT 1 FROM admin_control_agents other WHERE other.agent_id<>a.agent_id
                AND other.owner_epoch>=a.owner_epoch
                AND ($3::text=ANY(other.served_worlds) OR $4::text=ANY(other.served_nodes))))")
            .bind(&input.agent_id).bind(epoch).bind(&request.target.world_id).bind(&request.target.node_id)
            .fetch_one(&mut *tx).await?;
        if !owns {
            return Err(Error::Unavailable);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if request.status == Status::Delivered
            && request.expires_at <= now
            && input.status != Status::Expired
        {
            return Err(Error::IllegalTransition);
        }
        if !request.status.permits(input.status) {
            return Err(Error::IllegalTransition);
        }
        let result = sqlx::query_as("INSERT INTO admin_control_results
            (request_id,agent_id,game_ledger_id,error_code,status,payload,received_at) VALUES ($1,$2,$3,$4,$5,$6,clock_timestamp()) RETURNING *")
            .bind(input.request_id).bind(&input.agent_id).bind(&input.game_ledger_id)
            .bind(&input.error_code).bind(input.status).bind(&input.payload).fetch_one(&mut *tx).await?;
        sqlx::query(
            "UPDATE admin_control_requests SET status=$2,updated_at=clock_timestamp() WHERE request_id=$1",
        )
        .bind(input.request_id)
        .bind(input.status)
        .execute(&mut *tx)
        .await?;
        audit(
            &mut tx,
            input.request_id,
            None,
            Some(request.status),
            input.status,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Accepted work is agent-owned. Only undelivered or unacknowledged requests
    /// expire locally; neither class is ever automatically re-enqueued.
    pub async fn expire_due(&self) -> Result<u64, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT request_id,status FROM admin_control_requests
            WHERE status IN ('queued','delivered') AND expires_at<=clock_timestamp() FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await?;
        for row in &rows {
            let id: Uuid = row.try_get("request_id")?;
            let status: Status = row.try_get("status")?;
            sqlx::query("UPDATE admin_control_requests SET status='expired',updated_at=clock_timestamp() WHERE request_id=$1")
                .bind(id).execute(&mut *tx).await?;
            audit(&mut tx, id, None, Some(status), Status::Expired).await?;
        }
        tx.commit().await?;
        Ok(rows.len() as u64)
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<Request>, Error> {
        self.enabled()?;
        if id.is_nil() {
            return Err(Error::InvalidInput);
        }
        Ok(
            sqlx::query_as("SELECT * FROM admin_control_requests WHERE request_id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
    pub async fn list(&self, filter: &Filter, limit: u32) -> Result<Vec<Request>, Error> {
        self.enabled()?;
        if !(1..=100).contains(&limit)
            || filter.actor.is_some_and(|id| id.is_nil())
            || filter.operation.as_ref().is_some_and(|s| !bounded(s, 128))
        {
            return Err(Error::InvalidInput);
        }
        Ok(sqlx::query_as(
            "SELECT * FROM admin_control_requests WHERE ($1::uuid IS NULL OR actor=$1)
            AND ($2::text IS NULL OR operation=$2) AND ($3::text IS NULL OR status=$3)
            AND ($4::timestamptz IS NULL OR (created_at,request_id)<($4,$5))
            ORDER BY created_at DESC,request_id DESC LIMIT $6",
        )
        .bind(filter.actor)
        .bind(&filter.operation)
        .bind(filter.status)
        .bind(filter.before.map(|v| v.0))
        .bind(filter.before.map(|v| v.1))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }
    pub async fn results(&self, id: Uuid, limit: u32) -> Result<Vec<ResultRow>, Error> {
        self.enabled()?;
        if id.is_nil() || !(1..=100).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        Ok(sqlx::query_as(
            "SELECT * FROM admin_control_results WHERE request_id=$1
            ORDER BY received_at DESC,result_id DESC LIMIT $2",
        )
        .bind(id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?)
    }
    pub async fn agents(&self) -> Result<Vec<Agent>, Error> {
        self.enabled()?;
        Ok(sqlx::query_as("SELECT *, (last_seen_at>clock_timestamp()-interval '30 seconds' AND last_seen_at<=clock_timestamp()) AS connected
            FROM admin_control_agents ORDER BY agent_id LIMIT 1024").fetch_all(&self.pool).await?)
    }
}

fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn valid_error(s: &str) -> bool {
    s.strip_prefix("ADM_").is_some_and(|tail| {
        !tail.is_empty()
            && tail.len() <= 100
            && tail
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    })
}
fn validate_request(input: &NewRequest) -> Result<(), Error> {
    if input.request_id.get_version_num() != 7
        || input.admin_operation_id.get_version_num() != 7
        || input.actor.is_nil()
        || !bounded(&input.operation, 128)
        || input.operation_version <= 0
        || (input.kind == Kind::Query && input.admin_operation_id != input.request_id)
        || !input.target.valid()
        || !input.params.is_object()
        || input.params.to_string().len() > 1048576
        || input.params_digest.len() != 32
        || !input.reason.is_object()
        || input.reason.to_string().len() > 16384
        || input.approvals.len() > 32
        || input.expires_at <= input.issued_at
        || input.nonce.len() != 16
        || !bounded(&input.signer_key_id, 128)
        || input.canonical_envelope.is_empty()
        || input.canonical_envelope.len() > 1048576
        || input.signature.len() != 64
    {
        return Err(Error::InvalidInput);
    }
    for (i, approval) in input.approvals.iter().enumerate() {
        if approval.approver == input.actor {
            return Err(Error::SelfApproval);
        }
        if approval.approval_id.get_version_num() != 7
            || approval.approver.is_nil()
            || approval.preview_digest.len() != 32
            || approval.approved_at > input.issued_at
            || input.approvals[..i]
                .iter()
                .any(|a| a.approver == approval.approver || a.approval_id == approval.approval_id)
        {
            return Err(Error::InvalidInput);
        }
    }
    Ok(())
}
async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    actor: Option<Uuid>,
    before: Option<Status>,
    after: Status,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO audit_log (actor,action,target,before,after,request_id)
        VALUES ($1,'admin.control.transition',$2,$3,$4,$5)",
    )
    .bind(actor)
    .bind(id.to_string())
    .bind(serde_json::json!({"status":before.map(Status::as_str)}))
    .bind(serde_json::json!({"status":after.as_str()}))
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Expiry has no outbound I/O and is registered only when the feature is enabled.
#[derive(Debug)]
pub struct ExpiryJob(pub Store);
impl oteryn_db::outbox::Cleanup for ExpiryJob {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            self.0
                .expire_due()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    }
}

// Scope changes serialize with claims/results; shared locks still allow parallel
// channel claims to exercise row-level SKIP LOCKED without blocking each other.
async fn ownership_lock(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    shared: bool,
) -> Result<(), Error> {
    if shared {
        sqlx::query(
            "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-ownership',0))",
        )
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('admin-control-ownership',0))")
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
