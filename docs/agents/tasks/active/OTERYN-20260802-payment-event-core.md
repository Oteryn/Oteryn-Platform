---
task_id: OTERYN-20260802-payment-event-core
status: validating
agent: payment-event-core continuation owner
project_lane: payments
track: backend-security
created: 2026-08-02T14:45:00+02:00
updated: 2026-08-05T09:38:00+02:00
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

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T09:38:00+02:00
head: 72ed8af55c2dc48ebdfd83cc5861aa2e45f81691
branch: feat/OTERYN-20260802-payment-event-core
pr: 471
status: validating
context_routes:
  - payments
  - security
  - database
  - testing
owned_paths:
  - app/Payments/**
  - app/Operations/ProductionConfigurationVerifier.php
  - app/Providers/AppServiceProvider.php
  - config/payments.php
  - database/migrations/*payment*
  - tests/Unit/Payments/**
  - tests/Feature/Payments/**
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
  - docs/architecture/adr/*payment*
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/active/OTERYN-20260802-payment-event-core.md
  - docs/agents/tasks/archive/OTERYN-20260802-payment-event-core.md
  - docs/agents/evidence/OTERYN-20260802-payment-event-core/**
proven:
  - PAY-CORE-001 is remediated by one guard before database work or provider resolution in all three public payment actions.
  - The two PHPStan mixed-argument failures are removed by narrowing the pcntl wait status to int without suppressions.
  - The deterministic test provider refuses production execution and production configuration remains fail-closed.
  - The branch is synchronized with main commit 11541f2cce94ed6026d4d72a0a4013e64cafc380.
  - Independent review found zero material findings and zero unresolved review threads.
derived:
  - The provider-neutral producer slice is ready for terminal exact-head validation and protected merge once the workflow matrix passes.
unknown:
  - Terminal conclusion of the exact-final-head workflow matrix.
conflicts: []
first_failure:
  marker: Agent Governance checkpoint contract
  evidence: The prior shortened checkpoint used unsupported nested keys; this revision restores contract version 1 and every required field.
rejected_hypotheses:
  - The temporary payment bootstrap workflow is required after generated source exists.
  - A disabled payment action may safely resolve a provider before checking the feature flag.
  - Issue 321 can close with this producer-only slice.
changed_paths:
  - app/Payments/**
  - app/Operations/ProductionConfigurationVerifier.php
  - app/Providers/AppServiceProvider.php
  - config/payments.php
  - database/migrations/2026_08_02_124700_create_payment_event_core_tables.php
  - tests/Feature/Payments/**
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
  - tests/Unit/Payments/**
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/evidence/OTERYN-20260802-payment-event-core/README.md
  - docs/agents/tasks/active/OTERYN-20260802-payment-event-core.md
validation:
  - command: GitHub Actions Agent Governance on 72ed8af55c2dc48ebdfd83cc5861aa2e45f81691
    result: FAIL
    evidence: The validator rejected unsupported nested checkpoint keys; production code was not implicated and this checkpoint now conforms to contract version 1.
  - command: Independent payment security and persistence review
    result: PASS
    evidence: Zero material findings, zero submitted reviews and zero unresolved inline review threads.
blockers:
  - Exact-final-head GitHub Actions have not yet reached a terminal green result.
next_action: Run the exact-head workflow matrix on the corrected checkpoint, then archive the task, mark PR 471 ready and perform the protected squash merge while leaving Issue 321 open.
```
