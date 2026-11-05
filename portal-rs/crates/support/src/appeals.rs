//! Appeal workflow persistence; behavioural reference:
//! app/Support/Actions/ManageEnforcementRecord.php.
//!
//! T34 is not available. Its adapter must lock and authorize the authoritative
//! sanction in the same transaction, revoke Platform enforcement on acceptance,
//! and write staff audit and notification outbox entries before committing.
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
    #[error("invalid input")]
    Invalid,
    #[error("stale version or invalid transition")]
    Conflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
impl Error {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Disabled | Self::NotFound => 404,
            Self::Invalid => 422,
            Self::Conflict => 409,
            Self::Database(_) => 503,
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
                if !matches!(self.status, Status::None | Status::Rejected) {
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
    /// Trusted T34 integration seam. Initialize only alongside sanction creation.
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
    async fn change(
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
