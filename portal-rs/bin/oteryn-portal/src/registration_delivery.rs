//! Delivery of registration verification jobs through the existing outbox worker.
use crate::worker::{Handler, HandlerFuture, Registry};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use oteryn_identity::verify::token_hash;
use sqlx::PgPool;
use std::sync::Arc;

struct Mailer {
    pool: PgPool,
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    from: lettre::message::Mailbox,
    origin: String,
}
impl std::fmt::Debug for Mailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistrationMailer([redacted])")
    }
}
pub fn register(registry: &mut Registry, pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(host) = std::env::var("OTERYN_SMTP_HOST") else {
        return Ok(());
    };
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")?;
    let uri: axum::http::Uri = origin.parse()?;
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none()
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err("registration mail origin must be an HTTPS origin".into());
    }
    let smtp = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
        .credentials(Credentials::new(
            std::env::var("OTERYN_SMTP_USERNAME")?,
            std::env::var("OTERYN_SMTP_PASSWORD")?,
        ))
        .build();
    registry.register(
        "email.verify",
        Arc::new(Mailer {
            pool,
            smtp,
            from: std::env::var("OTERYN_MAIL_FROM")?.parse()?,
            origin: origin.trim_end_matches('/').into(),
        }),
    )?;
    Ok(())
}
#[derive(serde::Deserialize)]
struct Payload {
    account_id: uuid::Uuid,
    locale: String,
    token: String,
}
impl Mailer {
    async fn prepare(&self, payload: &Payload) -> Result<Option<Message>, String> {
        if payload.token.len() != 64 || !payload.token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid verification payload".into());
        }
        // Recheck the exact account/token pair at delivery time. Replaced, used and
        // expired links must not be mailed; locale comes from the owning account.
        let account: Option<(String, String)> = sqlx::query_as(
            "SELECT a.email,a.locale FROM accounts a JOIN account_email_tokens t ON t.account_id=a.id WHERE a.id=$1 AND a.status='pending_verification' AND a.email_verified_at IS NULL AND t.purpose='verify_email' AND t.token_hash=$2 AND t.used_at IS NULL AND t.expires_at>clock_timestamp()",
        )
        .bind(payload.account_id)
        .bind(token_hash(&payload.token))
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| "verification delivery unavailable")?;
        let Some((email, locale)) = account else {
            return Ok(None);
        };
        if locale != payload.locale || !matches!(locale.as_str(), "en" | "pl") {
            return Err("invalid verification locale".into());
        }
        let (subject, introduction) = if locale == "pl" {
            ("Potwierdź adres e-mail", "Potwierdź swój adres e-mail:")
        } else {
            ("Verify your email address", "Verify your email address:")
        };
        Message::builder()
            .from(self.from.clone())
            .to(email
                .parse()
                .map_err(|_| "invalid verification recipient")?)
            .subject(subject)
            .body(format!(
                "{introduction}\n\n{}/{locale}/verify-email?token={}",
                self.origin, payload.token
            ))
            .map(Some)
            .map_err(|_| "verification message unavailable".into())
    }
}
impl Handler for Mailer {
    fn run<'a>(&'a self, job: &'a oteryn_db::outbox::Job) -> HandlerFuture<'a> {
        Box::pin(async move {
            let payload: Payload =
                serde_json::from_str(&job.payload).map_err(|_| "invalid verification payload")?;
            let Some(message) = self.prepare(&payload).await? else {
                return Ok(());
            };
            self.smtp
                .send(message)
                .await
                .map(|_| ())
                .map_err(|_| "verification delivery failed".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reset_http::tests::db;

    #[tokio::test]
    async fn verification_delivery_checks_token_owner_freshness_and_locale()
    -> Result<(), Box<dyn std::error::Error>> {
        let Some(db) = db::Database::open().await? else {
            return Ok(());
        };
        let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status) VALUES ('verify@example.test','unused','pl','pending_verification') RETURNING id")
            .fetch_one(&db.pool).await?;
        let mut tx = db.pool.begin().await?;
        oteryn_identity::verify::issue(&mut tx, id, "pl").await?;
        tx.commit().await?;
        let payload: String =
            sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind='email.verify'")
                .fetch_one(&db.pool)
                .await?;
        let mut payload: Payload = serde_json::from_str(&payload)?;
        let mailer = Mailer {
            pool: db.pool.clone(),
            smtp: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1").build(),
            from: "portal@example.test".parse()?,
            origin: "https://portal.example.test".into(),
        };
        let raw = String::from_utf8(
            mailer
                .prepare(&payload)
                .await?
                .ok_or("missing mail")?
                .formatted(),
        )?
        .replace("=\r\n", "")
        .replace("=3D", "=");
        assert!(raw.contains(&format!("/pl/verify-email?token={}", payload.token)));
        payload.locale = "en".into();
        assert!(mailer.prepare(&payload).await.is_err());
        payload.locale = "pl".into();
        payload.account_id = uuid::Uuid::now_v7();
        assert!(mailer.prepare(&payload).await?.is_none());
        payload.account_id = id;
        sqlx::query("UPDATE account_email_tokens SET expires_at=now()-interval '1 second'")
            .execute(&db.pool)
            .await?;
        assert!(mailer.prepare(&payload).await?.is_none());
        sqlx::query(
            "UPDATE account_email_tokens SET expires_at=now()+interval '1 hour',used_at=now()",
        )
        .execute(&db.pool)
        .await?;
        assert!(mailer.prepare(&payload).await?.is_none());
        payload.token = "invalid".into();
        assert!(mailer.prepare(&payload).await.is_err());
        db.close().await
    }
}
