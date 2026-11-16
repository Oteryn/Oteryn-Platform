//! Deny-by-default authorization and transactional role storage.
//!
//! Mutations retain canonical account/resource/fence locks while durable session,
//! MFA and permission checks run on the same transaction connection. Authentication
//! context and proof hashes are supplied only by trusted session middleware.

mod catalog;
pub mod guard;
mod storage;

pub use catalog::{BuiltInRole, Permission};
pub use storage::Change;

use sqlx::{PgConnection, PgPool};
use std::future::Future;
use uuid::Uuid;

/// Establish a transaction-wide three second acquisition deadline before the
/// first lock. Repeated calls retain the original deadline.
pub async fn mutation_budget(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('oteryn.mutation_deadline', coalesce(nullif(current_setting('oteryn.mutation_deadline',true),''),(extract(epoch FROM clock_timestamp())+3)::text),true)")
        .execute(&mut *connection).await?;
    sqlx::query("SELECT set_config('lock_timeout', greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint::text || 'ms',true), set_config('statement_timeout', greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint::text || 'ms',true)")
        .execute(connection).await?;
    Ok(())
}

/// The canonical staff lock protocol: ordered account rows, then RBAC. All
/// callers use their existing transaction and share one acquisition deadline.
pub async fn lock_accounts_and_mutation(
    connection: &mut PgConnection,
    accounts: &[Uuid],
) -> Result<(), sqlx::Error> {
    lock_accounts_resources_and_mutation(connection, accounts, |connection| {
        Box::pin(async move {
            let _ = connection;
            Ok(())
        })
    })
    .await
}

/// Acquire domain resources after accounts and before the global fence. The
/// callback must only acquire bounded database resources on this connection.
pub async fn lock_accounts_resources_and_mutation<F>(
    connection: &mut PgConnection,
    accounts: &[Uuid],
    resources: F,
) -> Result<(), sqlx::Error>
where
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>,
    >,
{
    lock_accounts_resources_and_mutation_with(connection, accounts, resources).await
}

/// The same protocol preserving an owning domain's refusal error type.
pub async fn lock_accounts_resources_and_mutation_with<E, F>(
    connection: &mut PgConnection,
    accounts: &[Uuid],
    resources: F,
) -> Result<(), E>
where
    E: From<sqlx::Error>,
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), E>> + Send + 'a>>,
{
    mutation_budget(&mut *connection).await?;
    sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(accounts)
        .execute(&mut *connection)
        .await?;
    lock_mutation_resources_with(connection, resources).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *connection)
        .await?;
    mutation_budget(connection).await?;
    Ok(())
}

/// Acquire domain resources within the same transaction-wide budget used by
/// account/fence acquisition. Domains whose global order puts resources first
/// use this before the account/fence helper; it never acquires a separate fence.
pub async fn lock_mutation_resources_with<E, F>(
    connection: &mut PgConnection,
    resources: F,
) -> Result<(), E>
where
    E: From<sqlx::Error>,
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), E>> + Send + 'a>>,
{
    mutation_budget(&mut *connection).await?;
    let remaining: i64 = sqlx::query_scalar("SELECT greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint")
        .fetch_one(&mut *connection).await?;
    tokio::time::timeout(
        std::time::Duration::from_millis(remaining as u64),
        resources(&mut *connection),
    )
    .await
    .map_err(|_| {
        E::from(sqlx::Error::Io(std::io::Error::from(
            std::io::ErrorKind::TimedOut,
        )))
    })??;
    mutation_budget(&mut *connection).await?;
    Ok(())
}

/// Fence-only callers (including tests) still use the same deadline protocol.
pub async fn lock_mutation(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    lock_accounts_and_mutation(connection, &[]).await
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

/// Current account and session authority, supplied by identity or an explicit test adapter.
pub trait AccountStateReader: Send + Sync {
    fn state(
        &self,
        account: Uuid,
    ) -> impl Future<Output = Result<AccountState, AccountStateUnavailable>> + Send;
    fn authenticated_in(
        &self,
        connection: &mut PgConnection,
        context: CheckContext,
    ) -> impl Future<Output = Result<bool, AccountStateUnavailable>> + Send {
        async move {
            if let Some(proof) = context.session_proof {
                durable_session_in(connection, context.account, proof).await
            } else {
                Ok(false)
            }
        }
    }
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

/// Revalidate session lifetime, security generation, MFA enrollment and the
/// current password-bound grant on the caller's connection.
pub async fn durable_session_in(
    connection: &mut PgConnection,
    account: Uuid,
    proof: SessionProof,
) -> Result<bool, AccountStateUnavailable> {
    use sha2::{Digest, Sha256};
    let password: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_optional(&mut *connection)
            .await
            .map_err(|_| AccountStateUnavailable)?;
    let Some(password) = password else {
        return Ok(false);
    };
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions s JOIN accounts a ON a.id=s.account_id JOIN account_mfa_totp m ON m.account_id=a.id JOIN account_mfa_tokens t ON t.account_id=a.id AND t.generation=m.generation WHERE a.id=$1 AND a.status='active' AND a.email_verified_at IS NOT NULL AND s.id_hash=$2 AND sha256(convert_to(s.data->>'mfa_grant','UTF8'))=$3 AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp() AND s.authorization_generation=a.native_security_generation AND m.confirmed AND t.token_hash=$3 AND t.purpose='grant' AND t.expires_at>clock_timestamp() AND t.created_at<=clock_timestamp() AND t.credential_hash=$4 FOR UPDATE OF s,m,t)")
        .bind(account).bind(proof.session_hash.as_slice()).bind(proof.grant_hash.as_slice())
        .bind(Sha256::digest(password.as_bytes()).to_vec()).fetch_one(connection).await.map_err(|_| AccountStateUnavailable)
}

/// Trusted, per-request authentication state. Staff MFA is required by default.
#[derive(Clone, Copy)]
pub struct CheckContext {
    pub account: Uuid,
    pub require_staff_mfa: bool,
    pub mfa_verified: bool,
    pub session_proof: Option<SessionProof>,
}

/// Durable identities captured by trusted session middleware, rechecked under
/// the actor account lock. Debug deliberately omits proof material.
#[derive(Clone, Copy)]
pub struct SessionProof {
    pub session_hash: [u8; 32],
    pub grant_hash: [u8; 32],
}
impl std::fmt::Debug for SessionProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionProof").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for CheckContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckContext")
            .field("account", &self.account)
            .finish_non_exhaustive()
    }
}

impl CheckContext {
    pub fn new(account: Uuid) -> Self {
        Self {
            account,
            require_staff_mfa: true,
            mfa_verified: false,
            session_proof: None,
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

    pub(crate) async fn account_active(&self, account: Uuid) -> bool {
        matches!(self.accounts.state(account).await, Ok(AccountState::Active))
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
            || !self
                .accounts
                .authenticated_in(&mut *connection, context)
                .await
                .unwrap_or(false)
            || !matches!(
                self.accounts.state_in(connection, context.account).await,
                Ok(AccountState::Active)
            )
        {
            return Err(Error::Denied);
        }
        Self::check_permission(connection, context.account, permission).await
    }

    /// Recheck a workflow proposer's current account and grants. The proposer
    /// is a subject of approval, never the authenticated actor of this request.
    pub(crate) async fn check_subject_in(
        &self,
        connection: &mut PgConnection,
        account: Uuid,
        permission: Permission,
    ) -> Result<(), Error> {
        if !self.account_active_in(&mut *connection, account).await {
            return Err(Error::Denied);
        }
        Self::check_permission(connection, account, permission).await
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
/// Trusted transaction-local authority used by the canonical mutation protocol.
pub trait MutationCheck: Send + Sync {
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
        Box::pin(async move { self.check_in(connection, context, permission).await })
    }
}
impl MutationAuthorization {
    pub fn new<A: AccountStateReader + 'static>(auth: std::sync::Arc<Authorizer<A>>) -> Self {
        Self(auth)
    }
    /// Acquire actor/fence locks and revalidate before mutation. Domain resources
    /// must first be prepared through the canonical resource-lock helper; retain
    /// the caller transaction through domain and audit commit.
    pub async fn check_in(
        &self,
        connection: &mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> Result<(), Error> {
        authorize_mutation(
            connection,
            &[context.account],
            self,
            context,
            &[Permission::Access, permission],
        )
        .await
    }
}

impl MutationCheck for MutationAuthorization {
    fn check<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        context: CheckContext,
        permission: Permission,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        self.0.check(connection, context, permission)
    }
}

/// Lock all accounts and revalidate durable authority before any domain write.
pub async fn authorize_mutation(
    connection: &mut PgConnection,
    accounts: &[Uuid],
    authority: &dyn MutationCheck,
    context: CheckContext,
    permissions: &[Permission],
) -> Result<(), Error> {
    authorize_mutation_with_resources(
        connection,
        accounts,
        |_| Box::pin(async { Ok(()) }),
        authority,
        context,
        permissions,
    )
    .await
}

/// Lock accounts, domain resources and the fence, then revalidate live authority.
pub async fn authorize_mutation_with_resources<F>(
    connection: &mut PgConnection,
    accounts: &[Uuid],
    resources: F,
    authority: &dyn MutationCheck,
    context: CheckContext,
    permissions: &[Permission],
) -> Result<(), Error>
where
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    )
        -> std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>,
{
    authorize_mutation_with_resources_domain(
        connection,
        accounts,
        resources,
        authority,
        context,
        permissions,
    )
    .await
}

/// The canonical protocol preserving a domain's refusal error type.
pub async fn authorize_mutation_with_resources_domain<E, F>(
    connection: &mut PgConnection,
    accounts: &[Uuid],
    resources: F,
    authority: &dyn MutationCheck,
    context: CheckContext,
    permissions: &[Permission],
) -> Result<(), E>
where
    E: From<sqlx::Error> + From<Error>,
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), E>> + Send + 'a>>,
{
    if permissions.is_empty() {
        return Err(Error::InvalidInput.into());
    }
    let mut accounts = accounts.to_vec();
    accounts.push(context.account);
    lock_accounts_resources_and_mutation_with(connection, &accounts, resources).await?;
    for permission in permissions {
        authority
            .check(connection, context, *permission)
            .await
            .map_err(E::from)?;
    }
    Ok(())
}
