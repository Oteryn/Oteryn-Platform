//! Appeal workflow persistence; behavioural reference:
//! app/Support/Actions/ManageEnforcementRecord.php.
//!
//! Browser submissions lock the authoritative sanction in the workflow transaction.
//! Staff decisions use the Platform enforcement adapter for revocation, audit and notifications.
//! This module never issues game commands. See the enforcement command contract.
use sqlx::{PgConnection, Row};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct Appeals {
    /// Explicit account feature flag, disabled by default.
    pub enabled: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("feature disabled")]
    Disabled,
    #[error("not found")]
    NotFound,
    #[error("session is no longer authorized")]
    SessionRevoked,
    #[error("invalid input")]
    Invalid,
    #[error("sanction is no longer active")]
    SanctionInactive,
    #[error("stale version or invalid transition")]
    Conflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("audit unavailable")]
    AuditUnavailable,
    #[error("appeal submission deadline exceeded")]
    Deadline,
}
impl Error {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Disabled | Self::NotFound => 404,
            Self::SessionRevoked => 403,
            Self::Invalid => 422,
            Self::Conflict | Self::SanctionInactive => 409,
            Self::Database(_) | Self::AuditUnavailable | Self::Deadline => 503,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    None,
    Requested,
    Reviewing,
    Accepted,
    Rejected,
}
const SUBMITTABLE: [Status; 2] = [Status::None, Status::Rejected];

impl Status {
    fn parse(s: &str) -> Result<Self, Error> {
        match s {
            "none" => Ok(Self::None),
            "requested" => Ok(Self::Requested),
            "reviewing" => Ok(Self::Reviewing),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            _ => Err(Error::Conflict),
        }
    }
    fn wire(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Requested => "requested",
            Self::Reviewing => "reviewing",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}
/// Public-safe workflow data; moderator notes are never included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    pub sanction_id: Uuid,
    pub acknowledged: bool,
    pub status: Status,
    pub message: Option<String>,
    pub outcome: Option<String>,
    pub version: i64,
}
#[derive(Debug)]
pub enum Action<'a> {
    Acknowledge,
    Submit(&'a str),
    Decide { status: Status, outcome: &'a str },
}
fn text(s: &str, required: bool) -> Result<Option<String>, Error> {
    let s = s.trim();
    if (required && s.is_empty()) || s.chars().count() > 4000 || s.contains('\0') {
        return Err(Error::Invalid);
    }
    Ok((!s.is_empty()).then(|| s.to_owned()))
}
impl History {
    fn apply(&mut self, action: Action<'_>) -> Result<bool, Error> {
        match action {
            Action::Acknowledge => {
                if self.acknowledged {
                    return Ok(false);
                }
                self.acknowledged = true;
            }
            Action::Submit(message) => {
                let message = text(message, true)?;
                if !SUBMITTABLE.contains(&self.status) {
                    return Err(Error::Conflict);
                }
                self.message = message;
                self.outcome = None;
                self.status = Status::Requested;
            }
            Action::Decide { status, outcome } => {
                if !matches!(
                    status,
                    Status::Reviewing | Status::Accepted | Status::Rejected
                ) {
                    return Err(Error::Invalid);
                }
                let outcome = text(outcome, status != Status::Reviewing)?;
                if !matches!(self.status, Status::Requested | Status::Reviewing) {
                    return Err(Error::Conflict);
                }
                self.status = status;
                self.outcome = outcome;
            }
        }
        self.version = self.version.checked_add(1).ok_or(Error::Conflict)?;
        Ok(true)
    }
}
fn decode(row: &sqlx::postgres::PgRow) -> Result<History, Error> {
    Ok(History {
        sanction_id: row.try_get("sanction_id")?,
        acknowledged: row.try_get("acknowledged")?,
        status: Status::parse(row.try_get("status")?)?,
        message: row.try_get("message")?,
        outcome: row.try_get("outcome")?,
        version: row.try_get("version")?,
    })
}
impl Appeals {
    fn check(self) -> Result<(), Error> {
        if self.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    /// Initialize during sanction creation or while holding the authoritative sanction lock.
    pub async fn register(
        self,
        db: &mut PgConnection,
        sanction: Uuid,
        owner: Uuid,
    ) -> Result<(), Error> {
        self.check()?;
        if sanction.is_nil() || owner.is_nil() {
            return Err(Error::Invalid);
        }
        sqlx::query("INSERT INTO support_appeal_workflows (sanction_id, owner_id) VALUES ($1, $2)")
            .bind(sanction)
            .bind(owner)
            .execute(db)
            .await?;
        Ok(())
    }
    /// Owner-scoped history, newest first, with bounded pagination.
    pub async fn history(
        self,
        db: &mut PgConnection,
        owner: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<History>, Error> {
        self.check()?;
        if owner.is_nil() || !(1..=100).contains(&limit) || offset < 0 {
            return Err(Error::Invalid);
        }
        let rows = sqlx::query("SELECT * FROM support_appeal_workflows WHERE owner_id = $1 ORDER BY created_at DESC, sanction_id DESC LIMIT $2 OFFSET $3")
            .bind(owner).bind(limit).bind(offset).fetch_all(db).await?;
        rows.iter().map(decode).collect()
    }
    /// Trusted transaction seam, not an HTTP handler. Caller must authorize owner
    /// mutations or staff decisions against T34 and commit their effects atomically.
    pub(crate) async fn change(
        self,
        db: &mut PgConnection,
        owner: Uuid,
        sanction: Uuid,
        expected: i64,
        action: Action<'_>,
    ) -> Result<History, Error> {
        self.check()?;
        if expected < 1 || owner.is_nil() || sanction.is_nil() {
            return Err(Error::Invalid);
        }
        let row = sqlx::query("SELECT * FROM support_appeal_workflows WHERE sanction_id = $1 AND owner_id = $2 FOR UPDATE")
            .bind(sanction).bind(owner).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
        let mut state = decode(&row)?;
        if state.version != expected {
            return Err(Error::Conflict);
        }
        if !state.apply(action)? {
            return Ok(state);
        }
        let changed = sqlx::query("UPDATE support_appeal_workflows SET acknowledged_at = CASE WHEN $1 THEN COALESCE(acknowledged_at, now()) ELSE acknowledged_at END, status = $2, message = $3, outcome = $4, version = $5, updated_at = now() WHERE sanction_id = $6 AND owner_id = $7 AND version = $8")
            .bind(state.acknowledged).bind(state.status.wire()).bind(&state.message).bind(&state.outcome).bind(state.version).bind(sanction).bind(owner).bind(expected).execute(db).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        Ok(state)
    }
}

/// Authoritative T34 integration. Implementations must fail closed on missing or
/// ambiguous state. All calls use the workflow transaction connection.
pub trait Sanctions: Send + Sync {
    /// Verify current authenticated account access; missing or ambiguous state denies.
    /// A game/admission sanction alone must not prevent reading or appealing it.
    fn account_access(
        &self,
        db: &mut PgConnection,
        account: Uuid,
    ) -> impl std::future::Future<Output = Result<bool, Error>> + Send;

    /// Lock the sanction and verify ownership for player actions, or staff
    /// permission and MFA for decisions. The owner must match authoritative T34 data.
    fn authorize(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        owner: Uuid,
        sanction: Uuid,
        decision: bool,
    ) -> impl std::future::Future<Output = Result<bool, Error>> + Send;

    /// Persist staff audit for every decision, revoke Platform enforcement on
    /// acceptance, and enqueue terminal notifications. A game revoke/replace
    /// intent requires a newer authorized decision and its separate off-by-default
    /// enforcement flag. Any error rolls back the entire workflow mutation.
    fn decision_effects(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        owner: Uuid,
        history: &History,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}

/// Safe composition seam while T34 is absent: all access is refused.
#[derive(Debug, Default)]
pub struct UnavailableSanctions;
impl Sanctions for UnavailableSanctions {
    async fn account_access(&self, _: &mut PgConnection, _: Uuid) -> Result<bool, Error> {
        Ok(false)
    }
    async fn authorize(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: bool,
    ) -> Result<bool, Error> {
        Ok(false)
    }
    async fn decision_effects(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: &History,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }
}

/// Authenticated service. Actor IDs come from the server authentication layer,
/// never from submitted form fields. No enabled default or fallback authority.
#[derive(Debug)]
pub struct Workflow<A> {
    pub pool: sqlx::PgPool,
    pub appeals: Appeals,
    pub sanctions: A,
}
impl<A: Sanctions> Workflow<A> {
    pub async fn history(&self, actor: Uuid, page: i64) -> Result<Vec<History>, Error> {
        self.appeals.check()?;
        if actor.is_nil() || page < 0 {
            return Err(Error::Invalid);
        }
        let offset = page.checked_mul(20).ok_or(Error::Invalid)?;
        let mut tx = self.pool.begin().await?;
        if !self.sanctions.account_access(&mut tx, actor).await? {
            return Err(Error::NotFound);
        }
        let history = self.appeals.history(&mut tx, actor, 20, offset).await?;
        tx.commit().await?;
        Ok(history)
    }

    /// Owner-scoped detail; authoritative T34 state is checked on every read.
    pub async fn detail(&self, actor: Uuid, sanction: Uuid) -> Result<History, Error> {
        self.appeals.check()?;
        if actor.is_nil() || sanction.is_nil() {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        if !self.sanctions.account_access(&mut tx, actor).await?
            || !self
                .sanctions
                .authorize(&mut tx, actor, actor, sanction, false)
                .await?
        {
            return Err(Error::NotFound);
        }
        let row = sqlx::query(
            "SELECT * FROM support_appeal_workflows WHERE sanction_id = $1 AND owner_id = $2",
        )
        .bind(sanction)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let history = decode(&row)?;
        tx.commit().await?;
        Ok(history)
    }

    pub async fn change(
        &self,
        actor: Uuid,
        owner: Uuid,
        sanction: Uuid,
        expected: i64,
        action: Action<'_>,
    ) -> Result<History, Error> {
        self.appeals.check()?;
        if actor.is_nil() || owner.is_nil() || sanction.is_nil() || expected < 1 {
            return Err(Error::Invalid);
        }
        let decision = matches!(action, Action::Decide { .. });
        if !decision && actor != owner {
            return Err(Error::NotFound);
        }
        let mut tx = self.pool.begin().await?;
        if !self.sanctions.account_access(&mut tx, actor).await? {
            return Err(Error::NotFound);
        }
        if !self
            .sanctions
            .authorize(&mut tx, actor, owner, sanction, decision)
            .await?
        {
            return Err(Error::NotFound);
        }
        let history = self
            .appeals
            .change(&mut tx, owner, sanction, expected, action)
            .await?;
        if decision {
            self.sanctions
                .decision_effects(&mut tx, actor, owner, &history)
                .await?;
        }
        tx.commit().await?;
        Ok(history)
    }
}

#[cfg(test)]
mod tests;

pub mod web;

#[derive(sqlx::FromRow)]
struct Entry {
    id: Uuid,
    public_reason: String,
    status: String,
    message: Option<String>,
    outcome: Option<String>,
    version: i64,
    can_submit: bool,
}

macro_rules! active_sanction {
    () => {
        "s.status = 'active' AND s.effective_at <= clock_timestamp()
            AND (s.expires_at IS NULL OR s.expires_at > clock_timestamp())"
    };
}

async fn player_entries(
    pool: &sqlx::PgPool,
    actor: Uuid,
    id: Option<Uuid>,
    offset: i64,
) -> Result<Vec<Entry>, Error> {
    // A completed or pending appeal stays visible when the sanction ends.
    let query = concat!(
        "SELECT s.id, s.public_reason, coalesce(a.status, 'none') AS status,
                a.message, a.outcome, coalesce(a.version, 1) AS version,
                (",
        active_sanction!(),
        " AND coalesce(a.status, 'none') = ANY($4)) AS can_submit
         FROM sanctions s
         LEFT JOIN support_appeal_workflows a ON a.sanction_id = s.id AND a.owner_id = s.account_id
         WHERE s.account_id = $1 AND ($2::uuid IS NULL OR s.id = $2)
           AND ((",
        active_sanction!(),
        ") OR a.status <> 'none')
         ORDER BY s.created_at DESC, s.id DESC LIMIT 21 OFFSET $3"
    );
    Ok(sqlx::query_as(query)
        .bind(actor)
        .bind(id)
        .bind(offset)
        .bind(SUBMITTABLE.map(Status::wire).as_slice())
        .fetch_all(pool)
        .await?)
}

async fn browser_session_expiry<'e>(
    db: impl sqlx::PgExecutor<'e>,
    actor: Uuid,
    session_hash: &[u8],
) -> Result<Option<sqlx::types::time::OffsetDateTime>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT least(s.idle_expires_at, s.absolute_expires_at)
            FROM web_sessions s JOIN accounts a ON a.id = s.account_id
            WHERE s.id_hash = $1 AND s.account_id = $2
              AND a.status IN ('active', 'suspended')
              AND s.authorization_generation = a.native_security_generation
              AND s.idle_expires_at > clock_timestamp()
              AND s.absolute_expires_at > clock_timestamp()",
    )
    .bind(session_hash)
    .bind(actor)
    .fetch_optional(db)
    .await
}

async fn submit_player(
    pool: &sqlx::PgPool,
    actor: Uuid,
    session_hash: &[u8],
    id: Uuid,
    version: i64,
    message: &str,
    deadline: &oteryn_web::session::SessionMutationDeadline,
) -> Result<(), Error> {
    use oteryn_admin::audit::{self, AuditContext};
    use serde_json::json;

    if id.is_nil() || version < 1 {
        return Err(Error::Invalid);
    }
    text(message, true)?;
    let mut tx = deadline.begin(pool).await?;
    sqlx::query("SELECT id FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(actor)
        .execute(&mut *tx)
        .await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    // Lock the exact session first, then evaluate its eligibility using the live clock.
    sqlx::query(
        "SELECT id_hash FROM web_sessions WHERE id_hash = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(session_hash)
    .bind(actor)
    .execute(&mut *tx)
    .await?;
    if browser_session_expiry(&mut *tx, actor, session_hash)
        .await?
        .is_none()
    {
        return Err(Error::SessionRevoked);
    }
    oteryn_db::mutation_budget(&mut tx).await?;
    let query = concat!(
        "WITH locked AS MATERIALIZED (
            SELECT status, effective_at, expires_at FROM sanctions
            WHERE id = $1 AND account_id = $2 FOR UPDATE
         )
         SELECT (",
        active_sanction!(),
        ") FROM locked s"
    );
    let active: bool = sqlx::query_scalar(query)
        .bind(id)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
    if !active {
        return Err(Error::SanctionInactive);
    }
    oteryn_db::mutation_budget(&mut tx).await?;
    let state: Option<String> = sqlx::query_scalar(
        "SELECT status FROM support_appeal_workflows WHERE sanction_id = $1 AND owner_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(&mut *tx)
    .await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    let appeals = Appeals { enabled: true };
    let prior = state
        .as_deref()
        .map(Status::parse)
        .transpose()?
        .unwrap_or(Status::None);
    if state.is_none() {
        appeals.register(&mut tx, id, actor).await?;
    }
    oteryn_db::mutation_budget(&mut tx).await?;
    let history = appeals
        .change(&mut tx, actor, id, version, Action::Submit(message))
        .await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    audit::append(
        &mut tx,
        audit::Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action: "support.enforcement_appeal_submitted",
            target: &id.to_string(),
            before: json!({"status": prior.wire(), "version": version}),
            after: json!({"status": history.status.wire(), "version": history.version}),
        },
    )
    .await
    .map_err(|_| Error::AuditUnavailable)?;
    oteryn_db::mutation_budget(&mut tx).await?;
    let expiry = browser_session_expiry(&mut *tx, actor, session_hash)
        .await?
        .ok_or(Error::SessionRevoked)?;
    if !oteryn_db::transaction::arm_commit(&mut tx, Some(expiry)).await? {
        return Err(Error::Deadline);
    }
    tx.commit().await?;
    Ok(())
}
