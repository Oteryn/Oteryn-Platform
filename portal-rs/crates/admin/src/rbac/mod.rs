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

/// Role checks never cache grants, so revocations take effect on the next check.
#[derive(Debug, Clone)]
pub struct Authorizer<A> {
    pool: PgPool,
    accounts: A,
}

impl<A: AccountStateReader> Authorizer<A> {
    pub fn new(pool: PgPool, accounts: A) -> Self {
        Self { pool, accounts }
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
        self.validate_account(context).await?;
        let mut connection = self.pool.acquire().await?;
        Self::check_permission(&mut connection, context.account, permission).await
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
                WHERE ar.account_id = $1 AND rp.permission = $2
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
