use super::http::{Locale, failure};
use super::{Commerce, Effects, Error, valid_id};
use crate::rbac;
use askama::Template;
use axum::{
    Extension,
    extract::{Query, State, rejection::QueryRejection},
    http::HeaderMap,
    response::Response,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::{fmt::Display, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Source {
    Store,
    Wallet,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Filter {
    source: Option<Source>,
    id: Option<String>,
    #[serde(rename = "lang")]
    _lang: Option<String>,
}

struct Fact {
    label: &'static str,
    value: String,
}

#[derive(Default)]
struct Evidence(Vec<Fact>);
impl Evidence {
    fn add(&mut self, label: &'static str, value: impl Display) {
        self.0.push(Fact {
            label,
            value: value.to_string(),
        });
    }

    fn optional(&mut self, label: &'static str, value: Option<impl Display>) {
        if let Some(value) = value {
            self.add(label, value);
        }
    }
}

async fn store(pool: &PgPool, id: Uuid, locale: Locale) -> Result<Option<Evidence>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT o.operation_kind, o.state, o.binding_digest, o.source_revision,
                u.state AS unit_state, l.order_id, r.receipt_digest, r.outcome, r.result_code,
                r.recovery_generation::text AS receipt_generation,
                fence.new_recovery_generation::text AS fence_generation,
                restore.restore_event_id
         FROM store_operations o
         JOIN store_fulfilment_units u USING (entitlement_id)
         JOIN store_order_lines l USING (line_id)
         LEFT JOIN store_receipts r
           ON r.issuer_authority = o.issuer_authority AND r.operation_id = o.operation_id
           AND r.superseded_by_restore_event_id IS NULL
         LEFT JOIN store_delivery_fence fence ON fence.id = 1
         LEFT JOIN LATERAL (
             SELECT restore_event_id FROM store_reconcile_state
             WHERE operation_id = o.operation_id
             ORDER BY new_recovery_generation DESC LIMIT 1
         ) restore ON true
         WHERE o.operation_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let mut evidence = Evidence::default();
    let state: &str = row.try_get("state")?;
    let digest: Option<&str> = row.try_get("receipt_digest")?;
    let outcome: Option<&str> = row.try_get("outcome")?;
    evidence.add("status-kind", row.try_get::<&str, _>("operation_kind")?);
    evidence.add(
        "status-state",
        match (state, outcome) {
            ("PENDING", _) => locale.text("status-pending"),
            ("REOPENED", _) => locale.text("status-reopened"),
            ("COMMITTED", Some("COMMITTED")) => locale.text("status-store-committed"),
            ("REJECTED", Some("REJECTED")) => locale.text("status-store-rejected"),
            _ => locale.text("unavailable"),
        },
    );
    evidence.add("status-unit-state", row.try_get::<&str, _>("unit_state")?);
    evidence.add("status-order", row.try_get::<Uuid, _>("order_id")?);
    evidence.add("status-revision", row.try_get::<i64, _>("source_revision")?);
    evidence.add("status-binding", row.try_get::<&str, _>("binding_digest")?);
    evidence.add(
        "status-receipt",
        digest.map_or_else(|| locale.text("unavailable"), str::to_owned),
    );
    evidence.optional(
        "status-result",
        row.try_get::<Option<&str>, _>("result_code")?,
    );
    evidence.optional(
        "status-receipt-generation",
        row.try_get::<Option<&str>, _>("receipt_generation")?,
    );
    let fence: Option<&str> = row.try_get("fence_generation")?;
    evidence.add(
        "status-fence-generation",
        match fence {
            Some("0") | None => locale.text("status-fence-unavailable"),
            Some(generation) => generation.to_owned(),
        },
    );
    evidence.optional(
        "status-restore",
        row.try_get::<Option<Uuid>, _>("restore_event_id")?,
    );
    Ok(Some(evidence))
}

async fn wallet(pool: &PgPool, id: Uuid, locale: Locale) -> Result<Option<Evidence>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT o.kind, o.amount, o.account_id, o.counterparty_id,
                reservation.id AS reservation_id, reservation.status,
                CASE WHEN reservation.status = 'active'
                     THEN reservation.amount - reservation.consumed
                     WHEN reservation.id IS NOT NULL THEN 0 END AS remaining
         FROM wallet_operations o
         LEFT JOIN wallet_reservations reservation
           ON reservation.id = CASE WHEN o.kind = 'reserve' THEN o.id ELSE o.reference_id END
         LEFT JOIN wallet_operations offer ON offer.id = reservation.id
         WHERE o.id = $1
           AND (o.kind IN ('transfer', 'settlement')
                OR (o.kind IN ('reserve', 'release') AND offer.reason LIKE 'gold-price:%'))
           AND EXISTS (SELECT 1 FROM wallet_entries e
                       WHERE e.operation_id = o.id AND e.account_id = o.account_id)",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let mut evidence = Evidence::default();
    let kind: &str = row.try_get("kind")?;
    let label = match kind {
        "transfer" => "status-transfer",
        "settlement" => "status-settlement",
        "reserve" => "status-reserve",
        "release" => "status-release",
        _ => "unavailable",
    };
    evidence.add("status-kind", locale.text(label));
    evidence.add("status-state", locale.text("status-wallet-committed"));
    evidence.add("status-amount", row.try_get::<i64, _>("amount")?);
    evidence.add("status-holder", row.try_get::<Uuid, _>("account_id")?);
    evidence.optional(
        "status-counterparty",
        row.try_get::<Option<Uuid>, _>("counterparty_id")?,
    );
    if let Some(reservation) = row.try_get::<Option<Uuid>, _>("reservation_id")? {
        evidence.add("status-reservation", reservation);
        let label = match row.try_get::<&str, _>("status")? {
            "active" => "status-active",
            "captured" => "status-filled",
            "released" => "status-released",
            _ => "unavailable",
        };
        evidence.add("status-reservation-state", locale.text(label));
        evidence.add("status-remaining", row.try_get::<i64, _>("remaining")?);
    }
    if kind != "transfer" {
        evidence.add("status-gold-state", locale.text("unavailable"));
    }
    Ok(Some(evidence))
}

#[derive(Template)]
#[template(
    path = "commerce/operation_status.html",
    config = "src/commerce/askama.toml"
)]
struct Page {
    locale: Locale,
    lookup: Option<(Source, Uuid)>,
    evidence: Option<Evidence>,
}

impl Page {
    fn wallet_selected(&self) -> bool {
        matches!(self.lookup, Some((Source::Wallet, _)))
    }
}

pub(super) async fn page<A: rbac::AccountStateReader, E: Effects>(
    State(service): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    headers: HeaderMap,
    filter: Result<Query<Filter>, QueryRejection>,
) -> Response {
    let locale = Locale::from_headers(&headers);
    if let Err(error) = service.authorize(context, &super::Command::Reconcile).await {
        return failure(locale, error);
    }
    let Ok(Query(filter)) = filter else {
        return failure(locale, Error::Invalid);
    };
    let lookup = match (filter.source, filter.id.as_deref()) {
        (None, None) => None,
        (Some(source), Some(raw)) if raw.len() == 36 => {
            let Ok(id) = Uuid::parse_str(raw) else {
                return failure(locale, Error::Invalid);
            };
            if !valid_id(id) || id.to_string() != raw {
                return failure(locale, Error::Invalid);
            }
            Some((source, id))
        }
        _ => return failure(locale, Error::Invalid),
    };
    let evidence = match lookup {
        Some((Source::Store, id)) => store(&service.pool, id, locale).await,
        Some((Source::Wallet, id)) => wallet(&service.pool, id, locale).await,
        None => Ok(None),
    };
    match evidence {
        Ok(evidence) => oteryn_web::render(&Page {
            locale,
            lookup,
            evidence,
        }),
        Err(error) => failure(locale, error.into()),
    }
}
