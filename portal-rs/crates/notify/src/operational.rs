//! The existing notification worker delivers approved operational mail.
use crate::{Account, Category, EMAIL_JOB, Email, EmailSender, Result, Unsubscribe, service};
use askama::Template;
use oteryn_db::outbox::{self, OffsetDateTime, PgPool, Postgres, Transaction, Uuid};
use std::time::Duration;

pub const SUBJECT_LIMIT: usize = 200;
pub const BODY_LIMIT: usize = 4000;

pub fn valid_content(subject: &str, body: &str) -> bool {
    !subject.trim().is_empty()
        && subject.len() <= SUBJECT_LIMIT
        && !subject.chars().any(char::is_control)
        && !body.trim().is_empty()
        && body.len() <= BODY_LIMIT
        && !body
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
}

pub fn unsubscribe_label(locale: crate::Locale) -> Result<String> {
    service::translate(locale, "operational-unsubscribe")
}

#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><body><h1>{{ subject }}</h1>{% for line in body.lines() %}{{ line }}<br>{% endfor %}<p><a href=\"{{ url }}\">{{ footer }}</a></p></body></html>",
    ext = "html"
)]
struct Html<'a> {
    lang: &'a str,
    subject: &'a str,
    body: &'a str,
    url: &'a str,
    footer: &'a str,
}

/// Add one approved recipient to the caller's approval transaction.
pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    unsubscribe: &Unsubscribe,
    account_id: Uuid,
    account: Account,
    subject: &str,
    body: &str,
) -> Result<(Uuid, Uuid)> {
    if !valid_content(subject, body) {
        return Err("invalid operational mail content".into());
    }
    let _: lettre::message::Mailbox = account.email.parse()?;
    let now = OffsetDateTime::now_utc();
    let mut url = unsubscribe.operational_base(account.locale)?;
    let token = unsubscribe.token(
        account_id,
        Category::Operational,
        now.unix_timestamp() + 30 * 86400,
    )?;
    url.query_pairs_mut().append_pair("token", &token);
    let url = url.as_str();
    let footer = unsubscribe_label(account.locale)?;
    let plain = format!("{body}\n\n{footer}: {url}\n");
    let html = Html {
        lang: match account.locale {
            crate::Locale::Pl => "pl",
            crate::Locale::En => "en",
        },
        subject,
        body,
        url,
        footer: &footer,
    }
    .render()?;
    let id: Uuid = outbox::query_scalar(
        "INSERT INTO email_outbox(account_id,category,recipient,subject,text_body,html_body,unsubscribe_url)
         VALUES($1,'operational',$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(account_id)
    .bind(account.email)
    .bind(subject)
    .bind(plain)
    .bind(html)
    .bind(url)
    .fetch_one(&mut **tx)
    .await?;
    // SMTP cannot confirm whether a timed-out request was accepted. One attempt
    // keeps an uncertain result from triggering a second delivery.
    let job = outbox::enqueue(
        tx,
        EMAIL_JOB,
        &serde_json::to_string(&id)?,
        &id.to_string(),
        now,
        1,
    )
    .await?;
    Ok((id, job))
}

pub(crate) async fn deliver(
    pool: &PgPool,
    account: Uuid,
    email: &Email,
    sender: &dyn EmailSender,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    oteryn_db::mutation_budget(&mut tx).await?;
    // The account and consent locks serialize dispatch with preference changes
    // and address/locale changes. SMTP is bounded while the locks are held.
    let account_matches: Option<bool> = outbox::query_scalar(
        "SELECT a.status='active' AND a.email=$2 AND a.locale=r.locale
                AND a.email_verified_at IS NOT DISTINCT FROM r.verified_at
         FROM accounts a JOIN operational_mail_recipients r ON r.account_id=a.id
         WHERE a.id=$1 AND r.mail_id=$3 FOR SHARE OF a",
    )
    .bind(account)
    .bind(&email.recipient)
    .bind(email.id)
    .fetch_optional(&mut *tx)
    .await?;
    let consent_matches: Option<bool> = outbox::query_scalar(
        "SELECT p.email_enabled AND p.updated_at=r.consent_at
         FROM account_notification_prefs p
         JOIN operational_mail_recipients r ON r.account_id=p.account_id
         WHERE p.account_id=$1 AND p.category='operational' AND r.mail_id=$2 FOR SHARE OF p",
    )
    .bind(account)
    .bind(email.id)
    .fetch_optional(&mut *tx)
    .await?;
    if account_matches != Some(true) || consent_matches != Some(true) {
        outbox::query("UPDATE operational_mail_recipients SET skipped_at=now() WHERE mail_id=$1")
            .bind(email.id)
            .execute(&mut *tx)
            .await?;
    } else {
        tokio::time::timeout(Duration::from_secs(10), sender.send(email))
            .await
            .map_err(|_| "operational mail delivery failed")?
            .map_err(|_| "operational mail delivery failed")?;
        outbox::query("UPDATE email_outbox SET delivered_at=now(),updated_at=now() WHERE id=$1")
            .bind(email.id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Locale;

    #[test]
    fn operational_links_use_the_recipient_language_and_category() -> Result<()> {
        let links = Unsubscribe::new(vec![3; 32], "https://portal.example/en/unsubscribe".into())?;
        assert_eq!(
            links.operational_base(Locale::Pl)?.path(),
            "/pl/unsubscribe"
        );
        assert_eq!(
            links.operational_base(Locale::En)?.path(),
            "/en/unsubscribe"
        );
        let account = Uuid::now_v7();
        let token = links.token(account, Category::Operational, 60)?;
        assert_eq!(links.verify(&token, 59)?, (account, Category::Operational));
        assert!(links.verify(&token, 60).is_err());
        let wrong = Unsubscribe::new(vec![3; 32], "https://portal.example/en/account".into())?;
        assert!(!wrong.operational_ready());
        assert_ne!(
            unsubscribe_label(Locale::Pl)?,
            unsubscribe_label(Locale::En)?
        );
        Ok(())
    }
}
