# Oteryn Platform payment event integrity audit

## Verdict

`AUDIT_COMPLETE_WITH_FINDING`

The first bounded continuous-audit cycle after the merged exhaustive portal baseline found one new high-confidence, high-risk payment-security defect in a material post-baseline runtime addition.

No product code, workflow, migration, provider configuration, live environment or external repository was changed.

## Baseline reconciliation

The programme state incorrectly said that no coverage inventory existed. The authoritative baseline is the merged exhaustive current-main portal audit in PR #483 and its evidence directory. From its merge `cbbd7613cee13cf01931a0ba0f7ac089122132e0` to audited `main` `3ab77c072dce796b09004c54b649db009a75d524`, the repository advanced by 37 commits.

Material changed domains include:

- provider-neutral payment event core;
- native gameplay protocol producer/contracts;
- GameAuth and Game Gateway behavior;
- Cloudflare/HSTS operations;
- deep validation and governance infrastructure.

The payment event core was selected first because it is a newly merged financial/security boundary, its implementation ownership is released, and current open native-protocol work owns a different domain.

## Scope inspected

- payment availability and provider resolution;
- order and checkout creation;
- verified provider-event DTO and verifier contract;
- deterministic non-production provider;
- event processing and state machine;
- order/event/transition/reconciliation persistence;
- focused SQLite and MariaDB concurrency tests;
- ADR 0021 and payment operations documentation;
- related Issues, PRs, task ownership and previous audit evidence.

## Finding OPA-SEC-0001 — HIGH

**Title:** Verified provider events can change settlement state without amount or currency matching.

**Evidence state:** `PROVEN`  
**Confidence:** high  
**Remediation owner:** Issue #547  
**Related owners:** #321, #489; completed slice #470/#471

### Expected

Before a provider event can mark an order succeeded, refunded, disputed or charged back, provider-authenticated settlement facts must match the immutable order and the relevant checkout/provider object. At minimum the provider-neutral contract must carry enough verified amount, currency and object identity information for deterministic validation.

### Actual

`VerifiedProviderEvent` contains no currency or minor-unit settlement facts. The deterministic verifier does not extract them. `ProcessPaymentProviderEvent` resolves an order by public ID, checks only provider equality and then applies the state-machine decision. It does not compare amount/currency and does not bind `provider_object_reference` to a stored checkout attempt.

The focused success test demonstrates that a signed event containing no settlement amount or currency marks a EUR order for 9,999 minor units succeeded.

### Impact

Payments remain disabled and no public production webhook exists, so the audit does not claim an active production exploit. The defect is nevertheless a mandatory pre-activation blocker: a future provider adapter could authenticate an event while the core still applies financial truth without proving that the event's amount, currency and provider object belong to the referenced order.

### Required acceptance

1. Extend the verified-event contract with provider-authenticated settlement facts appropriate to each event type.
2. Match full success/capture amount and currency against the immutable order before success.
3. Validate cumulative/refund/dispute/chargeback amounts using explicit monotonic invariants.
4. Bind provider object/reference to the order's checkout attempt or reconcile the mismatch.
5. Add signed negative tests for wrong amount, wrong currency, wrong object reference and over-refund/cumulative mismatch.
6. Preserve replay, ordering, data minimization, concurrency and fail-closed production behavior.

## Deduplication

No duplicate root-cause Issue was found. The broad payment programme #321 and commerce audit owner #489 do not contain the exact verified-event settlement-integrity defect or its bounded remediation metadata, so Issue #547 was created using taxonomy version 1.2.

## Audit result

```yaml
audited_head: 3ab77c072dce796b09004c54b649db009a75d524
domain: payment-event-integrity
findings:
  critical: 0
  high: 1
  medium: 0
  low: 0
  informational: 0
new_issue: 547
product_repairs: 0
e2e: NOT_APPLICABLE_WITH_REASON
e2e_reason: documentation-only audit with no runtime mutation
production_operations: none
external_writes: none
```

## Next audit domain

After this audit PR reaches terminal closeout, the next cycle should reconcile current native-protocol work only after its active producer PR becomes terminal, or select the non-overlapping Cloudflare/HSTS operational delta if it remains unowned.