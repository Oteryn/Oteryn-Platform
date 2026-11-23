//! Account-owned email preferences. Security messages have no editable preference.
use crate::{AccountId, mutation_actor::MutationActor};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailLanguage {
    Pl,
    En,
}

impl MailLanguage {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Choices {
    pub news: (bool, bool),
    pub support: (bool, bool),
    pub events: bool,
    pub forum_digest: bool,
    pub operational: Option<bool>,
}

#[derive(Debug)]
pub struct Preferences {
    pub language: String,
    pub choices: Choices,
}

#[derive(sqlx::FromRow)]
struct PreferenceRow {
    language: String,
    category: Option<String>,
    email_enabled: Option<bool>,
    portal_enabled: Option<bool>,
}

pub async fn current(
    pool: &PgPool,
    account: AccountId,
) -> Result<Option<Preferences>, sqlx::Error> {
    let rows: Vec<PreferenceRow> = sqlx::query_as(
        "SELECT coalesce(a.mail_locale, a.locale) AS language, p.category, p.email_enabled, p.portal_enabled
         FROM accounts a
         LEFT JOIN account_notification_prefs p ON p.account_id = a.id
         WHERE a.id = $1 AND a.status = 'active' AND a.email_verified_at IS NOT NULL",
    )
    .bind(account)
    .fetch_all(pool)
    .await?;
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let mut choices = Choices {
        news: (false, true),
        support: (false, true),
        ..Choices::default()
    };
    for row in &rows {
        let email = row.email_enabled.unwrap_or(false);
        let portal = row.portal_enabled.unwrap_or(true);
        match row.category.as_deref() {
            Some("news") => choices.news = (email, portal),
            Some("support") => choices.support = (email, portal),
            Some("events") => choices.events = email,
            Some("forum_digest") => choices.forum_digest = email,
            Some("operational") => choices.operational = Some(email),
            _ => continue,
        }
    }
    Ok(Some(Preferences {
        language: first.language.clone(),
        choices,
    }))
}

/// A missing language indicates a submission from the earlier two-category form.
pub async fn save(
    pool: &PgPool,
    actor: &MutationActor,
    language: Option<MailLanguage>,
    choices: Choices,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    let account = actor.account;
    let eligible = sqlx::query_scalar::<_, AccountId>(
        "SELECT id FROM accounts
         WHERE id = $1 AND status = 'active' AND email_verified_at IS NOT NULL
         FOR UPDATE",
    )
    .bind(account)
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !eligible || !actor.live(&mut tx).await? {
        return Ok(false);
    }
    let mut categories = vec!["news", "support"];
    let mut email = vec![choices.news.0, choices.support.0];
    let mut portal = vec![Some(choices.news.1), Some(choices.support.1)];
    if let Some(language) = language {
        sqlx::query("UPDATE accounts SET mail_locale = $2 WHERE id = $1")
            .bind(account)
            .bind(language.code())
            .execute(&mut *tx)
            .await?;
        categories.extend(["events", "forum_digest"]);
        email.extend([choices.events, choices.forum_digest]);
        portal.extend([None, None]);
    }
    if let Some(operational) = choices.operational {
        categories.push("operational");
        email.push(operational);
        portal.push(Some(false));
    }
    sqlx::query(
        "INSERT INTO account_notification_prefs(account_id, category, email_enabled, portal_enabled)
         SELECT $1, category, email, coalesce(portal, true)
         FROM unnest($2::text[], $3::boolean[], $4::boolean[]) AS input(category, email, portal)
         ON CONFLICT (account_id, category) DO UPDATE SET
             email_enabled = excluded.email_enabled,
             portal_enabled = CASE WHEN excluded.category IN ('news', 'support')
                 THEN excluded.portal_enabled ELSE account_notification_prefs.portal_enabled END,
             updated_at = CASE WHEN excluded.category = 'operational'
                 THEN clock_timestamp() ELSE now() END
         WHERE excluded.category <> 'operational'
             OR account_notification_prefs.email_enabled IS DISTINCT FROM excluded.email_enabled",
    )
    .bind(account)
    .bind(categories)
    .bind(email)
    .bind(portal)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

#[cfg(test)]
#[path = "notification_preferences_tests.rs"]
mod tests;
