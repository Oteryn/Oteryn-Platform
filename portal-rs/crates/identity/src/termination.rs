//! Platform account termination. No game mutation is authorized before the lifecycle gate.
//!
//! T08c is not available in the base. Its adapter must validate the authenticated actor,
//! recent sudo, CSRF and rate limit, and revoke all credential families transactionally.
//! Blocker checks must serialize with email changes, bazaar activity and wallet reserves.
//! Unknown state must return a refusal. No production adapter is assumed here.
pub mod game_closure;

use crate::{AccountId, accounts::Account, password};
use sqlx::{PgPool, Postgres, Transaction, types::time::OffsetDateTime};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;
pub type BoundaryFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

#[derive(Debug)]
pub enum Error {
    Disabled,
    InvalidInput,
    Unavailable,
    NoPending,
    Unauthorized,
    UnknownState,
    Database(sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disabled => "account termination disabled",
            Self::InvalidInput => "invalid termination input",
            Self::Unavailable => "account termination unavailable",
            Self::NoPending => "no pending termination",
            Self::Unauthorized => "termination authorization refused",
            Self::UnknownState => "termination dependencies unavailable",
            Self::Database(_) => "termination database failure",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub enabled: bool,
    pub grace_days: i32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            grace_days: 14,
        }
    }
}
impl Config {
    /// Invalid or ambiguous configuration fails closed; only an explicit true enables it.
    pub fn from_env() -> Result<Self> {
        let enabled = match std::env::var("PORTAL_ACCOUNT_TERMINATION_ENABLED") {
            Ok(value) if value == "true" => true,
            Ok(value) if value == "false" => false,
            Err(std::env::VarError::NotPresent) => false,
            _ => return Err(Error::InvalidInput),
        };
        let grace_days = match std::env::var("PORTAL_ACCOUNT_TERMINATION_GRACE_DAYS") {
            Ok(value) => value.parse().map_err(|_| Error::InvalidInput)?,
            Err(std::env::VarError::NotPresent) => 14,
            _ => return Err(Error::InvalidInput),
        };
        Self {
            enabled,
            grace_days,
        }
        .validate()
    }
    pub fn validate(self) -> Result<Self> {
        if !(1..=90).contains(&self.grace_days) {
            return Err(Error::InvalidInput);
        }
        Ok(self)
    }
}

/// Opaque request evidence interpreted only by the trusted T08c boundary adapter.
/// Never log this value or accept an account ID as proof of authentication.
pub struct ActorProof(pub String);
impl std::fmt::Debug for ActorProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ActorProof([redacted])")
    }
}

/// Every method runs under the account row lock in the lifecycle transaction.
/// Implementations must fail on missing state and use only public domain APIs.
/// Revocation includes web sessions, OAuth families, devices, MFA, passkeys,
/// recovery credentials and outstanding email/reset tokens on finalization.
pub trait Boundary: std::fmt::Debug + Send + Sync {
    /// Lifecycle hooks commit with the account transaction; never perform network I/O.
    fn requested<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn prepare<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn cancelled<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: &'a Termination,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Ok(()) })
    }

    fn authorize<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        proof: &'a ActorProof,
    ) -> BoundaryFuture<'a>;
    fn check_blockers<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
    ) -> BoundaryFuture<'a>;
    fn revoke<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: AccountId,
        finalizing: bool,
    ) -> BoundaryFuture<'a>;
}

/// Safe wiring until the owning dependency provides a complete adapter.
#[derive(Debug)]
pub struct UnavailableBoundary;
impl Boundary for UnavailableBoundary {
    fn authorize<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: AccountId,
        _: &'a ActorProof,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Err(Error::UnknownState) })
    }
    fn check_blockers<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: AccountId,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Err(Error::UnknownState) })
    }
    fn revoke<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        _: AccountId,
        _: bool,
    ) -> BoundaryFuture<'a> {
        Box::pin(async { Err(Error::UnknownState) })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Termination {
    pub id: Uuid,
    pub account_id: AccountId,
    pub scheduled_for: OffsetDateTime,
    pub cancelled_at: Option<OffsetDateTime>,
    pub finalized_at: Option<OffsetDateTime>,
}
#[derive(Debug, Clone)]
pub struct Terminations {
    pool: PgPool,
    config: Config,
    boundary: Arc<dyn Boundary>,
}
impl Terminations {
    pub fn new(pool: PgPool, config: Config, boundary: Arc<dyn Boundary>) -> Result<Self> {
        Ok(Self {
            pool,
            config: config.validate()?,
            boundary,
        })
    }
    fn enabled(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(Error::Disabled);
        }
        Ok(())
    }
    /// Repeated requests preserve the original deadline, after fresh authorization.
    pub async fn request(
        &self,
        account: AccountId,
        proof: &ActorProof,
        current_password: &str,
        confirmation: &str,
    ) -> Result<Termination> {
        self.enabled()?;
        if current_password.is_empty()
            || current_password.len() > 1024
            || confirmation != "TERMINATE"
        {
            return Err(Error::InvalidInput);
        }
        let mut tx = self.pool.begin().await?;
        let locked = lock(&mut tx, account).await?.ok_or(Error::Unavailable)?;
        if locked.status != "active" {
            return Err(Error::Unavailable);
        }
        self.boundary.authorize(&mut tx, account, proof).await?;
        verify_password(current_password, locked.password_hash).await?;
        if let Some(pending) = pending(&mut tx, account).await? {
            tx.commit().await?;
            return Ok(pending);
        }
        self.boundary.check_blockers(&mut tx, account).await?;
        let termination: Termination = sqlx::query_as("INSERT INTO account_terminations (account_id,scheduled_for) VALUES ($1,now()+make_interval(days=>$2)) RETURNING *")
            .bind(account).bind(self.config.grace_days).fetch_one(&mut *tx).await?;
        self.boundary.requested(&mut tx, &termination).await?;
        self.boundary.revoke(&mut tx, account, false).await?;
        sqlx::query("UPDATE accounts SET native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1")
            .bind(account).execute(&mut *tx).await?;
        event(&mut tx, termination.id, "requested").await?;
        tx.commit().await?;
        Ok(termination)
    }
    /// Cancellation is permitted after the deadline until a worker commits finalization.
    pub async fn cancel(
        &self,
        account: AccountId,
        proof: &ActorProof,
        current_password: &str,
    ) -> Result<()> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let locked = lock(&mut tx, account).await?.ok_or(Error::Unavailable)?;
        if !matches!(locked.status.as_str(), "active" | "suspended") {
            return Err(Error::Unavailable);
        }
        self.boundary.authorize(&mut tx, account, proof).await?;
        verify_password(current_password, locked.password_hash).await?;
        let termination = pending(&mut tx, account).await?.ok_or(Error::NoPending)?;
        self.boundary.cancelled(&mut tx, &termination).await?;
        sqlx::query(
            "UPDATE account_terminations SET cancelled_at=now(),updated_at=now() WHERE id=$1",
        )
        .bind(termination.id)
        .execute(&mut *tx)
        .await?;
        event(&mut tx, termination.id, "cancelled").await?;
        tx.commit().await?;
        Ok(())
    }
    /// Worker-only operation; retries and concurrent cancellation serialize on the account.
    async fn finalize(&self, account: AccountId) -> Result<bool> {
        self.enabled()?;
        // Persist the immutable close intent independently of proof availability. Network
        // delivery is an outbox concern outside both account-lock transactions.
        {
            let mut tx = self.pool.begin().await?;
            if let Some(locked) = lock(&mut tx, account).await?
                && let Some(termination) = pending(&mut tx, account).await?
            {
                if !matches!(locked.status.as_str(), "active" | "suspended") {
                    return Err(Error::Unavailable);
                }
                let due: bool = sqlx::query_scalar("SELECT $1::timestamptz <= now()")
                    .bind(termination.scheduled_for)
                    .fetch_one(&mut *tx)
                    .await?;
                if due {
                    self.boundary.prepare(&mut tx, &termination).await?;
                }
            }
            tx.commit().await?;
        }
        let mut tx = self.pool.begin().await?;
        let Some(locked) = lock(&mut tx, account).await? else {
            return Ok(false);
        };
        if locked.status == "closed" {
            return Ok(false);
        }
        if !matches!(locked.status.as_str(), "active" | "suspended") {
            return Err(Error::Unavailable);
        }
        let Some(termination) = pending(&mut tx, account).await? else {
            return Ok(false);
        };
        let due: bool = sqlx::query_scalar("SELECT $1::timestamptz <= now()")
            .bind(termination.scheduled_for)
            .fetch_one(&mut *tx)
            .await?;
        if !due {
            return Ok(false);
        }
        self.boundary.check_blockers(&mut tx, account).await?;
        self.boundary.revoke(&mut tx, account, true).await?;
        // Closed status and a non-PHC password prevent credential use even after a retry.
        sqlx::query("UPDATE accounts SET email=$2,password_hash='!closed',status='closed',email_verified_at=NULL,public_account_association=false,public_status_visible=false,native_security_generation=native_security_generation+1,updated_at=now() WHERE id=$1")
            .bind(account).bind(format!("terminated+{account}@invalid.oteryn")).execute(&mut *tx).await?;
        sqlx::query(
            "UPDATE account_terminations SET finalized_at=now(),updated_at=now() WHERE id=$1",
        )
        .bind(termination.id)
        .execute(&mut *tx)
        .await?;
        event(&mut tx, termination.id, "finalized").await?;
        tx.commit().await?;
        Ok(true)
    }
    /// Bounded sweep. Blocked rows stay pending and are retried by the next schedule.
    pub async fn run_worker(&self, limit: i64) -> Result<WorkerResult> {
        if !(1..=1000).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        if !self.config.enabled {
            return Ok(WorkerResult::default());
        }
        let accounts: Vec<AccountId> = sqlx::query_scalar("SELECT account_id FROM account_terminations WHERE cancelled_at IS NULL AND finalized_at IS NULL AND scheduled_for<=now() ORDER BY updated_at,account_id LIMIT $1")
            .bind(limit).fetch_all(&self.pool).await?;
        let mut result = WorkerResult::default();
        for account in accounts {
            match self.finalize(account).await {
                Ok(true) => result.finalized += 1,
                Ok(false) => {}
                Err(Error::Database(error)) => return Err(Error::Database(error)),
                Err(_) => {
                    result.blocked += 1;
                    // Rotate refused work so a full blocked batch cannot starve later rows.
                    sqlx::query("UPDATE account_terminations SET updated_at=now() WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL")
                        .bind(account).execute(&self.pool).await?;
                }
            }
        }
        Ok(result)
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WorkerResult {
    pub finalized: usize,
    pub blocked: usize,
}
impl oteryn_db::outbox::Cleanup for Terminations {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async {
            self.run_worker(100)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
    }
}
async fn verify_password(candidate: &str, hash: String) -> Result<()> {
    if candidate.is_empty() || candidate.len() > 1024 {
        return Err(Error::InvalidInput);
    }
    let candidate = candidate.to_owned();
    if !tokio::task::spawn_blocking(move || password::verify(&candidate, &hash))
        .await
        .map_err(|_| Error::Unavailable)?
    {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, account: AccountId) -> Result<Option<Account>> {
    Ok(
        sqlx::query_as("SELECT * FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut **tx)
            .await?,
    )
}
async fn pending(
    tx: &mut Transaction<'_, Postgres>,
    account: AccountId,
) -> Result<Option<Termination>> {
    Ok(sqlx::query_as("SELECT * FROM account_terminations WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL").bind(account).fetch_optional(&mut **tx).await?)
}
async fn event(tx: &mut Transaction<'_, Postgres>, id: Uuid, kind: &str) -> Result<()> {
    sqlx::query("INSERT INTO account_termination_events (termination_id,kind) VALUES ($1,$2)")
        .bind(id)
        .bind(kind)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "termination_tests.rs"]
mod tests;
