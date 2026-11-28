use askama::Template;
use hmac::{Hmac, Mac};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{MultiPart, header},
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
    Forum,
    Operational,
    JournalReminder,
}
impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::News => "news",
            Self::Support => "support",
            Self::Forum => "forum",
            Self::Operational => "operational",
            Self::JournalReminder => "journal_reminder",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "security" => Ok(Self::Security),
            "news" => Ok(Self::News),
            "support" => Ok(Self::Support),
            "forum" => Ok(Self::Forum),
            "operational" => Ok(Self::Operational),
            "journal_reminder" => Ok(Self::JournalReminder),
            _ => Err(invalid()),
        }
    }

    fn email_only(self) -> bool {
        matches!(self, Self::Operational | Self::JournalReminder)
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
    fn recipient(&self, account: Uuid, _category: Category) -> NotifyFuture<'_, Option<Account>> {
        self.active_verified(account)
    }
    fn active_verified(&self, account: Uuid) -> NotifyFuture<'_, Option<Account>>;
    /// Check the feature flags and current authority for this account and intent
    /// on every delivery attempt. The default keeps reminder delivery disabled.
    /// Return false for missing or stale evidence, and an error for a retriable failure.
    fn journal_reminder_eligible(&self, _account: Uuid, _intent: Uuid) -> NotifyFuture<'_, bool> {
        Box::pin(async { Ok(false) })
    }
    /// Pool-backed adapters must override this to use the caller's transaction.
    /// The default is for adapters whose state is already available in memory.
    fn active_verified_in<'a>(
        &'a self,
        _tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> NotifyFuture<'a, Option<Account>> {
        self.active_verified(account)
    }
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
    Newsletter { title: String, body: String },
    Support { body: String },
    JournalReminder { intent: Uuid },
}
#[derive(Debug, Clone)]
pub struct Rendered {
    pub subject: String,
    pub text: String,
    pub html: String,
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><body><h1>{{ subject }}</h1>{% if let Some((label, url)) = footer %}<p>{% for line in body.lines() %}{% if !loop.first %}<br>{% endif %}{{ line }}{% endfor %}</p><p><a href=\"{{ url }}\">{{ label }}</a></p>{% else %}<p>{{ body }}</p>{% endif %}</body></html>",
    ext = "html"
)]
struct Html<'a> {
    lang: &'a str,
    subject: &'a str,
    body: &'a str,
    footer: Option<(&'a str, &'a str)>,
}
#[derive(Template)]
#[template(source = "{{ subject }}\n\n{{ body }}\n", ext = "txt")]
struct Text<'a> {
    subject: &'a str,
    body: &'a str,
}
impl EmailTemplate {
    fn kind(&self) -> &'static str {
        match self {
            Self::NewDevice { .. } => "new-device",
            Self::PasswordChanged => "password-changed",
            Self::News { .. } | Self::Newsletter { .. } => "news",
            Self::Support { .. } => "support",
            Self::JournalReminder { .. } => "journal-reminder",
        }
    }
    pub fn category(&self) -> Category {
        match self {
            Self::NewDevice { .. } | Self::PasswordChanged => Category::Security,
            Self::News { .. } | Self::Newsletter { .. } => Category::News,
            Self::Support { .. } => Category::Support,
            Self::JournalReminder { .. } => Category::JournalReminder,
        }
    }
    pub fn render(&self, locale: Locale) -> Result<Rendered> {
        self.render_for_delivery(locale, None)
    }

    fn render_for_delivery(&self, locale: Locale, unsubscribe: Option<&str>) -> Result<Rendered> {
        let pl = matches!(locale, Locale::Pl);
        let tr = |key| translate(locale, key);
        let (subject, body) = match self {
            Self::NewDevice { device } => (
                tr("new-device")?,
                format!("{}: {device}", tr("device-login")?),
            ),
            Self::PasswordChanged => (tr("password-changed")?, tr("password-body")?),
            Self::News { title, body } | Self::Newsletter { title, body } => {
                (title.clone(), body.clone())
            }
            Self::Support { body } => (tr("support")?, body.clone()),
            Self::JournalReminder { .. } => match locale {
                Locale::Pl => (
                    "Przypomnienie z dziennika Oteryn".into(),
                    "Zaloguj się do portalu Oteryn, aby sprawdzić przypomnienie.".into(),
                ),
                Locale::En => (
                    "Oteryn journal reminder".into(),
                    "Sign in to the Oteryn portal to view your reminder.".into(),
                ),
            },
        };
        if subject.trim().is_empty()
            || subject.len() > 200
            || subject.contains(['\r', '\n'])
            || body.trim().is_empty()
            || body.len() > 16384
        {
            return Err(invalid());
        }
        let label = if matches!(self, Self::Newsletter { .. }) {
            unsubscribe
                .map(|_| tr("newsletter-unsubscribe"))
                .transpose()?
        } else {
            None
        };
        let footer = label.as_deref().zip(unsubscribe);
        let html = Html {
            lang: if pl { "pl" } else { "en" },
            subject: &subject,
            body: &body,
            footer,
        }
        .render()?;
        let text_body = match footer {
            Some((label, url)) => format!("{body}\n\n{label}: {url}"),
            None => body,
        };
        let text = Text {
            subject: &subject,
            body: &text_body,
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
    pub fn configured() -> Result<Self> {
        Self::new(
            std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_KEY")?.into_bytes(),
            std::env::var("OTERYN_NOTIFY_UNSUBSCRIBE_URL")?,
        )
    }

    pub fn operational_ready(&self) -> bool {
        self.operational_base(Locale::En).is_ok()
    }

    pub(crate) fn operational_base(&self, locale: Locale) -> Result<url::Url> {
        let mut base = url::Url::parse(&self.base)?;
        if base.scheme() != "https"
            || base.host().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || !matches!(
                base.path(),
                "/unsubscribe" | "/en/unsubscribe" | "/pl/unsubscribe"
            )
        {
            return Err(invalid());
        }
        base.set_path(match locale {
            Locale::Pl => "/pl/unsubscribe",
            Locale::En => "/en/unsubscribe",
        });
        Ok(base)
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

    pub fn link(&self, account: Uuid, category: Category, now: i64) -> Result<String> {
        let token = self.token(account, category, now + 30 * 86_400)?;
        Ok(format!("{}?token={token}", self.base))
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
    pub fn configured() -> Result<Self> {
        Self::new(
            &std::env::var("OTERYN_SMTP_HOST")?,
            std::env::var("OTERYN_SMTP_USERNAME")?,
            std::env::var("OTERYN_SMTP_PASSWORD")?,
            std::env::var("OTERYN_SMTP_FROM")?,
        )
    }

    /// Uses the shared TLS/port policy with certificate validation.
    pub fn new(host: &str, username: String, password: String, from: String) -> Result<Self> {
        let _: lettre::message::Mailbox = from.parse()?;
        Ok(Self {
            transport: crate::smtp::transport(host, username, password)?,
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
    async fn account_in(&self, tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Account> {
        self.accounts
            .active_verified_in(tx, id)
            .await?
            .ok_or_else(invalid)
    }
    /// Missing optional preferences mean email off. Portal is on for categories that support it.
    pub async fn preference(&self, account: Uuid, category: Category) -> Result<(bool, bool)> {
        self.account(account).await?;
        if category == Category::Security {
            return Ok((true, true));
        }
        let value = outbox::query_as("SELECT email_enabled,portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2")
            .bind(account).bind(category.as_str()).fetch_optional(&self.pool).await?;
        Ok(value.unwrap_or((false, !category.email_only())))
    }
    pub async fn preferences(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        category: Category,
        email: bool,
        portal: bool,
    ) -> Result<()> {
        self.account_in(tx, account).await?;
        if category == Category::Security && (!email || !portal) {
            return Err(invalid());
        }
        if category.email_only() && portal {
            return Err(invalid());
        }
        outbox::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,$2,$3,$4) ON CONFLICT(account_id,category) DO UPDATE SET email_enabled=$3,portal_enabled=$4,updated_at=clock_timestamp() WHERE (account_notification_prefs.email_enabled,account_notification_prefs.portal_enabled) IS DISTINCT FROM ($3,$4)")
            .bind(account).bind(category.as_str()).bind(email).bind(portal).execute(&mut **tx).await?;
        Ok(())
    }
    /// Caller owns commit/rollback. Retry with the returned ID, never a second enqueue.
    /// Journal reminders carry an opaque intent ID. They deduplicate by account and
    /// intent, returning None on replay, and keep journal details in the signed-in portal.
    pub async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        template: &EmailTemplate,
    ) -> Result<Option<Uuid>> {
        let reminder = match template {
            EmailTemplate::JournalReminder { intent } if intent.is_nil() => return Err(invalid()),
            EmailTemplate::JournalReminder { intent } => Some(*intent),
            _ => None,
        };
        let state = self.account_in(tx, account).await?;
        let _: lettre::message::Mailbox = state.email.parse()?;
        let category = template.category();
        let pref: Option<(bool,bool)> = outbox::query_as("SELECT email_enabled,portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2").bind(account).bind(category.as_str()).fetch_optional(&mut **tx).await?;
        let (email, portal) = if category == Category::Security {
            (true, true)
        } else {
            pref.unwrap_or((false, !category.email_only()))
        };
        let email_only =
            category.email_only() || matches!(template, EmailTemplate::Newsletter { .. });
        if email_only && !email {
            return Ok(None);
        }
        let now = OffsetDateTime::now_utc();
        let unsubscribe = if email && category != Category::Security {
            Some(
                self.unsubscribe
                    .link(account, category, now.unix_timestamp())?,
            )
        } else {
            None
        };
        let rendered = template.render_for_delivery(state.locale, unsubscribe.as_deref())?;
        if portal && !email_only {
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
        let id: Option<Uuid> = outbox::query_scalar(
            "INSERT INTO email_outbox
                 (account_id,category,recipient,subject,text_body,html_body,unsubscribe_url,template_kind,journal_reminder_intent)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (account_id,journal_reminder_intent) WHERE journal_reminder_intent IS NOT NULL DO NOTHING
             RETURNING id",
        )
        .bind(account)
        .bind(category.as_str())
        .bind(state.email)
        .bind(rendered.subject)
        .bind(rendered.text)
        .bind(rendered.html)
        .bind(unsubscribe)
        .bind(template.kind())
        .bind(reminder)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(id) = id else {
            return Ok(None);
        };
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
        if category.email_only() {
            // The signed link can withdraw existing consent even when sign-in is
            // disabled. Creating a new preference requires an active account.
            outbox::query(
                "UPDATE account_notification_prefs SET email_enabled=false,updated_at=clock_timestamp()
                 WHERE account_id=$1 AND category=$2 AND email_enabled",
            )
            .bind(account)
            .bind(category.as_str())
            .execute(&mut **tx)
            .await?;
        } else {
            self.account_in(tx, account).await?;
            outbox::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled) VALUES($1,$2,false) ON CONFLICT(account_id,category) DO UPDATE SET email_enabled=false,updated_at=now()")
                .bind(account).bind(category.as_str()).execute(&mut **tx).await?;
        }
        Ok(())
    }
    pub async fn list(
        &self,
        account: Uuid,
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        Self::list_for(
            &self.pool,
            self.accounts.as_ref(),
            account,
            unread_only,
            limit,
        )
        .await
    }

    /// Read portal notifications without configuring email delivery or unsubscribe links.
    pub async fn list_for(
        pool: &PgPool,
        accounts: &dyn AccountState,
        account: Uuid,
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        accounts
            .active_verified(account)
            .await?
            .ok_or_else(invalid)?;
        if !(1..=100).contains(&limit) {
            return Err(invalid());
        }
        let rows: Vec<(Uuid,String,String,Option<OffsetDateTime>)> = outbox::query_as("SELECT id,subject,body,read_at FROM notifications WHERE account_id=$1 AND (NOT $2 OR read_at IS NULL) ORDER BY created_at DESC,id DESC LIMIT $3").bind(account).bind(unread_only).bind(limit).fetch_all(pool).await?;
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
    async fn suppress_reminder(&self, id: Uuid) -> Result<()> {
        outbox::query(
            "UPDATE email_outbox SET journal_reminder_suppressed_at=now(),updated_at=now()
             WHERE id=$1 AND journal_reminder_suppressed_at IS NULL AND delivered_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Called by a lease-renewing worker. SMTP is at least once after a process crash.
    pub async fn deliver(&self, job: &outbox::Job, sender: &dyn EmailSender) -> Result<()> {
        if job.kind != EMAIL_JOB {
            return Err(invalid());
        }
        let id = Uuid::parse_str(&serde_json::from_str::<String>(&job.payload)?)?;
        type EmailRow = (
            Uuid,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            bool,
            Option<Uuid>,
        );
        let (account, category, recipient, subject, text, html, unsubscribe, finished, reminder): EmailRow =
            outbox::query_as(
                "SELECT account_id,category,recipient,subject,text_body,html_body,unsubscribe_url,
                        delivered_at IS NOT NULL OR journal_reminder_suppressed_at IS NOT NULL,
                        journal_reminder_intent
                 FROM email_outbox WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if finished {
            return Ok(());
        }
        let category = Category::parse(&category)?;
        if category == Category::Forum && !crate::forum::deliverable(&self.pool, id).await? {
            return Ok(());
        }
        let email = Email {
            id,
            recipient,
            subject,
            text,
            html,
            unsubscribe,
        };
        if category == Category::Operational {
            return crate::operational::deliver(&self.pool, job.id, account, &email, sender).await;
        }
        if let Some(intent) = reminder
            && !self
                .accounts
                .journal_reminder_eligible(account, intent)
                .await?
        {
            return self.suppress_reminder(id).await;
        }
        let state = self
            .accounts
            .recipient(account, category)
            .await?
            .ok_or_else(invalid)?;
        if state.email != email.recipient {
            return Err(invalid());
        }
        if category != Category::Security {
            let enabled: Option<bool> = outbox::query_scalar("SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category=$2").bind(account).bind(category.as_str()).fetch_optional(&self.pool).await?;
            if enabled != Some(true) {
                if reminder.is_some() {
                    return self.suppress_reminder(id).await;
                }
                return Ok(());
            }
        }
        sender.send(&email).await?;
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

pub(crate) fn translate(locale: Locale, key: &str) -> Result<String> {
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
    fn newsletter_formats_both_parts() -> Result<()> {
        let template = EmailTemplate::Newsletter {
            title: "Newsletter".into(),
            body: "Line one\nLine <two>".into(),
        };
        let link = "https://portal.example.invalid/en/unsubscribe?token=fixture";
        for (locale, language, label) in [
            (Locale::Pl, "pl", "Wypisz się z newslettera"),
            (Locale::En, "en", "Unsubscribe from the newsletter"),
        ] {
            let rendered = template.render_for_delivery(locale, Some(link))?;
            assert!(rendered.text.contains(&format!("{label}: {link}")));
            assert!(rendered.html.contains(&format!("lang=\"{language}\"")));
            assert!(
                rendered
                    .html
                    .contains(&format!("<a href=\"{link}\">{label}</a>"))
            );
            assert!(rendered.html.contains("Line one<br>Line "));
            assert!(!rendered.html.contains("<two>"));
        }
        Ok(())
    }

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

#[cfg(test)]
mod reminder_tests {
    use super::*;

    #[derive(Debug)]
    struct ReminderAccounts(Option<(Uuid, Uuid)>);

    impl AccountState for ReminderAccounts {
        fn active_verified(&self, _: Uuid) -> NotifyFuture<'_, Option<Account>> {
            Box::pin(async {
                Ok(Some(Account {
                    email: "reader@example.invalid".into(),
                    locale: Locale::En,
                }))
            })
        }

        fn journal_reminder_eligible(&self, account: Uuid, intent: Uuid) -> NotifyFuture<'_, bool> {
            Box::pin(async move { Ok(self.0 == Some((account, intent))) })
        }
    }

    async fn enqueue(service: &Notifications, account: Uuid, intent: Uuid) -> Result<Option<Uuid>> {
        let mut tx = service.pool.begin().await?;
        let id = service
            .enqueue(&mut tx, account, &EmailTemplate::JournalReminder { intent })
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    async fn exercise(pool: PgPool) -> Result<()> {
        let (account, eligible, stale, opted_out) = (
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
        );
        let service = Notifications {
            pool,
            accounts: Arc::new(ReminderAccounts(Some((account, eligible)))),
            unsubscribe: Unsubscribe::new(
                vec![42; 32],
                "https://portal.example.invalid/unsubscribe".into(),
            )?,
        };

        let mut tx = service.pool.begin().await?;
        for category in [Category::News, Category::Support] {
            service
                .preferences(&mut tx, account, category, true, false)
                .await?;
        }
        tx.commit().await?;
        assert_eq!(
            service
                .preference(account, Category::JournalReminder)
                .await?,
            (false, false)
        );
        assert!(enqueue(&service, account, eligible).await?.is_none());
        assert_eq!(outbox::counts(&service.pool).await?.pending, 0);

        let mut tx = service.pool.begin().await?;
        assert!(
            service
                .preferences(&mut tx, account, Category::JournalReminder, true, true)
                .await
                .is_err()
        );
        service
            .preferences(&mut tx, account, Category::JournalReminder, true, false)
            .await?;
        for category in [Category::News, Category::Support] {
            service
                .preferences(&mut tx, account, category, false, false)
                .await?;
        }
        assert!(
            service
                .enqueue(
                    &mut tx,
                    account,
                    &EmailTemplate::JournalReminder {
                        intent: Uuid::nil()
                    }
                )
                .await
                .is_err()
        );
        tx.commit().await?;

        let mut tx = service.pool.begin().await?;
        assert!(
            service
                .enqueue(
                    &mut tx,
                    account,
                    &EmailTemplate::JournalReminder { intent: eligible }
                )
                .await?
                .is_some()
        );
        tx.rollback().await?;
        assert_eq!(outbox::counts(&service.pool).await?.pending, 0);

        let (first, second) = tokio::join!(
            enqueue(&service, account, eligible),
            enqueue(&service, account, eligible)
        );
        let mail = match (first?, second?) {
            (Some(id), None) | (None, Some(id)) => id,
            _ => return Err("exactly one concurrent enqueue must insert".into()),
        };
        assert_eq!(outbox::counts(&service.pool).await?.pending, 1);
        assert!(service.list(account, false, 10).await?.is_empty());

        let capture = CaptureSender::default();
        let mut jobs = outbox::claim(&service.pool, 1, 60).await?;
        let job = jobs.pop().ok_or("missing reminder job")?;
        service.deliver(&job, &capture).await?;
        service.deliver(&job, &capture).await?;
        outbox::finish(&service.pool, &job, None, false).await?;
        {
            let emails = capture.emails.lock().map_err(|_| invalid())?;
            assert_eq!(emails.len(), 1);
            assert_eq!(emails[0].id, mail);
            assert_eq!(emails[0].recipient, "reader@example.invalid");
            assert!(emails[0].text.contains("Sign in"));
            assert!(!emails[0].text.contains(&eligible.to_string()));
            assert!(!emails[0].html.contains(&eligible.to_string()));
        }
        assert!(enqueue(&service, account, eligible).await?.is_none());

        assert!(enqueue(&service, account, stale).await?.is_some());
        let mut jobs = outbox::claim(&service.pool, 1, 60).await?;
        let stale_job = jobs.pop().ok_or("missing stale reminder job")?;
        service.deliver(&stale_job, &capture).await?;
        let recovered_authority = Notifications {
            pool: service.pool.clone(),
            accounts: Arc::new(ReminderAccounts(Some((account, stale)))),
            unsubscribe: service.unsubscribe.clone(),
        };
        recovered_authority.deliver(&stale_job, &capture).await?;
        outbox::finish(&service.pool, &stale_job, None, false).await?;

        assert!(enqueue(&service, account, opted_out).await?.is_some());
        let token = service
            .unsubscribe
            .token(account, Category::JournalReminder, i64::MAX)?;
        let mut tx = service.pool.begin().await?;
        service.unsubscribe(&mut tx, &token).await?;
        tx.commit().await?;
        assert_eq!(
            service
                .preference(account, Category::JournalReminder)
                .await?,
            (false, false)
        );
        let delivery = Notifications {
            accounts: Arc::new(ReminderAccounts(Some((account, opted_out)))),
            ..service
        };
        let mut jobs = outbox::claim(&delivery.pool, 1, 60).await?;
        let opted_out_job = jobs.pop().ok_or("missing opted-out reminder job")?;
        delivery.deliver(&opted_out_job, &capture).await?;
        let mut tx = delivery.pool.begin().await?;
        delivery
            .preferences(&mut tx, account, Category::JournalReminder, true, false)
            .await?;
        tx.commit().await?;
        delivery.deliver(&opted_out_job, &capture).await?;
        outbox::finish(&delivery.pool, &opted_out_job, None, false).await?;
        assert_eq!(capture.emails.lock().map_err(|_| invalid())?.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn reminder_consent_deduplication_and_delivery() -> Result<()> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(());
        };
        let admin = outbox::PgPoolOptions::new().connect(&url).await?;
        let schema = format!("notify_{}", Uuid::now_v7().simple());
        outbox::query(outbox::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = outbox::PgPoolOptions::new()
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    outbox::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        let result = async {
            oteryn_db::migrate(&pool).await?;
            exercise(pool.clone()).await
        }
        .await;
        pool.close().await;
        outbox::query(outbox::AssertSqlSafe(format!(
            "DROP SCHEMA {schema} CASCADE"
        )))
        .execute(&admin)
        .await?;
        result
    }
}
