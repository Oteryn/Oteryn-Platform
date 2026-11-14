//! Native ticket bootstrap. OAuth owns bearer verification and revocation;
//! implementations must participate in the caller's database transaction.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

pub type ResultFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unauthorized,
    Unavailable,
    RateLimited,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Suspended,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Principal {
    pub account_id: Uuid,
    pub status: AccountStatus,
    pub game_generation: u64,
    pub native_generation: u64,
}
/// Must lock the token, first-party client and authoritative account until commit.
/// Check expiry, revocation, game:ticket scope, current generation and explicit
/// active account state. Suspended, disabled, terminated or unknown state denies.
/// With consume=true, revoke access and all associated refresh tokens atomically.
pub trait OAuthAuthority: std::fmt::Debug + Send + Sync {
    fn verify<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        bearer: &'a str,
        consume: bool,
    ) -> ResultFuture<'a, Principal>;
}
#[derive(Debug)]
pub struct DenyOAuth;
impl OAuthAuthority for DenyOAuth {
    fn verify<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a str,
        _: bool,
    ) -> ResultFuture<'a, Principal> {
        Box::pin(async { Err(Error::Unauthorized) })
    }
}
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub enabled: bool,
    /// Native bootstrap is restricted to testing and preproduction.
    pub nonproduction: bool,
}
#[derive(Debug, Clone)]
pub struct Tickets {
    pool: PgPool,
    settings: Settings,
    oauth: Arc<dyn OAuthAuthority>,
}
#[derive(serde::Serialize)]
pub struct Issued {
    pub protocol_version: u8,
    pub ticket: String,
    pub expires_in: u8,
}
impl Tickets {
    pub fn new(pool: PgPool, settings: Settings, oauth: Arc<dyn OAuthAuthority>) -> Self {
        Self {
            pool,
            settings,
            oauth,
        }
    }
    pub fn available(&self) -> bool {
        self.settings.enabled && self.settings.nonproduction
    }
    /// The transaction also rolls back token consumption if persistence fails.
    pub async fn issue(&self, bearer: &str) -> Result<Issued, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(|_| Error::Unavailable)?;
        let principal = self.oauth.verify(&mut tx, bearer, true).await?;
        let family: Option<Uuid> = sqlx::query_scalar("SELECT d.id FROM device_sessions d JOIN oauth_tokens t ON d.enrollment_token_id=t.id::text WHERE t.access_hash=$1 AND d.account_id=$2 AND d.game_generation::text=$3 AND d.native_generation::text=$4 AND d.revoked_at IS NULL AND d.absolute_expires_at>extract(epoch FROM clock_timestamp()) AND d.idle_expires_at>extract(epoch FROM clock_timestamp()) FOR SHARE OF d")
            .bind(&Sha256::digest(bearer.as_bytes())[..]).bind(principal.account_id).bind(principal.game_generation.to_string()).bind(principal.native_generation.to_string()).fetch_optional(&mut *tx).await.map_err(|_| Error::Unavailable)?;
        let issued = issue_on_connection(&mut tx, principal, family).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(issued)
    }
    pub async fn account(&self, bearer: &str) -> Result<Principal, Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(|_| Error::Unavailable)?;
        let principal = self.oauth.verify(&mut tx, bearer, false).await?;
        validate(principal)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(principal)
    }
    /// Durable per-bearer, per-purpose budget shared by all API processes.
    pub async fn budget(&self, bearer: &str, issuance: bool) -> Result<(), Error> {
        if !self.available() {
            return Err(Error::Unavailable);
        }
        let key = Sha256::digest(format!(
            "{}:{bearer}",
            if issuance { "ticket" } else { "characters" }
        ));
        let count: i64 = sqlx::query_scalar("INSERT INTO game_ticket_budgets (credential_hash, window_start, requests) VALUES ($1,clock_timestamp(),1) ON CONFLICT (credential_hash) DO UPDATE SET requests = CASE WHEN game_ticket_budgets.window_start <= clock_timestamp() - interval '60 seconds' THEN 1 ELSE game_ticket_budgets.requests + 1 END, window_start = CASE WHEN game_ticket_budgets.window_start <= clock_timestamp() - interval '60 seconds' THEN clock_timestamp() ELSE game_ticket_budgets.window_start END RETURNING requests")
            .bind(&key[..]).fetch_one(&self.pool).await.map_err(|_| Error::Unavailable)?;
        if count > if issuance { 5 } else { 10 } {
            Err(Error::RateLimited)
        } else {
            Ok(())
        }
    }
}
/// Worker retention for `game_ticket_budgets`. A window lasts 60 seconds; a row
/// is kept until ten minutes after its window started, so pruning never
/// removes an active window, even beside a concurrent budget check.
#[derive(Debug, Clone)]
pub struct BudgetRetention {
    pub pool: PgPool,
}
impl BudgetRetention {
    pub const BATCH: i64 = 1000;
    /// Deletes at most [`Self::BATCH`] expired rows and returns how many it removed.
    pub async fn prune(&self) -> Result<u64, sqlx::Error> {
        // The outer predicate re-checks a row that a concurrent budget check
        // reset after the subquery saw it.
        Ok(sqlx::query(
            "DELETE FROM game_ticket_budgets WHERE credential_hash IN \
             (SELECT credential_hash FROM game_ticket_budgets \
              WHERE window_start < clock_timestamp() - interval '10 minutes' \
              LIMIT $1 FOR UPDATE SKIP LOCKED) \
             AND window_start < clock_timestamp() - interval '10 minutes'",
        )
        .bind(Self::BATCH)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }
}
fn validate(principal: Principal) -> Result<(), Error> {
    if principal.status != AccountStatus::Active
        || principal.account_id.get_version_num() != 7
        || principal.account_id.get_variant() != uuid::Variant::RFC4122
        || principal.native_generation == 0
    {
        Err(Error::Unauthorized)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;

impl std::fmt::Debug for Issued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Issued")
            .field("protocol_version", &self.protocol_version)
            .field("ticket", &"[redacted]")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// Shared issuer for already locked native authority; all effects belong to the caller.
pub(crate) async fn issue_on_connection(
    db: &mut PgConnection,
    principal: Principal,
    family: Option<Uuid>,
) -> Result<Issued, Error> {
    validate(principal)?;
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).map_err(|_| Error::Unavailable)?;
    let ticket = URL_SAFE_NO_PAD.encode(secret);
    sqlx::query("INSERT INTO game_login_tickets (ticket_hash, account_id, game_generation, native_generation, audience, expires_at, device_family_id) VALUES ($1,$2,$3,$4,'oteryn-native-game-gateway',clock_timestamp() + interval '60 seconds',$5)")
        .bind(&Sha256::digest(ticket.as_bytes())[..]).bind(principal.account_id)
        .bind(principal.game_generation.to_string()).bind(principal.native_generation.to_string()).bind(family)
        .execute(db).await.map_err(|_| Error::Unavailable)?;
    Ok(Issued {
        protocol_version: 1,
        ticket,
        expires_in: 60,
    })
}
