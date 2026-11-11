//! Default-off tell-a-friend campaigns. Store callbacks own paid evidence.
use super::{Error, Flags, Kind, Policy, Stage, valid};
use oteryn_db::retry::TransactionError;
use sqlx::{PgConnection, PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
mod tests;
#[path = "web.rs"]
pub mod web;

#[derive(Debug, Clone, Copy)]
pub struct DatabasePolicy;
impl Policy for DatabasePolicy {
    async fn allowed(
        &self,
        conn: &mut PgConnection,
        actor: Uuid,
        other: Uuid,
        kind: Kind,
        stage: Stage,
    ) -> bool {
        let active = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts WHERE id=ANY($1) AND status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=clock_timestamp()")
            .bind(if actor == other { vec![actor] } else { vec![actor, other] })
            .fetch_one(&mut *conn).await;
        if active.ok() != Some(if actor == other { 1 } else { 2 }) {
            return false;
        }
        if matches!(stage, Stage::Statistics) {
            return true;
        }
        if actor == other {
            return false;
        }
        if matches!(stage, Stage::Invite) && kind == Kind::Returning {
            return sqlx::query_scalar("SELECT last_activity<=clock_timestamp()-interval '12 months' AND created_at<=clock_timestamp()-interval '12 months' FROM referral_activity WHERE account_id=$1 FOR UPDATE")
                .bind(other).fetch_optional(&mut *conn).await.ok().flatten() == Some(true);
        }
        true
    }
}
impl oteryn_store::premium::Accounts for DatabasePolicy {
    fn state<'a>(
        &'a self,
        account: Uuid,
        conn: &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        oteryn_store::premium::AccountState,
                        oteryn_store::premium::Error,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let active: Option<bool> = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL AND email_change_blocked_until<=clock_timestamp() FROM accounts WHERE id=$1 FOR SHARE")
                .bind(account).fetch_optional(conn).await?;
            Ok(if active == Some(true) {
                oteryn_store::premium::AccountState::Known
            } else {
                oteryn_store::premium::AccountState::Unknown
            })
        })
    }
}
#[derive(Debug, Clone)]
pub struct Campaign<P = DatabasePolicy> {
    pub pool: PgPool,
    pub flags: Flags,
    pub policy: P,
    pub origin: String,
}
impl<P: Policy> Campaign<P> {
    pub fn new(pool: PgPool, flags: Flags, policy: P, origin: String) -> Result<Self, Error> {
        let uri: axum::http::Uri = origin.parse().map_err(|_| Error::Invalid)?;
        if uri.scheme_str() != Some("https")
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || uri.path() != "/"
            || uri.query().is_some()
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            pool,
            flags,
            policy,
            origin: origin.trim_end_matches('/').into(),
        })
    }
    async fn begin(
        &self,
        accounts: &[Uuid],
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, TransactionError<Error>> {
        if !self.flags.enabled {
            return Err(Error::Disabled.into());
        }
        let mut tx = self.pool.begin().await?;
        let enabled: bool =
            sqlx::query_scalar("SELECT enabled FROM referral_campaign_settings WHERE singleton")
                .fetch_one(&mut *tx)
                .await?;
        if !enabled {
            return Err(Error::Disabled.into());
        }
        // Attribution and reward foreign keys must not acquire account locks
        // after 5600: purchases hold accounts FOR UPDATE before qualification.
        sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR KEY SHARE")
            .bind(accounts)
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT referral_campaign_lock($1)")
            .bind(accounts)
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
    async fn actor(
        &self,
        conn: &mut PgConnection,
        actor: Uuid,
    ) -> Result<(), TransactionError<Error>> {
        valid(actor)?;
        if self
            .policy
            .allowed(conn, actor, actor, Kind::New, Stage::Statistics)
            .await
        {
            Ok(())
        } else {
            Err(Error::Denied.into())
        }
    }
    pub async fn link(&self, actor: Uuid) -> Result<Uuid, Error> {
        oteryn_db::retry::transaction(|| self.link_attempt(actor)).await
    }
    async fn link_attempt(&self, actor: Uuid) -> Result<Uuid, TransactionError<Error>> {
        let mut tx = self.begin(&[actor]).await?;
        self.actor(&mut tx, actor).await?;
        let code=sqlx::query_scalar("INSERT INTO referral_links(account_id) VALUES($1) ON CONFLICT(account_id) DO UPDATE SET account_id=EXCLUDED.account_id RETURNING code").bind(actor).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(code)
    }
    /// Each actual send consumes a rolling budget, including permitted resends.
    pub async fn email(&self, actor: Uuid, recipient: &str, kind: Kind) -> Result<(), Error> {
        oteryn_db::retry::transaction(|| self.email_attempt(actor, recipient, kind)).await
    }
    async fn email_attempt(
        &self,
        actor: Uuid,
        recipient: &str,
        kind: Kind,
    ) -> Result<(), TransactionError<Error>> {
        let recipient = recipient.trim().to_ascii_lowercase();
        let email = regex::Regex::new(
            r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?\.[a-z]{2,63}$",
        )
        .map_err(|_| Error::Unavailable)?;
        if recipient.len() > 254
            || recipient.contains("..")
            || recipient.starts_with('.')
            || recipient.contains(".@")
            || !email.is_match(&recipient)
        {
            return Err(Error::Invalid.into());
        }
        // Resolve returning recipients before the global qualification lock.
        let target: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1")
                .bind(&recipient)
                .fetch_optional(&self.pool)
                .await?;
        let mut tx = self.begin(&[actor, target.unwrap_or(actor)]).await?;
        self.actor(&mut tx, actor).await?;
        let own: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND lower(email)=$2)",
        )
        .bind(actor)
        .bind(&recipient)
        .fetch_one(&mut *tx)
        .await?;
        if own {
            return Err(Error::Denied.into());
        }
        let old: Option<(Uuid,bool)>=sqlx::query_as("SELECT id,created_at<=clock_timestamp()-interval '14 days' FROM referral_email_invitations WHERE inviter_id=$1 AND recipient=$2 AND kind=$3")
            .bind(actor).bind(&recipient).bind(kind.name()).fetch_optional(&mut *tx).await?;
        if old.is_some_and(|(_, allowed)| !allowed) {
            return Err(Error::Denied.into());
        }
        let used: i64=sqlx::query_scalar("SELECT count(*) FROM referral_email_attempts WHERE inviter_id=$1 AND created_at>clock_timestamp()-interval '24 hours'").bind(actor).fetch_one(&mut *tx).await?;
        if used >= 25 {
            return Err(Error::Denied.into());
        }
        if kind == Kind::Returning {
            let used: i64=sqlx::query_scalar("SELECT count(*) FROM referral_email_attempts a JOIN referral_email_invitations i ON i.id=a.invitation_id WHERE a.inviter_id=$1 AND i.kind='returning' AND a.created_at>clock_timestamp()-interval '30 days'").bind(actor).fetch_one(&mut *tx).await?;
            if used >= 20 {
                return Err(Error::Denied.into());
            }
            let target = target.ok_or(Error::Denied)?;
            let matches: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND lower(email)=$2)",
            )
            .bind(target)
            .bind(&recipient)
            .fetch_one(&mut *tx)
            .await?;
            if !matches {
                return Err(Error::Denied.into());
            }
            // Replays keep the original eligibility snapshot and reward deadline.
            if old.is_none() {
                self.bind(&mut tx, actor, target, kind).await?;
            }
        } else {
            let registered: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE lower(email)=$1)")
                    .bind(&recipient)
                    .fetch_one(&mut *tx)
                    .await?;
            if registered {
                return Err(Error::Denied.into());
            }
        }
        let id = if let Some((id, _)) = old {
            id
        } else {
            sqlx::query_scalar("INSERT INTO referral_email_invitations(inviter_id,recipient,kind) VALUES($1,$2,$3) RETURNING id")
                .bind(actor).bind(&recipient).bind(kind.name()).fetch_one(&mut *tx).await?
        };
        sqlx::query("INSERT INTO referral_email_attempts(invitation_id,inviter_id) VALUES($1,$2)")
            .bind(id)
            .bind(actor)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    async fn invitation_kind(&self, code: Uuid) -> Result<Kind, Error> {
        oteryn_db::retry::transaction(|| self.invitation_kind_attempt(code)).await
    }
    async fn invitation_kind_attempt(&self, code: Uuid) -> Result<Kind, TransactionError<Error>> {
        valid(code)?;
        let mut tx = self.begin(&[]).await?;
        let name: Option<String>=sqlx::query_scalar("SELECT 'new' FROM referral_links WHERE code=$1 UNION ALL SELECT kind FROM referral_email_invitations WHERE id=$1")
            .bind(code).fetch_optional(&mut *tx).await?;
        match name.as_deref() {
            Some("new") => Ok(Kind::New),
            Some("returning") => Ok(Kind::Returning),
            _ => Err(Error::Denied.into()),
        }
    }
    pub async fn accept(&self, actor: Uuid, code: Uuid) -> Result<(), Error> {
        oteryn_db::retry::transaction(|| self.accept_attempt(actor, code)).await
    }
    async fn accept_attempt(&self, actor: Uuid, code: Uuid) -> Result<(), TransactionError<Error>> {
        valid(code)?;
        if !self.flags.enabled {
            return Err(Error::Disabled.into());
        }
        // Discover immutable invitation ownership before taking 5600; reread
        // the invitation and all eligibility evidence inside the transaction.
        let inviter: Option<Uuid> = sqlx::query_scalar("SELECT account_id FROM referral_links WHERE code=$1 UNION ALL SELECT inviter_id FROM referral_email_invitations WHERE id=$1")
            .bind(code).fetch_optional(&self.pool).await?;
        let mut tx = self.begin(&[actor, inviter.unwrap_or(actor)]).await?;
        self.actor(&mut tx, actor).await?;
        let owner: Option<(Uuid,time::OffsetDateTime,Option<String>,String)>=sqlx::query_as("SELECT account_id,created_at,NULL::text,'new' FROM referral_links WHERE code=$1 UNION ALL SELECT inviter_id,created_at,recipient,kind FROM referral_email_invitations WHERE id=$1")
            .bind(code).fetch_optional(&mut *tx).await?;
        let (owner, created, recipient, kind) = owner.ok_or(Error::Denied)?;
        if Some(owner) != inviter {
            return Err(Error::Denied.into());
        }
        if kind == "returning" {
            let bound: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.invitee_id WHERE i.inviter_id=$1 AND i.invitee_id=$2 AND i.kind='returning' AND lower(a.email)=$3)")
                .bind(owner).bind(actor).bind(recipient).fetch_one(&mut *tx).await?;
            return if bound {
                Ok(())
            } else {
                Err(Error::Denied.into())
            };
        }
        let eligible: bool=sqlx::query_scalar("SELECT created_at>=$2 AND created_at>clock_timestamp()-interval '7 days' AND ($3::text IS NULL OR lower(email)=$3) AND NOT EXISTS(SELECT 1 FROM store_orders WHERE account_id=$1) FROM accounts WHERE id=$1")
            .bind(actor).bind(created).bind(recipient).fetch_one(&mut *tx).await?;
        if !eligible {
            return Err(Error::Denied.into());
        }
        self.bind(&mut tx, owner, actor, Kind::New).await?;
        sqlx::query("SELECT referral_campaign_reward('starter:'||i.id,invitee_id,'starter',3,NULL,a.created_at+interval '7 days') FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.invitee_id WHERE i.invitee_id=$1")
            .bind(actor).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    async fn bind(
        &self,
        conn: &mut PgConnection,
        inviter: Uuid,
        invitee: Uuid,
        kind: Kind,
    ) -> Result<(), TransactionError<Error>> {
        if inviter == invitee
            || !self
                .policy
                .allowed(conn, inviter, invitee, kind, Stage::Invite)
                .await
        {
            return Err(Error::Denied.into());
        }
        let existing: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT inviter_id,kind FROM referral_campaign_invitations WHERE invitee_id=$1",
        )
        .bind(invitee)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some((owner, name)) = existing {
            return if owner == inviter && name == kind.name() {
                Ok(())
            } else {
                Err(Error::Denied.into())
            };
        }
        let cycle: bool=sqlx::query_scalar("WITH RECURSIVE edges AS (SELECT inviter_id,invitee_id FROM referral_invitations UNION SELECT inviter_id,invitee_id FROM referral_campaign_invitations), ancestors(id) AS (SELECT $1::uuid UNION SELECT e.inviter_id FROM edges e JOIN ancestors a ON e.invitee_id=a.id) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=$2) OR EXISTS(SELECT 1 FROM referral_invitations WHERE invitee_id=$2)")
            .bind(inviter).bind(invitee).fetch_one(&mut *conn).await?;
        if cycle {
            return Err(Error::Denied.into());
        }
        sqlx::query("INSERT INTO referral_campaign_invitations(inviter_id,invitee_id,kind) VALUES($1,$2,$3)").bind(inviter).bind(invitee).bind(kind.name()).execute(conn).await?;
        Ok(())
    }
}

/// Periodic reconciliation uses the same premium ledger as other store grants.
#[derive(Debug)]
pub struct Worker {
    pub campaign: Campaign,
    pub sender: Option<Arc<dyn oteryn_notify::EmailSender>>,
}
impl Worker {
    pub fn configured(pool: PgPool) -> Result<Self, Box<dyn std::error::Error>> {
        let enabled = std::env::var("PORTAL_REFERRALS_ENABLED").as_deref() == Ok("true");
        let origin = if enabled {
            std::env::var("OTERYN_PUBLIC_ORIGIN")?
        } else {
            "https://localhost".into()
        };
        let sender = if enabled {
            match std::env::var("OTERYN_SMTP_HOST") {
                Ok(host) => Some(Arc::new(
                    oteryn_notify::SmtpSender::new(
                        &host,
                        std::env::var("OTERYN_SMTP_USERNAME")?,
                        std::env::var("OTERYN_SMTP_PASSWORD")?,
                        std::env::var("OTERYN_MAIL_FROM")?,
                    )
                    .map_err(|_| "referral mail configuration invalid")?,
                ) as Arc<dyn oteryn_notify::EmailSender>),
                Err(_) => None,
            }
        } else {
            None
        };
        Ok(Self {
            campaign: Campaign::new(pool, Flags { enabled }, DatabasePolicy, origin)?,
            sender,
        })
    }
    pub async fn process(&self) -> Result<(), Error> {
        if !self.campaign.flags.enabled {
            return Ok(());
        }
        let enabled: bool =
            sqlx::query_scalar("SELECT enabled FROM referral_campaign_settings WHERE singleton")
                .fetch_one(&self.campaign.pool)
                .await?;
        if !enabled {
            return Ok(());
        }
        let candidates: Vec<(Uuid,String)>=sqlx::query_as("SELECT DISTINCT s.account_id,s.data->>'referral.code' FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE s.data ? 'referral.code' AND a.status='active' AND a.email_verified_at IS NOT NULL AND a.email_change_blocked_until<=clock_timestamp() ORDER BY s.account_id,s.data->>'referral.code' LIMIT 100").fetch_all(&self.campaign.pool).await?;
        for (account, code) in candidates {
            if let Ok(code) = Uuid::parse_str(&code) {
                match self.campaign.accept(account, code).await {
                    Ok(()) | Err(Error::Denied | Error::Invalid) => {}
                    Err(error) => return Err(error),
                }
            }
            // Success, permanent refusal and malformed evidence are terminal.
            // A transient failure above leaves the evidence available for retry.
            sqlx::query("UPDATE web_sessions SET data=data-'referral.code',updated_at=clock_timestamp() WHERE account_id=$1 AND data->>'referral.code'=$2")
                .bind(account).bind(&code).execute(&self.campaign.pool).await?;
        }
        let ledger = oteryn_store::premium::repository::Ledger::new(
            self.campaign.pool.clone(),
            Arc::new(DatabasePolicy),
            true,
        );
        let ids: Vec<Uuid>=sqlx::query_scalar("SELECT r.id FROM referral_campaign_rewards r JOIN accounts a ON a.id=r.account_id WHERE r.kind='premium' AND r.applied_at IS NULL AND a.status='active' AND a.email_verified_at IS NOT NULL AND a.email_change_blocked_until<=clock_timestamp() ORDER BY r.created_at LIMIT 100").fetch_all(&self.campaign.pool).await?;
        let mut premium_failed = false;
        for id in ids {
            let mut tx = self.campaign.pool.begin().await?;
            let locked: Option<Uuid>=sqlx::query_scalar("SELECT id FROM referral_campaign_rewards WHERE id=$1 AND applied_at IS NULL FOR UPDATE SKIP LOCKED")
                .bind(id).fetch_optional(&mut *tx).await?;
            if locked.is_none() {
                continue;
            }
            if ledger.grant_referral_in(&mut tx, id).await.is_err() {
                premium_failed = true;
                continue;
            }
            sqlx::query("UPDATE referral_campaign_rewards SET applied_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1").bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if let Some(sender) = &self.sender {
            for _ in 0..25 {
                let mut tx = self.campaign.pool.begin().await?;
                // Serialize delivery budgets across workers without holding the
                // referral qualification lock (5600) during SMTP I/O.
                sqlx::query("SELECT pg_advisory_xact_lock(5601)")
                    .execute(&mut *tx)
                    .await?;
                let row=sqlx::query("SELECT a.id,i.id AS code,i.recipient,owner.locale FROM referral_email_attempts a JOIN referral_email_invitations i ON i.id=a.invitation_id JOIN accounts owner ON owner.id=i.inviter_id WHERE a.delivered_at IS NULL AND owner.status='active' AND owner.email_verified_at IS NOT NULL AND (SELECT count(*) FROM referral_email_attempts sent WHERE sent.inviter_id=a.inviter_id AND sent.delivered_at>clock_timestamp()-interval '24 hours')<25 AND (i.kind<>'returning' OR (SELECT count(*) FROM referral_email_attempts sent JOIN referral_email_invitations invitation ON invitation.id=sent.invitation_id WHERE sent.inviter_id=a.inviter_id AND invitation.kind='returning' AND sent.delivered_at>clock_timestamp()-interval '30 days')<20) ORDER BY a.created_at FOR UPDATE OF a SKIP LOCKED LIMIT 1")
                    .fetch_optional(&mut *tx).await?;
                let Some(row) = row else {
                    break;
                };
                let id: Uuid = row.try_get("id")?;
                let lang: String = row.try_get("locale")?;
                let url = format!(
                    "{}/{lang}/referrals/{}",
                    self.campaign.origin,
                    row.try_get::<Uuid, _>("code")?
                );
                let subject = web::translate(&lang, "mail-title")?;
                let body = format!("{}\n\n{url}", web::translate(&lang, "mail-body")?);
                sender
                    .send(&oteryn_notify::Email {
                        id,
                        recipient: row.try_get("recipient")?,
                        subject,
                        text: body.clone(),
                        html: format!("<p>{}</p><p>{url}</p>", web::translate(&lang, "mail-body")?),
                        unsubscribe: None,
                    })
                    .await
                    .map_err(|_| Error::Unavailable)?;
                sqlx::query("UPDATE referral_email_attempts SET delivered_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1").bind(id).execute(&mut *tx).await?;
                tx.commit().await?;
            }
        }
        if premium_failed {
            Err(Error::Unavailable)
        } else {
            Ok(())
        }
    }
}
impl oteryn_db::outbox::Cleanup for Worker {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async { self.process().await.map_err(|e| e.to_string()) })
    }
}
