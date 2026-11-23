//! Adapts campaign copy to the existing optional News email outbox.
use crate::notification_preferences::MailLanguage;
use oteryn_notify::{Account, AccountState, EmailTemplate, Locale, Notifications, Unsubscribe};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

const SUBJECT_BYTES: usize = 200;
const MARKDOWN_BYTES: usize = 8_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid newsletter copy")]
    Copy,
    #[error("newsletter delivery is unavailable")]
    Delivery,
}

#[derive(Debug)]
pub struct Copy {
    subject: String,
    body: String,
}

impl Copy {
    pub fn new(subject: &str, markdown: &str) -> Result<Self, Error> {
        let subject = subject.trim();
        if subject.is_empty()
            || subject.len() > SUBJECT_BYTES
            || subject.chars().any(char::is_control)
            || markdown.len() > MARKDOWN_BYTES
            || markdown
                .chars()
                .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(Error::Copy);
        }
        let body = mail_text(markdown);
        if body.trim().is_empty() || body.len() > MARKDOWN_BYTES {
            return Err(Error::Copy);
        }
        Ok(Self {
            subject: subject.to_owned(),
            body,
        })
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn lines(&self) -> std::str::Lines<'_> {
        self.body.lines()
    }
}

fn mail_text(markdown: &str) -> String {
    let mut body = String::new();
    let mut link = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Text(text) | Event::Code(text) => body.push_str(&text),
            Event::SoftBreak | Event::HardBreak | Event::Rule => body.push('\n'),
            Event::Start(Tag::Item) => body.push_str("• "),
            Event::End(TagEnd::Item) => body.push('\n'),
            Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock | TagEnd::List(_),
            ) => {
                body.push_str("\n\n");
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link = Url::parse(&dest_url)
                    .ok()
                    .filter(|url| matches!(url.scheme(), "https" | "http") && url.host().is_some());
            }
            Event::End(TagEnd::Link) => {
                if let Some(url) = link.take() {
                    body.push_str(" (");
                    body.push_str(url.as_str());
                    body.push(')');
                }
            }
            _ => continue,
        }
    }
    body.trim().to_owned()
}

#[derive(Clone, Debug)]
pub struct Mailer {
    signer: Unsubscribe,
}

impl Mailer {
    pub fn configured() -> Option<Self> {
        let key = std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_KEY").ok()?;
        let base = std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_URL").ok()?;
        Self::new(key.into_bytes(), base).ok()
    }

    pub fn new(key: Vec<u8>, base: String) -> Result<Self, Error> {
        let unsubscribe_url = Url::parse(&base).map_err(|_| Error::Delivery)?;
        if unsubscribe_url.host().is_none()
            || !unsubscribe_url.username().is_empty()
            || unsubscribe_url.password().is_some()
        {
            return Err(Error::Delivery);
        }
        let signer = Unsubscribe::new(key, base).map_err(|_| Error::Delivery)?;
        Ok(Self { signer })
    }

    pub async fn enqueue(
        &self,
        pool: &PgPool,
        tx: &mut Transaction<'_, Postgres>,
        recipient: Uuid,
        email: String,
        language: MailLanguage,
        copy: &Copy,
    ) -> Result<Option<Uuid>, Error> {
        let template = EmailTemplate::Newsletter {
            title: copy.subject.clone(),
            body: copy.body.clone(),
        };
        let locale = match language {
            MailLanguage::Pl => Locale::Pl,
            MailLanguage::En => Locale::En,
        };
        Notifications {
            pool: pool.clone(),
            accounts: Arc::new(Recipient {
                id: recipient,
                account: Account { email, locale },
            }),
            unsubscribe: self.signer.clone(),
        }
        .enqueue(tx, recipient, &template)
        .await
        .map_err(|_| Error::Delivery)
    }
}

/// The campaign service resolves the recipient on the same transaction that queues mail.
struct Recipient {
    id: Uuid,
    account: Account,
}

impl std::fmt::Debug for Recipient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NewsletterRecipient([redacted])")
    }
}

impl AccountState for Recipient {
    fn active_verified(&self, id: Uuid) -> oteryn_notify::NotifyFuture<'_, Option<Account>> {
        Box::pin(async move { Ok((id == self.id).then(|| self.account.clone())) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_notify::Category;
    use sqlx::types::time::OffsetDateTime;

    #[test]
    fn copy_limits_and_readable_markdown() -> Result<(), Error> {
        let copy = Copy::new(
            " Update ",
            "Hello **traveler**\n\n[Site](https://example.org) [Other](javascript:sample) <i>news</i>",
        )?;
        assert_eq!(copy.subject(), "Update");
        assert_eq!(
            copy.body,
            "Hello traveler\n\nSite (https://example.org/) Other news"
        );
        assert!(Copy::new("Newsletter\nSubject", "Body").is_err());
        assert!(Copy::new("Newsletter", &"x".repeat(8_001)).is_err());
        assert!(Copy::new("Newsletter", "   ").is_err());
        Ok(())
    }

    #[test]
    fn campaign_link_binds_category_recipient_and_expiry() -> oteryn_notify::Result<()> {
        let mailer = Mailer::new(vec![7; 32], "https://example.org/en/unsubscribe".into())?;
        let recipient = Uuid::now_v7();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let url = Url::parse(&mailer.signer.link(recipient, Category::News, now)?)?;
        let (_, token) = url
            .query_pairs()
            .next()
            .ok_or("missing unsubscribe token")?;
        assert_eq!(
            mailer.signer.verify(&token, now)?,
            (recipient, Category::News)
        );
        assert!(mailer.signer.verify(&token, now + 30 * 86_400).is_err());
        Ok(())
    }
}
