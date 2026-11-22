use oteryn_payments::{Amount, IntentState};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::staff_actions::AuditChange;

/// Typed projections for the commerce portion of the shared staff audit.
#[derive(Debug)]
pub enum CommerceAudit {
    CatalogPriceProposal {
        target: Uuid,
        base: String,
        proposal: String,
        before: i64,
        after: i64,
    },
    CatalogActivation {
        id: Uuid,
        before: Option<String>,
        after: String,
    },
    PaymentReview {
        id: Uuid,
        intent: Uuid,
        state: IntentState,
    },
    RefundApproval {
        id: Uuid,
        intent: Uuid,
        proposer: Uuid,
        amount: Amount,
        queued: bool,
    },
    RefundCancellation {
        id: Uuid,
        intent: Uuid,
        proposer: Uuid,
    },
    RefundRequest {
        id: Uuid,
        intent: Uuid,
        amount: Amount,
    },
    ValueDecision {
        id: Uuid,
        account: Uuid,
        proposer: Uuid,
        change: ValueDecision,
    },
}

#[derive(Debug)]
pub enum ValueDecision {
    WalletApproved {
        coins: i64,
        ledger: Uuid,
    },
    WalletCancelled {
        coins: i64,
    },
    PremiumApproved {
        days: u16,
        entitlement: Uuid,
        revision: i64,
    },
    PremiumCancelled {
        days: u16,
    },
}

impl From<CommerceAudit> for AuditChange {
    fn from(change: CommerceAudit) -> Self {
        Self::Commerce(change)
    }
}

impl CommerceAudit {
    pub(crate) fn request_id(&self) -> Uuid {
        match self {
            Self::PaymentReview { id, .. }
            | Self::RefundApproval { id, .. }
            | Self::RefundCancellation { id, .. }
            | Self::RefundRequest { id, .. }
            | Self::ValueDecision { id, .. } => *id,
            _ => Uuid::now_v7(),
        }
    }

    pub(crate) fn project(self) -> (&'static str, Uuid, Value, Value) {
        match self {
            Self::CatalogPriceProposal {
                target,
                base,
                proposal,
                before,
                after,
            } => (
                "store.catalog.price.staged",
                target,
                json!({"revision": base, "unit_price": before}),
                json!({"revision": proposal, "unit_price": after}),
            ),
            Self::CatalogActivation { id, before, after } => (
                "store.catalog.activated",
                id,
                json!({"revision": before}),
                json!({"revision": after}),
            ),
            Self::PaymentReview { id, intent, state } => (
                "commerce.payment.reviewed",
                intent,
                json!({"payment_state": state}),
                json!({"payment_state": state, "review": id}),
            ),
            Self::RefundApproval {
                id,
                intent,
                proposer,
                amount,
                queued,
            } => (
                "commerce.approved",
                intent,
                json!({"pending": true}),
                json!({"pending": false, "operation": id, "requested_by": proposer, "command": "refund",
                       "result": {"intent": intent, "amount": amount.minor_units, "currency": amount.currency, "queued": queued}}),
            ),
            Self::RefundCancellation {
                id,
                intent,
                proposer,
            } => (
                "commerce.cancelled",
                intent,
                json!({"pending": true}),
                json!({"pending": false, "operation": id, "requested_by": proposer, "command": "refund", "status": "cancelled"}),
            ),
            Self::RefundRequest { id, intent, amount } => (
                "commerce.requested",
                intent,
                json!({"pending": false}),
                json!({"pending": true, "operation": id, "command": "refund",
                       "amount": amount.minor_units, "currency": amount.currency}),
            ),
            Self::ValueDecision {
                id,
                account,
                proposer,
                change,
            } => {
                let (action, command, result) = match change {
                    ValueDecision::WalletApproved { coins, ledger } => (
                        "commerce.approved",
                        "adjust",
                        json!({"coins": coins, "operation": ledger}),
                    ),
                    ValueDecision::WalletCancelled { coins } => (
                        "commerce.cancelled",
                        "adjust",
                        json!({"coins": coins, "status": "cancelled"}),
                    ),
                    ValueDecision::PremiumApproved {
                        days,
                        entitlement,
                        revision,
                    } => (
                        "commerce.approved",
                        "grant",
                        json!({"days": days, "entitlement": entitlement, "revision": revision}),
                    ),
                    ValueDecision::PremiumCancelled { days } => (
                        "commerce.cancelled",
                        "grant",
                        json!({"days": days, "status": "cancelled"}),
                    ),
                };
                (
                    action,
                    account,
                    json!({"pending": true}),
                    json!({"pending": false, "operation": id, "requested_by": proposer,
                           "command": command, "result": result}),
                )
            }
        }
    }
}
