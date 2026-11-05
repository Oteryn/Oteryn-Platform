use askama::Template;
use hmac::{Hmac, Mac};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{MultiPart, header},
    transport::smtp::authentication::Credentials,
};
use oteryn_db::outbox::{self, OffsetDateTime, PgPool, Postgres, Transaction, Uuid};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type NotifyFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub const EMAIL_JOB: &str = "notify.email";
fn invalid() -> Box<dyn std::error::Error + Send + Sync> {
    "notification refused or invalid input".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Security,
    News,
    Support,
}
impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::News => "news",
            Self::Support => "support",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "security" => Ok(Self::Security),
            "news" => Ok(Self::News),
            "support" => Ok(Self::Support),
            _ => Err(invalid()),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Locale {
    Pl,
    En,
}
#[derive(Debug, Clone)]
pub struct Account {
    pub email: String,
    pub locale: Locale,
}
/// Return None for unknown, inactive, unverified or ambiguous account state.
pub trait AccountState: std::fmt::Debug + Send + Sync {
    fn active_verified(&self, account: Uuid) -> NotifyFuture<'_, Option<Account>>;
}
#[derive(Debug)]
pub struct DenyAccounts;
impl AccountState for DenyAccounts {
    fn active_verified(&self, _: Uuid) -> NotifyFuture<'_, Option<Account>> {
        Box::pin(async { Ok(None) })
    }
}
#[derive(Debug, Clone)]
pub enum EmailTemplate {
    NewDevice { device: String },
    PasswordChanged,
    News { title: String, body: String },
    Support { body: String },
}
#[derive(Debug, Clone)]
pub struct Rendered {
    pub subject: String,
    pub text: String,
    pub html: String,
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><body><h1>{{ subject }}</h1><p>{{ body }}</p></body></html>",
    ext = "html"
)]
struct Html<'a> {
    lang: &'a str,
    subject: &'a str,
    body: &'a str,
}
#[derive(Template)]
#[template(source = "{{ subject }}\n\n{{ body }}\n", ext = "txt")]
struct Text<'a> {
    subject: &'a str,
    body: &'a str,
}
impl EmailTemplate {
    pub fn category(&self) -> Category {
        match self {
            Self::NewDevice { .. } | Self::PasswordChanged => Category::Security,
            Self::News { .. } => Category::News,
            Self::Support { .. } => Category::Support,
        }
    }
    pub fn render(&self, locale: Locale) -> Result<Rendered> {
        let pl = matches!(locale, Locale::Pl);
        let tr = |key| translate(locale, key);
        let (subject, body) = match self {
            Self::NewDevice { device } => (
                tr("new-device")?,
                format!("{}: {device}", tr("device-login")?),
            ),
            Self::PasswordChanged => (tr("password-changed")?, tr("password-body")?),
            Self::News { title, body } => (title.clone(), body.clone()),
            Self::Support { body } => (tr("support")?, body.clone()),
        };
        if subject.trim().is_empty()
            || subject.len() > 200
            || subject.contains(['\r', '\n'])
            || body.trim().is_empty()
            || body.len() > 16384
        {
            return Err(invalid());
        }
        let html = Html {
            lang: if pl { "pl" } else { "en" },
            subject: &subject,
            body: &body,
        }
        .render()?;
        let text = Text {
            subject: &subject,
            body: &body,
        }
        .render()?;
        Ok(Rendered {
            subject,
            text,
            html,
        })
    }
}
/// HMAC key must contain at least 32 random bytes. Tokens are purpose-bound and expire.
#[derive(Clone)]
pub struct Unsubscribe {
    key: Vec<u8>,
    base: String,
}
impl std::fmt::Debug for Unsubscribe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Unsubscribe { redacted }")
    }
}
impl Unsubscribe {
    pub fn new(key: Vec<u8>, base: String) -> Result<Self> {
        if key.len() < 32
            || !base.starts_with("https://")
            || base.contains(['\r', '\n', '?', '#', '<', '>', ' '])
        {
            return Err(invalid());
        }
        Ok(Self { key, base })
    }
    pub fn token(&self, account: Uuid, category: Category, expires: i64) -> Result<String> {
        if category == Category::Security {
            return Err(invalid());
        }
        let payload = format!("unsubscribe:{account}:{}:{expires}", category.as_str());
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&self.key)?;
        mac.update(payload.as_bytes());
        Ok(format!(
            "{payload}:{}",
            hex::encode(mac.finalize().into_bytes())
        ))
    }
    pub fn verify(&self, token: &str, now: i64) -> Result<(Uuid, Category)> {
        if token.len() > 256 {
            return Err(invalid());
        }
        let (payload, signature) = token.rsplit_once(':').ok_or_else(invalid)?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&self.key)?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&hex::decode(signature)?)?;
        let parts: Vec<_> = payload.split(':').collect();
        if parts.len() != 4 || parts[0] != "unsubscribe" || parts[3].parse::<i64>()? <= now {
            return Err(invalid());
        }
        let category = Category::parse(parts[2])?;
        if category == Category::Security {
            return Err(invalid());
        }
        Ok((Uuid::parse_str(parts[1])?, category))
    }
}
#[derive(Debug, Clone)]
pub struct Email {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
    pub text: String,
    pub html: String,
    pub unsubscribe: Option<String>,
}
pub trait EmailSender: std::fmt::Debug + Send + Sync {
    fn send<'a>(&'a self, email: &'a Email) -> NotifyFuture<'a, ()>;
}
#[derive(Debug, Default)]
pub struct CaptureSender {
    pub emails: Mutex<Vec<Email>>,
}
impl EmailSender for CaptureSender {
    fn send<'a>(&'a self, email: &'a Email) -> NotifyFuture<'a, ()> {
        Box::pin(async move {
            self.emails
                .lock()
                .map_err(|_| invalid())?
                .push(email.clone());
            Ok(())
        })
    }
}
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}
impl SmtpSender {
    /// Uses STARTTLS with certificate validation; credentials are never logged.
    pub fn new(host: &str, username: String, password: String, from: String) -> Result<Self> {
        let _: lettre::message::Mailbox = from.parse()?;
        Ok(Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?
                .credentials(Credentials::new(username, password))
                .build(),
            from,
        })
    }
}
impl EmailSender for SmtpSender {
    fn send<'a>(&'a self, email: &'a Email) -> NotifyFuture<'a, ()> {
        Box::pin(async move {
            self.transport
                .send(smtp_message(&self.from, email)?)
                .await?;
            Ok(())
        })
    }
}
#[derive(Debug)]
pub struct Notifications {
    pub pool: PgPool,
    pub accounts: Arc<dyn AccountState>,
    pub unsubscribe: Unsubscribe,
}
impl Notifications {
    async fn account(&self, id: Uuid) -> Result<Account> {
        self.accounts.active_verified(id).await?.ok_or_else(invalid)
    }
    /// Missing optional preferences mean email off and portal on.
    pub async fn preference(&self, account: Uuid, category: Category) -> Result<(bool, bool)> {
        self.account(account).await?;
        if category == Category::Security {
            return Ok((true, true));
        }
        let value = outbox::query_as("SELECT email_enabled,portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2")
            .bind(account).bind(category.as_str()).fetch_optional(&self.pool).await?;
        Ok(value.unwrap_or((false, true)))
    }
    pub async fn preferences(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        category: Category,
        email: bool,
        portal: bool,
    ) -> Result<()> {
        self.account(account).await?;
        if category == Category::Security && (!email || !portal) {
            return Err(invalid());
        }
        outbox::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,$2,$3,$4) ON CONFLICT(account_id,category) DO UPDATE SET email_enabled=$3,portal_enabled=$4,updated_at=now()")
            .bind(account).bind(category.as_str()).bind(email).bind(portal).execute(&mut **tx).await?;
        Ok(())
    }
    /// Caller owns commit/rollback. Retry with the returned ID, never a second enqueue.
    pub async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        template: &EmailTemplate,
    ) -> Result<Option<Uuid>> {
        let state = self.account(account).await?;
        let _: lettre::message::Mailbox = state.email.parse()?;
        let rendered = template.render(state.locale)?;
        let category = template.category();
        let pref: Option<(bool,bool)> = outbox::query_as("SELECT email_enabled,portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2").bind(account).bind(category.as_str()).fetch_optional(&mut **tx).await?;
        let (email, portal) = if category == Category::Security {
            (true, true)
        } else {
            pref.unwrap_or((false, true))
        };
        if portal {
            outbox::query(
                "INSERT INTO notifications(account_id,category,subject,body) VALUES($1,$2,$3,$4)",
            )
            .bind(account)
            .bind(category.as_str())
            .bind(&rendered.subject)
            .bind(&rendered.text)
            .execute(&mut **tx)
            .await?;
        }
        if !email {
            return Ok(None);
        }
        let now = OffsetDateTime::now_utc();
        let unsubscribe = if category == Category::Security {
            None
        } else {
            Some(format!(
                "{}?token={}",
                self.unsubscribe.base,
                self.unsubscribe
                    .token(account, category, now.unix_timestamp() + 30 * 86400)?
            ))
        };
        let id: Uuid = outbox::query_scalar("INSERT INTO email_outbox(account_id,category,recipient,subject,text_body,html_body,unsubscribe_url) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(account).bind(category.as_str()).bind(state.email).bind(rendered.subject).bind(rendered.text).bind(rendered.html).bind(unsubscribe).fetch_one(&mut **tx).await?;
        outbox::enqueue(
            tx,
            EMAIL_JOB,
            &serde_json::to_string(&id.to_string())?,
            &id.to_string(),
            now,
            5,
        )
        .await?;
        Ok(Some(id))
    }
    /// HTTP adapters must invoke only for a one-click POST, never a GET.
    pub async fn unsubscribe(&self, tx: &mut Transaction<'_, Postgres>, token: &str) -> Result<()> {
        let (account, category) = self
            .unsubscribe
            .verify(token, OffsetDateTime::now_utc().unix_timestamp())?;
        self.account(account).await?;
        outbox::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled) VALUES($1,$2,false) ON CONFLICT(account_id,category) DO UPDATE SET email_enabled=false,updated_at=now()")
            .bind(account).bind(category.as_str()).execute(&mut **tx).await?;
        Ok(())
    }
    pub async fn list(
        &self,
        account: Uuid,
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        self.account(account).await?;
        if !(1..=100).contains(&limit) {
            return Err(invalid());
        }
        let rows: Vec<(Uuid,String,String,Option<OffsetDateTime>)> = outbox::query_as("SELECT id,subject,body,read_at FROM notifications WHERE account_id=$1 AND (NOT $2 OR read_at IS NULL) ORDER BY created_at DESC,id DESC LIMIT $3").bind(account).bind(unread_only).bind(limit).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(id, subject, body, read_at)| Notification {
                id,
                subject,
                body,
                read_at,
            })
            .collect())
    }
    pub async fn mark_read(&self, account: Uuid, id: Uuid, read: bool) -> Result<bool> {
        self.account(account).await?;
        Ok(outbox::query("UPDATE notifications SET read_at=CASE WHEN $3 THEN now() ELSE NULL END,updated_at=now() WHERE id=$1 AND account_id=$2").bind(id).bind(account).bind(read).execute(&self.pool).await?.rows_affected()==1)
    }
    /// Called by a lease-renewing worker. SMTP is at least once after a process crash.
    pub async fn deliver(&self, job: &outbox::Job, sender: &dyn EmailSender) -> Result<()> {
        if job.kind != EMAIL_JOB {
            return Err(invalid());
        }
        let id = Uuid::parse_str(&serde_json::from_str::<String>(&job.payload)?)?;
        let row: (Uuid,String,String,String,String,String,Option<String>,Option<OffsetDateTime>) = outbox::query_as("SELECT account_id,category,recipient,subject,text_body,html_body,unsubscribe_url,delivered_at FROM email_outbox WHERE id=$1").bind(id).fetch_one(&self.pool).await?;
        let (account, category, recipient, subject, text, html, unsubscribe, delivered) = row;
        if delivered.is_some() {
            return Ok(());
        }
        let state = self.account(account).await?;
        if state.email != recipient {
            return Err(invalid());
        }
        if Category::parse(&category)? != Category::Security {
            let enabled: Option<bool> = outbox::query_scalar("SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2").bind(account).bind(category).fetch_optional(&self.pool).await?;
            if enabled != Some(true) {
                return Ok(());
            }
        }
        sender
            .send(&Email {
                id,
                recipient,
                subject,
                text,
                html,
                unsubscribe,
            })
            .await?;
        outbox::query("UPDATE email_outbox SET delivered_at=now(),updated_at=now() WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct Notification {
    pub id: Uuid,
    pub subject: String,
    pub body: String,
    pub read_at: Option<OffsetDateTime>,
}

/// Adapter state for worker registration, with an explicit account-state provider.
#[derive(Debug)]
pub struct Delivery {
    pub notifications: Notifications,
    pub sender: Arc<dyn EmailSender>,
}

fn translate(locale: Locale, key: &str) -> Result<String> {
    use fluent_bundle::{FluentBundle, FluentResource};
    let (language, source) = match locale {
        Locale::Pl => ("pl", include_str!("../locales/pl.ftl")),
        Locale::En => ("en", include_str!("../locales/en.ftl")),
    };
    let mut bundle = FluentBundle::new(vec![language.parse::<unic_langid::LanguageIdentifier>()?]);
    let resource = FluentResource::try_new(source.to_owned()).map_err(|_| invalid())?;
    bundle.add_resource(resource).map_err(|_| invalid())?;
    let message = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or_else(invalid)?;
    let mut errors = Vec::new();
    let value = bundle
        .format_pattern(message, None, &mut errors)
        .into_owned();
    if !errors.is_empty() {
        return Err(invalid());
    }
    Ok(value)
}

impl std::fmt::Debug for SmtpSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SmtpSender { redacted }")
    }
}

fn smtp_message(from: &str, email: &Email) -> Result<Message> {
    let mut builder = Message::builder()
        .from(from.parse()?)
        .to(email.recipient.parse()?)
        .subject(&email.subject)
        .message_id(Some(format!("<{}@notify.oteryn.invalid>", email.id)));
    if let Some(url) = &email.unsubscribe {
        builder = builder
            .raw_header(header::HeaderValue::new(
                header::HeaderName::new_from_ascii_str("List-Unsubscribe"),
                format!("<{url}>"),
            ))
            .raw_header(header::HeaderValue::new(
                header::HeaderName::new_from_ascii_str("List-Unsubscribe-Post"),
                "List-Unsubscribe=One-Click".into(),
            ));
    }
    Ok(builder.multipart(MultiPart::alternative_plain_html(
        email.text.clone(),
        email.html.clone(),
    ))?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smtp_headers_and_multipart() -> Result<()> {
        let mut email = Email {
            id: Uuid::now_v7(),
            recipient: "recipient@example.invalid".into(),
            subject: "News".into(),
            text: "Hello".into(),
            html: "<p>Hello</p>".into(),
            unsubscribe: Some("https://portal.example.invalid/unsubscribe?token=signed".into()),
        };
        let raw = String::from_utf8(smtp_message("sender@example.invalid", &email)?.formatted())?;
        assert!(raw.contains("List-Unsubscribe: <https://"));
        assert!(raw.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"));
        assert!(raw.contains("multipart/alternative"));
        assert!(raw.contains("text/plain"));
        assert!(raw.contains("text/html"));
        email.unsubscribe = None;
        let raw = String::from_utf8(smtp_message("sender@example.invalid", &email)?.formatted())?;
        assert!(!raw.contains("List-Unsubscribe"));
        email.recipient = "invalid address".into();
        assert!(smtp_message("sender@example.invalid", &email).is_err());
        Ok(())
    }
}
