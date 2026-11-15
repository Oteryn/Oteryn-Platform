//! Delivery of registration verification jobs through the existing outbox worker.
use crate::worker::{Handler, HandlerFuture, Registry};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
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
    let host =
        std::env::var("OTERYN_SMTP_HOST").map_err(|_| "registration SMTP host is required")?;
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")?;
    let uri: axum::http::Uri = origin.parse()?;
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none()
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err("registration mail origin must be an HTTPS origin".into());
    }
    let smtp = oteryn_notify::smtp::transport(
        &host,
        std::env::var("OTERYN_SMTP_USERNAME")?,
        std::env::var("OTERYN_SMTP_PASSWORD")?,
    )
    .map_err(|_| "SMTP configuration invalid")?;
    let mailer = Arc::new(Mailer {
        pool,
        smtp,
        from: std::env::var("OTERYN_MAIL_FROM")?.parse()?,
        origin: origin.trim_end_matches('/').into(),
    });
    registry.register("email.verify", mailer.clone())?;
    registry.register("email.account_exists", mailer)?;
    Ok(())
}
#[derive(serde::Deserialize)]
struct Payload {
    account_id: uuid::Uuid,
    locale: String,
    token: String,
}
#[derive(serde::Deserialize)]
struct ExistingPayload {
    account_id: uuid::Uuid,
    locale: String,
    email: Option<String>,
}
impl Mailer {
    async fn prepare_existing(&self, payload: &ExistingPayload) -> Result<Option<Message>, String> {
        let account: Option<(String, String)> = sqlx::query_as(
            "SELECT email,locale FROM accounts WHERE id=$1 AND ($2::text IS NULL OR lower(email)=$2) AND status IN ('active','pending_verification','suspended')"
        ).bind(payload.account_id).bind(&payload.email).fetch_optional(&self.pool).await
            .map_err(|_| "account-exists delivery unavailable")?;
        let Some((email, locale)) = account else {
            return Ok(None);
        };
        if !matches!(payload.locale.as_str(), "en" | "pl")
            || !matches!(locale.as_str(), "en" | "pl")
        {
            return Err("invalid account-exists locale".into());
        }
        let (subject, body) = if locale == "pl" {
            (
                "Konto już istnieje",
                "Ten adres e-mail jest już powiązany z kontem. Jeśli to Ty próbujesz się zarejestrować, zaloguj się lub odzyskaj hasło.",
            )
        } else {
            (
                "Account already exists",
                "This email address already belongs to an account. If you tried to register, sign in or reset your password.",
            )
        };
        Message::builder()
            .from(self.from.clone())
            .to(email
                .parse()
                .map_err(|_| "invalid account-exists recipient")?)
            .subject(subject)
            .body(format!("{body}\n\n{}/{locale}/login", self.origin))
            .map(Some)
            .map_err(|_| "account-exists message unavailable".into())
    }
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
            let message = if job.kind == "email.account_exists" {
                let payload = serde_json::from_str(&job.payload)
                    .map_err(|_| "invalid account-exists payload")?;
                self.prepare_existing(&payload).await?
            } else {
                let payload = serde_json::from_str(&job.payload)
                    .map_err(|_| "invalid verification payload")?;
                self.prepare(&payload).await?
            };
            let Some(message) = message else {
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

#[cfg(test)]
mod account_exists_tests {
    use super::*;
    #[tokio::test]
    async fn account_exists_delivery_revalidates_current_address_and_account_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let started = std::time::Instant::now();
        let db = crate::reset_http::tests::db::Database::open()
            .await?
            .ok_or("PORTAL_TEST_DATABASE_URL required")?;
        let id: uuid::Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,locale,status,email_verified_at) VALUES('existing@example.test','unused','en','active',now()) RETURNING id")
            .fetch_one(&db.pool).await?;
        let mailer = Mailer {
            pool: db.pool.clone(),
            smtp: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1").build(),
            from: "portal@example.test".parse()?,
            origin: "https://portal.example.test".into(),
        };
        let payload = ExistingPayload {
            account_id: id,
            locale: "en".into(),
            email: Some("existing@example.test".into()),
        };
        assert!(mailer.prepare_existing(&payload).await?.is_some());
        // Pending jobs created by the reviewed base carry only account ID/locale.
        let legacy: ExistingPayload =
            serde_json::from_value(serde_json::json!({"account_id": id, "locale": "en"}))?;
        assert!(mailer.prepare_existing(&legacy).await?.is_some());
        sqlx::query("UPDATE accounts SET email='changed@example.test' WHERE id=$1")
            .bind(id)
            .execute(&db.pool)
            .await?;
        assert!(mailer.prepare_existing(&payload).await?.is_none());
        sqlx::query(
            "UPDATE accounts SET email='existing@example.test',status='closed' WHERE id=$1",
        )
        .bind(id)
        .execute(&db.pool)
        .await?;
        assert!(mailer.prepare_existing(&payload).await?.is_none());
        assert!(mailer.prepare_existing(&legacy).await?.is_none());
        db.close().await?;
        eprintln!(
            "PG account_exists_delivery_revalidates_current_address_and_account_state {:.3}s",
            started.elapsed().as_secs_f64()
        );
        Ok(())
    }
}

/// Freeze delivery readiness at startup, before registration body/session work.
/// Serve and worker use the identical validated configuration.
pub fn admission(app: axum::Router, pool: PgPool) -> axum::Router {
    let ready = register(&mut Registry::default(), pool).is_ok();
    admission_with_readiness(app, ready)
}
fn admission_with_readiness(app: axum::Router, ready: bool) -> axum::Router {
    app.layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| async move {
            use axum::{
                http::{Method, StatusCode},
                response::IntoResponse,
            };
            if !ready
                && request.method() == Method::POST
                && matches!(
                    request.uri().path(),
                    "/register" | "/pl/register" | "/en/register"
                )
            {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    [("cache-control", "no-store")],
                )
                    .into_response();
            }
            next.run(request).await
        },
    ))
}
