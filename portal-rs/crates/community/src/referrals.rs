//! Targeted account invitations and atomic, once-only referral rewards.
//!
//! No HTTP or game evidence adapter is registered: no authoritative referral
//! wire or eligibility thresholds exist yet. Trusted callers must authenticate
//! the actor, enforce CSRF for browsers, and supply a transaction-aware policy.
//! Reward configuration is server-owned; the wallet audit adapter must authorize
//! its actor. Unknown account, return history, qualification or abuse state denies.
use oteryn_wallet::{AccountStateReader, AuditHook, TransferBlockCheck};
use sqlx::{PgConnection, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    New,
    Returning,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Returning => "returning",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Stage {
    Invite,
    Claim,
    Statistics,
}
/// Implementations must verify active, verified accounts, authoritative new or
/// returning status, disjoint identities, abuse restrictions and rate limits.
/// Claim additionally requires authoritative qualifying activity after invitation.
/// Read/lock state in this connection; missing or stale evidence returns false.
/// Statistics checks only the authenticated account's access and account status.
pub trait Policy: Send + Sync {
    fn allowed(
        &self,
        connection: &mut PgConnection,
        actor: Uuid,
        other: Uuid,
        kind: Kind,
        stage: Stage,
    ) -> impl std::future::Future<Output = bool> + Send;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Invalid,
    Denied,
    Conflict,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "referral operation refused: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
#[derive(Debug, Clone)]
pub struct Invitation {
    /// Retain this UUIDv7 for retries. It is scoped to the complete payload.
    pub id: Uuid,
    pub inviter: Uuid,
    pub invitee: Uuid,
    pub kind: Kind,
    pub expires_at: OffsetDateTime,
}
#[derive(Debug, Clone, Copy)]
pub struct Reward {
    pub actor: Uuid,
    /// Coins awarded to the inviter after the invitee qualifies and claims.
    pub amount: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub reward_key: Uuid,
    pub amount: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statistics {
    pub invitations: i64,
    pub returning: i64,
    pub rewarded: i64,
    pub expired: i64,
    pub coins: i64,
}
/// Construct only with trusted policy and wallet adapters. There is no permissive
/// default adapter, automatic enabling, or public endpoint.
#[derive(Debug)]
pub struct Referrals<P, A, B, H> {
    pool: PgPool,
    flags: Flags,
    policy: P,
    wallet: oteryn_wallet::Ledger<A, B, H>,
    notifications: oteryn_notify::Notifications,
    reward: Reward,
}
impl<P: Policy, A: AccountStateReader, B: TransferBlockCheck, H: AuditHook> Referrals<P, A, B, H> {
    pub fn new(
        pool: PgPool,
        flags: Flags,
        policy: P,
        wallet: oteryn_wallet::Ledger<A, B, H>,
        notifications: oteryn_notify::Notifications,
        reward: Reward,
    ) -> Self {
        Self {
            pool,
            flags,
            policy,
            wallet,
            notifications,
            reward,
        }
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    pub async fn invite(&self, request: &Invitation) -> Result<(), Error> {
        self.enabled()?;
        for id in [
            request.id,
            request.inviter,
            request.invitee,
            self.reward.actor,
        ] {
            valid(id)?;
        }
        if self.reward.amount <= 0 {
            return Err(Error::Invalid);
        }
        if request.inviter == request.invitee {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await?;
        // A single lock serializes graph changes, preventing concurrent cycles and
        // reciprocal invitations. Rate limiting remains mandatory in the policy.
        sqlx::query("SELECT pg_advisory_xact_lock(5600)")
            .execute(&mut *tx)
            .await?;
        if !self
            .policy
            .allowed(
                &mut tx,
                request.inviter,
                request.invitee,
                request.kind,
                Stage::Invite,
            )
            .await
        {
            return Err(Error::Denied);
        }
        if let Some(row) = sqlx::query(
            "SELECT inviter_id,invitee_id,kind,expires_at FROM referral_invitations WHERE id=$1",
        )
        .bind(request.id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if row.try_get::<Uuid, _>("inviter_id")? != request.inviter
                || row.try_get::<Uuid, _>("invitee_id")? != request.invitee
                || row.try_get::<&str, _>("kind")? != request.kind.name()
                || row
                    .try_get::<OffsetDateTime, _>("expires_at")?
                    .unix_timestamp_nanos()
                    / 1000
                    != request.expires_at.unix_timestamp_nanos() / 1000
            {
                return Err(Error::Conflict);
            }
            return Ok(());
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if request.expires_at <= now {
            return Err(Error::Invalid);
        }
        // Follow the inviter's ancestry: a candidate invitee must not be an ancestor.
        let cycle: bool = sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT $1::uuid UNION SELECT r.inviter_id FROM referral_invitations r JOIN ancestors a ON r.invitee_id=a.id) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=$2)")
            .bind(request.inviter).bind(request.invitee).fetch_one(&mut *tx).await?;
        let duplicate: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM referral_invitations WHERE invitee_id=$1)",
        )
        .bind(request.invitee)
        .fetch_one(&mut *tx)
        .await?;
        if cycle || duplicate {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO referral_invitations(id,inviter_id,invitee_id,kind,expires_at,reward_amount,reward_actor,reward_key) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(request.id).bind(request.inviter).bind(request.invitee).bind(request.kind.name()).bind(request.expires_at)
            .bind(self.reward.amount).bind(self.reward.actor).bind(Uuid::now_v7()).execute(&mut *tx).await?;
        self.notify(&mut tx, request.invitee, "invited").await?;
        tx.commit().await?;
        Ok(())
    }
    /// Only the authenticated invitee can claim. The stored wallet key is reused
    /// on every retry; eligibility, reward, notification and statistics commit once.
    pub async fn claim(&self, invitee: Uuid, invitation: Uuid) -> Result<Receipt, Error> {
        self.enabled()?;
        valid(invitee)?;
        valid(invitation)?;
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT * FROM referral_invitations WHERE id=$1 AND invitee_id=$2 FOR UPDATE",
        )
        .bind(invitation)
        .bind(invitee)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Denied)?;
        let inviter: Uuid = row.try_get("inviter_id")?;
        let kind = match row.try_get::<&str, _>("kind")? {
            "new" => Kind::New,
            "returning" => Kind::Returning,
            _ => return Err(Error::Denied),
        };
        if !self
            .policy
            .allowed(&mut tx, invitee, inviter, kind, Stage::Claim)
            .await
        {
            return Err(Error::Denied);
        }
        let receipt = Receipt {
            reward_key: row.try_get("reward_key")?,
            amount: row.try_get("reward_amount")?,
        };
        if row
            .try_get::<Option<OffsetDateTime>, _>("claimed_at")?
            .is_some()
        {
            return Ok(receipt);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if row.try_get::<OffsetDateTime, _>("expires_at")? <= now {
            return Err(Error::Denied);
        }
        self.wallet
            .adjust_in(
                &mut tx,
                receipt.reward_key,
                inviter,
                receipt.amount,
                row.try_get("reward_actor")?,
                "Friend referral reward",
            )
            .await
            .map_err(|_| Error::Denied)?;
        sqlx::query("UPDATE referral_invitations SET claimed_at=$2,updated_at=$2 WHERE id=$1")
            .bind(invitation)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        self.notify(&mut tx, inviter, "rewarded").await?;
        tx.commit().await?;
        Ok(receipt)
    }
    pub async fn statistics(&self, actor: Uuid) -> Result<Statistics, Error> {
        self.enabled()?;
        valid(actor)?;
        let mut tx = self.pool.begin().await?;
        if !self
            .policy
            .allowed(&mut tx, actor, actor, Kind::New, Stage::Statistics)
            .await
        {
            return Err(Error::Denied);
        }
        let row = sqlx::query("SELECT count(*) AS invitations,count(*) FILTER(WHERE kind='returning') AS returning,count(*) FILTER(WHERE claimed_at IS NOT NULL) AS rewarded,count(*) FILTER(WHERE claimed_at IS NULL AND expires_at<=clock_timestamp()) AS expired,COALESCE(sum(reward_amount) FILTER(WHERE claimed_at IS NOT NULL),0)::bigint AS coins FROM referral_invitations WHERE inviter_id=$1")
            .bind(actor).fetch_one(&mut *tx).await?;
        Ok(Statistics {
            invitations: row.try_get("invitations")?,
            returning: row.try_get("returning")?,
            rewarded: row.try_get("rewarded")?,
            expired: row.try_get("expired")?,
            coins: row.try_get("coins")?,
        })
    }
    async fn notify(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: Uuid,
        key: &str,
    ) -> Result<(), Error> {
        let state = self
            .notifications
            .accounts
            .active_verified(account)
            .await
            .map_err(|_| Error::Denied)?
            .ok_or(Error::Denied)?;
        let template = notification(state.locale, key)?;
        self.notifications
            .enqueue(tx, account, &template)
            .await
            .map_err(|_| Error::Denied)?;
        Ok(())
    }
}
fn notification(
    locale: oteryn_notify::Locale,
    key: &str,
) -> Result<oteryn_notify::EmailTemplate, Error> {
    let locale = match locale {
        oteryn_notify::Locale::Pl => "pl",
        oteryn_notify::Locale::En => "en",
    };
    let language = locale.parse().map_err(|_| Error::Unavailable)?;
    let mut bundle = fluent_bundle::FluentBundle::new(vec![language]);
    let source = if locale == "pl" {
        "title = Polecenia znajomych\ninvited = Otrzymano zaproszenie do programu poleceń.\nrewarded = Przyznano nagrodę za polecenie znajomego.\n"
    } else {
        "title = Friend referrals\ninvited = You received an invitation to the referral programme.\nrewarded = Your friend referral reward was granted.\n"
    };
    bundle
        .add_resource(
            fluent_bundle::FluentResource::try_new(source.into())
                .map_err(|_| Error::Unavailable)?,
        )
        .map_err(|_| Error::Unavailable)?;
    let render = |id| -> Result<String, Error> {
        let pattern = bundle
            .get_message(id)
            .and_then(|m| m.value())
            .ok_or(Error::Unavailable)?;
        let mut errors = Vec::new();
        let text = bundle
            .format_pattern(pattern, None, &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(text)
        } else {
            Err(Error::Unavailable)
        }
    };
    Ok(oteryn_notify::EmailTemplate::News {
        title: render("title")?,
        body: render(key)?,
    })
}
fn valid(id: Uuid) -> Result<(), Error> {
    if id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122 {
        Ok(())
    } else {
        Err(Error::Invalid)
    }
}
#[cfg(test)]
#[path = "referrals_tests.rs"]
mod tests;
