use super::*;
use fluent_bundle::{FluentBundle, FluentResource};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};

const EN: &str = "title = Email security\nconfirm = Confirm your new email address. Without a recovery key the change takes seven days.\nrevert = This was not me: cancel or revert this email change. This link expires after 30 days.\nassisted = Staff proposed a change to the email address below. If you recognize this request, sign in to Email security and submit the address with your current password and security proof.\nemail = New email\nrepeat = Repeat email\npassword = Current password\nproof = MFA code or recovery key\nrecovery = Use recovery key\nbackup = Set backup email\nsubmit = Continue\nfailed = Request refused or invalid input.\nrequested = Check both email addresses for confirmation and cancellation links.\nconfirmed = Address confirmed. The change will take effect after the waiting period.\nchanged = Email updated. Sign in again. Trading and further email changes are blocked for 72 hours.\ncancelled = Email change cancelled.\nreverted = Email change reverted. Sign in again.\n";
const PL: &str = "title = Bezpieczeństwo adresu e-mail\nconfirm = Potwierdź nowy adres e-mail. Bez klucza odzyskiwania zmiana nastąpi po siedmiu dniach.\nrevert = To nie ja: anuluj lub cofnij zmianę adresu e-mail. Link wygasa po 30 dniach.\nassisted = Zespół zaproponował zmianę na poniższy adres. Jeśli rozpoznajesz tę prośbę, zaloguj się, przejdź do ustawień adresu e-mail i podaj ten adres, aktualne hasło oraz kod 2FA lub klucz odzyskiwania.\nemail = Nowy adres e-mail\nrepeat = Powtórz adres e-mail\npassword = Obecne hasło\nproof = Kod 2FA lub klucz odzyskiwania\nrecovery = Użyj klucza odzyskiwania\nbackup = Ustaw zapasowy e-mail\nsubmit = Kontynuuj\nfailed = Odmowa lub nieprawidłowe dane.\nrequested = Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.\nconfirmed = Adres potwierdzony. Zmiana nastąpi po okresie oczekiwania.\nchanged = E-mail zmieniony. Zaloguj się ponownie. Handel i kolejna zmiana adresu są zablokowane przez 72 godziny.\ncancelled = Zmiana adresu anulowana.\nreverted = Zmiana adresu cofnięta. Zaloguj się ponownie.\n";
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

fn link_origin(link: &str) -> Option<&str> {
    const HTTPS: &str = "https://";
    let (authority, route) = link.strip_prefix(HTTPS)?.split_once('/')?;
    if authority.contains('@') {
        return None;
    }
    authority.parse::<axum::http::uri::Authority>().ok()?;
    let token = route
        .strip_prefix("email-change/confirm/")
        .or_else(|| route.strip_prefix("email-change/revert/"))?;
    if !valid_token(token) {
        return None;
    }
    let origin = &link[..HTTPS.len() + authority.len()];
    url::Url::parse(origin).ok()?;
    Some(origin)
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
        let mfa = crate::mfa::Mfa::configured(pool.clone())
            .map_err(|_| "email security configuration unavailable".to_owned())?;
        Ok(Some(Self {
            service: Service {
                pool,
                enabled: true,
                security: Arc::new(TransactionalSecurity(mfa)),
                origin: env("OTERYN_PUBLIC_ORIGIN")?,
            },
            sender: Arc::new(sender),
        }))
    }
    pub async fn tick(&self) -> Result<(), String> {
        if !self.service.ready() {
            return Err("email changes disabled or misconfigured".into());
        }
        // The outbox has 30 seconds for execution and acknowledgement.
        let deadline = Instant::now() + Duration::from_secs(25);
        timeout_at(deadline, self.service.apply_due())
            .await
            .map_err(|_| "email change application deadline exceeded".to_owned())?
            .map_err(|_| "email change application failed".to_owned())?;
        let last_send = deadline - Duration::from_secs(3);
        for _ in 0..100 {
            if Instant::now() >= last_send
                || !timeout_at(deadline, self.deliver_one(last_send))
                    .await
                    .map_err(|_| "email delivery deadline exceeded".to_owned())??
            {
                break;
            }
        }
        Ok(())
    }

    /// At-least-once SMTP with a stable Message-ID. The row lock
    /// prevents concurrent sends while a bounded delivery is in progress.
    async fn deliver_one(&self, last_send: Instant) -> Result<bool, String> {
        let mut tx = self
            .service
            .pool
            .begin()
            .await
            .map_err(|_| "email delivery unavailable".to_owned())?;
        let row: Option<(Uuid, String, String, String, bool)> = sqlx::query_as(
            "SELECT m.id,m.recipient,m.subject,m.body,
                    c.initiated_by_staff AND c.owner_approved_at IS NULL
             FROM account_email_change_mail m JOIN account_email_changes c ON c.id=m.change_id
             WHERE m.delivered_at IS NULL AND m.next_attempt_at<=clock_timestamp()
               AND c.revert_until>now() AND c.cancelled_at IS NULL AND c.reverted_at IS NULL
               AND (NOT c.initiated_by_staff OR c.owner_approved_at IS NOT NULL
                    OR (c.expires_at>clock_timestamp() AND m.recipient=c.old_email))
             ORDER BY m.created_at,m.id LIMIT 1 FOR UPDATE OF m SKIP LOCKED",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| "email delivery unavailable".to_owned())?;
        let Some((id, recipient, subject, text, owner_proposal)) = row else {
            return Ok(false);
        };
        let link_origin: fn(&str) -> Option<&str> = if owner_proposal {
            owner_notice_origin
        } else {
            link_origin
        };
        if Instant::now() >= last_send {
            return Ok(false);
        }
        let Some((message, url, origin)) = text
            .rsplit_once("\n\n")
            .and_then(|(message, url)| Some((message, url, link_origin(url)?)))
        else {
            // An invalid stored payload cannot heal on retry. Keep the row
            // distinguishable from a successful send and remove its token.
            sqlx::query(
                "UPDATE account_email_change_mail
                 SET body='', next_attempt_at='infinity', updated_at=clock_timestamp()
                 WHERE id=$1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|_| "email delivery unavailable".to_owned())?;
            tx.commit()
                .await
                .map_err(|_| "email delivery unavailable".to_owned())?;
            return Ok(true);
        };
        let sent = if origin == self.service.origin {
            let escape = |value: &str| {
                value
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
            };
            let html = format!("<p><a href=\"{}\">{}</a></p>", escape(url), escape(message));
            let email = oteryn_notify::Email {
                id,
                recipient,
                subject,
                text,
                html,
                unsubscribe: None,
            };
            let send_deadline = last_send.min(Instant::now() + Duration::from_secs(5));
            matches!(
                timeout_at(send_deadline, self.sender.send(&email)).await,
                Ok(Ok(()))
            )
        } else {
            // An origin change can be temporary. Preserve the original link and
            // share SMTP's per-message backoff without logging either origin.
            eprintln!("email-change mail deferred: public origin mismatch");
            false
        };
        if !sent {
            sqlx::query("UPDATE account_email_change_mail SET delivery_attempts=delivery_attempts+1,next_attempt_at=clock_timestamp()+make_interval(secs=>LEAST(3600,30*power(2,LEAST(delivery_attempts,7)))::double precision),updated_at=clock_timestamp() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(|_| "email delivery unavailable".to_owned())?;
        } else {
            sqlx::query("UPDATE account_email_change_mail SET delivered_at=now(),body='',updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(|_|"email delivery unavailable".to_owned())?;
        }
        tx.commit()
            .await
            .map_err(|_| "email delivery unavailable".to_owned())?;
        Ok(true)
    }
}
fn owner_notice_origin(link: &str) -> Option<&str> {
    for suffix in ["/pl/account/security/email", "/en/account/security/email"] {
        if let Some(origin) = link.strip_suffix(suffix) {
            let authority = origin.strip_prefix("https://")?;
            if !authority.contains('@') && authority.parse::<axum::http::uri::Authority>().is_ok() {
                return Some(origin);
            }
        }
    }
    None
}

impl oteryn_db::outbox::Cleanup for Worker {
    fn clean(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(self.tick())
    }
}

#[cfg(test)]
mod tests {
    use super::{link_origin, owner_notice_origin};

    #[test]
    fn owner_notice_links_are_localized_settings_routes_without_tokens() {
        let origin = "https://portal.example.invalid";
        for lang in ["pl", "en"] {
            let url = format!("{origin}/{lang}/account/security/email");
            assert_eq!(owner_notice_origin(&url), Some(origin));
            assert_eq!(link_origin(&url), None);
        }
        for invalid in [
            "http://portal.example.invalid/en/account/security/email",
            "https://user@portal.example.invalid/en/account/security/email",
            "https://portal.example.invalid/en/account/security/email?return_to=roles",
            "https://portal.example.invalid/en/account/security/email#confirmation",
            "https://portal.example.invalid/de/account/security/email",
        ] {
            assert_eq!(owner_notice_origin(invalid), None);
        }
    }
}
