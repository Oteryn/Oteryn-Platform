mod http;
pub(crate) use http::{configured_router, label};

use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, Permission},
};
use oteryn_identity::{
    credentials,
    newsletter_mail::{Copy, Mailer},
    notification_preferences::MailLanguage,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction, types::time::OffsetDateTime};
use std::time::{Duration, Instant};
use uuid::Uuid;

const BATCH_SIZE: usize = 50;
const BATCHES_PER_REQUEST: usize = 20;
const SEND_AUTH_LIFETIME: Duration = Duration::from_secs(300);

struct SendApproval(Instant);

impl SendApproval {
    fn ensure_recent(&self) -> Result<(), Error> {
        if self.0.elapsed() >= SEND_AUTH_LIFETIME {
            return Err(Error::RecentAuth);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("campaign copy is invalid")]
    Invalid,
    #[error("campaign does not exist")]
    Missing,
    #[error("campaign cannot perform this action in its current state")]
    State,
    #[error("staff authentication must be renewed")]
    RecentAuth,
    #[error("the staff account needs newsletter email enabled")]
    SelfOptIn,
    #[error("newsletter mail is unavailable")]
    Unavailable,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    #[serde(rename = "_csrf")]
    _csrf: String,
    audience: Audience,
    subject_pl: String,
    body_pl: String,
    subject_en: String,
    body_en: String,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Audience {
    All,
    Pl,
    En,
}

impl Audience {
    const fn locale(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Pl => Some("pl"),
            Self::En => Some("en"),
        }
    }
}

#[derive(sqlx::FromRow)]
struct Campaign {
    id: Uuid,
    audience_locale: Option<String>,
    subject_pl: String,
    body_pl: String,
    subject_en: String,
    body_en: String,
    cursor: Option<Uuid>,
    eligible_count: i64,
    queued_count: i64,
    started_at: Option<OffsetDateTime>,
    queued_at: Option<OffsetDateTime>,
    tested: bool,
}

impl Campaign {
    fn copy(&self, language: MailLanguage) -> Result<Copy, Error> {
        let (subject, body) = match language {
            MailLanguage::Pl => (&self.subject_pl, &self.body_pl),
            MailLanguage::En => (&self.subject_en, &self.body_en),
        };
        Copy::new(subject, body).map_err(|_| Error::Invalid)
    }

    fn state(&self) -> &'static str {
        if self.queued_at.is_some() {
            "queued"
        } else if self.started_at.is_some() {
            "queuing"
        } else {
            "draft"
        }
    }
}

#[derive(Clone)]
struct Newsletters {
    pool: PgPool,
    mailer: Option<Mailer>,
}

struct Staff {
    context: CheckContext,
    authorization: MutationAuthorization,
    request_id: Uuid,
}

impl Staff {
    async fn check(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        permission: Permission,
    ) -> Result<(), Error> {
        self.authorization
            .check_in(tx, self.context, permission)
            .await?;
        Ok(())
    }

    async fn audit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        campaign: Uuid,
        action: &str,
        before: Value,
        after: Value,
    ) -> Result<(), Error> {
        audit::append(
            tx,
            Entry {
                context: AuditContext {
                    actor: Some(self.context.account),
                    request_id: self.request_id,
                },
                action,
                target: &campaign.to_string(),
                before,
                after,
            },
        )
        .await?;
        Ok(())
    }
}

impl Newsletters {
    async fn create(&self, staff: &Staff, draft: Draft) -> Result<Uuid, Error> {
        let pl = Copy::new(&draft.subject_pl, &draft.body_pl).map_err(|_| Error::Invalid)?;
        let en = Copy::new(&draft.subject_en, &draft.body_en).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        staff.check(&mut tx, Permission::NewsletterCompose).await?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO newsletter_campaigns
                (created_by, audience_locale, subject_pl, body_pl, subject_en, body_en)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(staff.context.account)
        .bind(draft.audience.locale())
        .bind(pl.subject())
        .bind(&draft.body_pl)
        .bind(en.subject())
        .bind(&draft.body_en)
        .fetch_one(&mut *tx)
        .await?;
        staff
            .audit(
                &mut tx,
                id,
                "newsletter.campaign.saved",
                json!(null),
                json!({"audience_locale": draft.audience.locale()}),
            )
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn load(
        connection: &mut sqlx::PgConnection,
        id: Uuid,
        lock: bool,
    ) -> Result<Campaign, Error> {
        if lock {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM newsletter_campaigns WHERE id = $1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or(Error::Missing)?;
        }
        sqlx::query_as(
            "SELECT c.*, EXISTS (SELECT 1 FROM newsletter_campaign_tests WHERE campaign_id = c.id) AS tested
             FROM newsletter_campaigns c WHERE c.id = $1",
        )
        .bind(id)
        .fetch_optional(connection)
        .await?
        .ok_or(Error::Missing)
    }

    async fn test(&self, staff: &Staff, id: Uuid, language: MailLanguage) -> Result<(), Error> {
        let mailer = self.mailer.as_ref().ok_or(Error::Unavailable)?;
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        let campaign = Self::load(&mut tx, id, true).await?;
        staff.check(&mut tx, Permission::NewsletterTest).await?;
        if campaign.started_at.is_some() {
            return Err(Error::State);
        }
        let actor = staff.context.account;
        let inserted = sqlx::query(
            "INSERT INTO newsletter_campaign_tests (campaign_id, actor, locale)
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(actor)
        .bind(language.code())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 0 {
            return Ok(());
        }
        let recipient = sqlx::query_scalar(
            "SELECT a.email::text FROM accounts a
             JOIN account_notification_prefs p ON p.account_id = a.id
             WHERE a.id = $1 AND a.status = 'active' AND a.email_verified_at IS NOT NULL
               AND p.category = 'news' AND p.email_enabled",
        )
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::SelfOptIn)?;
        if mailer
            .enqueue(
                &self.pool,
                &mut tx,
                actor,
                recipient,
                language,
                &campaign.copy(language)?,
            )
            .await
            .map_err(|_| Error::Unavailable)?
            .is_none()
        {
            return Err(Error::SelfOptIn);
        }
        staff
            .audit(
                &mut tx,
                id,
                "newsletter.campaign.test_queued",
                json!(null),
                json!({"locale": language.code()}),
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn authorize_send(&self, staff: &Staff, password: String) -> Result<SendApproval, Error> {
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        staff.check(&mut tx, Permission::NewsletterSend).await?;
        let proof = staff.context.session_proof.ok_or(Error::RecentAuth)?;
        let Some((actor, hash)) = credentials::authenticated(&mut tx, &proof.session_hash).await?
        else {
            return Err(Error::RecentAuth);
        };
        if actor != staff.context.account || !credentials::verify(password, hash).await? {
            return Err(Error::RecentAuth);
        }
        tx.commit().await?;
        Ok(SendApproval(Instant::now()))
    }

    async fn send(&self, staff: &Staff, id: Uuid, password: String) -> Result<(), Error> {
        let approval = self.authorize_send(staff, password).await?;
        for _ in 0..BATCHES_PER_REQUEST {
            if self.batch(staff, id, &approval).await? {
                break;
            }
        }
        Ok(())
    }

    async fn batch(&self, staff: &Staff, id: Uuid, approval: &SendApproval) -> Result<bool, Error> {
        let mailer = self.mailer.as_ref().ok_or(Error::Unavailable)?;
        let mut tx = self.pool.begin().await?;
        rbac::mutation_budget(&mut tx).await?;
        let campaign = Self::load(&mut tx, id, true).await?;
        staff.check(&mut tx, Permission::NewsletterSend).await?;
        approval.ensure_recent()?;
        if campaign.queued_at.is_some() {
            return Ok(true);
        }
        if !campaign.tested {
            return Err(Error::State);
        }
        let eligible = if campaign.started_at.is_none() {
            let count = sqlx::query(
                "INSERT INTO newsletter_campaign_recipients (campaign_id, account_id)
                 SELECT $1, a.id FROM account_notification_prefs p JOIN accounts a ON a.id = p.account_id
                 WHERE p.category = 'news' AND p.email_enabled
                   AND a.status = 'active' AND a.email_verified_at IS NOT NULL
                   AND ($2::text IS NULL OR coalesce(a.mail_locale, a.locale) = $2)",
            )
            .bind(id)
            .bind(&campaign.audience_locale)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            i64::try_from(count).map_err(|_| Error::Unavailable)?
        } else {
            campaign.eligible_count
        };
        let mut recipients: Vec<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT r.account_id, a.email::text, coalesce(a.mail_locale, a.locale)
             FROM newsletter_campaign_recipients r
             LEFT JOIN account_notification_prefs p
               ON p.account_id = r.account_id AND p.category = 'news' AND p.email_enabled
             LEFT JOIN accounts a
               ON a.id = p.account_id AND a.status = 'active' AND a.email_verified_at IS NOT NULL
               AND ($4::text IS NULL OR coalesce(a.mail_locale, a.locale) = $4)
             WHERE r.campaign_id = $1 AND ($2::uuid IS NULL OR r.account_id > $2)
             ORDER BY r.account_id LIMIT $3",
        )
        .bind(id)
        .bind(campaign.cursor)
        .bind(BATCH_SIZE as i64 + 1)
        .bind(&campaign.audience_locale)
        .fetch_all(&mut *tx)
        .await?;
        let done = recipients.len() <= BATCH_SIZE;
        recipients.truncate(BATCH_SIZE);
        let cursor = recipients.last().map(|r| r.0).or(campaign.cursor);
        let pl = campaign.copy(MailLanguage::Pl)?;
        let en = campaign.copy(MailLanguage::En)?;
        let mut queued = campaign.queued_count;
        for (recipient, email, locale) in recipients {
            let Some(email) = email else { continue };
            let (language, copy) = match locale.as_deref() {
                Some("pl") => (MailLanguage::Pl, &pl),
                Some("en") => (MailLanguage::En, &en),
                _ => return Err(Error::Unavailable),
            };
            if mailer
                .enqueue(&self.pool, &mut tx, recipient, email, language, copy)
                .await
                .map_err(|_| Error::Unavailable)?
                .is_some()
            {
                queued += 1;
            }
        }
        approval.ensure_recent()?;
        sqlx::query(
            "UPDATE newsletter_campaigns SET cursor = $2, eligible_count = $3,
                 queued_count = $4, started_at = coalesce(started_at, now()),
                 queued_at = CASE WHEN $5 THEN now() ELSE NULL END
             WHERE id = $1",
        )
        .bind(id)
        .bind(cursor)
        .bind(eligible)
        .bind(queued)
        .bind(done)
        .execute(&mut *tx)
        .await?;
        if done {
            sqlx::query("DELETE FROM newsletter_campaign_recipients WHERE campaign_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        staff.audit(&mut tx, id, "newsletter.campaign.batch_queued",
            json!({"state": campaign.state(), "queued_count": campaign.queued_count}),
            json!({"state": if done {"queued"} else {"queuing"}, "eligible_count": eligible, "queued_count": queued})
        ).await?;
        tx.commit().await?;
        Ok(done)
    }
}

#[cfg(test)]
mod tests;
