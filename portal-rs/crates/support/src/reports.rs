//! Private reporting and moderation for player, content, guild and Atlas reports.
//!
//! Behavioural reference: app/Support/Actions/ManagePlayerReport.php and
//! app/Support/Http/Requests/PlayerReportRequest.php. The plan additionally requires
//! a 30-day history, a ceiling of 20 pending reports and refusal after repeated
//! unjustified reports. T33 is absent from the base; this module needs only the
//! public account and browser-session APIs, not ticket internals.

use oteryn_identity::AccountId;
use serde::Deserialize;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Transaction};
use uuid::Uuid;

pub(crate) mod web;
pub use web::{configured_router, router};

/// Deployment policy. Defaults retain the reference's stricter pending limit.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub enabled: bool,
    pub pending_limit: u8,
    /// Rejections in the last 30 days; closed reports retain their rejection date.
    pub unjustified_limit: u8,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: false,
            pending_limit: 5,
            unjustified_limit: 3,
        }
    }
}
impl Policy {
    fn valid(self) -> bool {
        (1..=20).contains(&self.pending_limit) && self.unjustified_limit > 0
    }
}

/// Untrusted form input. Private report text is deliberately excluded from Debug.
#[derive(Clone, Deserialize)]
pub struct Submission {
    pub request_key: Uuid,
    pub report_type: String,
    pub category: String,
    pub target_reference: String,
    pub evidence_summary: Option<String>,
}
impl std::fmt::Debug for Submission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Submission").finish_non_exhaustive()
    }
}

pub const TYPES: &[&str] = &["player", "content", "guild"];
pub fn categories(report_type: &str) -> &'static [&'static str] {
    match report_type {
        "player" => &["harassment", "cheating", "name", "conduct", "other"],
        "content" => &[
            "illegal_content",
            "privacy",
            "spam",
            "misinformation",
            "other",
        ],
        "guild" => &[
            "name",
            "description",
            "harassment",
            "impersonation",
            "other",
        ],
        _ => &[],
    }
}
impl Submission {
    fn validate(&self) -> Result<(), Error> {
        if self.request_key.is_nil()
            || !categories(&self.report_type).contains(&self.category.as_str())
            || self.target_reference.trim().is_empty()
            || self.target_reference.chars().count() > 160
            || self
                .evidence_summary
                .as_ref()
                .is_some_and(|v| v.chars().count() > 4000)
            || self.target_reference.contains('\0')
            || self
                .evidence_summary
                .as_ref()
                .is_some_and(|v| v.contains('\0'))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

/// Owner-safe projection: internal notes, assignee and reporter identifiers never leave storage.
#[derive(Clone, sqlx::FromRow)]
pub struct Report {
    pub id: Uuid,
    pub report_type: String,
    pub category: String,
    pub target_reference: String,
    pub evidence_summary: Option<String>,
    pub status: String,
    pub public_outcome: Option<String>,
    pub created_at: time::OffsetDateTime,
}
impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Report")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
const PROJECTION: &str = "id, report_type, category, target_reference, evidence_summary, status, public_outcome, created_at";
const PAGE_SIZE: usize = 20;
const MAX_PAGE: u32 = 10_000;

fn status_key<'a>(report_type: &str, status: &'a str) -> &'a str {
    if report_type == "atlas" && status == "actioned" {
        "resolved"
    } else {
        status
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reporting unavailable")]
    Disabled,
    #[error("invalid report")]
    Invalid,
    #[error("account access denied")]
    Forbidden,
    #[error("report not found")]
    NotFound,
    #[error("request key payload conflict")]
    Conflict,
    #[error("pending report limit reached")]
    PendingLimit,
    #[error("reporting blocked after unjustified reports")]
    UnjustifiedLimit,
    #[error("submission rate limit reached")]
    RateLimited,
    #[error("report storage unavailable")]
    Storage(#[from] sqlx::Error),
}

#[derive(Clone, Debug)]
pub struct Reports {
    pool: PgPool,
    policy: Policy,
}
impl Reports {
    pub fn new(pool: PgPool, policy: Policy) -> Self {
        Self { pool, policy }
    }
    fn available(&self) -> Result<(), Error> {
        if !self.policy.enabled || !self.policy.valid() {
            return Err(Error::Disabled);
        }
        Ok(())
    }
    // Locking the account serializes limits and idempotency across every process.
    async fn account(tx: &mut Transaction<'_, Postgres>, owner: AccountId) -> Result<(), Error> {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(owner)
                .fetch_optional(&mut **tx)
                .await?;
        match status.as_deref() {
            Some("active" | "pending_verification") => Ok(()),
            _ => Err(Error::Forbidden),
        }
    }
    pub async fn submit(&self, owner: AccountId, input: Submission) -> Result<Report, Error> {
        self.available()?;
        let mut tx = self.pool.begin().await?;
        Self::account(&mut tx, owner).await?;
        let allowed = sqlx::query_scalar::<_, i32>("INSERT INTO player_report_submission_limits (account_id, attempts) VALUES ($1,1) ON CONFLICT (account_id) DO UPDATE SET attempts=CASE WHEN player_report_submission_limits.window_started_at <= now()-interval '1 minute' THEN 1 ELSE player_report_submission_limits.attempts+1 END, window_started_at=CASE WHEN player_report_submission_limits.window_started_at <= now()-interval '1 minute' THEN now() ELSE player_report_submission_limits.window_started_at END, updated_at=now() WHERE player_report_submission_limits.window_started_at <= now()-interval '1 minute' OR player_report_submission_limits.attempts < 3 RETURNING attempts")
            .bind(owner).fetch_optional(&mut *tx).await?;
        if allowed.is_none() {
            return Err(Error::RateLimited);
        }
        if let Err(error) = input.validate() {
            tx.commit().await?;
            return Err(error);
        }
        let existing = sqlx::query_as::<_, Report>(AssertSqlSafe(format!("SELECT {PROJECTION} FROM player_reports WHERE reporter_account_id=$1 AND request_key=$2")))
            .bind(owner).bind(input.request_key).fetch_optional(&mut *tx).await?;
        // Retries return the original report even when the account has reached its quota.
        if let Some(report) = existing {
            if report.report_type != input.report_type
                || report.category != input.category
                || report.target_reference != input.target_reference.trim()
                || report.evidence_summary.as_deref()
                    != input
                        .evidence_summary
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
            {
                tx.commit().await?;
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(report);
        }
        let rejected: i64 = sqlx::query_scalar("SELECT count(*) FROM player_reports WHERE reporter_account_id=$1 AND rejected_at >= now()-interval '30 days'")
            .bind(owner).fetch_one(&mut *tx).await?;
        if rejected >= i64::from(self.policy.unjustified_limit) {
            tx.commit().await?;
            return Err(Error::UnjustifiedLimit);
        }
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM player_reports WHERE reporter_account_id=$1 AND status IN ('submitted','reviewing')")
            .bind(owner).fetch_one(&mut *tx).await?;
        if pending >= i64::from(self.policy.pending_limit) {
            tx.commit().await?;
            return Err(Error::PendingLimit);
        }
        let report = sqlx::query_as::<_, Report>(AssertSqlSafe(format!("INSERT INTO player_reports (id,reporter_account_id,request_key,report_type,category,target_reference,evidence_summary) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {PROJECTION}")))
            .bind(oteryn_core::new_id()).bind(owner).bind(input.request_key).bind(input.report_type).bind(input.category)
            .bind(input.target_reference.trim()).bind(input.evidence_summary.as_deref().map(str::trim).filter(|s| !s.is_empty()))
            .fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(report)
    }
    pub async fn list(&self, owner: AccountId, page: u32) -> Result<Vec<Report>, Error> {
        self.history_page(owner, page)
            .await
            .map(|(reports, _)| reports)
    }
    async fn history_page(
        &self,
        owner: AccountId,
        page: u32,
    ) -> Result<(Vec<Report>, bool), Error> {
        self.available()?;
        if !(1..=MAX_PAGE).contains(&page) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        Self::account(&mut tx, owner).await?;
        let mut rows = sqlx::query_as(AssertSqlSafe(format!("SELECT {PROJECTION} FROM support_report_queue WHERE reporter_account_id=$1 AND (created_at >= now()-interval '30 days' OR (report_type='atlas' AND processed_at >= now()-interval '30 days')) ORDER BY created_at DESC,id DESC LIMIT $2 OFFSET $3")))
            .bind(owner).bind((PAGE_SIZE + 1) as i64).bind(i64::from(page-1) * PAGE_SIZE as i64).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        let has_next = rows.len() > PAGE_SIZE && page < MAX_PAGE;
        rows.truncate(PAGE_SIZE);
        Ok((rows, has_next))
    }
    pub async fn get(&self, owner: AccountId, id: Uuid) -> Result<Report, Error> {
        self.available()?;
        let mut tx = self.pool.begin().await?;
        Self::account(&mut tx, owner).await?;
        let report = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT {PROJECTION} FROM support_report_queue WHERE reporter_account_id=$1 AND id=$2"
        )))
        .bind(owner)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        tx.commit().await?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests;

impl Reports {
    /// Staff domain mutation in the caller's authorization/audit transaction.
    pub(crate) async fn staff_apply(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        binding: (Uuid, i64),
        command: &oteryn_admin::support::Command,
        notifications: &oteryn_notify::Notifications,
    ) -> Result<(serde_json::Value, serde_json::Value), oteryn_admin::support::Error> {
        use oteryn_admin::support::{Command, Error as StaffError};
        self.available().map_err(|_| StaffError::NotFound)?;
        let Command::Moderate {
            status,
            public_outcome,
            moderator_notes,
        } = command
        else {
            return Err(StaffError::Invalid);
        };
        command.validate()?;
        let (id, version) = binding;
        let owner: Uuid =
            sqlx::query_scalar("SELECT reporter_account_id FROM support_report_queue WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?
                .ok_or(StaffError::NotFound)?;
        // Staff resolve existing reports even when their reporter is suspended
        // or closed; submission/owner-access admission does not govern moderation.
        let owner_state: Option<String> =
            sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(owner)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|_| StaffError::Unavailable)?;
        if !matches!(
            owner_state.as_deref(),
            Some("active" | "pending_verification" | "suspended" | "closed")
        ) {
            return Err(StaffError::Denied);
        }
        // The adapter locks the underlying report in prepare before authorization.
        let (current, current_version, report_type): (String, i64, String) = sqlx::query_as(
            "SELECT status,lock_version,report_type FROM support_report_queue WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| StaffError::Unavailable)?
        .ok_or(StaffError::NotFound)?;
        if current_version != version {
            return Err(StaffError::Conflict);
        }
        command.transition(&current)?;
        let update = if report_type == "atlas" {
            "UPDATE atlas_map_reports SET
                status=CASE $2 WHEN 'actioned' THEN 'resolved' WHEN 'rejected' THEN 'rejected' ELSE status END,
                moderation_phase=CASE WHEN $2 IN ('reviewing','closed') THEN $2 END,
                public_outcome=nullif(btrim($3),''),moderator_notes=nullif(btrim($4),''),assigned_to=$5,lock_version=lock_version+1,
                processed_at=CASE WHEN $2 IN ('actioned','rejected','closed') THEN clock_timestamp() ELSE processed_at END,
                updated_at=clock_timestamp()
              WHERE id=$1"
        } else {
            "UPDATE player_reports SET
                status=$2,public_outcome=nullif(btrim($3),''),moderator_notes=nullif(btrim($4),''),assigned_to=$5,lock_version=lock_version+1,
                processed_at=CASE WHEN $2 IN ('actioned','rejected','closed') THEN clock_timestamp() ELSE processed_at END,
                updated_at=clock_timestamp()
              WHERE id=$1"
        };
        let updated = sqlx::query(AssertSqlSafe(update))
            .bind(id)
            .bind(status)
            .bind(public_outcome)
            .bind(moderator_notes)
            .bind(actor)
            .execute(&mut **tx)
            .await
            .map_err(|_| StaffError::Unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(StaffError::NotFound);
        }
        crate::tickets::staff::notify(tx, notifications, owner, |locale| {
            let label = web::Locale::from(locale).text(status_key(&report_type, status));
            let outcome = public_outcome.trim();
            if outcome.is_empty() {
                label
            } else {
                format!("{label}: {outcome}")
            }
        })
        .await?;
        Ok((
            serde_json::json!({"status":current,"lock_version":version}),
            serde_json::json!({"status":status,"lock_version":version+1}),
        ))
    }
}
