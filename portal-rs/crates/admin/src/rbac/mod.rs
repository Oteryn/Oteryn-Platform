//! Deny-by-default authorization and transactional role storage.
//!
//! The identity adapter must return current authoritative account state on every call.
//! No accounts table is assumed. The authentication layer supplies the trusted check
//! context; clients must never be allowed to set it or its MFA flags directly.

mod catalog;
pub mod guard;
mod storage;

pub use catalog::{BuiltInRole, Permission};
pub use storage::Change;

use sqlx::{PgConnection, PgPool};
use std::future::Future;
use uuid::Uuid;

/// Bound waits at the role-writer fence so blocked requests release pool capacity.
pub async fn lock_mutation(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("SET LOCAL lock_timeout = '3s'")
        .execute(&mut *connection)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(connection)
        .await?;
    Ok(())
}

/// Unavailable, missing and conflicting identity state all deny access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Disabled,
    Unknown,
    Ambiguous,
}

#[derive(Debug, Clone, Copy)]
pub struct AccountStateUnavailable;

/// Implemented by the future identity crate, or a deterministic adapter in tests.
pub trait AccountStateReader: Send + Sync {
    fn state(
        &self,
        account: Uuid,
    ) -> impl Future<Output = Result<AccountState, AccountStateUnavailable>> + Send;
    /// Database-backed adapters must read through this connection and must not
    /// acquire another pool connection while the caller holds a transaction.
    fn state_in(
        &self,
        _connection: &mut PgConnection,
        account: Uuid,
    ) -> impl Future<Output = Result<AccountState, AccountStateUnavailable>> + Send {
        self.state(account)
    }
}

/// Trusted, per-request authentication state. Staff MFA is required by default.
#[derive(Debug, Clone, Copy)]
pub struct CheckContext {
    pub account: Uuid,
    pub require_staff_mfa: bool,
    pub mfa_verified: bool,
}

impl CheckContext {
    pub fn new(account: Uuid) -> Self {
        Self {
            account,
            require_staff_mfa: true,
            mfa_verified: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("access denied")]
    Denied,
    #[error("invalid authorization input")]
    InvalidInput,
    #[error("role not found")]
    RoleNotFound,
    #[error("authorization storage unavailable")]
    Storage(#[from] sqlx::Error),
}

/// Game permissions and deployment each require explicit application enablement.
#[derive(Debug, Clone, Copy, Default)]
pub struct GamePermissionFlags {
    pub enabled: bool,
    pub deploy: bool,
}

/// Role checks never cache grants, so revocations take effect on the next check.
#[derive(Debug, Clone)]
pub struct Authorizer<A> {
    pool: PgPool,
    accounts: A,
    game_flags: GamePermissionFlags,
}

impl<A: AccountStateReader> Authorizer<A> {
    pub fn new(pool: PgPool, accounts: A) -> Self {
        Self {
            pool,
            accounts,
            game_flags: GamePermissionFlags::default(),
        }
    }

    /// Trusted application configuration only; never derive flags from request input.
    pub fn with_game_flags(mut self, flags: GamePermissionFlags) -> Self {
        self.game_flags = flags;
        self
    }

    async fn validate_account(&self, context: CheckContext) -> Result<(), Error> {
        if context.account.is_nil()
            || (context.require_staff_mfa && !context.mfa_verified)
            || !matches!(
                self.accounts.state(context.account).await,
                Ok(AccountState::Active)
            )
        {
            return Err(Error::Denied);
        }
        Ok(())
    }

    /// Checks one explicit permission. Missing roles and unknown account state deny.
    pub async fn check(&self, context: CheckContext, permission: Permission) -> Result<(), Error> {
        if permission.as_str().starts_with("game.")
            && (!self.game_flags.enabled
                || (permission == Permission::GameNodeDeploy && !self.game_flags.deploy))
        {
            return Err(Error::Denied);
        }
        self.validate_account(context).await?;
        let mut connection = self.pool.acquire().await?;
        Self::check_permission(&mut connection, context.account, permission).await
    }

    pub(crate) async fn account_active_in(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
    ) -> bool {
        !account.is_nil()
            && matches!(
                self.accounts.state_in(connection, account).await,
                Ok(AccountState::Active)
            )
    }

    /// Rechecks eligibility and grants on the mutation connection. Call after
    /// taking resource locks and the role-writer fence (71010).
    pub async fn check_in(
        &self,
        connection: &mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> Result<(), Error> {
        if permission.as_str().starts_with("game.")
            && (!self.game_flags.enabled
                || (permission == Permission::GameNodeDeploy && !self.game_flags.deploy))
        {
            return Err(Error::Denied);
        }
        if context.account.is_nil()
            || (context.require_staff_mfa && !context.mfa_verified)
            || !matches!(
                self.accounts.state_in(connection, context.account).await,
                Ok(AccountState::Active)
            )
        {
            return Err(Error::Denied);
        }
        Self::check_permission(connection, context.account, permission).await
    }

    async fn check_permission(
        connection: &mut PgConnection,
        account: Uuid,
        permission: Permission,
    ) -> Result<(), Error> {
        let granted: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM account_roles ar
                JOIN role_permissions rp ON rp.role_id = ar.role_id
                JOIN roles r ON r.id=ar.role_id
                WHERE ar.account_id = $1 AND rp.permission = $2
                AND ($2 NOT IN ('admin.analytics.view','admin.analytics.export') OR (r.key='platform_admin' AND r.built_in))
             )",
        )
        .bind(account)
        .bind(permission.as_str())
        .fetch_one(connection)
        .await?;
        if granted { Ok(()) } else { Err(Error::Denied) }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod panel_tests;

/// Request-scoped authorization using the guard's authoritative account adapter.
#[derive(Clone)]
pub struct MutationAuthorization(std::sync::Arc<dyn MutationCheck>);
impl std::fmt::Debug for MutationAuthorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutationAuthorization")
            .finish_non_exhaustive()
    }
}
trait MutationCheck: Send + Sync {
    fn check<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
}
impl<A: AccountStateReader> MutationCheck for Authorizer<A> {
    fn check<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move {
            self.check_in(connection, context, Permission::Access)
                .await?;
            self.check_in(connection, context, permission).await
        })
    }
}
impl MutationAuthorization {
    pub fn new<A: AccountStateReader + 'static>(auth: std::sync::Arc<Authorizer<A>>) -> Self {
        Self(auth)
    }
    /// Acquire after resource locks, then retain the transaction through commit.
    pub async fn check_in(
        &self,
        connection: &mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> Result<(), Error> {
        lock_mutation(connection).await?;
        self.0.check(connection, context, permission).await
    }
}
