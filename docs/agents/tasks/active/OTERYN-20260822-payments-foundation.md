---
task_id: OTERYN-20260822-payments-foundation
status: implementing
agent: ChatGPT autonomous payments foundation owner
project_lane: oteryn-platform-core
task_kind: implementation
product_issue: 321
parent_issue: 278
risk: high
feature_scope:
  type: full_stack
  user_facing: true
  backend_required: true
  frontend_required: true
  integration_required: true
  e2e_required: true
completion_claim: partial_producer
created: 2026-08-22T19:25:00+02:00
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
search_first:
  - Issue #321 and parent #278
  - existing Payments implementation and tests
  - overlapping active tasks and open PRs
---

# OTERYN-20260822-payments-foundation

## Goal

Complete the maximum mergeable provider-neutral, non-production Payments foundation still missing from Issue #321 by extending the existing payment event core with safe authenticated customer history/checkout presentation, test-adapter return/ingress behavior, and exact-permission confirmed-MFA operator reconciliation visibility/recovery where the existing domain permits it. Real provider selection, real charging, production webhook activation, provider credentials, Wallet mutation and entitlement delivery remain out of scope and fail closed.

## Acceptance criteria

- [ ] Existing provider-neutral payment core is reused rather than duplicated.
- [ ] Customer payment history and checkout/return presentation are owner-scoped, authenticated, EN/PL and never treat browser return as settlement proof.
- [ ] Deterministic test-adapter ingress is non-production only, verifies authenticated provider input before processing and cannot become a production operator.
- [ ] Reconciliation administration requires an exact permission plus confirmed MFA and records bounded audit evidence for any recovery action delivered by this slice.
- [ ] Payment/refund/dispute/chargeback state remains separate from Wallet and ProductsEntitlements; no value delivery is introduced.
- [ ] Focused security, authorization, idempotency/ordering, migration and integration tests pass.
- [ ] Real zero-retry desktop/tablet/mobile browser evidence covers the delivered test-adapter path, or the task records a concrete blocker before readiness.
- [ ] Exact-head CI and full-diff self-review pass before merge.
- [ ] Issue #321 remains open with real-provider, sandbox/legal/tax/privacy and production-activation gates explicit.

## Ownership

```yaml
owned_paths:
  - app/Admin/AdminPermission.php
  - app/Payments/**
  - config/payments.php
  - database/migrations/2026_08_22_173500_add_payment_reconciliation_operator_foundation.php
  - routes/api.php
  - routes/modules/payments.php
  - resources/views/identity/layout.blade.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/payments/**
  - resources/views/payments/**
  - lang/en/payments.php
  - lang/pl/payments.php
  - tests/Feature/Payments/**
  - scripts/acceptance/tests/payment-foundation-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/payments.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/playwright.config.mjs
  - docs/testing/PAYMENTS_FOUNDATION_E2E_EVIDENCE.md
  - docs/agents/tasks/active/OTERYN-20260822-payments-foundation.md
modules:
  - Payments
  - Admin
  - Audit
dependencies:
  - Issue #321
  - Issue #278
  - ADR 0021
blockers:
  - none for provider-neutral non-production repository work
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-22T20:14:42+02:00
head: d6eb3161f102a06eb476d79b9a0a1f95c457e72b
branch: agent/payments-foundation-20260822
pr: 1228
status: implementing
phase: focused_validation
session_id: chatgpt-20260822T1925+0200
session_role: implementer
execution_mode: remote-terminal-plus-github
execution_reason: isolated checkout and disposable repository CI image provide deterministic PHP validation while GitHub remains authoritative for PR/CI state
project_lane: oteryn-platform-core
context_routes:
  - payments
  - security
  - database
  - admin-rbac
  - testing
context_pressure: medium
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: cohesive Payments vertical slice reusing the existing core; browser acceptance remains the next bounded phase after focused tests
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
owned_paths:
  - app/Admin/AdminPermission.php
  - app/Payments/**
  - config/payments.php
  - database/migrations/2026_08_22_173500_add_payment_reconciliation_operator_foundation.php
  - routes/api.php
  - routes/modules/payments.php
  - resources/views/identity/layout.blade.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/payments/**
  - resources/views/payments/**
  - lang/en/payments.php
  - lang/pl/payments.php
  - tests/Feature/Payments/**
  - scripts/acceptance/tests/payment-foundation-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/payments.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/playwright.config.mjs
  - docs/testing/PAYMENTS_FOUNDATION_E2E_EVIDENCE.md
  - docs/agents/tasks/active/OTERYN-20260822-payments-foundation.md
proven:
  - Issue #321 remains open and authorizes repository/test-adapter work only, not real charges or production webhooks.
  - ADR 0021 and merged payment core are reused for provider-neutral orders, signed test events, replay/idempotency, reconciliation and refund-integrity evidence.
  - No overlapping payment task or open payment PR existed on protected main at invocation start.
  - Customer history/return, deterministic non-production test ingress, and exact-permission confirmed-MFA reconciliation surfaces are implemented in the isolated checkout.
  - All changed/new PHP files pass PHP 8.5 syntax validation in the repository CI image.
derived:
  - The delivered operator resolution is intentionally bounded to reviewing deterministic test evidence without changing payment, Wallet or entitlement state.
unknown:
  - Focused Laravel test, migration rollback, static-analysis and real browser acceptance results are not yet proven.
conflicts: []
first_failure:
  marker: php-syntax-resolve-payment-reconciliation
  evidence: initial PHP lint found a missing closing brace after idempotency validation; repaired before checkpoint
rejected_hypotheses: []
changed_paths:
  - app/Admin/AdminPermission.php
  - app/Payments/**
  - config/payments.php
  - database/migrations/2026_08_22_173500_add_payment_reconciliation_operator_foundation.php
  - routes/api.php
  - routes/modules/payments.php
  - resources/views/identity/layout.blade.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/payments/**
  - resources/views/payments/**
  - lang/en/payments.php
  - lang/pl/payments.php
  - tests/Feature/Payments/PaymentFoundationSurfaceTest.php
validation:
  - command: PHP 8.5 syntax validation for every changed/new PHP file
    result: PASS
    evidence: docker image oteryn-portal-e2e-1219-portal-e2e:latest reported no syntax errors after repair
blockers:
  - none
next_action: Install locked dependencies in the disposable PHP 8.5 CI image and run focused Payments feature/security tests plus migration rollback evidence.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260822T1925+0200
  session_started_at: 2026-08-22T19:25:00+02:00
  checkpointed_at: 2026-08-22T20:14:42+02:00
  last_progress_at: 2026-08-22T20:14:42+02:00
  phase: focused_validation
  exact_head: d6eb3161f102a06eb476d79b9a0a1f95c457e72b
  pull_request: 1228
  active_operation: locked dependency bootstrap and focused Payments validation
  external_run_ids: []
  operation_started_at: 2026-08-22T20:14:42+02:00
  wait_deadline_at: 2026-08-22T20:44:42+02:00
  check_generation: local-focused-1
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: branch agent/payments-foundation-20260822 and draft PR #1228 retain the declared payment ownership without overlap
  next_action: Install locked dependencies in the disposable PHP 8.5 CI image and run focused Payments feature/security tests plus migration rollback evidence.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The task must not close Issue #321. A real provider decision, provider sandbox/legal/tax/privacy work, production secrets, production webhook activation and live charging remain separate gates.