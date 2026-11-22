use super::{Language, enabled, localized, payment_actions_enabled, render};
use crate::{
    rbac::{CheckContext, MutationAuthorization, Permission},
    staff_actions::{self, Action, AuditChange, AuditValue, Error, Guard},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        Query, State,
        rejection::{FormRejection, QueryRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_ui::Chrome;
use oteryn_web::{csrf::Verified, session::BrowserSession};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/payment-review", get(review).post(record_review))
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
    exponent: Option<i16>,
    refund_settled: bool,
    refund_refused: bool,
    updated_at: String,
    refund_state: Option<String>,
    refund_delivery: Option<String>,
    refund_attempts: Option<i32>,
    refund_requested: bool,
    credited: bool,
    reversal_kind: Option<String>,
    reversal_outcome: Option<String>,
    purchase_hold: bool,
    outstanding_debt: i64,
    requires_review: bool,
    reviewed_at: Option<String>,
}

impl Payment {
    fn price(&self) -> String {
        match self.exponent.and_then(|value| u8::try_from(value).ok()) {
            Some(exponent) => oteryn_payments::coin::Price {
                exponent,
                minor_units: self.amount,
                currency: self.currency.clone(),
            }
            .display(),
            None => super::super::http::format_refund_amount(self.amount, &self.currency),
        }
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
            _ if self.refund_settled => "commerce-refund-done",
            _ if self.refund_refused => "commerce-refund-refused",
            (Some("done"), _) => "commerce-refund-dispatched",
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

    fn reversal_kind_key(&self) -> Option<&'static str> {
        match self.reversal_kind.as_deref() {
            Some("refund") => Some("commerce-refund-reversal"),
            Some("chargeback") => Some("commerce-payment-chargeback"),
            _ => None,
        }
    }

    fn reversal_key(&self) -> &'static str {
        match self.reversal_outcome.as_deref() {
            Some("debited") => "commerce-reversal-debited",
            Some("held") => "commerce-reversal-held",
            Some("fenced") => "commerce-reversal-fenced",
            _ => "commerce-state-unknown",
        }
    }
}

const PAYMENTS: &str = "
    SELECT p.id,p.account_id,p.provider,p.currency,p.amount,p.state,o.exponent,
        (p.state='refunded' OR (p.state='chargeback' AND EXISTS(
            SELECT 1 FROM payment_events WHERE intent_id=p.id AND state='refunded'
        ))) AS refund_settled,
        COALESCE(refund.result->>'refund_outcome'='refused',false) AS refund_refused,
        to_char(p.updated_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS updated_at,
        f.state AS refund_state, job.state AS refund_delivery,job.attempts AS refund_attempts,
        EXISTS(SELECT 1 FROM commerce_operations where target_id=p.id AND command->>'kind'='refund' AND result IS NULL) AS refund_requested,
        EXISTS(SELECT 1 FROM wallet_operations WHERE id=p.id AND kind='top_up') AS credited,
        r.kind AS reversal_kind, r.outcome AS reversal_outcome, h.account_id IS NOT NULL AS purchase_hold,
        COALESCE(d.owed,0) AS outstanding_debt,
        COALESCE(o.review_required AND (review.updated_at IS NULL OR GREATEST(o.updated_at,p.updated_at) > review.updated_at),false) AS requires_review,
        to_char(review.updated_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') AS reviewed_at
    FROM payment_intents p
    LEFT JOIN commerce_refunds f ON f.intent_id=p.id
    LEFT JOIN commerce_operations refund ON refund.id=f.operation_id
    LEFT JOIN outbox job ON job.kind='commerce.refund' AND job.idempotency_key=p.id::text AND job.payload->>'operation'=f.operation_id::text
    LEFT JOIN coin_purchase_orders o ON o.payment_intent_id=p.id
    LEFT JOIN coin_purchase_reversals r ON r.order_id=o.id
    LEFT JOIN coin_purchase_holds h ON h.order_id=r.order_id AND h.account_id=p.account_id
    LEFT JOIN coin_outstanding_debts d ON d.reversal_id=r.id
    LEFT JOIN LATERAL (
        SELECT updated_at FROM commerce_operations
        WHERE target_id=p.id AND command->>'kind'='reconcile' AND result->>'resolution_code'='reviewed_no_payment_state_change'
        ORDER BY updated_at DESC LIMIT 1
    ) review ON true
    WHERE ($1::uuid IS NULL OR p.id=$1) AND ($2::uuid IS NULL OR p.account_id=$2)
      AND (NOT $3 OR (o.review_required AND (review.updated_at IS NULL OR GREATEST(o.updated_at,p.updated_at) > review.updated_at)) OR job.state='dead')
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
    csrf: String,
    request_id: Uuid,
    refunds: bool,
}

async fn review(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
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
    let csrf = if detail {
        match session.csrf_token().await {
            Ok(token) => token,
            Err(_) => return Error::Unavailable.response(&chrome),
        }
    } else {
        String::new()
    };
    render(&Page {
        chrome,
        rows,
        detail,
        account: filter.account.map(|id| id.to_string()).unwrap_or_default(),
        queue,
        next,
        csrf,
        request_id: Uuid::now_v7(),
        refunds: payment_actions_enabled() && enabled("OTERYN_REFUNDS_ENABLED"),
    })
}

#[derive(Deserialize)]
struct ReviewForm {
    id: Uuid,
    request_id: Uuid,
    reason: String,
}

async fn record_review(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    csrf: Option<Extension<Verified>>,
    Query(query): Query<Language>,
    form: Result<Form<ReviewForm>, FormRejection>,
) -> Response {
    let chrome = localized(chrome, query.lang.as_deref());
    let Ok(Form(ReviewForm {
        id,
        request_id,
        reason,
    })) = form
    else {
        return Error::Invalid.response(&chrome);
    };
    let hash = format!("{:x}", Sha256::digest(reason.trim().as_bytes()));
    let result = staff_actions::run(
        &pool,
        Guard { actor, authority: &authority, csrf: csrf.map(|value| value.0) },
        Action { permission: Permission::ReconcilePayments, action: "commerce.payment.reviewed", reason: &reason },
        || {
            if !super::super::valid_id(id) || !super::super::valid_id(request_id) {
                return Err(Error::Invalid);
            }
            Ok((vec![], hash))
        },
        |conn, hash| Box::pin(async move {
            let state: String = sqlx::query_scalar("SELECT state FROM payment_intents WHERE id=$1 FOR UPDATE")
                .bind(id).fetch_optional(&mut *conn).await?.ok_or(Error::Invalid)?;
            let payment_state = match state.as_str() {
                "created" => "created", "pending" => "pending", "succeeded" => "succeeded",
                "failed" => "failed", "refunded" => "refunded", "chargeback" => "chargeback",
                _ => return Err(Error::Unavailable),
            };
            let inserted = sqlx::query(
                "INSERT INTO commerce_operations(id,actor_id,target_id,command,reason_sha256,requires_approval,result,created_at,updated_at)
                 VALUES($1,$2,$3,'{\"kind\":\"reconcile\"}',$4,false,
                        jsonb_build_object('resolution_code','reviewed_no_payment_state_change','state',$5::text,'intent',$3::uuid),statement_timestamp(),statement_timestamp())
                 ON CONFLICT DO NOTHING"
            ).bind(request_id).bind(actor.account).bind(id).bind(&hash).bind(payment_state)
                .execute(&mut *conn).await?.rows_affected();
            if inserted == 0 {
                let replay: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM commerce_operations
                     WHERE id=$1 AND actor_id=$2 AND target_id=$3 AND reason_sha256=$4
                       AND command->>'kind'='reconcile' AND result->>'resolution_code'='reviewed_no_payment_state_change')"
                ).bind(request_id).bind(actor.account).bind(id).bind(hash).fetch_one(&mut *conn).await?;
                return if replay { Ok(None) } else { Err(Error::Conflict) };
            }
            Ok(Some(AuditChange {
                target: id,
                before: [("payment_state", AuditValue::Label(payment_state))].into(),
                after: [
                    ("payment_state", AuditValue::Label(payment_state)),
                    ("review", AuditValue::Id(request_id)),
                ].into(),
            }))
        }),
    ).await;
    match result {
        Ok(()) => Redirect::to(&format!(
            "/admin/payment-review?lang={}&id={id}",
            chrome.lang
        ))
        .into_response(),
        Err(error) => error.response(&chrome),
    }
}
