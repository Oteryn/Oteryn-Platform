use super::{enabled, localized, payment_actions_enabled, render};
use askama::Template;
use axum::{
    Extension, Router,
    extract::{Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::Response,
    routing::get,
};
use oteryn_ui::Chrome;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/payment-review", get(review))
        .with_state(pool)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filter {
    lang: Option<String>,
    #[serde(default, deserialize_with = "optional_uuid")]
    id: Option<Uuid>,
    #[serde(default, deserialize_with = "optional_uuid")]
    account: Option<Uuid>,
    queue: Option<bool>,
    before: Option<Uuid>,
}

fn optional_uuid<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Uuid>, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() {
        return Ok(None);
    }
    Uuid::parse_str(&value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[derive(sqlx::FromRow)]
struct Payment {
    id: Uuid,
    account_id: Uuid,
    provider: String,
    currency: String,
    amount: i64,
    state: String,
    updated_at: String,
    refund_state: Option<String>,
    refund_delivery: Option<String>,
    refund_attempts: Option<i32>,
    refund_requested: bool,
    credited: bool,
    chargeback: Option<String>,
    chargeback_hold: bool,
    outstanding_debt: i64,
    requires_review: bool,
    reviewed_at: Option<String>,
}

impl Payment {
    fn price(&self) -> String {
        super::super::http::format_refund_amount(self.amount, &self.currency)
    }

    fn status_key(&self) -> &'static str {
        match self.state.as_str() {
            "created" => "commerce-payment-created",
            "pending" => "commerce-payment-pending",
            "succeeded" => "commerce-payment-succeeded",
            "failed" => "commerce-payment-failed",
            "refunded" => "commerce-payment-refunded",
            "chargeback" => "commerce-payment-chargeback",
            _ => "commerce-state-unknown",
        }
    }

    fn refund_key(&self) -> &'static str {
        match (
            self.refund_state.as_deref(),
            self.refund_delivery.as_deref(),
        ) {
            (Some("done"), _) => "commerce-refund-done",
            (_, Some("dead")) => "commerce-refund-exhausted",
            (Some("refunding"), _) => "commerce-refund-processing",
            (Some("pending"), _) => "commerce-refund-queued",
            _ if self.refund_requested => "commerce-refund-approval",
            _ if self.state != "succeeded" => "commerce-refund-ineligible-state",
            _ if self.credited => "commerce-refund-ineligible-credited",
            _ if self.provider != "test" => "commerce-refund-disabled",
            _ => "commerce-refund-eligible",
        }
    }

    fn reversal_key(&self) -> &'static str {
        match self.chargeback.as_deref() {
            Some("debited") => "commerce-reversal-debited",
            Some("held") => "commerce-reversal-held",
            Some("fenced") => "commerce-reversal-fenced",
            _ => "commerce-state-unknown",
        }
    }
}

const PAYMENTS: &str = "
    SELECT p.id,p.account_id,p.provider,p.currency,p.amount,p.state,
        to_char(p.updated_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS updated_at,
        f.state AS refund_state, job.state AS refund_delivery,job.attempts AS refund_attempts,
        EXISTS(SELECT 1 FROM commerce_operations where target_id=p.id AND command->>'kind'='refund' AND result IS NULL) AS refund_requested,
        EXISTS(SELECT 1 FROM wallet_operations WHERE id=p.id AND kind='top_up') AS credited,
        r.outcome AS chargeback, h.account_id IS NOT NULL AS chargeback_hold,
        COALESCE(d.owed,0) AS outstanding_debt, COALESCE(o.review_required,false) AS requires_review,
        to_char(review.updated_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS reviewed_at
    FROM payment_intents p
    LEFT JOIN commerce_refunds f ON f.intent_id=p.id
    LEFT JOIN outbox job ON job.kind='commerce.refund' AND job.idempotency_key=p.id::text AND job.payload->>'operation'=f.operation_id::text
    LEFT JOIN coin_purchase_orders o ON o.payment_intent_id=p.id
    LEFT JOIN coin_purchase_reversals r ON r.order_id=o.id AND r.kind='chargeback'
    LEFT JOIN coin_purchase_holds h ON h.order_id=r.order_id AND h.account_id=p.account_id
    LEFT JOIN coin_outstanding_debts d ON d.reversal_id=r.id
    LEFT JOIN LATERAL (
        SELECT updated_at FROM commerce_operations
        WHERE target_id=p.id AND command->>'kind'='reconcile' AND result->>'resolution_code'='reviewed_no_payment_state_change'
        ORDER BY updated_at DESC LIMIT 1
    ) review ON true
    WHERE ($1::uuid IS NULL OR p.id=$1) AND ($2::uuid IS NULL OR p.account_id=$2)
      AND (NOT $3 OR (o.review_required AND (review.updated_at IS NULL OR o.updated_at > review.updated_at)) OR job.state='dead')
      AND ($4::uuid IS NULL OR (p.created_at,p.id)<(SELECT created_at,id FROM payment_intents WHERE id=$4))
    ORDER BY p.created_at DESC,p.id DESC LIMIT 26";

#[derive(Template)]
#[template(
    path = "commerce/panels/payments.html",
    config = "src/commerce/askama.toml"
)]
struct Page {
    chrome: Chrome,
    rows: Vec<Payment>,
    detail: bool,
    account: String,
    queue: bool,
    next: Option<Uuid>,
    actions: bool,
    refunds: bool,
}

async fn review(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    filter: Result<Query<Filter>, QueryRejection>,
) -> Response {
    let Ok(Query(filter)) = filter else {
        return oteryn_ui::error(&chrome, StatusCode::UNPROCESSABLE_ENTITY);
    };
    let chrome = localized(chrome, filter.lang.as_deref());
    let queue = filter.queue.unwrap_or(false);
    let detail = filter.id.is_some();
    let mut rows: Vec<Payment> = match sqlx::query_as(PAYMENTS)
        .bind(filter.id)
        .bind(filter.account)
        .bind(queue)
        .bind(filter.before)
        .fetch_all(&pool)
        .await
    {
        Ok(rows) => rows,
        Err(_) => return oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE),
    };
    if detail && rows.is_empty() {
        return oteryn_ui::error(&chrome, StatusCode::NOT_FOUND);
    }
    let next = if rows.len() > 25 {
        rows.pop();
        rows.last().map(|row| row.id)
    } else {
        None
    };
    let actions = payment_actions_enabled();
    render(&Page {
        chrome,
        rows,
        detail,
        account: filter.account.map(|id| id.to_string()).unwrap_or_default(),
        queue,
        next,
        actions,
        refunds: actions && enabled("OTERYN_REFUNDS_ENABLED"),
    })
}
