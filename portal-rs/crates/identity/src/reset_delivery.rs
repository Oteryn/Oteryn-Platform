//! SMTP adapter for encrypted reset jobs; transport details never enter job errors.
use crate::{
    reset_http::text,
    worker::{Handler, HandlerFuture, Registry},
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use oteryn_identity::{
    reset::{DeliveryKey, audit},
    verify::token_hash,
};
use oteryn_web::Language;
use sqlx::PgPool;
use std::sync::Arc;
struct Mailer {
    pool: PgPool,
    key: DeliveryKey,
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    from: lettre::message::Mailbox,
    origin: String,
}
impl std::fmt::Debug for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResetMailer([redacted])")
    }
}
pub fn register(registry: &mut Registry, pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    if !std::env::var("OTERYN_ACCOUNT_PASSWORD_RESET").is_ok_and(|v| v == "true") {
        return Ok(());
    }
    let key = DeliveryKey::parse(&std::env::var("OTERYN_RESET_DELIVERY_KEY")?)
        .ok_or("invalid reset delivery key")?;
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")?;
    let uri: axum::http::Uri = origin.parse()?;
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none()
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err("reset origin must be an HTTPS origin".into());
    }
    let smtp = AsyncSmtpTransport::<Tokio1Executor>::relay(&std::env::var("OTERYN_SMTP_HOST")?)?
        .credentials(Credentials::new(
            std::env::var("OTERYN_SMTP_USERNAME")?,
            std::env::var("OTERYN_SMTP_PASSWORD")?,
        ))
        .build();
    let from = std::env::var("OTERYN_MAIL_FROM")?.parse()?;
    registry.register(
        "email.password-reset",
        Arc::new(Mailer {
            pool,
            key,
            smtp,
            from,
            origin: origin.trim_end_matches('/').into(),
        }),
    )?;
    Ok(())
}
#[derive(serde::Deserialize)]
struct Payload {
    account_id: uuid::Uuid,
    ciphertext: Vec<u8>,
}
impl Handler for Mailer {
    fn run<'a>(&'a self, job: &'a oteryn_db::outbox::Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            let payload: Payload =
                serde_json::from_str(&job.payload).map_err(|_| "invalid reset payload")?;
            let Some(message) = self.prepare(&payload).await? else {
                return Ok(());
            };
            let sent = self.smtp.send(message).await.is_ok();
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|_| "reset audit unavailable")?;
            audit(
                &mut tx,
                Some(payload.account_id),
                if sent {
                    "identity.password_reset_mail_sent"
                } else {
                    "identity.password_reset_mail_failed"
                },
            )
            .await
            .map_err(|_| "reset audit unavailable")?;
            tx.commit().await.map_err(|_| "reset audit unavailable")?;
            if sent {
                Ok(())
            } else {
                Err("reset delivery failed".into())
            }
        })
    }
}

impl Mailer {
    async fn prepare(&self, payload: &Payload) -> Result<Option<Message>, String> {
        let token = self
            .key
            .open(&payload.ciphertext)
            .ok_or("invalid reset ciphertext")?;
        let account:Option<(String,String)>=sqlx::query_as("SELECT a.email,a.locale FROM accounts a JOIN account_password_resets r ON r.account_id=a.id WHERE a.id=$1 AND a.status IN ('active','pending_verification') AND r.token_hash=$2 AND r.used_at IS NULL AND r.expires_at>clock_timestamp()")
        .bind(payload.account_id).bind(token_hash(&token)).fetch_optional(&self.pool).await.map_err(|_|"reset delivery unavailable")?;
        let Some((email, locale)) = account else {
            return Ok(None);
        };
        let language = match locale.as_str() {
            "pl" => Language::Pl,
            "en" => Language::En,
            _ => return Err("invalid reset locale".into()),
        };
        let body = format!(
            "{}\n\n{}/{}/reset-password/{}",
            text(language, "reset-mail-body"),
            self.origin,
            language.code(),
            token
        );
        let message = Message::builder()
            .from(self.from.clone())
            .to(email.parse().map_err(|_| "invalid reset recipient")?)
            .subject(text(language, "reset-mail-subject"))
            .body(body)
            .map_err(|_| "invalid reset message")?;
        Ok(Some(message))
    }
}

#[cfg(test)]
#[path = "reset_delivery_tests.rs"]
mod tests;
