//! Immutable consent snapshots and independent review of Platform operational email.
pub(super) mod http;

use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, Permission},
};
use oteryn_notify::{Account, Locale, Unsubscribe, operational};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub(super) const AUDIENCE_LIMIT: i64 = 1000;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("invalid campaign")]
    Invalid,
    #[error("campaign not found")]
    Missing,
    #[error("campaign cannot be reviewed")]
    Conflict,
    #[error("campaign has no eligible recipients")]
    Empty,
    #[error("campaign audience exceeds the limit")]
    AudienceLimit,
    #[error("campaign preview rate exceeded")]
    RateLimit(i64),
    #[error("operational mail unavailable")]
    Unavailable,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub(super) enum Audience {
    All,
    Pl,
    En,
}

impl std::str::FromStr for Audience {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        match value {
            "all" => Ok(Self::All),
            "pl" => Ok(Self::Pl),
            "en" => Ok(Self::En),
            _ => Err(Error::Invalid),
        }
    }
}

impl TryFrom<String> for Audience {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        value.parse()
    }
}

impl Audience {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
}

pub(super) struct Draft {
    pub audience: Audience,
    pub subject_pl: String,
    pub body_pl: String,
    pub subject_en: String,
    pub body_en: String,
}

impl Draft {
    pub fn validate(&self) -> Result<(), Error> {
        if operational::valid_content(&self.subject_pl, &self.body_pl)
            && operational::valid_content(&self.subject_en, &self.body_en)
        {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    auth: &MutationAuthorization,
    actor: CheckContext,
    permission: Permission,
) -> Result<(), Error> {
    rbac::authorize_mutation(tx, &[], auth, actor, &[Permission::Access, permission]).await?;
    Ok(())
}

pub(super) async fn preview(
    pool: &PgPool,
    auth: &MutationAuthorization,
    actor: CheckContext,
    id: Uuid,
    draft: &Draft,
) -> Result<(), Error> {
    draft.validate()?;
    if id.get_version_num() != 7 {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    authorize(&mut tx, auth, actor, Permission::ComposeOperationalMail).await?;
    let existing: Option<bool> = sqlx::query_scalar(
        "SELECT (author,audience,subject_pl,body_pl,subject_en,body_en)=($2,$3,$4,$5,$6,$7)
         FROM operational_mail_campaigns WHERE id=$1",
    )
    .bind(id)
    .bind(actor.account)
    .bind(draft.audience.key())
    .bind(&draft.subject_pl)
    .bind(&draft.body_pl)
    .bind(&draft.subject_en)
    .bind(&draft.body_en)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(existing_matches) = existing {
        return if existing_matches {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    sqlx::query(
        "INSERT INTO operational_mail_campaigns(id,author,audience,subject_pl,body_pl,subject_en,body_en)
         VALUES($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(actor.account)
    .bind(draft.audience.key())
    .bind(&draft.subject_pl)
    .bind(&draft.body_pl)
    .bind(&draft.subject_en)
    .bind(&draft.body_en)
    .execute(&mut *tx)
    .await?;
    let recipients = sqlx::query(
        "INSERT INTO operational_mail_recipients(campaign_id,account_id,locale,address_digest,verified_at,consent_at)
         SELECT $1,a.id,coalesce(a.mail_locale,a.locale),sha256(convert_to(a.email,'UTF8')),a.email_verified_at,p.updated_at
         FROM account_notification_prefs p JOIN accounts a ON a.id=p.account_id
         WHERE p.category='operational' AND p.email_enabled AND a.status='active'
           AND a.email_verified_at IS NOT NULL AND coalesce(a.mail_locale,a.locale) IN ('pl','en')
           AND ($2='all' OR coalesce(a.mail_locale,a.locale)=$2)
         ORDER BY a.id LIMIT $3",
    )
    .bind(id)
    .bind(draft.audience.key())
    .bind(AUDIENCE_LIMIT + 1)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if recipients == 0 {
        return Err(Error::Empty);
    }
    if recipients > AUDIENCE_LIMIT as u64 {
        return Err(Error::AudienceLimit);
    }
    record(
        &mut tx,
        actor.account,
        id,
        "mail.operational.previewed",
        json!({"audience":draft.audience.key(),"eligible":recipients}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Decision {
    Approve,
    Reject,
}

#[derive(sqlx::FromRow)]
pub(super) struct Campaign {
    pub id: Uuid,
    pub author: Uuid,
    pub audience: String,
    pub subject_pl: String,
    pub body_pl: String,
    pub subject_en: String,
    pub body_en: String,
    pub status: String,
}

pub(super) async fn review(
    pool: &PgPool,
    auth: &MutationAuthorization,
    actor: CheckContext,
    id: Uuid,
    decision: Decision,
    unsubscribe: Option<&Unsubscribe>,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    authorize(&mut tx, auth, actor, Permission::ReviewOperationalMail).await?;
    let campaign: Campaign = sqlx::query_as(
        "SELECT id,author,audience,subject_pl,body_pl,subject_en,body_en,
            coalesce(decision,CASE WHEN expires_at<=clock_timestamp() THEN 'expired' ELSE 'pending' END) AS status
         FROM operational_mail_campaigns WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::Missing)?;
    if campaign.author == actor.account {
        return Err(rbac::Error::Denied.into());
    }
    if campaign.status != "pending" {
        return Err(Error::Conflict);
    }
    let (status, action, queued) = match decision {
        Decision::Approve => (
            "approved",
            "mail.operational.approved",
            queue(
                &mut tx,
                &campaign,
                unsubscribe
                    .filter(|u| u.operational_ready())
                    .ok_or(Error::Unavailable)?,
            )
            .await?,
        ),
        Decision::Reject => ("rejected", "mail.operational.rejected", 0),
    };
    let changed = sqlx::query(
        "UPDATE operational_mail_campaigns SET decision=$2,reviewer=$3,reviewed_at=clock_timestamp()
         WHERE id=$1 AND expires_at>clock_timestamp()",
    )
    .bind(id)
    .bind(status)
    .bind(actor.account)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed != 1 {
        return Err(Error::Conflict);
    }
    record(&mut tx, actor.account, id, action, json!({"queued":queued})).await?;
    tx.commit().await?;
    Ok(())
}

async fn queue(
    tx: &mut Transaction<'_, Postgres>,
    campaign: &Campaign,
    unsubscribe: &Unsubscribe,
) -> Result<usize, Error> {
    let recipients: Vec<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT a.id,a.email,r.locale='pl'
         FROM operational_mail_recipients r
         JOIN accounts a ON a.id=r.account_id
         JOIN account_notification_prefs p ON p.account_id=a.id AND p.category='operational'
         WHERE r.campaign_id=$1 AND a.status='active' AND coalesce(a.mail_locale,a.locale)=r.locale
           AND a.email_verified_at=r.verified_at AND sha256(convert_to(a.email,'UTF8'))=r.address_digest
           AND p.email_enabled AND p.updated_at=r.consent_at
         ORDER BY a.id",
    )
    .bind(campaign.id)
    .fetch_all(&mut **tx)
    .await?;
    if recipients.is_empty() {
        return Err(Error::Empty);
    }
    for (id, email, pl) in &recipients {
        let (locale, subject, body) = if *pl {
            (Locale::Pl, &campaign.subject_pl, &campaign.body_pl)
        } else {
            (Locale::En, &campaign.subject_en, &campaign.body_en)
        };
        let account = Account {
            email: email.clone(),
            locale,
        };
        let (mail, job) = operational::enqueue(tx, unsubscribe, *id, account, subject, body)
            .await
            .map_err(|_| Error::Unavailable)?;
        sqlx::query(
            "UPDATE operational_mail_recipients SET mail_id=$3,job_id=$4
             WHERE campaign_id=$1 AND account_id=$2",
        )
        .bind(campaign.id)
        .bind(id)
        .bind(mail)
        .bind(job)
        .execute(&mut **tx)
        .await?;
    }
    Ok(recipients.len())
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    id: Uuid,
    action: &str,
    after: serde_json::Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before: json!(null),
            after,
        },
    )
    .await?;
    Ok(())
}
