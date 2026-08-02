---
task_id: OTERYN-20260802-payment-event-core
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - payment ADRs and provider boundaries
  - existing wallet idempotency and transaction patterns
  - production configuration verifier conventions
  - open payment tasks and pull requests
optional_reads:
  - docs/architecture/adr/**
  - docs/operations/**
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

- [ ] Additive reversible payment persistence uses integer minor units, immutable public IDs, identity ownership, monotonic versions and unique idempotency keys.
- [ ] Provider-neutral checkout and verified-event contracts do not expose provider secrets or accept browser return state as payment truth.
- [ ] Deterministic HMAC test adapter verifies bounded raw input before parsing and cannot be enabled in production.
- [ ] Duplicate, replayed, conflicting and out-of-order events are deterministic and cannot regress terminal order truth.
- [ ] Event inbox and transition/reconciliation history are append-oriented and exclude unnecessary raw payloads and secrets.
- [ ] Production configuration fails closed when payments are enabled without an approved real provider profile.
- [ ] Focused unit/feature/security tests and a real signed-input-to-persisted-state E2E path pass.
- [ ] Independent audit has no open material findings and exact-final-head CI passes.
- [ ] Issue #321 remains open and explicitly partial pending provider/UI/sandbox/entitlement consumers.

## Ownership

```yaml
owned_paths:
  - app/Payments/**
  - app/Operations/ProductionConfigurationVerifier.php
  - app/Providers/AppServiceProvider.php
  - config/payments.php
  - database/migrations/*payment*
  - tests/Unit/Payments/**
  - tests/Feature/Payments/**
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/adr/*payment*
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/active/OTERYN-20260802-payment-event-core.md
  - docs/agents/tasks/archive/OTERYN-20260802-payment-event-core.md
  - docs/agents/evidence/OTERYN-20260802-payment-event-core/**
modules:
  - Payments
dependencies:
  - issue #321
  - issue #470
  - programme #451
blockers:
  - real provider selection is excluded from this producer slice
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-02T14:45:00+02:00
head: UNKNOWN
branch: feat/OTERYN-20260802-payment-event-core
pr: none
status: investigating
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
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/adr/*payment*
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/active/OTERYN-20260802-payment-event-core.md
  - docs/agents/tasks/archive/OTERYN-20260802-payment-event-core.md
  - docs/agents/evidence/OTERYN-20260802-payment-event-core/**
proven:
  - Issue #321 authorizes repository design and deterministic test-adapter work but not real charges or production webhooks.
  - Issue #470 is the bounded backend/security producer slice and does not claim the user-facing payment feature complete.
  - Existing Wallet persistence is separate from payment-provider settlement and exposes an idempotent locked mutator.
  - The production configuration verifier already applies fail-closed checks to feature-flagged modules.
derived:
  - Payment settlement truth can be built independently from wallet and entitlement delivery.
  - A deterministic signed-event test adapter can prove security and ordering without selecting a production provider.
unknown:
  - Exact existing payment ADR filename and supersession state.
  - Smallest provider-binding pattern consistent with current AppServiceProvider conventions.
  - Exact focused test command set after source discovery.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - The Bazaar wallet is a payment provider settlement ledger.
  - A browser return may establish successful payment.
  - A production provider can be invented or selected inside this implementation slice.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260802-payment-event-core.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation discovery started
blockers:
  - none for the provider-neutral producer slice
next_action: Inspect existing transaction, model, provider-binding, migration, test and production-verifier patterns, then implement the smallest signed-event-to-persisted-state core.
```

## Notes

No real provider, payment credential, charge, customer financial data, public webhook ingress, wallet delivery, entitlement delivery, production activation, deployment, Canary mutation or external-repository write is authorized by this task.
