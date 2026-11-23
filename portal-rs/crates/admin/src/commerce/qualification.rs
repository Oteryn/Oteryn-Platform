//! Read-only evidence for reconciling sandbox payments and refunds.
use super::{http::Locale, *};
use oteryn_payments::{
    coin::{CoinSettings, Price},
    provider_webhook::WebhookSettings,
};

pub(super) struct Qualification {
    pub checkout: String,
    pub webhook: String,
    pub payment: Option<Vec<Fact>>,
    pub event: Option<Vec<Fact>>,
    pub intent_id: Option<Uuid>,
    pub event_id: Option<String>,
}

pub(super) struct Fact {
    pub label: String,
    pub value: String,
}

fn status(locale: Locale, key: Option<&str>) -> String {
    locale.text(key.unwrap_or("qualification-no-record"))
}

impl Qualification {
    pub async fn load(
        pool: &PgPool,
        locale: Locale,
        intent: Option<&str>,
        event: Option<&str>,
    ) -> Result<Self, Error> {
        let intent_id = intent
            .map(|id| oteryn_payments::coin::canonical_id(id).ok_or(Error::Invalid))
            .transpose()?;
        if event.is_some_and(|id| id.len() > 128 || !id.bytes().all(|b| b.is_ascii_graphic())) {
            return Err(Error::Invalid);
        }
        let lookup = |key: &str| std::env::var(key).ok();
        let settings = CoinSettings::from_lookup(lookup);
        let configured = |value| {
            locale.text(if value {
                "qualification-sandbox-configured"
            } else {
                "unavailable"
            })
        };
        let checkout = configured(settings.enabled && !settings.packages.is_empty());
        let webhook =
            configured(settings.enabled && WebhookSettings::from_lookup(lookup).is_some());
        let payment = match intent_id {
            Some(id) => payment(pool, locale, id).await?,
            None => None,
        };
        let event_facts = match event {
            Some(id) => delivery(pool, locale, id).await?,
            None => None,
        };
        Ok(Self {
            checkout,
            webhook,
            payment,
            event: event_facts,
            intent_id,
            event_id: event.map(str::to_owned),
        })
    }
}

async fn payment(
    pool: &PgPool,
    locale: Locale,
    id: Uuid,
) -> Result<Option<Vec<Fact>>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT p.state AS payment, p.amount, p.currency, i.id AS receipt,
                c.state AS coin_order, c.review_required, c.id AS coin_order_id, c.exponent,
                w.available_delta AS credit, t.state AS top_up_job,
                r.state AS refund, f.state AS refund_job, v.outcome AS reversal, v.debited
         FROM payment_intents p
         LEFT JOIN invoices i ON i.intent_id=p.id AND i.account_id=p.account_id
         LEFT JOIN coin_purchase_orders c ON c.payment_intent_id=p.id AND c.payer_account_id=p.account_id
         LEFT JOIN wallet_operations wo ON wo.id=p.id AND wo.kind='top_up'
         LEFT JOIN wallet_entries w ON w.operation_id=wo.id AND w.account_id=p.account_id
         LEFT JOIN outbox t ON t.kind='wallet.top_up' AND t.idempotency_key=p.id::text
         LEFT JOIN commerce_refunds r ON r.intent_id=p.id AND r.account_id=p.account_id
         LEFT JOIN outbox f ON f.kind='commerce.refund' AND f.idempotency_key=p.id::text
         LEFT JOIN coin_purchase_reversals v ON v.order_id=c.id AND v.payer_account_id=p.account_id
         WHERE p.id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        let fact = |key, value| Fact {
            label: locale.text(key),
            value,
        };
        let state = |column| {
            let value: Option<String> = r.try_get(column)?;
            Ok::<_, sqlx::Error>(value.map_or_else(
                || status(locale, None),
                |value| locale.text(&format!("qualification-state-{value}")),
            ))
        };
        let number = |column| {
            let value: Option<i64> = r.try_get(column)?;
            Ok::<_, sqlx::Error>(value.map_or_else(|| status(locale, None), |n| n.to_string()))
        };
        let identifier = |column| {
            let value: Option<Uuid> = r.try_get(column)?;
            Ok::<_, sqlx::Error>(value.map_or_else(|| status(locale, None), |id| id.to_string()))
        };
        let review = r
            .try_get::<Option<bool>, _>("review_required")?
            .map(|required| {
                if required {
                    "qualification-yes"
                } else {
                    "qualification-no"
                }
            });
        let minor_units: i64 = r.try_get("amount")?;
        let currency: String = r.try_get("currency")?;
        let amount = match r.try_get::<Option<i16>, _>("exponent")? {
            Some(exponent) => Price {
                minor_units,
                currency,
                exponent: u8::try_from(exponent)
                    .map_err(|_| sqlx::Error::Protocol("invalid coin price exponent".into()))?,
            }
            .display(),
            None => super::http::format_refund_amount(minor_units, &currency),
        };
        Ok(vec![
            fact("qualification-payment", state("payment")?),
            fact("qualification-price", amount),
            fact("qualification-receipt", identifier("receipt")?),
            fact("qualification-order-id", identifier("coin_order_id")?),
            fact("qualification-order", state("coin_order")?),
            fact("qualification-credit", number("credit")?),
            fact("qualification-top-up-job", state("top_up_job")?),
            fact("qualification-refund", state("refund")?),
            fact("qualification-refund-job", state("refund_job")?),
            fact("qualification-reversal", state("reversal")?),
            fact("qualification-debited", number("debited")?),
            fact("qualification-review", status(locale, review)),
        ])
    })
    .transpose()
}

async fn delivery(
    pool: &PgPool,
    locale: Locale,
    id: &str,
) -> Result<Option<Vec<Fact>>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT e.event_type, e.outcome, p.id AS payment_id
         FROM payment_provider_webhook_events e
         LEFT JOIN payment_intents p ON p.provider='test' AND p.provider_reference=e.order_reference
             AND EXISTS (SELECT 1 FROM coin_purchase_orders c WHERE c.payment_intent_id=p.id)
         WHERE e.event_id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        let kind: String = r.try_get("event_type")?;
        let outcome: String = r.try_get("outcome")?;
        let payment: Option<Uuid> = r.try_get("payment_id")?;
        Ok(vec![
            Fact {
                label: locale.text("qualification-event-type"),
                value: locale.text(match kind.as_str() {
                    "refund" => "qualification-refund",
                    "chargeback" => "qualification-state-chargeback",
                    _ => "qualification-other-event",
                }),
            },
            Fact {
                label: locale.text("qualification-event-result"),
                value: locale.text(&format!("qualification-state-{outcome}")),
            },
            Fact {
                label: locale.text("qualification-intent-id"),
                value: payment.map_or_else(|| status(locale, None), |id| id.to_string()),
            },
        ])
    })
    .transpose()
}
