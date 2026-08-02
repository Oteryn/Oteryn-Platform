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
| Replay | Timestamp tolerance plus unique provider event identity; exact duplicate is a no-op, conflicting duplicate fails. |
| Out-of-order event | Explicit state machine refuses regressions and creates reconciliation work. |
| Duplicate checkout | Unique idempotency key and exact replay matching. |
| Ambiguous checkout response | Persist attempt as ambiguous and open reconciliation; never report success. |
| Browser return grants value | No browser-return consumer exists in this core; only verified provider events may change settlement state. |
| Raw payload or personal-data leakage | Store only digest and bounded sanitized fields; no raw body. |
| Test adapter used in production | Adapter runtime refusal and production configuration violation. |
| Direct wallet/entitlement mutation | No dependency on Wallet or Products/Entitlements in this core. |
| Concurrent transition race | Database transaction, row lock, unique event key and monotonic order version. |
| Operator/admin abuse | No privileged mutation UI is introduced; future reconciliation UI requires exact permission, confirmed MFA and audit. |

## Consequences

- Issue #470 may reach `producer_complete` after database, focused security, concurrency, audit and exact-head validation.
- Issue #321 remains incomplete until a real provider decision, sandbox adapter, signed public ingress, customer frontend and provider-specific operational evidence exist.
- Issue #322 remains the owner of product, entitlement, coin-delivery and service-history behavior.
- Production and customer charging remain disabled.
- The test adapter is not evidence of provider sandbox or production correctness.

## Rollback

The schema is additive and reversible while no downstream consumer has been activated. Disable payments, stop event ingestion, preserve evidence as required, roll back the migration, and remove provider bindings. No wallet, Canary or entitlement rollback is needed because this core does not deliver value.
