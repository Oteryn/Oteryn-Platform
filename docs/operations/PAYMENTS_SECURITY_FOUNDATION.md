# Payments security foundation operations

## Current classification

`REPOSITORY_PROVEN_ONLY` after Issue #470 completes.

This runbook covers the provider-neutral event core and deterministic test adapter. It does not authorize or describe live payment activation.

## Hard production gates

Payments must remain disabled unless all conditions are satisfied:

- a real provider is selected through a separate product/legal/security decision;
- the configured provider is not `test`;
- provider adapter and webhook verifier classes are explicitly configured;
- provider verification is recorded for the exact release and sandbox profile;
- secrets are injected outside Git and have an owner/rotation procedure;
- the public webhook path, size limit, rate limit and edge policy are separately reviewed;
- provider sandbox checkout, signed webhook, replay, delayed/out-of-order, repeated/concurrent refund and dispute evidence passes;
- customer checkout/history UI and Products/Entitlements integration have independent audit and E2E;
- production activation receives separate authorization.

`production:verify-configuration` fails closed when payments are enabled without these prerequisites or when the deterministic test adapter is selected.

## Deterministic test adapter

The adapter:

- is provider name `test`;
- uses synthetic HMAC headers and a non-production secret;
- verifies payload size, signature timestamp tolerance and HMAC before JSON parsing;
- requires signed ISO currency and positive integer minor-unit amount facts;
- generates deterministic synthetic checkout references;
- refuses to execute when `APP_ENV=production`;
- must never receive production credentials or customer data;
- is not a substitute for a provider sandbox.

## Event handling

The system boundary is:

```text
raw bytes + headers
→ signature/timestamp verification
→ bounded parsing of authenticated identifiers and settlement facts
→ append-oriented provider-event inbox
→ locked amount/currency/object integrity matching
→ locked refund-value integrity accounting
→ locked state-machine transition
→ transition or reconciliation record
→ observable persisted order state
```

Operational interpretation:

- `processed` means the verified event was applied or was an exact state no-op;
- `reconciliation` means no unsafe state transition occurred and operator review is required;
- `settlement_integrity_mismatch` means signed currency or relevant minor-unit amount did not match immutable order semantics;
- `refund_integrity_mismatch` means durable refund history is missing/inconsistent or a new incremental partial refund would reach or exceed the immutable order total;
- `provider_object_mismatch` means a supplied provider object reference was unknown, belonged to another order or belonged to another provider;
- `event_id_conflict` is a security/integrity failure and must not be retried with changed content;
- `ambiguous_checkout_creation` requires provider-side lookup before retrying or creating another checkout;
- browser return state is never settlement proof.

For success, full refund, dispute and chargeback, the authenticated amount must equal the immutable order total and the currency must match exactly. For `payment.partially_refunded`, authenticated `amount_minor` is an incremental refund delta: it must use the same currency, be positive, be below the order total itself, and keep the locked cumulative refunded total strictly below the immutable order amount. Each distinct accepted partial-refund event creates a versioned transition containing its verified delta and resulting cumulative total. `payment.refunded.amount_minor` is cumulative terminal refund truth and must equal the immutable order total; its accepted transition records that terminal total. A `partially_refunded` state without durable refund-value history is reconciliation work rather than a basis for guessing prior settlement value.

A present provider object reference must bind to a checkout attempt for that same order and provider. Any mismatch is reconciliation work, not a state transition.

## Data handling

Do not log or persist:

- raw webhook bodies;
- card or bank data;
- provider secrets or bearer credentials;
- complete customer addresses or unnecessary personal data;
- signatures;
- unbounded provider errors.

Permitted durable evidence is limited to public/bounded identifiers, event type, payload digest, signature timestamp, state, monotonic version, sanitized error code, bounded reconciliation metadata and authenticated refund minor-unit facts required to reconstruct refund truth. Accepted refund transitions may persist `verified_refund_amount_minor` and `refunded_total_minor`; these are bounded financial values, not raw provider payloads or personal data. Other authenticated amount/currency values remain in memory for integrity matching unless a bounded mismatch is recorded for reconciliation.

## Alerts required before activation

A future real-provider rollout must alert on:

- signature verification failures;
- repeated event-ID conflicts;
- settlement-integrity mismatches;
- refund-integrity mismatches;
- provider-object mismatches;
- reconciliation entries older than the agreed threshold;
- ambiguous checkout attempts;
- event processing failures;
- provider/order state drift;
- refunds, disputes and chargebacks;
- disabled or incomplete payment production configuration.

## Recovery and rollback

- Disable payment ingress and checkout creation before repair.
- Do not edit order state, provider-event rows or refund-value history directly.
- Reconcile using a future audited operator action with exact permission and confirmed MFA.
- Preserve append-oriented records.
- Application behavior may be rolled back while payments remain disabled, but populated authenticated refund-value columns are forward-only financial evidence and must not be dropped.
- Wallet and entitlement state require no action for this producer slice because it performs no delivery.
