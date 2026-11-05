use super::*;
use fluent_bundle::{FluentBundle, FluentResource};

const EN: &str = "title = Email security\nconfirm = Confirm your new email address. Without a recovery key the change takes seven days.\nrevert = This was not me: cancel or revert this email change. This link expires after 30 days.\nemail = New email\nrepeat = Repeat email\npassword = Current password\nproof = MFA code or recovery key\nrecovery = Use recovery key\nbackup = Set backup email\nsubmit = Continue\nfailed = Request refused or invalid input.\nrequested = Check both email addresses for confirmation and cancellation links.\nconfirmed = Address confirmed. The change will take effect after the waiting period.\nchanged = Email updated. Sign in again. Trading and further email changes are blocked for 72 hours.\ncancelled = Email change cancelled.\nreverted = Email change reverted. Sign in again.\n";
const PL: &str = "title = Bezpieczeństwo adresu e-mail\nconfirm = Potwierdź nowy adres e-mail. Bez klucza odzyskiwania zmiana nastąpi po siedmiu dniach.\nrevert = To nie ja: anuluj lub cofnij zmianę adresu e-mail. Link wygasa po 30 dniach.\nemail = Nowy adres e-mail\nrepeat = Powtórz adres e-mail\npassword = Obecne hasło\nproof = Kod 2FA lub klucz odzyskiwania\nrecovery = Użyj klucza odzyskiwania\nbackup = Ustaw zapasowy e-mail\nsubmit = Kontynuuj\nfailed = Odmowa lub nieprawidłowe dane.\nrequested = Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.\nconfirmed = Adres potwierdzony. Zmiana nastąpi po okresie oczekiwania.\nchanged = E-mail zmieniony. Zaloguj się ponownie. Handel i kolejna zmiana adresu są zablokowane przez 72 godziny.\ncancelled = Zmiana adresu anulowana.\nreverted = Zmiana adresu cofnięta. Zaloguj się ponownie.\n";
pub fn text(locale: &str, key: &str) -> Result<String, sqlx::Error> {
    let invalid = || sqlx::Error::Protocol("email translation unavailable".into());
    let source = match locale {
        "en" => EN,
        "pl" => PL,
        _ => return Err(invalid()),
    };
    let resource = FluentResource::try_new(source.to_owned()).map_err(|_| invalid())?;
    let lang = locale
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|_| invalid())?;
    let mut bundle = FluentBundle::new(vec![lang]);
    bundle.add_resource(resource).map_err(|_| invalid())?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or_else(invalid)?;
    let mut errors = Vec::new();
    let value = bundle
        .format_pattern(pattern, None, &mut errors)
        .into_owned();
    if !errors.is_empty() {
        return Err(invalid());
    }
    Ok(value)
}
pub(super) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    change: Uuid,
    recipient: &str,
    locale: &str,
    key: &str,
    url: &str,
) -> Result<(), sqlx::Error> {
    let body = format!("{}\n\n{url}", text(locale, key)?);
    sqlx::query("INSERT INTO account_email_change_mail(change_id,recipient,subject,body) VALUES($1,$2,$3,$4)").bind(change).bind(recipient).bind(text(locale,"title")?).bind(body).execute(&mut **tx).await?;
    Ok(())
}
/// T36's sender is reused; its current account-only recipient policy cannot
/// deliver confirmation or old-address recovery mail. This owns that narrow seam.
pub struct Worker {
    pub service: Service,
    pub sender: Arc<dyn oteryn_notify::EmailSender>,
}
impl std::fmt::Debug for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EmailChangeWorker { redacted }")
    }
}
impl Worker {
    pub fn configured(pool: PgPool) -> Result<Option<Self>, String> {
        if std::env::var("OTERYN_ACCOUNT_EMAIL_CHANGE").as_deref() != Ok("true") {
            return Ok(None);
        }
        let env = |key| {
            std::env::var(key).map_err(|_| "email delivery configuration unavailable".to_owned())
        };
        let sender = oteryn_notify::SmtpSender::new(
            &env("OTERYN_SMTP_HOST")?,
            env("OTERYN_SMTP_USERNAME")?,
            env("OTERYN_SMTP_PASSWORD")?,
            env("OTERYN_SMTP_FROM")?,
        )
        .map_err(|_| "email delivery configuration invalid".to_owned())?;
        Ok(Some(Self {
            service: Service {
                pool,
                enabled: true,
                security: Arc::new(DenySecurity),
                origin: env("OTERYN_PUBLIC_ORIGIN")?,
            },
            sender: Arc::new(sender),
        }))
    }
    pub async fn tick(&self) -> Result<(), String> {
        if !self.service.ready() {
            return Err("email changes disabled or misconfigured".into());
        }
        self.service
            .apply_due()
            .await
            .map_err(|_| "email change application failed".to_owned())?;
        // At-least-once SMTP, with a stable Message-ID. Row locks prevent concurrent sends.
        for _ in 0..100 {
            let mut tx = self
                .service
                .pool
                .begin()
                .await
                .map_err(|_| "email delivery unavailable".to_owned())?;
            let row:Option<(Uuid,String,String,String)>=sqlx::query_as("SELECT m.id,m.recipient,m.subject,m.body FROM account_email_change_mail m JOIN account_email_changes c ON c.id=m.change_id WHERE m.delivered_at IS NULL AND c.revert_until>now() AND c.cancelled_at IS NULL AND c.reverted_at IS NULL ORDER BY m.created_at LIMIT 1 FOR UPDATE OF m SKIP LOCKED").fetch_optional(&mut *tx).await.map_err(|_|"email delivery unavailable".to_owned())?;
            let Some((id, recipient, subject, text)) = row else {
                break;
            };
            let (message, url) = text
                .rsplit_once("\n\n")
                .ok_or_else(|| "invalid email payload".to_owned())?;
            if !url.starts_with(&format!("{}/email-change/", self.service.origin)) {
                return Err("invalid email link".into());
            }
            let escape = |value: &str| {
                value
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
            };
            let html = format!("<p><a href=\"{}\">{}</a></p>", escape(url), escape(message));
            self.sender
                .send(&oteryn_notify::Email {
                    id,
                    recipient,
                    subject,
                    text,
                    html,
                    unsubscribe: None,
                })
                .await
                .map_err(|_| "email delivery failed".to_owned())?;
            sqlx::query("UPDATE account_email_change_mail SET delivered_at=now(),body='',updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(|_|"email delivery unavailable".to_owned())?;
            tx.commit()
                .await
                .map_err(|_| "email delivery unavailable".to_owned())?;
        }
        Ok(())
    }
}
impl oteryn_db::outbox::Cleanup for Worker {
    fn clean(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(self.tick())
    }
}
