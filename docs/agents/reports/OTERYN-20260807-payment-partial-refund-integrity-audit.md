# OTERYN-20260807 payment partial-refund integrity audit

## Verdict

`PROVEN_MATERIAL_FINDING`

Finding: `OPA-SEC-0002` / Issue #797 — HIGH / P1.

## Audited boundary

Current main: `f7abc6096264aee890e0ab475087adeba7265397`.

Inspected:
- closed Issue #547 and merged repair PR #595;
- `VerifiedProviderEvent` and deterministic verifier;
- `ProcessPaymentProviderEvent` settlement/object checks;
- `PaymentOrderStateMachine`;
- `PaymentOrder` and `PaymentOrderTransition` persistence;
- focused payment tests;
- ADR 0021 and payment security runbook;
- parent payment programme Issue #321.

## Confirmed repaired boundary

OPA-SEC-0001 is no longer present in its original form. Verified events carry authenticated currency and positive minor-unit amount, relevant settlement-changing events compare those facts with the immutable order, and supplied provider-object references are bound to the same order/provider or reconciled.

The prior programme conflict claiming that the verified-event contract cannot carry amount/currency is therefore stale and must not remain as current truth.

## New finding

Repeated partial refunds are not modeled as financial value changes.

`PaymentOrderStateMachine` maps `payment.partially_refunded` to the single state `partially_refunded`. Once that state is reached, another distinct partial-refund event returns `NOOP / duplicate_state`.

Before that state-machine decision, `settlementMismatch()` checks only the individual event: same currency and `0 < amount_minor < order.amount_minor`. No cumulative refunded amount exists on the order or transition, and successful event metadata intentionally does not persist authenticated settlement amount/currency. A second legitimate partial refund can therefore be accepted as processed while its amount disappears from reconstructable Platform financial truth.

This also leaves no aggregate value against which sequential or concurrent partial refunds can be checked for over-refund.

## Contract conflict

ADR 0021 protects append-oriented financial/reconciliation history but defines only that a partial-refund amount is positive and below the order total. It does not define incremental-versus-cumulative refund semantics. Parent Issue #321 explicitly requires partial-refund lifecycle, refund records, immutable financial/audit records and concurrency evidence.

## Regression gap

Existing payment tests cover signed fact requirements, settlement mismatches, refund/dispute/chargeback mismatch, object rebinding, replay conflict and out-of-order state. No test proves two distinct sequential partial refunds, aggregate over-refund prevention or concurrent distinct partial refunds.

## Impact

A future real provider can legitimately issue multiple partial refunds. Under the current core, Platform refund truth may diverge from the provider after the second event, affecting reconciliation, customer history and later refund decisions. Production is still disabled, so this is a pre-activation financial-integrity blocker rather than evidence of an active production exploit.

## Duplicate result

No actionable duplicate was found for repeated/cumulative partial-refund accounting. Issue #547 fixed per-event amount/currency/object matching; Issue #321 is the broader programme owner.

## Delivery boundary

Audit-only. No payment runtime, migration, workflow, production, provider sandbox or external-repository mutation is included.

Runtime/product E2E: `NOT_APPLICABLE` for this documentation/evidence package.
