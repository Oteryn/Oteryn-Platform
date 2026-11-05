use super::{Error, Features, valid_id};
use oteryn_gameauth::admission::{
    self, AccountState, AccountStateLookup, AdmissionAuthority, AdmissionRequest,
    AuthorizedAccount, Future, Material,
};
use sqlx::{PgConnection, PgPool};
use std::{future::Future as StdFuture, pin::Pin};
use uuid::Uuid;

/// Mandatory ticket/admission guard. Hold this transaction through ticket issuance
/// or admission commit. Suspension writers lock the same account row exclusively,
/// so a decision cannot appear between this check and the protected effect.
/// A disabled flag denies the operation; it never bypasses enforcement.
pub async fn require_game_access(
    connection: &mut PgConnection,
    account: Uuid,
    features: Features,
) -> Result<(), Error> {
    if !features.game_access {
        return Err(Error::Disabled);
    }
    valid_id(account)?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *connection)
            .await?;
    if status.as_deref() != Some("active") {
        return Err(Error::Denied);
    }
    let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sanctions WHERE account_id=$1 AND (category NOT IN ('warning','restriction','suspension') OR status NOT IN ('pending','active','expired','revoked') OR (category='suspension' AND status='active' AND effective_at <= clock_timestamp() AND (expires_at IS NULL OR expires_at > clock_timestamp()))))")
        .bind(account).fetch_one(connection).await?;
    if blocked { Err(Error::Denied) } else { Ok(()) }
}

/// Narrow seam for the ticket task: bearer/scope validation remains with that
/// authority. The callback must write the ticket using only this connection and
/// must not perform external side effects. Refusal rolls back every write.
pub async fn ticket_transaction<T, F>(
    pool: &PgPool,
    account: Uuid,
    features: Features,
    issue: F,
) -> Result<T, Error>
where
    T: Send,
    F: for<'a> FnOnce(
        &'a mut PgConnection,
    ) -> Pin<Box<dyn StdFuture<Output = Result<T, Error>> + Send + 'a>>,
{
    let mut tx = pool.begin().await?;
    require_game_access(&mut tx, account, features).await?;
    let ticket = issue(&mut tx).await?;
    tx.commit().await?;
    Ok(ticket)
}

#[derive(Debug, Clone, Copy)]
pub struct AdmissionAccounts(pub Features);
impl AccountStateLookup for AdmissionAccounts {
    fn state<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        account: Uuid,
    ) -> Future<'a, AccountState> {
        Box::pin(async move {
            match require_game_access(connection, account, self.0).await {
                Ok(()) => Ok(AccountState::Active),
                Err(Error::Denied) => Ok(AccountState::Suspended),
                Err(Error::InvalidInput) => Ok(AccountState::Unknown),
                Err(_) => Err(admission::Error::Unavailable),
            }
        })
    }
}

/// Decorate the ticket/signing authority, including idempotent replay (which T19
/// deliberately delegates to its authority). Mount this with AdmissionAccounts.
#[derive(Debug)]
pub struct GuardedAdmission<A> {
    authority: A,
    features: Features,
}
impl<A> GuardedAdmission<A> {
    pub fn new(authority: A, features: Features) -> Self {
        Self {
            authority,
            features,
        }
    }
}
fn admission_error(error: Error) -> admission::Error {
    match error {
        Error::Denied | Error::InvalidInput => admission::Error::SecurityDenied,
        _ => admission::Error::Unavailable,
    }
}
impl<A: AdmissionAuthority> AdmissionAuthority for GuardedAdmission<A> {
    fn redeem<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
    ) -> Future<'a, AuthorizedAccount> {
        Box::pin(async move {
            let account = self.authority.redeem(connection, request).await?;
            require_game_access(connection, account.id, self.features)
                .await
                .map_err(admission_error)?;
            Ok(account)
        })
    }
    fn validate_replay<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        body: &'a [u8],
    ) -> Future<'a, ()> {
        Box::pin(async move {
            let account: Uuid =
                sqlx::query_scalar("SELECT account_id FROM native_admissions WHERE attempt_ref=$1")
                    .bind(request.attempt_ref)
                    .fetch_one(&mut *connection)
                    .await
                    .map_err(|_| admission::Error::ReconciliationRequired)?;
            require_game_access(connection, account, self.features)
                .await
                .map_err(admission_error)?;
            self.authority
                .validate_replay(connection, request, body)
                .await
        })
    }
    fn issue<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        account: AuthorizedAccount,
    ) -> Future<'a, Material> {
        Box::pin(async move {
            require_game_access(connection, account.id, self.features)
                .await
                .map_err(admission_error)?;
            self.authority.issue(connection, request, account).await
        })
    }
}

/// Composes both mandatory checks using T19's existing wire/status implementation.
/// Private ingress and service authentication remain governed by AdmissionConfig.
pub fn admission_router<A: AdmissionAuthority + 'static>(
    pool: PgPool,
    config: admission::AdmissionConfig,
    authority: A,
    features: Features,
) -> axum::Router {
    admission::router(
        pool,
        config,
        std::sync::Arc::new(AdmissionAccounts(features)),
        std::sync::Arc::new(GuardedAdmission::new(authority, features)),
    )
}
