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
  - app/Payments/**
  - app/Http/Controllers/Payments/**
  - routes/modules/payments.php
  - resources/views/payments/**
  - tests/Feature/Payments/**
  - tests/Browser/Payments/**
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
updated_at: 2026-08-22T19:25:00+02:00
head: 20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4
branch: agent/payments-foundation-20260822
pr: none
status: implementing
phase: investigate
session_id: chatgpt-20260822T1925+0200
session_role: implementer
execution_mode: remote-terminal-plus-github
execution_reason: multi-file Laravel/UI/test work needs an isolated checkout and focused test loop; GitHub remains authoritative for PR/CI state
project_lane: oteryn-platform-core
context_routes:
  - payments
  - security
  - database
  - admin-rbac
  - testing
context_pressure: medium
context_growth: stable
context_score: 9
estimate_confidence: medium
decomposition_decision: phased
decomposition_reason: one cohesive Payments vertical slice reusing the existing core; implementation and browser validation may require separate bounded phases
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
owned_paths:
  - app/Payments/**
  - app/Http/Controllers/Payments/**
  - routes/modules/payments.php
  - resources/views/payments/**
  - tests/Feature/Payments/**
  - tests/Browser/Payments/**
proven:
  - Issue #321 remains open and authorizes repository/test-adapter work only, not real charges or production webhooks.
  - ADR 0021 and merged payment core already provide provider-neutral orders, checkout abstraction, signed deterministic test events, idempotency, reconciliation and refund-integrity evidence.
  - No payment task exists in docs/agents/tasks/active on protected main at invocation start.
derived:
  - The remaining safe foundation is primarily customer/admin/integration/E2E completion around the existing core, not a replacement payment domain.
unknown:
  - Exact frontend/acceptance-harness paths that must be extended after local discovery.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260822-payments-foundation.md
validation:
  - command: repository discovery
    result: PASS
    evidence: protected main, Issue #321/#278, payment ADR/core paths, active tasks and open payment PR search inspected
blockers:
  - none
next_action: Inspect the isolated checkout for current Payments, Admin/Audit, localization and browser-acceptance conventions, then implement the smallest complete non-production vertical slice.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-20260822T1925+0200
  session_started_at: 2026-08-22T19:25:00+02:00
  checkpointed_at: 2026-08-22T19:25:00+02:00
  last_progress_at: 2026-08-22T19:25:00+02:00
  phase: investigate
  exact_head: 20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4
  pull_request: none
  active_operation: isolated checkout discovery
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: branch agent/payments-foundation-20260822 still owns the declared payment paths without overlap
  next_action: Inspect the isolated checkout for current Payments, Admin/Audit, localization and browser-acceptance conventions, then implement the smallest complete non-production vertical slice.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The task must not close Issue #321. A real provider decision, provider sandbox/legal/tax/privacy work, production secrets, production webhook activation and live charging remain separate gates.