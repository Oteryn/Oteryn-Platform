//! Durable operations alerts, queued for delivery through the portal mail outbox.
//! Set OTERYN_ALERT_EMAILS to a comma-separated list of addresses. The optional
//! OTERYN_ALERT_5XX_THRESHOLD is a count of responses in the preceding five minutes.
use super::{Handler, HandlerFuture, Registry};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};
use oteryn_db::outbox::{self, Cleanup, Job, OffsetDateTime, PgPool, Postgres, Transaction, Uuid};
use std::{collections::BTreeSet, sync::Arc};

pub(super) const ALERT_JOB: &str = "observability.alerts";
pub(super) const MAIL_JOB: &str = "observability.email";
const DEFAULT_THRESHOLD: i64 = 25;
const SCHEDULE_SECONDS: i32 = 10;
const FINGERPRINT_BATCH: i64 = 20;

#[derive(Debug, PartialEq, Eq)]
enum Error {
    Recipients,
    Threshold,
    SmtpConfig,
    Storage,
    Delivery,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Recipients => {
                "OTERYN_ALERT_EMAILS must contain 1 to 25 comma-separated email addresses"
            }
            Self::Threshold => "OTERYN_ALERT_5XX_THRESHOLD must be a positive integer",
            Self::SmtpConfig => "operations SMTP configuration is incomplete or invalid",
            Self::Storage => "operations alert storage unavailable",
            Self::Delivery => "operations alert delivery failed",
        })
    }
}

impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Storage
    }
}

fn recipients(input: &str) -> Result<Vec<String>, Error> {
    let mut recipients = Vec::new();
    let mut seen = BTreeSet::new();
    for item in input.split(',').map(str::trim) {
        let address = item
            .parse::<lettre::Address>()
            .map_err(|_| Error::Recipients)?;
        if seen.insert(item.to_ascii_lowercase()) {
            recipients.push(address.to_string());
        }
        if recipients.len() > 25 {
            return Err(Error::Recipients);
        }
    }
    Ok(recipients)
}

#[derive(Clone)]
struct Alerts {
    pool: PgPool,
    recipients: Vec<String>,
    threshold: i64,
}
impl std::fmt::Debug for Alerts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Alerts").finish_non_exhaustive()
    }
}

impl Alerts {
    async fn scan(&self) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        let new_groups: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, route, error_kind FROM portal_errors
             WHERE alert_enqueued_at IS NULL
             ORDER BY first_seen, id LIMIT $1 FOR UPDATE SKIP LOCKED",
        )
        .bind(FINGERPRINT_BATCH)
        .fetch_all(&mut *tx)
        .await?;
        for (id, route, kind) in new_groups {
            self.enqueue(
                &mut tx,
                "Oteryn Portal | New error group / Nowa grupa błędów",
                &format!("{route} · {kind}\n\n/admin/errors/{id}\n"),
            )
            .await?;
            sqlx::query("UPDATE portal_errors SET alert_enqueued_at = now() WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        let (above, cooled_down): (bool, bool) = sqlx::query_as(
            "SELECT above_threshold, last_enqueued_at IS NULL
                  OR last_enqueued_at <= clock_timestamp() - interval '1 hour'
             FROM portal_error_spike WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let errors: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(count), 0)::bigint FROM portal_error_rate
             WHERE second BETWEEN floor(extract(epoch FROM statement_timestamp()))::bigint - 299
                              AND floor(extract(epoch FROM statement_timestamp()))::bigint",
        )
        .fetch_one(&mut *tx)
        .await?;
        let elevated = errors > self.threshold;
        let send = elevated && !above && cooled_down;
        if send {
            self.enqueue(
                &mut tx,
                "Oteryn Portal | Elevated 5xx rate / Wzrost liczby błędów 5xx",
                &format!(
                    "5xx responses in the past 5 minutes / Odpowiedzi 5xx w ciągu ostatnich 5 minut: {errors}\n\
                     Threshold / Próg: {}\n\n/admin/errors\n",
                    self.threshold,
                ),
            )
            .await?;
        }
        if above != elevated {
            sqlx::query(
                "UPDATE portal_error_spike SET above_threshold = $1,
                    last_enqueued_at = CASE WHEN $2 THEN clock_timestamp() ELSE last_enqueued_at END
                 WHERE singleton",
            )
            .bind(elevated)
            .bind(send)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: &str,
        text: &str,
    ) -> Result<(), Error> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO email_outbox(category, recipient, staff_recipients, subject, text_body, html_body)
             VALUES ('operations', '', $1, $2, $3, '') RETURNING id",
        )
        .bind(&self.recipients)
        .bind(subject)
        .bind(text)
        .fetch_one(&mut **tx)
        .await?;
        let key = id.to_string();
        outbox::enqueue(
            tx,
            MAIL_JOB,
            &format!("\"{key}\""),
            &key,
            OffsetDateTime::now_utc(),
            5,
        )
        .await?;
        Ok(())
    }
}

impl Cleanup for Alerts {
    fn clean(&self) -> HandlerFuture<'_> {
        Box::pin(async move { self.scan().await.map_err(|error| error.to_string()) })
    }
}

struct Delivery {
    pool: PgPool,
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}
impl std::fmt::Debug for Delivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationsMailDelivery")
            .finish_non_exhaustive()
    }
}

impl Delivery {
    async fn send(&self, job: &Job) -> Result<(), Error> {
        let id: Uuid = serde_json::from_str(&job.payload).map_err(|_| Error::Delivery)?;
        let pending: Option<(Vec<String>, String, String)> = sqlx::query_as(
            "SELECT staff_recipients, subject, text_body FROM email_outbox
             WHERE id = $1 AND category = 'operations' AND delivered_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((recipients, subject, text)) = pending else {
            return Ok(());
        };
        let mut mail = Message::builder()
            .from(self.from.clone())
            .subject(subject)
            .message_id(Some(format!("<{id}@notify.oteryn.invalid>")))
            .header(ContentType::TEXT_PLAIN);
        for address in recipients {
            mail = mail.bcc(address.parse().map_err(|_| Error::Delivery)?);
        }
        self.smtp
            .send(mail.body(text).map_err(|_| Error::Delivery)?)
            .await
            .map_err(|_| Error::Delivery)?;
        sqlx::query(
            "UPDATE email_outbox SET delivered_at = now(), updated_at = now()
             WHERE id = $1 AND category = 'operations'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl Handler for Delivery {
    fn run<'a>(&'a self, job: &'a Job) -> HandlerFuture<'a> {
        Box::pin(async move { self.send(job).await.map_err(|error| error.to_string()) })
    }
}

pub(super) fn register(
    registry: &mut Registry,
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(addresses) = std::env::var_os("OTERYN_ALERT_EMAILS") else {
        return Ok(());
    };
    let addresses = addresses.to_str().ok_or(Error::Recipients)?;
    if addresses.trim().is_empty() {
        return Ok(());
    }
    let recipients = recipients(addresses)?;
    let threshold = std::env::var_os("OTERYN_ALERT_5XX_THRESHOLD")
        .map(|value| {
            value
                .to_str()
                .and_then(|value| value.parse::<i64>().ok())
                .filter(|value| *value > 0)
                .ok_or(Error::Threshold)
        })
        .transpose()?
        .unwrap_or(DEFAULT_THRESHOLD);
    let setting = |key| std::env::var(key).map_err(|_| Error::SmtpConfig);
    let smtp = oteryn_notify::smtp::transport(
        &setting("OTERYN_SMTP_HOST")?,
        setting("OTERYN_SMTP_USERNAME")?,
        setting("OTERYN_SMTP_PASSWORD")?,
    )
    .map_err(|_| Error::SmtpConfig)?;
    let from = setting("OTERYN_SMTP_FROM")
        .or_else(|_| setting("OTERYN_MAIL_FROM"))?
        .parse()
        .map_err(|_| Error::SmtpConfig)?;

    registry.cleanup(
        ALERT_JOB,
        SCHEDULE_SECONDS,
        Arc::new(Alerts {
            pool: pool.clone(),
            recipients,
            threshold,
        }),
    )?;
    registry.register(MAIL_JOB, Arc::new(Delivery { pool, smtp, from }))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::tests::Database;

    #[test]
    fn alert_recipients_validate_deduplicate_and_bound_fanout() -> Result<(), Error> {
        assert_eq!(
            recipients("one@example.invalid, two@example.invalid, ONE@example.invalid")?,
            ["one@example.invalid", "two@example.invalid"]
        );
        for invalid in [
            "",
            "one",
            "ok@example.invalid,",
            "one@example.invalid\nBcc: two@example.invalid",
        ] {
            assert_eq!(recipients(invalid), Err(Error::Recipients));
        }
        let excessive = (0..26)
            .map(|n| format!("ops{n}@example.invalid"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(recipients(&excessive), Err(Error::Recipients));
        Ok(())
    }

    #[tokio::test]
    async fn alerts_enqueue_one_message_per_group_and_throttle_spikes()
    -> Result<(), Box<dyn std::error::Error>> {
        let Some(db) = Database::new().await? else {
            return Ok(());
        };
        let alerts = Alerts {
            pool: db.pool.clone(),
            recipients: recipients("ops-en@example.invalid,ops-pl@example.invalid")?,
            threshold: 2,
        };
        sqlx::query(
            "INSERT INTO portal_errors(route, error_kind, last_request_id)
                      VALUES('/fixture/{id}', 'http_500', uuidv7())",
        )
        .execute(&db.pool)
        .await?;

        let rate = |count| {
            sqlx::query(
                "WITH t AS (SELECT floor(extract(epoch FROM statement_timestamp()))::bigint AS second)
                 INSERT INTO portal_error_rate(slot, second, count)
                 SELECT second % 300, second, $1 FROM t
                 ON CONFLICT (slot) DO UPDATE SET second = excluded.second, count = excluded.count",
            ).bind(count).execute(&db.pool)
        };
        let messages = async || -> Result<Vec<(String, Vec<String>)>, sqlx::Error> {
            sqlx::query_as(
                "SELECT e.subject, e.staff_recipients FROM email_outbox e JOIN outbox j
                 ON j.idempotency_key = e.id::text WHERE e.category = 'operations'
                 AND j.kind = $1 ORDER BY e.id",
            )
            .bind(MAIL_JOB)
            .fetch_all(&db.pool)
            .await
        };
        rate(2i64).await?;
        let (first, duplicate) = tokio::join!(alerts.scan(), alerts.scan());
        first?;
        duplicate?;
        let queued = messages().await?;
        assert_eq!(queued.len(), 1);
        assert!(queued[0].0.contains("New error group"));
        assert_eq!(queued[0].1, alerts.recipients);

        sqlx::query("DELETE FROM portal_error_rate")
            .execute(&db.pool)
            .await?;
        rate(3i64).await?;
        alerts.scan().await?;
        alerts.scan().await?;
        let queued = messages().await?;
        assert_eq!(queued.len(), 2);
        assert!(queued[1].0.contains("Elevated 5xx rate"));

        // A second crossing in the active cooldown is suppressed.
        sqlx::query("DELETE FROM portal_error_rate")
            .execute(&db.pool)
            .await?;
        alerts.scan().await?;
        rate(3i64).await?;
        alerts.scan().await?;
        assert_eq!(messages().await?.len(), 2);

        // A crossing after the cooldown may notify again.
        sqlx::query("DELETE FROM portal_error_rate")
            .execute(&db.pool)
            .await?;
        alerts.scan().await?;
        sqlx::query("UPDATE portal_error_spike SET last_enqueued_at = now() - interval '1 hour'")
            .execute(&db.pool)
            .await?;
        rate(3i64).await?;
        alerts.scan().await?;
        assert_eq!(messages().await?.len(), 3);
        db.close().await
    }
}
