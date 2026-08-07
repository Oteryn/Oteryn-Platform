# ADR 0021: Provider-neutral payment security core

- Status: Accepted for repository producer implementation
- Date: 2026-08-02
- Parent: Issues #321 and #470
- Production activation: Not authorized

## Context

Oteryn has a Platform-owned Oteryn Coins wallet for gameplay marketplace accounting, but it has no payment-provider settlement authority. Browser redirects are untrusted presentation events, a future provider is an external trust boundary, and duplicate, delayed or out-of-order provider events can otherwise create inconsistent financial truth.

The production-completion programme authorizes a provider-neutral repository foundation and deterministic non-production testing. It does not authorize selecting a provider, accepting real charges, exposing a production webhook, storing provider credentials, granting coins or entitlements, or activating commerce.

## Decision

Introduce a Platform-owned payment event core with these boundaries:

1. Payment orders store integer minor-unit totals in the initially approved currencies PLN and EUR.
2. Provider checkout creation is accessed through `PaymentProviderGateway`.
3. Raw webhook input is authenticated by `PaymentWebhookVerifier` before JSON parsing or domain processing.
4. The provider-event inbox is append-oriented and uniquely identifies `(provider, provider_event_id)`.
5. Raw provider payloads, card data, reusable credentials and unnecessary personal data are not stored. The inbox stores a SHA-256 digest, bounded identifiers, timestamps and sanitized metadata.
6. Payment-order transitions use row locking, monotonic versions and an append-oriented transition history.
7. Duplicate events are idempotent only when their bounded verified identity and payload digest match. Conflicts fail closed.
8. Unsupported or out-of-order events do not regress order truth; they create an explicit reconciliation entry.
9. Checkout ambiguity is persisted as a reconciliation case rather than treated as success.
10. Payment settlement remains separate from Wallet and future Products/Entitlements. This core performs no value delivery.
11. A deterministic HMAC adapter exists only for tests and non-production validation. It refuses production execution.
12. Production configuration rejects enabled payments until an approved real provider, adapter, verifier and direct verification flag are configured outside Git.
13. A verified provider event must carry provider-authenticated ISO currency and a positive integer minor-unit amount. Identifiers alone cannot authorize a settlement transition.
14. Before success, full refund, dispute or chargeback, the verified currency and amount must exactly match the immutable order. A partial-refund event must use the same currency and a positive amount smaller than the order total. Mismatches create reconciliation and never mutate order truth.
15. When a verified provider object reference is present, it must resolve to a checkout attempt owned by the same order and provider. Unknown or cross-order references create reconciliation.
16. `payment.partially_refunded.amount_minor` is a provider-authenticated incremental refund delta. Every distinct accepted partial-refund event is recorded as its own versioned payment-order transition, including the verified delta and resulting cumulative refunded minor-unit total.
17. The cumulative refunded total is calculated while the payment-order row is locked. A partial refund is accepted only when the prior durable refund history is internally consistent and the resulting total remains strictly below the immutable order total. A partial event that would reach or exceed the total creates reconciliation instead of financial truth.
18. `payment.refunded.amount_minor` is cumulative terminal refund truth and must equal the immutable order total. Its accepted transition records the terminal cumulative total without erasing earlier partial-refund history. A legacy `partially_refunded` state without durable refund-value history fails closed into reconciliation.
19. Refund-value columns on payment-order transitions are forward-only financial evidence. Runtime rollback may stop ingestion or revert application behavior, but a schema rollback must not drop populated authenticated refund settlement history.

## Threat model

### Protected assets

- payment-order ownership and amount/currency integrity;
- provider-event authenticity and ordering;
- idempotency keys and checkout references;
- append-oriented financial/reconciliation history;
- future customer value delivery boundaries;
- provider credentials and personal data.

### Untrusted inputs

- browser return/query state;
- raw webhook bytes and headers;
- provider event identifiers and object references;
- event order and timing;
- repeated checkout requests;
- configuration values supplied at deployment.

### Material threats and controls

| Threat | Control |
|---|---|
| Forged webhook | Verify timestamp and HMAC before parsing in the deterministic adapter; real adapters must implement equivalent provider-authentic verification. |
| Signed event with wrong amount or currency | Carry authenticated settlement facts in the verified-event contract and compare them with immutable order semantics before transition; reconcile mismatches. |
| Repeated or concurrent partial refunds | Treat partial amounts as incremental deltas; serialize on the locked order, persist every accepted delta plus cumulative total, and reconcile any event that would reach/exceed the immutable order total. |
| Missing historic partial-refund value | A `partially_refunded` order without durable refund-value history cannot accept another refund transition; create reconciliation instead of guessing prior value. |
| Provider object rebound to another order | Resolve any supplied provider object reference to the same provider and payment order before transition; reconcile unknown or cross-order references. |
| Replay | Timestamp tolerance plus unique provider event identity; exact duplicate is a no-op, conflicting duplicate fails. |
| Out-of-order event | Explicit state machine refuses regressions and creates reconciliation work. |
| Duplicate checkout | Unique idempotency key and exact replay matching. |
| Ambiguous checkout response | Persist attempt as ambiguous and open reconciliation; never report success. |
| Browser return grants value | No browser-return consumer exists in this core; only verified provider events may change settlement state. |
| Raw payload or personal-data leakage | Store only digest and bounded sanitized fields; no raw body. |
| Test adapter used in production | Adapter runtime refusal and production configuration violation. |
| Direct wallet/entitlement mutation | No dependency on Wallet or Products/Entitlements in this core. |
| Concurrent transition race | Database transaction, payment-order row lock, unique event key and monotonic order version; refund accumulation occurs inside that same serialized boundary. |
| Operator/admin abuse | No privileged mutation UI is introduced; future reconciliation UI requires exact permission, confirmed MFA and audit. |

## Consequences

- Issue #470 may reach `producer_complete` after database, focused security, concurrency, audit and exact-head validation.
- Issue #321 remains incomplete until a real provider decision, sandbox adapter, signed public ingress, customer frontend and provider-specific operational evidence exist.
- Issue #322 remains the owner of product, entitlement, coin-delivery and service-history behavior.
- Production and customer charging remain disabled.
- The test adapter is not evidence of provider sandbox or production correctness.
- Accepted partial refunds now create same-state versioned transitions while the order remains `partially_refunded`; this is intentional financial history, not duplicate-state churn.

## Rollback

Disable payments and provider ingress before application rollback. Preserve provider events, reconciliation records, payment-order transitions and authenticated refund-value columns. Application code can be reverted while payments remain disabled, but populated refund settlement evidence is forward-only and must not be dropped by a migration rollback. No wallet, Canary or entitlement rollback is needed because this core does not deliver value.
