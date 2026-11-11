//! Staff player tools. Identity/security adapters must use authoritative public APIs.
//! Missing adapters refuse rather than reporting a partial logout or MFA reset.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::future::Future;
use uuid::Uuid;

pub mod http;
mod projection;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("access denied")]
    Denied,
    #[error("invalid input")]
    BadInput,
    #[error("not found")]
    NotFound,
    #[error("dependency unavailable")]
    Unavailable,
    #[error("storage unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("authorization unavailable")]
    Authorization(#[from] crate::rbac::Error),
}
#[derive(Debug, Clone, Default)]
pub struct Flags {
    pub players: bool,
    pub characters: bool,
}
/// No credentials, email, network address, device fingerprint or session token.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: Uuid,
    pub status: String,
    pub locale: String,
    pub email_verified: bool,
}
#[derive(Debug, Clone)]
pub struct Session {
    pub kind: String,
    pub expires_at: String,
}
#[derive(Debug, Clone)]
pub struct Character {
    pub id: Uuid,
    pub name: String,
    pub world: Uuid,
}
/// Implementations must reject unknown/ambiguous state. Search is bounded to 50 rows.
/// All security changes, revocation fences, events and outbox writes must use `connection`.
/// `revoke_all` covers web, OAuth families, device sessions and pending game authorizations.
/// `reset_mfa` clears TOTP/recovery state AND performs the same complete revocation.
/// `verified` checks current, unconsumed support evidence for this exact target; never
/// accept a browser checkbox or free-text reason as proof. It must remain valid until
/// reset commits, and reset must consume it in that same transaction.
pub trait Accounts: Clone + Send + Sync + 'static {
    fn search(&self, query: &str) -> impl Future<Output = Result<Vec<Account>, Error>> + Send;
    fn account(&self, id: Uuid) -> impl Future<Output = Result<Option<Account>, Error>> + Send;
    fn sessions(&self, id: Uuid) -> impl Future<Output = Result<Vec<Session>, Error>> + Send;
    fn characters(&self, id: Uuid) -> impl Future<Output = Result<Vec<Character>, Error>> + Send;
    fn verified(
        &self,
        connection: &mut PgConnection,
        id: Uuid,
        evidence: Uuid,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
    fn revoke_all(
        &self,
        connection: &mut PgConnection,
        id: Uuid,
    ) -> impl Future<Output = Result<(), Error>> + Send;
    fn reset_mfa(
        &self,
        connection: &mut PgConnection,
        id: Uuid,
        evidence: Uuid,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
/// Available identity API supports exact ID/email lookup; security APIs await T08.
#[derive(Debug, Clone)]
pub struct IdentityAccounts {
    pub accounts: oteryn_identity::accounts::Accounts,
    pub projection: Option<oteryn_characters::lcfa::Projection>,
}
impl IdentityAccounts {
    fn safe(account: oteryn_identity::accounts::Account) -> Result<Account, Error> {
        if !matches!(
            account.status.as_str(),
            "pending_verification" | "active" | "suspended" | "closed"
        ) || !matches!(account.locale.as_str(), "pl" | "en")
        {
            return Err(Error::Denied);
        }
        Ok(Account {
            id: account.id,
            status: account.status,
            locale: account.locale,
            email_verified: account.email_verified_at.is_some(),
        })
    }
}
impl Accounts for IdentityAccounts {
    async fn search(&self, query: &str) -> Result<Vec<Account>, Error> {
        let account = if let Ok(id) = query.parse::<Uuid>() {
            self.accounts.find_by_id(id).await?
        } else {
            if !query.contains('@') {
                return Err(Error::BadInput);
            }
            self.accounts.find_by_email(query).await?
        };
        account
            .map(Self::safe)
            .transpose()
            .map(|a| a.into_iter().collect())
    }
    async fn account(&self, id: Uuid) -> Result<Option<Account>, Error> {
        self.accounts
            .find_by_id(id)
            .await?
            .map(Self::safe)
            .transpose()
    }
    async fn sessions(&self, _: Uuid) -> Result<Vec<Session>, Error> {
        Err(Error::Unavailable)
    }
    async fn characters(&self, id: Uuid) -> Result<Vec<Character>, Error> {
        projection::characters(self.projection.as_ref().ok_or(Error::Unavailable)?, id).await
    }
    async fn verified(&self, _: &mut PgConnection, _: Uuid, _: Uuid) -> Result<bool, Error> {
        Err(Error::Unavailable)
    }
    async fn revoke_all(&self, _: &mut PgConnection, _: Uuid) -> Result<(), Error> {
        Err(Error::Unavailable)
    }
    async fn reset_mfa(&self, _: &mut PgConnection, _: Uuid, _: Uuid) -> Result<(), Error> {
        Err(Error::Unavailable)
    }
}

#[derive(Debug)]
pub struct Players<A, I> {
    pool: PgPool,
    flags: Flags,
    auth: Authorizer<A>,
    accounts: I,
}
impl<A: AccountStateReader, I: Accounts> Players<A, I> {
    pub fn new(pool: PgPool, flags: Flags, staff: A, accounts: I) -> Self {
        Self {
            auth: Authorizer::new(pool.clone(), staff),
            pool,
            flags,
            accounts,
        }
    }
    async fn check(&self, context: CheckContext, permission: Permission) -> Result<(), Error> {
        if !self.flags.players {
            return Err(Error::NotFound);
        }
        // Even a caller outside HTTP may not relax the staff MFA requirement.
        if !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        self.auth.check(context, Permission::Access).await?;
        self.auth.check(context, permission).await?;
        Ok(())
    }
    async fn target(&self, id: Uuid) -> Result<Account, Error> {
        valid_id(id)?;
        self.accounts.account(id).await?.ok_or(Error::NotFound)
    }
    async fn record(&self, context: CheckContext, action: &str, target: Uuid) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        append(&mut tx, context, action, target, json!({})).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn search(&self, context: CheckContext, query: &str) -> Result<Vec<Account>, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        valid_text(query, 254)?;
        let results = self.accounts.search(query.trim()).await?;
        if results.len() > 50 {
            return Err(Error::Unavailable);
        }
        // Search terms can be email addresses, so they never enter the audit log.
        self.record(context, "players.search", context.account)
            .await?;
        Ok(results)
    }
    pub async fn account(&self, context: CheckContext, id: Uuid) -> Result<Account, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        let account = self.target(id).await?;
        self.record(context, "players.account.view", id).await?;
        Ok(account)
    }
    pub async fn sessions(&self, context: CheckContext, id: Uuid) -> Result<Vec<Session>, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        self.target(id).await?;
        let sessions = self.accounts.sessions(id).await?;
        if sessions.len() > 100 {
            return Err(Error::Unavailable);
        }
        self.record(context, "players.sessions.view", id).await?;
        Ok(sessions)
    }
    pub async fn characters(
        &self,
        context: CheckContext,
        id: Uuid,
    ) -> Result<Vec<Character>, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        if !self.flags.characters {
            return Err(Error::NotFound);
        }
        self.target(id).await?;
        let characters = self.accounts.characters(id).await?;
        if characters.len() > 256 {
            return Err(Error::Unavailable);
        }
        self.record(context, "players.characters.view", id).await?;
        Ok(characters)
    }
    pub async fn notes(&self, context: CheckContext, id: Uuid) -> Result<Vec<String>, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        self.target(id).await?;
        let notes = sqlx::query_scalar("SELECT body FROM player_notes WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 50").bind(id).fetch_all(&self.pool).await?;
        self.record(context, "players.notes.view", id).await?;
        Ok(notes)
    }
    pub async fn note(&self, context: CheckContext, id: Uuid, body: &str) -> Result<(), Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        self.target(id).await?;
        valid_text(body, 4000)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        let note: Uuid = sqlx::query_scalar(
            "INSERT INTO player_notes(account_id,author,body) VALUES($1,$2,$3) RETURNING id",
        )
        .bind(id)
        .bind(context.account)
        .bind(body.trim())
        .fetch_one(&mut *tx)
        .await?;
        append(
            &mut tx,
            context,
            "players.note.added",
            id,
            json!({"note":note}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn logout_all(
        &self,
        context: CheckContext,
        id: Uuid,
        reason: &str,
    ) -> Result<(), Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        valid_text(reason, 500)?;
        self.target(id).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        self.accounts.revoke_all(&mut tx, id).await?;
        append(
            &mut tx,
            context,
            "players.logout_all",
            id,
            json!({"reason":reason.trim()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Staff kick. Only the native security generation advances (published with
    /// cause 4 on the security revocation stream; device sessions are revoked by
    /// the existing trigger). No status, sanction or credential changes. The
    /// audit row (actor, target, time, reason) commits in the same transaction.
    pub async fn kick(&self, context: CheckContext, id: Uuid, reason: &str) -> Result<i64, Error> {
        self.check(context, Permission::KickAccountSessions).await?;
        valid_text(reason, 500)?;
        self.target(id).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        self.check(context, Permission::KickAccountSessions).await?;
        let generation = oteryn_identity::security_revocations::staff_kick(&mut tx, id)
            .await
            .map_err(|error| match error {
                oteryn_identity::security_revocations::Error::Storage(error) => {
                    Error::Storage(error)
                }
                oteryn_identity::security_revocations::Error::Invalid => Error::BadInput,
                oteryn_identity::security_revocations::Error::Encoding => Error::Unavailable,
            })?
            .ok_or(Error::NotFound)?;
        append(
            &mut tx,
            context,
            "players.kick",
            id,
            json!({"reason":reason.trim(),"native_security_generation":generation}),
        )
        .await?;
        tx.commit().await?;
        Ok(generation)
    }
    pub async fn request_reset(
        &self,
        context: CheckContext,
        id: Uuid,
        evidence: Uuid,
        reason: &str,
    ) -> Result<Uuid, Error> {
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        valid_id(evidence)?;
        valid_text(reason, 500)?;
        self.target(id).await?;
        if id == context.account {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        if !self.accounts.verified(&mut tx, id, evidence).await? {
            return Err(Error::Denied);
        }
        let request: Uuid = sqlx::query_scalar("INSERT INTO player_mfa_reset_requests(account_id,requested_by,evidence,reason) VALUES($1,$2,$3,$4) RETURNING id").bind(id).bind(context.account).bind(evidence).bind(reason.trim()).fetch_one(&mut *tx).await?;
        append(
            &mut tx,
            context,
            "players.mfa_reset.requested",
            id,
            json!({"request":request,"evidence":evidence,"reason":reason.trim()}),
        )
        .await?;
        tx.commit().await?;
        Ok(request)
    }
    pub async fn approve_reset(
        &self,
        context: CheckContext,
        id: Uuid,
        request: Uuid,
    ) -> Result<(), Error> {
        self.check(context, Permission::ManageRoles).await?;
        valid_id(request)?;
        self.target(id).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(71010)")
            .execute(&mut *tx)
            .await?;
        let row: Option<(Uuid,Uuid)> = sqlx::query_as("SELECT requested_by,evidence FROM player_mfa_reset_requests WHERE id=$1 AND account_id=$2 AND approved_by IS NULL AND expires_at>now() FOR UPDATE").bind(request).bind(id).fetch_optional(&mut *tx).await?;
        let (requester, evidence) = row.ok_or(Error::Denied)?;
        if context.account == requester || context.account == id {
            return Err(Error::Denied);
        }
        self.check(context, Permission::ManageRoles).await?;
        self.check(
            CheckContext {
                account: requester,
                ..context
            },
            Permission::ManageSupportTickets,
        )
        .await?;
        if !self.accounts.verified(&mut tx, id, evidence).await? {
            return Err(Error::Denied);
        }
        self.accounts.reset_mfa(&mut tx, id, evidence).await?;
        sqlx::query(
            "UPDATE player_mfa_reset_requests SET approved_by=$2,updated_at=now() WHERE id=$1",
        )
        .bind(request)
        .bind(context.account)
        .execute(&mut *tx)
        .await?;
        append(
            &mut tx,
            context,
            "players.mfa_reset.completed",
            id,
            json!({"request":request,"requested_by":requester}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn valid_id(id: Uuid) -> Result<(), Error> {
    if id.is_nil() {
        Err(Error::BadInput)
    } else {
        Ok(())
    }
}
fn valid_text(text: &str, max: usize) -> Result<(), Error> {
    if text.trim().is_empty()
        || text.chars().count() > max
        || text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        Err(Error::BadInput)
    } else {
        Ok(())
    }
}
async fn append(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: CheckContext,
    action: &str,
    target: Uuid,
    after: serde_json::Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &target.to_string(),
            before: json!(null),
            after,
        },
    )
    .await?;
    Ok(())
}
#[cfg(test)]
mod tests;
