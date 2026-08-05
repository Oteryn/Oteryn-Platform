---
task_id: OTERYN-20260802-payment-event-core
status: validating
agent: payment-event-core continuation owner
project_lane: payments
track: backend-security
created: 2026-08-02T14:45:00+02:00
updated: 2026-08-05T09:35:00+02:00
product_issue: 470
parent_issue: 321
product_pr: 471
product_head: a128c184c5b581c20b7cda1e2e6980c63bf1117a
risk: high
execution_mode: github-only
---

# OTERYN-20260802 payment event core

## Goal

Deliver the first provider-neutral backend/security producer slice of Issue #321: deterministic signed-event verification, idempotent order/event persistence, explicit settlement transitions and a fail-closed production configuration boundary without real provider activation.

## Delivery classification

```yaml
feature_scope:
  type: backend_only
  user_facing: false
  backend_required: true
  frontend_required: false
  integration_required: true
  e2e_required: true
implementation_status_target: producer_complete
complete_user_facing_feature: false
missing_consumers:
  - selected real provider adapter and sandbox proof
  - authenticated checkout and payment-history frontend
  - public signed webhook ingress and provider rate-limit profile
  - products and entitlement delivery under Issue #322
```

## Acceptance criteria

- [x] Additive reversible payment persistence uses integer minor units, immutable public IDs, identity ownership, monotonic versions and unique idempotency keys.
- [x] Provider-neutral checkout and verified-event contracts do not expose provider secrets or accept browser return state as payment truth.
- [x] Deterministic HMAC test adapter verifies bounded raw input before parsing and cannot be enabled in production.
- [x] Duplicate, replayed, conflicting and out-of-order events are deterministic and cannot regress terminal order truth.
- [x] Event inbox and transition/reconciliation history are append-oriented and exclude unnecessary raw payloads and secrets.
- [x] Production configuration fails closed when payments are enabled without an approved real provider profile.
- [x] `payments.enabled=false` rejects order creation, checkout creation and provider-event processing before provider resolution or persistence.
- [x] Focused unit, feature, security and MariaDB concurrency coverage is present.
- [x] Independent audit has no open material findings or review threads.
- [ ] Exact-final-head CI passes and the protected squash merge completes.
- [x] Issue #321 remains open and explicitly partial pending provider/UI/sandbox/entitlement consumers.

## Delivered scope

- provider-neutral payment order, attempt, provider-event, transition and reconciliation persistence;
- deterministic non-production HMAC checkout/webhook adapter;
- idempotency, replay, conflict, ordering and reconciliation rules;
- fail-closed production configuration verifier;
- shared runtime availability guard covering all three public payment actions;
- deferred provider resolution so the disabled path cannot instantiate an adapter;
- SQLite feature/security coverage and isolated MariaDB duplicate-event concurrency proof;
- ADR, operations guide and evidence record.

## PAY-CORE-001 remediation

The audit finding is remediated by `PaymentAvailability::ensureEnabled()` at the beginning of:

- `CreatePaymentOrder::execute()`;
- `CreatePaymentCheckout::execute()`;
- `ProcessPaymentProviderEvent::execute()`.

`PaymentProviderResolver` defers gateway/verifier construction until after the guard. Default-disabled tests use `provider=null` and prove all actions return `payments_disabled` with zero order, attempt, transition, provider-event or reconciliation persistence.

## PHPStan repair

`PaymentEventConcurrencyMariaDbTest.php` now narrows the by-reference `pcntl_waitpid()` status to `int` before calling `pcntl_wifexited()` and `pcntl_wexitstatus()`. This removes both reported mixed-argument errors without suppressions.

## Safety boundary

No real provider, payment credential, charge, customer financial data, public webhook ingress, wallet delivery, entitlement delivery, production activation, deployment, Canary mutation or external-repository write is authorized or performed.

## Validation checkpoint

```yaml
checkpoint_version: 2
head: a128c184c5b581c20b7cda1e2e6980c63bf1117a
branch: feat/OTERYN-20260802-payment-event-core
pr: 471
status: validating
base_synced_to: 11541f2cce94ed6026d4d72a0a4013e64cafc380
observed_passes:
  - Agent Governance
  - Portal Exhaustive Audit
pending:
  - terminal exact-head CI matrix
  - ready-for-review transition
  - protected squash merge
  - archive move
material_findings: 0
unresolved_review_threads: 0
next_action: Complete exact-head validation, archive this record, mark PR ready, squash-merge PR #471 and close Issue #470 while leaving Issue #321 open.
```
