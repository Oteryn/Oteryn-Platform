use super::{AdmissionRequest, Error, GrantSigner, Material, wire};
use sqlx::{PgConnection, PgPool, Row};
use std::{future::Future as StdFuture, pin::Pin};
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub type Future<'a, T> = Pin<Box<dyn StdFuture<Output = Result<T, Error>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Suspended,
    Unknown,
}
/// Implementations must lock authoritative account state until transaction commit.
/// Missing, deleted, disabled or ambiguous accounts must return Unknown or Suspended.
pub trait AccountStateLookup: Send + Sync + std::fmt::Debug {
    fn state<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        account: Uuid,
    ) -> Future<'a, AccountState>;
}
#[derive(Debug, Clone, Copy)]
pub struct AuthorizedAccount {
    pub id: Uuid,
    pub security_generation: u64,
}
/// Ticket redemption and grant preparation are explicit dependencies owned by
/// their respective tasks. Every mutation must use this transaction connection;
/// no network side effects or independent commits are permitted. Redemption must
/// lock and validate audience, expiry, single use and account security generation.
/// Issue must validate character ownership, fresh runtime and Registry revisions,
/// and obtain the deterministic signed grant from the signing authority.
pub trait AdmissionAuthority: Send + Sync + std::fmt::Debug {
    fn redeem<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
    ) -> Future<'a, AuthorizedAccount>;
    /// Revalidate the stored grant's binding, signing-key availability and current
    /// Registry route revision. Retired keys or corrupted bindings must return
    /// ReconciliationRequired; a changed Registry route returns RouteUnavailable.
    fn validate_replay<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        stored_response: &'a [u8],
    ) -> Future<'a, ()>;
    fn issue<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        account: AuthorizedAccount,
    ) -> Future<'a, Material>;
}
/// Fail-closed authority and lookup for compositions without a configured issuer.
#[derive(Debug)]
pub struct Unavailable;
impl AccountStateLookup for Unavailable {
    fn state<'a>(&'a self, _: &'a mut PgConnection, _: Uuid) -> Future<'a, AccountState> {
        Box::pin(async { Ok(AccountState::Unknown) })
    }
}
impl AdmissionAuthority for Unavailable {
    fn redeem<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a AdmissionRequest,
    ) -> Future<'a, AuthorizedAccount> {
        Box::pin(async { Err(Error::Unavailable) })
    }
    fn validate_replay<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a AdmissionRequest,
        _: &'a [u8],
    ) -> Future<'a, ()> {
        Box::pin(async { Err(Error::ReconciliationRequired) })
    }
    fn issue<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a AdmissionRequest,
        _: AuthorizedAccount,
    ) -> Future<'a, Material> {
        Box::pin(async { Err(Error::Unavailable) })
    }
}
async fn lock(connection: &mut PgConnection, key: &[u8]) -> Result<(), Error> {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&key[..8]);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(i64::from_be_bytes(bytes))
        .execute(connection)
        .await
        .map_err(|_| Error::Unavailable)?;
    Ok(())
}
async fn now(connection: &mut PgConnection) -> Result<i64, Error> {
    sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
        .fetch_one(connection)
        .await
        .map_err(|_| Error::Unavailable)
}
pub(super) async fn admit(
    pool: &PgPool,
    request: &AdmissionRequest,
    accounts: &dyn AccountStateLookup,
    authority: &dyn AdmissionAuthority,
    signer: Option<&dyn GrantSigner>,
) -> Result<Vec<u8>, Error> {
    let mut tx = pool.begin().await.map_err(|_| Error::Unavailable)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::Unavailable)?;
    // A transaction lock also covers the absent-row case across all processes.
    lock(&mut tx, request.attempt_ref.as_bytes()).await?;
    let existing = sqlx::query("SELECT ticket_hash, request_digest, response_body, expires_at_ms FROM native_admissions WHERE attempt_ref = $1 FOR UPDATE")
        .bind(request.attempt_ref).fetch_optional(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    if let Some(row) = existing {
        let ticket: Vec<u8> = row
            .try_get("ticket_hash")
            .map_err(|_| Error::ReconciliationRequired)?;
        let digest: Vec<u8> = row
            .try_get("request_digest")
            .map_err(|_| Error::ReconciliationRequired)?;
        if !bool::from(ticket.ct_eq(request.ticket_hash()) & digest.ct_eq(request.digest())) {
            return Err(Error::AttemptConflict);
        }
        let body: Vec<u8> = row
            .try_get("response_body")
            .map_err(|_| Error::ReconciliationRequired)?;
        let expires = row
            .try_get("expires_at_ms")
            .map_err(|_| Error::ReconciliationRequired)?;
        let response = wire::replay(&body, expires, now(&mut tx).await?)?;
        authority.validate_replay(&mut tx, request, &body).await?;
        // A signed grant is stored as its signing input only (contract §6.1);
        // Ed25519 is deterministic, so re-signing returns the same token.
        let input = wire::token(&body)?;
        if input.matches('.').count() != 1 {
            return Ok(response);
        }
        let signer = signer.ok_or(Error::ReconciliationRequired)?;
        signer
            .prepare()
            .await
            .map_err(|_| Error::ReconciliationRequired)?;
        let at = now(&mut tx).await? / 1000;
        let token = signer.resign(
            &input,
            u64::try_from(expires / 1000).map_err(|_| Error::ReconciliationRequired)?,
            u64::try_from(at).map_err(|_| Error::ReconciliationRequired)?,
        )?;
        return wire::with_token(&response, &token);
    }
    lock(&mut tx, request.ticket_hash()).await?;
    let consumed: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_admissions WHERE ticket_hash = $1)")
            .bind(request.ticket_hash())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| Error::Unavailable)?;
    if consumed {
        return Err(Error::TicketRejected);
    }
    let account = authority.redeem(&mut tx, request).await?;
    if account.security_generation == 0 || wire::uuid(&account.id.to_string()).is_err() {
        return Err(Error::SecurityDenied);
    }
    lock(&mut tx, account.id.as_bytes()).await?;
    if accounts.state(&mut tx, account.id).await? != AccountState::Active {
        return Err(Error::SecurityDenied);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions WHERE account_id = $1 AND created_at >= clock_timestamp() - interval '60 seconds'")
        .bind(account.id).fetch_one(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    if count >= 10 {
        return Err(Error::RateLimited);
    }
    let material = authority.issue(&mut tx, request, account).await?;
    let body = material.encode(request, now(&mut tx).await?)?;
    let stored = match signer {
        Some(_) => match material.token.rsplit_once('.') {
            Some((input, _)) if input.matches('.').count() == 1 => wire::with_token(&body, input)?,
            _ => return Err(Error::Unavailable),
        },
        None => body.clone(),
    };
    sqlx::query("INSERT INTO native_admissions (attempt_ref, ticket_hash, request_digest, account_id, response_body, expires_at_ms) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(request.attempt_ref).bind(request.ticket_hash()).bind(request.digest()).bind(account.id).bind(&stored).bind(material.expires_at_ms)
        .execute(&mut *tx).await.map_err(|_| Error::Unavailable)?;
    tx.commit()
        .await
        .map_err(|_| Error::ReconciliationRequired)?;
    Ok(body)
}
pub(super) async fn budget(pool: &PgPool, credential: &[u8], limit: u32) -> Result<(), Error> {
    let count: i64 = sqlx::query_scalar("INSERT INTO native_admission_budgets (credential_hash, window_start, requests) VALUES ($1, clock_timestamp(), 1) ON CONFLICT (credential_hash) DO UPDATE SET requests = CASE WHEN native_admission_budgets.window_start <= clock_timestamp() - interval '60 seconds' THEN 1 ELSE native_admission_budgets.requests + 1 END, window_start = CASE WHEN native_admission_budgets.window_start <= clock_timestamp() - interval '60 seconds' THEN clock_timestamp() ELSE native_admission_budgets.window_start END RETURNING requests")
        .bind(credential).fetch_one(pool).await.map_err(|_| Error::Unavailable)?;
    if count > i64::from(limit) {
        Err(Error::RateLimited)
    } else {
        Ok(())
    }
}
