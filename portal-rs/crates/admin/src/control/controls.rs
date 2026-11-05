//! ADM-01 control evidence. Inputs from catalog, sessions and verified results are
//! server-owned. No browser input may supply actor identity or a dry-run receipt.
use crate::{audit, rbac};
use oteryn_web::session::{BrowserSession, SessionKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::future::Future;
use uuid::Uuid;

const STEP_UP: SessionKey<StepUp> = SessionKey::new("admin_control_step_up");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reason {
    pub code: String,
    pub text: String,
    pub ticket_ref: Option<String>,
}
impl Reason {
    fn valid(&self) -> bool {
        bounded(&self.code, 64)
            && self
                .code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            && bounded(&self.text, 2000)
            && self.ticket_ref.as_ref().is_none_or(|s| bounded(s, 128))
    }
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn recent(at: i64, now: i64, seconds: i64) -> bool {
    at > 0 && at <= now && now.saturating_sub(at) < seconds
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StepUp {
    pub account: Uuid,
    pub step_up_at: i64,
}
/// Trusted authentication metadata, resolved afresh by the authority adapter.
#[derive(Debug, Clone)]
pub struct Identity {
    pub admin_id: Uuid,
    pub roles: Vec<String>,
    pub mfa_at: i64,
    pub ip_hash: String,
    pub session_ref: String,
}
/// A catalog decision must include current hello advertisement, world enablement,
/// minimum risk and exact permission; missing or ambiguous state returns a refusal.
#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub permission: String,
    pub risk: RiskClass,
}
/// Pure policy comparison snapshot. Stored-request integration derives the digest
/// from operation/version, kind, target and parameters excluding preview_digest;
/// transport/idempotency identities remain bound by the immutable signed request.
#[derive(Debug, Clone)]
pub struct Intent {
    pub operation: String,
    pub digest: String,
}
/// Trusted current operation authority. Implementations must check current
/// account status, staff MFA, session ownership,
/// operation permission and advertisement. No wildcard or fallback authorization.
pub trait Authority: Send + Sync + 'static {
    fn authorize(
        &self,
        session: &BrowserSession,
        operation: &str,
    ) -> impl Future<Output = Result<(Identity, Operation), Refusal>> + Send;
    fn staff(&self, session: &BrowserSession)
    -> impl Future<Output = Result<Uuid, Refusal>> + Send;
    fn approver(
        &self,
        admin: Uuid,
        operation: &Operation,
    ) -> impl Future<Output = Result<(), Refusal>> + Send;
}
/// Identity owns verification, secret storage and TOTP replay prevention. The
/// adapter must consume a newly verified TOTP counter, never accept a recovery code.
/// Live grants use identity's revocation, expiry and credential-generation checks.
pub trait TotpVerifier: Send + Sync + 'static {
    fn grant_valid_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: Uuid,
        grant: &str,
    ) -> impl Future<Output = Result<bool, Refusal>> + Send {
        let _ = (tx, account, grant);
        async { Err(Refusal::Unavailable) }
    }

    fn grant_valid(
        &self,
        account: oteryn_identity::AccountId,
        grant: &str,
    ) -> impl Future<Output = Result<bool, Refusal>> + Send {
        let _ = (account, grant);
        async { Err(Refusal::Unavailable) }
    }

    fn verify_totp(
        &self,
        account: oteryn_identity::AccountId,
        code: &str,
    ) -> impl Future<Output = Result<bool, Refusal>> + Send;
}
impl TotpVerifier for oteryn_identity::mfa::Mfa {
    async fn grant_valid_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: Uuid,
        grant: &str,
    ) -> Result<bool, Refusal> {
        Ok(self.satisfied_in_transaction(tx, account, grant).await?)
    }

    async fn verify_totp(&self, account: Uuid, code: &str) -> Result<bool, Refusal> {
        Ok(self.verify_totp(account, code).await?)
    }
    async fn grant_valid(&self, account: Uuid, grant: &str) -> Result<bool, Refusal> {
        Ok(self.satisfied(account, grant).await?)
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Unavailable;
impl TotpVerifier for Unavailable {
    async fn verify_totp(&self, _: oteryn_identity::AccountId, _: &str) -> Result<bool, Refusal> {
        Err(Refusal::Unavailable)
    }
}
impl Authority for Unavailable {
    async fn authorize(
        &self,
        _: &BrowserSession,
        _: &str,
    ) -> Result<(Identity, Operation), Refusal> {
        Err(Refusal::Unavailable)
    }
    async fn staff(&self, _: &BrowserSession) -> Result<Uuid, Refusal> {
        Err(Refusal::Unavailable)
    }
    async fn approver(&self, _: Uuid, _: &Operation) -> Result<(), Refusal> {
        Err(Refusal::Unavailable)
    }
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Refusal {
    #[error("control plane disabled")]
    Disabled,
    #[error("invalid control input")]
    InvalidInput,
    #[error("access denied")]
    Denied,
    #[error("step-up required")]
    StepUpRequired,
    #[error("dry run required")]
    DryRunRequired,
    #[error("second administrator approval required")]
    ApprovalRequired,
    #[error("self approval refused")]
    SelfApproval,
    #[error("approval expired")]
    StaleApproval,
    #[error("preview digest mismatch")]
    DigestMismatch,
    #[error("too many step-up attempts")]
    RateLimited,
    #[error("control evidence unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for Refusal {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
impl From<rbac::Error> for Refusal {
    fn from(_: rbac::Error) -> Self {
        Self::Unavailable
    }
}
impl From<super::store::Error> for Refusal {
    fn from(error: super::store::Error) -> Self {
        match error {
            super::store::Error::Disabled => Self::Disabled,
            super::store::Error::SelfApproval => Self::SelfApproval,
            super::store::Error::IllegalTransition | super::store::Error::NotFound => Self::Denied,
            _ => Self::Unavailable,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ApprovalState {
    pub approval_id: Uuid,
    pub admin_id: Uuid,
    pub approved_at: i64,
    pub step_up_at: i64,
    pub requester: Uuid,
    pub preview_digest: String,
    pub intent_digest: String,
}
#[derive(Debug, Serialize)]
pub struct Actor {
    pub admin_id: Uuid,
    pub roles: Vec<String>,
    pub permission: String,
    pub mfa_at: String,
    pub step_up_at: Option<String>,
    pub ip_hash: String,
    pub session_ref: String,
}
#[derive(Debug, Serialize)]
pub struct Approval {
    pub admin_id: Uuid,
    pub approved_at: String,
    pub approval_id: Uuid,
}
#[derive(Debug, Serialize)]
pub struct Evidence {
    pub actor: Actor,
    pub approvals: Vec<Approval>,
    pub reason: Option<Reason>,
}
fn timestamp(at: i64) -> Result<String, Refusal> {
    time::OffsetDateTime::from_unix_timestamp(at)
        .map_err(|_| Refusal::InvalidInput)?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| Refusal::InvalidInput)
}
/// Pure control policy for a trusted session/catalog/approval snapshot. Dry runs
/// require requester step-up and reason, but cannot require their own approval.
#[allow(clippy::too_many_arguments)]
pub fn evaluate(
    operation: &Operation,
    identity: &Identity,
    step_up: Option<StepUp>,
    reason: Option<Reason>,
    intent_digest: &str,
    preview_digest: Option<&str>,
    approval: Option<&ApprovalState>,
    dry_run: bool,
    now: i64,
) -> Result<Evidence, Refusal> {
    if identity.admin_id.is_nil()
        || !recent(identity.mfa_at, now, i64::MAX)
        || identity.roles.is_empty()
        || identity.roles.iter().any(|r| !bounded(r, 128))
        || !bounded(&identity.ip_hash, 128)
        || !bounded(&identity.session_ref, 128)
        || !bounded(&operation.name, 128)
        || !bounded(&operation.permission, 128)
        || !digest(intent_digest)
        || reason.as_ref().is_some_and(|r| !r.valid())
    {
        return Err(Refusal::InvalidInput);
    }
    if operation.risk >= RiskClass::R2 && reason.is_none() {
        return Err(Refusal::InvalidInput);
    }
    let step_up =
        step_up.filter(|s| s.account == identity.admin_id && recent(s.step_up_at, now, 300));
    if operation.risk >= RiskClass::R3 && step_up.is_none() {
        return Err(Refusal::StepUpRequired);
    }
    let mut approvals = Vec::new();
    if operation.risk == RiskClass::R4 && !dry_run {
        let supplied = preview_digest.ok_or(Refusal::DryRunRequired)?;
        if !digest(supplied) {
            return Err(Refusal::InvalidInput);
        }
        let a = approval.ok_or(Refusal::ApprovalRequired)?;
        if a.admin_id == identity.admin_id {
            return Err(Refusal::SelfApproval);
        }
        if a.admin_id.is_nil() || a.approval_id.is_nil() || a.requester != identity.admin_id {
            return Err(Refusal::Denied);
        }
        if !recent(a.approved_at, now, 1800) || !recent(a.step_up_at, a.approved_at, 300) {
            return Err(Refusal::StaleApproval);
        }
        if a.preview_digest != supplied || a.intent_digest != intent_digest {
            return Err(Refusal::DigestMismatch);
        }
        approvals.push(Approval {
            admin_id: a.admin_id,
            approved_at: timestamp(a.approved_at)?,
            approval_id: a.approval_id,
        });
    }
    Ok(Evidence {
        actor: Actor {
            admin_id: identity.admin_id,
            roles: identity.roles.clone(),
            permission: operation.permission.clone(),
            mfa_at: timestamp(identity.mfa_at)?,
            step_up_at: step_up.map(|s| timestamp(s.step_up_at)).transpose()?,
            ip_hash: identity.ip_hash.clone(),
            session_ref: identity.session_ref.clone(),
        },
        approvals,
        reason,
    })
}

#[derive(Debug)]
pub struct Controls<A, T> {
    pool: PgPool,
    authority: A,
    totp: T,
    enabled: bool,
}
impl<A: Authority, T: TotpVerifier> Controls<A, T> {
    pub fn new(pool: PgPool, authority: A, totp: T, enabled: bool) -> Self {
        Self {
            pool,
            authority,
            totp,
            enabled,
        }
    }
    fn enabled(&self) -> Result<(), Refusal> {
        if self.enabled {
            Ok(())
        } else {
            Err(Refusal::Disabled)
        }
    }
    async fn authorized(
        &self,
        session: &BrowserSession,
        name: &str,
    ) -> Result<(Identity, Operation), Refusal> {
        self.enabled()?;
        let (identity, operation) = self.authority.authorize(session, name).await?;
        if identity.admin_id.is_nil()
            || session.account_id().await != Some(identity.admin_id)
            || operation.name != name
        {
            return Err(Refusal::Denied);
        }
        Ok((identity, operation))
    }
    async fn now(&self) -> Result<i64, Refusal> {
        Ok(
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&self.pool)
                .await?,
        )
    }
    async fn step_up(&self, session: &BrowserSession, code: &str) -> Result<(), Refusal> {
        self.enabled()?;
        let account = self.authority.staff(session).await?;
        if account.is_nil() || session.account_id().await != Some(account) {
            return Err(Refusal::Denied);
        }
        let now = self.now().await?;
        // Atomic, account-wide limit persists across sessions and processes. Failed
        // verification and malformed codes consume the same five attempts/minute.
        let attempts: i32 = sqlx::query_scalar("INSERT INTO admin_control_step_up_attempts(account_id,window_start,attempts) VALUES($1,$2,1) ON CONFLICT(account_id) DO UPDATE SET window_start=CASE WHEN admin_control_step_up_attempts.window_start <= $2-60 THEN $2 ELSE admin_control_step_up_attempts.window_start END, attempts=CASE WHEN admin_control_step_up_attempts.window_start <= $2-60 THEN 1 ELSE LEAST(admin_control_step_up_attempts.attempts+1,1000000) END,updated_at=now() RETURNING attempts")
            .bind(account).bind(now).fetch_one(&self.pool).await?;
        if attempts > 5 {
            return Err(Refusal::RateLimited);
        }
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Refusal::InvalidInput);
        }
        if !self.totp.verify_totp(account, code).await? {
            return Err(Refusal::Denied);
        }
        // Recheck staff status after verification, before persisting elevation.
        if self.authority.staff(session).await? != account {
            return Err(Refusal::Denied);
        }
        let mut tx = self.pool.begin().await?;
        audit::append(
            &mut tx,
            audit::Entry {
                context: audit::AuditContext {
                    actor: Some(account),
                    request_id: Uuid::now_v7(),
                },
                action: "admin.control.totp.verified",
                target: &account.to_string(),
                before: Value::Null,
                after: json!({"verified":true}),
            },
        )
        .await?;
        tx.commit().await?;
        let until =
            time::OffsetDateTime::from_unix_timestamp(self.now().await?.saturating_add(300))
                .map_err(|_| Refusal::Unavailable)?;
        oteryn_web::session::rotate(session, oteryn_web::session::PrivilegeChange::Sudo(until))
            .await?;
        session
            .set(
                STEP_UP,
                &StepUp {
                    account,
                    step_up_at: self.now().await?,
                },
            )
            .await?;
        Ok(())
    }
    async fn preview(&self, id: Uuid) -> Result<Preview, Refusal> {
        let r = sqlx::query("SELECT id,requester,operation,permission,preview_digest,preview,reason FROM admin_control_previews WHERE id=$1").bind(id).fetch_optional(&self.pool).await?.ok_or(Refusal::DryRunRequired)?;
        Ok(Preview {
            id: r.try_get("id")?,
            requester: r.try_get("requester")?,
            operation: r.try_get("operation")?,
            permission: r.try_get("permission")?,
            preview_digest: r.try_get("preview_digest")?,
            preview: r.try_get("preview")?,
            reason: serde_json::from_value(r.try_get("reason")?)
                .map_err(|_| Refusal::Unavailable)?,
        })
    }
}
struct Preview {
    id: Uuid,
    requester: Uuid,
    operation: String,
    permission: String,
    preview_digest: String,
    preview: Value,
    reason: Reason,
}

#[path = "integration.rs"]
mod integration;
pub use integration::ControlEvidence;
pub use integration::Submission;

#[path = "controls_http.rs"]
pub mod http;
#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
