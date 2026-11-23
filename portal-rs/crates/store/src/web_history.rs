use std::collections::HashMap;

use oteryn_wallet::HistoryEntry;
use sqlx::Row;

use super::{Locale, PgPool, Uuid};

pub(super) struct Entry {
    pub(super) wallet: HistoryEntry,
    pub(super) label: String,
    pub(super) is_purchase: bool,
    pub(super) is_settlement: bool,
    pub(super) units: Vec<Unit>,
}

pub(super) struct Unit {
    pub(super) operation: Option<Uuid>,
    pub(super) product: String,
    pub(super) quantity: i32,
    pub(super) state: String,
    pub(super) awaiting_evidence: bool,
}

fn delivery_label(unit: &str, operation: Option<&str>, has_receipt: bool) -> (&'static str, bool) {
    match (unit, operation) {
        (_, Some("REOPENED")) => ("delivery-reopened", true),
        (_, None) => ("unavailable", false),
        ("DELIVERY_PENDING", _) => ("delivery-pending", true),
        ("CLAIM_PENDING", _) | (_, Some("CLAIM_PENDING")) => ("delivery-claim-pending", true),
        _ if !has_receipt => ("unavailable", false),
        ("DELIVERED", _) => ("delivery-delivered", false),
        ("CLAIMED", _) => ("delivery-claimed", false),
        ("REVOKED", _) => ("delivery-revoked", false),
        ("RECLAIMED", _) => ("delivery-reclaimed", false),
        ("MANUAL_REVIEW", _) => ("delivery-review", false),
        ("REVERSAL_PARTIAL", _) => ("delivery-partial", false),
        _ => ("unavailable", false),
    }
}

pub(super) async fn entries(
    pool: &PgPool,
    locale: Locale,
    account: Uuid,
    wallet: Vec<HistoryEntry>,
) -> Result<Vec<Entry>, sqlx::Error> {
    let purchases: Vec<_> = wallet
        .iter()
        .filter(|entry| entry.kind == "store_purchase")
        .map(|entry| entry.operation_id)
        .collect();
    let mut by_purchase = HashMap::<Uuid, Vec<Unit>>::new();
    if !purchases.is_empty() {
        let units = sqlx::query(
            "SELECT o.wallet_operation_id, u.grant_key, u.quantity, u.state,
                    latest.operation_id, latest.operation_state,
                    COALESCE(latest.has_receipt, false) AS has_receipt
             FROM store_orders o
             JOIN store_order_lines l USING (order_id)
             JOIN store_fulfilment_units u USING (line_id)
             LEFT JOIN LATERAL (
                 SELECT s.operation_id, CASE WHEN s.state = 'PENDING' AND s.operation_kind = 'CLAIM'
                             AND u.state = 'DELIVERED' THEN 'CLAIM_PENDING'
                             ELSE s.state END AS operation_state,
                        bool_or(r.receipt_id IS NOT NULL) OVER () AS has_receipt
                 FROM store_operations s
                 LEFT JOIN store_receipts r ON r.operation_id = s.operation_id
                     AND r.issuer_authority = s.issuer_authority
                     AND r.superseded_by_restore_event_id IS NULL
                 WHERE s.entitlement_id = u.entitlement_id
                 ORDER BY (s.state = 'REOPENED') DESC, s.source_revision DESC LIMIT 1
             ) latest ON true
             WHERE o.account_id = $1 AND u.account_id = $1 AND o.wallet_operation_id = ANY($2)
             ORDER BY o.wallet_operation_id, u.unit_index",
        )
        .bind(account)
        .bind(&purchases)
        .fetch_all(pool)
        .await?;
        for row in units {
            let (key, awaiting_evidence) = delivery_label(
                row.try_get("state")?,
                row.try_get("operation_state")?,
                row.try_get("has_receipt")?,
            );
            by_purchase
                .entry(row.try_get("wallet_operation_id")?)
                .or_default()
                .push(Unit {
                    operation: row.try_get("operation_id")?,
                    product: row.try_get("grant_key")?,
                    quantity: row.try_get("quantity")?,
                    state: locale.text(key),
                    awaiting_evidence,
                });
        }
    }
    Ok(wallet
        .into_iter()
        .map(|wallet| {
            let key = match wallet.kind.as_str() {
                "store_purchase" => "history-store-purchase",
                "transfer" => "history-transfer",
                "reserve" => "history-reserve",
                "settlement" => "history-settlement",
                "release" => "history-release",
                _ => "operation",
            };
            Entry {
                label: locale.text(key),
                is_purchase: wallet.kind == "store_purchase",
                is_settlement: wallet.kind == "settlement",
                units: by_purchase.remove(&wallet.operation_id).unwrap_or_default(),
                wallet,
            }
        })
        .collect())
}

#[cfg(test)]
#[test]
fn delivery_evidence_states() {
    for (unit, operation, has_receipt, expected) in [
        (
            "DELIVERY_PENDING",
            Some("PENDING"),
            false,
            ("delivery-pending", true),
        ),
        (
            "CLAIM_PENDING",
            Some("PENDING"),
            false,
            ("delivery-claim-pending", true),
        ),
        (
            "CLAIMED",
            Some("COMMITTED"),
            true,
            ("delivery-claimed", false),
        ),
        (
            "MANUAL_REVIEW",
            Some("REJECTED"),
            true,
            ("delivery-review", false),
        ),
        (
            "REVERSAL_PARTIAL",
            Some("COMMITTED"),
            true,
            ("delivery-partial", false),
        ),
        (
            "DELIVERED",
            Some("REOPENED"),
            false,
            ("delivery-reopened", true),
        ),
        ("DELIVERED", None, false, ("unavailable", false)),
        (
            "DELIVERED",
            Some("COMMITTED"),
            false,
            ("unavailable", false),
        ),
    ] {
        assert_eq!(delivery_label(unit, operation, has_receipt), expected);
        for language in ["pl", "en"] {
            assert!(!Locale(language).text(expected.0).is_empty());
        }
    }
}
