//! Delivery of registration verification jobs through the existing outbox worker.
//! Delivery is at least once: a crash after SMTP acceptance and before outbox acknowledgement can resend the message.
use crate::worker::{DeliveryFuture, DeliveryOutcome, Handler, HandlerFuture, Registry};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::SinglePart};
use oteryn_admin::staff_invites;
use oteryn_identity::{reset::DeliveryKey, verify::token_hash};
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
/// Mail settings whose presence means delivery is expected. With none of them
/// registration delivery is off: serve refuses registration and the worker
/// keeps queued verification jobs as a disabled consumer. Any partial set is a
/// configuration error that refuses startup.
const MAIL_SETTINGS: [&str; 7] = [
    "OTERYN_SMTP_HOST",
    "OTERYN_SMTP_PORT",
    "OTERYN_SMTP_TLS_MODE",
    "OTERYN_SMTP_USERNAME",
    "OTERYN_SMTP_PASSWORD",
    "OTERYN_SMTP_FROM",
    "OTERYN_MAIL_FROM",
];
struct MailTransport {
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    from: lettre::message::Mailbox,
    origin: String,
}

fn mail_settings_present() -> bool {
    MAIL_SETTINGS
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

fn required_mail_env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is required and must be valid UTF-8"))
}

fn mail_transport() -> Result<Option<MailTransport>, Box<dyn std::error::Error>> {
    if !mail_settings_present() {
        return Ok(None);
    }
    let host = required_mail_env("OTERYN_SMTP_HOST")?;
    let origin = required_mail_env("OTERYN_PUBLIC_ORIGIN")?;
    let uri: axum::http::Uri = origin
        .parse()
        .map_err(|_| "invalid registration mail origin")?;
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none()
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err("registration mail origin must be an HTTPS origin".into());
    }
    let smtp = oteryn_notify::smtp::transport(
        &host,
        required_mail_env("OTERYN_SMTP_USERNAME")?,
        required_mail_env("OTERYN_SMTP_PASSWORD")?,
    )
    .map_err(|_| "SMTP configuration invalid")?;
    let from = required_mail_env("OTERYN_MAIL_FROM")?
        .parse()
        .map_err(|_| "invalid registration sender address")?;
    Ok(Some(MailTransport {
        smtp,
        from,
        origin: origin.trim_end_matches('/').into(),
    }))
}

pub fn staff_invite_key(pool: &PgPool) -> Option<DeliveryKey> {
    let panel_enabled = std::env::var("OTERYN_ADMIN_PANEL").as_deref() == Ok("true");
    // The worker also initializes the shared security sender when SMTP is set.
    let smtp_ready = panel_enabled
        && mail_transport().is_ok_and(|transport| transport.is_some())
        && crate::sessions_http::delivery(pool.clone()).is_ok_and(|delivery| delivery.is_some());
    staff_invite_key_from(panel_enabled, smtp_ready, invite_key_env)
}

fn invite_key_env() -> Option<String> {
    std::env::var("OTERYN_RESET_DELIVERY_KEY").ok()
}

fn staff_invite_key_from(
    panel_enabled: bool,
    mail_configured: bool,
    key: impl FnOnce() -> Option<String>,
) -> Option<DeliveryKey> {
    if !panel_enabled || !mail_configured {
        return None;
    }
    key().and_then(|value| DeliveryKey::parse(&value))
}

struct StaffInviteMailer {
    mailer: Arc<Mailer>,
    key: DeliveryKey,
}

impl std::fmt::Debug for StaffInviteMailer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StaffInviteMailer([redacted])")
    }
}

impl Handler for StaffInviteMailer {
    fn run<'a>(&'a self, job: &'a oteryn_db::outbox::Job) -> HandlerFuture<'a> {
        Box::pin(async move { self.deliver(job).await.map(|_| ()) })
    }

    fn deliver<'a>(&'a self, job: &'a oteryn_db::outbox::Job) -> DeliveryFuture<'a> {
        Box::pin(async move {
            use staff_invites::DeliveryDecision;
            let (email, locale, token, expires_at) =
                match staff_invites::delivery(&self.mailer.pool, &self.key, &job.payload).await? {
                    DeliveryDecision::Send {
                        email,
                        locale,
                        token,
                        expires_at,
                    } => (email, locale, token, expires_at),
                    DeliveryDecision::AwaitingApproval
                    | DeliveryDecision::AwaitingIssuerAuthority => {
                        return Ok(DeliveryOutcome::Deferred);
                    }
                    DeliveryDecision::Closed => return Ok(DeliveryOutcome::Complete),
                };
            let (subject, body) = staff_invites::mail_text(&locale, expires_at);
            let message = Message::builder()
                .from(self.mailer.from.clone())
                .to(email
                    .parse()
                    .map_err(|_| "invalid staff invite recipient")?)
                .subject(subject)
                .body(format!(
                    "{body}\n\n{}/{locale}/staff-invite?token={token}",
                    self.mailer.origin
                ))
                .map_err(|_| "invalid staff invite message")?;
            self.mailer
                .smtp
                .send(message)
                .await
                .map_err(|_| "staff invite delivery failed")?;
            Ok(DeliveryOutcome::Complete)
        })
    }
}

pub fn register(registry: &mut Registry, pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let Some(MailTransport { smtp, from, origin }) = mail_transport()? else {
        return Ok(());
    };
    let mailer = Arc::new(Mailer {
        pool,
        smtp,
        from,
        origin,
    });
    let panel_enabled = std::env::var("OTERYN_ADMIN_PANEL").as_deref() == Ok("true");
    if let Some(key) = staff_invite_key_from(panel_enabled, true, invite_key_env) {
        registry.register(
            staff_invites::MAIL_KIND,
            Arc::new(StaffInviteMailer {
                mailer: mailer.clone(),
                key,
            }),
        )?;
    }
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
        // Primary email changes permanently advance the hold from its initial -infinity.
        // A legacy payload has no address to match once that has happened.
        let account: Option<(String, String)> = sqlx::query_as(
            "SELECT email,locale FROM accounts WHERE id=$1 AND (lower(email)=$2 OR ($2::text IS NULL AND email_change_blocked_until='-infinity')) AND status IN ('active','pending_verification','suspended')"
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
            .singlepart(SinglePart::plain(format!(
                "{body}\n\n{}/{locale}/login",
                self.origin
            )))
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
            .singlepart(SinglePart::plain(format!(
                "{introduction}\n\n{}/{locale}/verify-email?token={}",
                self.origin, payload.token
            )))
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
    async fn unavailable_delivery_blocks_resend_aliases() -> Result<(), Box<dyn std::error::Error>>
    {
        use axum::{
            Router,
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;

        let app =
            admission_with_readiness(Router::new().fallback(|| async { StatusCode::OK }), false);
        for path in [
            "/verify-email/resend",
            "/en/verify-email/resend",
            "/pl/verify-email/resend",
        ] {
            let response = app
                .clone()
                .oneshot(Request::post(path).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
        }
        Ok(())
    }

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
        assert!(raw.contains("MIME-Version: 1.0\r\n"));
        assert!(raw.contains("Content-Type: text/plain; charset=utf-8\r\n"));
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
        sqlx::query("UPDATE accounts SET email='changed@example.test',email_change_blocked_until=now()-interval '1 day' WHERE id=$1")
            .bind(id)
            .execute(&db.pool)
            .await?;
        assert!(mailer.prepare_existing(&payload).await?.is_none());
        assert!(
            mailer.prepare_existing(&legacy).await?.is_none(),
            "legacy notice targets replacement address"
        );
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
pub fn admission(app: axum::Router) -> axum::Router {
    let ready = mail_transport().is_ok_and(|transport| transport.is_some());
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
                    "/register"
                        | "/pl/register"
                        | "/en/register"
                        | "/verify-email/resend"
                        | "/pl/verify-email/resend"
                        | "/en/verify-email/resend"
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

#[cfg(test)]
#[test]
fn staff_invite_delivery_requires_panel_mail_and_key() {
    let configured_key = || Some("ab".repeat(32));
    assert!(staff_invite_key_from(false, true, configured_key).is_none());
    assert!(staff_invite_key_from(true, false, configured_key).is_none());
    assert!(staff_invite_key_from(true, true, || None).is_none());
    assert!(staff_invite_key_from(true, true, || Some("invalid".into())).is_none());
    assert!(staff_invite_key_from(true, true, configured_key).is_some());
}
