---
task_id: OTERYN-20260807-payment-partial-refund-integrity
issue: 797
programme_id: OTERYN_PLATFORM_REMEDIATION
status: validating
risk: high
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
branch: repair/issue-797
base_branch: main
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
search_first:
  - Issue #797
  - deterministic branch repair/issue-797
---

# OTERYN-20260807-payment-partial-refund-integrity

## Goal

Repair Issue #797 so every distinct verified partial-refund event contributes to durable, bounded financial truth and repeated or concurrent partial refunds cannot be lost or exceed the immutable order amount.

## Acceptance criteria

- [x] Provider-neutral partial-refund amount semantics are explicit and unambiguous.
- [x] Durable payment truth records reconstructable refunded minor units without raw provider payloads or unnecessary personal data.
- [x] Distinct sequential partial refunds are independently accounted for after the order is already `partially_refunded`.
- [x] Cumulative verified refunds cannot exceed the immutable order amount and fail closed into reconciliation on mismatch.
- [x] Full-refund handling is consistent with accumulated refund truth.
- [x] Duplicate provider-event IDs remain idempotent while distinct event IDs remain independent.
- [x] Order locking/transaction boundaries prevent concurrent partial refunds from losing value or over-refunding.
- [x] Additive migration safely upgrades schemas and refuses destructive rollback when authenticated refund evidence is populated.
- [ ] Focused payment regressions and applicable repository CI pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - app/Payments/**
  - tests/Feature/Payments/**
  - database/migrations/**
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
forbidden_paths:
  - app/Wallet/**
  - app/Products/**
  - routes/**
  - resources/**
  - .github/workflows/**
  - production systems
  - external repositories
coordination_key: module:payment-partial-refund-integrity
blockers: []
cross_repository_tasks: []
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: OTERYN-20260807-payment-partial-refund-integrity
  classified_at: 2026-08-07T14:22:13Z
  risk: high
  triggers:
    - financial integrity
    - persistence and migration semantics
    - concurrency
    - external provider contract
  unknown_or_conflict: []
  rationale: The repair changes durable financial truth and concurrent refund accounting before real-provider activation.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T14:54:16Z
head: e3bc5ec5670383d5f87e4b025129d9f6c0bb7de3
branch: repair/issue-797
pr: 826
status: validating
context_routes:
  - payments
  - database-persistence
  - security
  - testing
owned_paths:
  - app/Payments/**
  - tests/Feature/Payments/**
  - database/migrations/**
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
proven:
  - Issue #797 is implementation-authorized, P1/high and unblocked; deterministic branch repair/issue-797 is exclusively claimed by this task and PR #826 is its single delivery PR.
  - Root cause is the combination of duplicate-state handling for repeated partial refunds and absence of durable refund-value accounting.
  - Provider-neutral contract defines payment.partially_refunded amount_minor as an incremental authenticated refund delta and payment.refunded amount_minor as cumulative terminal truth equal to the immutable order total.
  - Accepted refund events persist verified_refund_amount_minor and resulting refunded_total_minor on append-oriented versioned payment-order transitions.
  - Repeated partial refunds on an already partially_refunded order intentionally create same-state versioned transitions instead of duplicate-state NOOPs.
  - Refund accumulation executes inside the existing payment-order row lock and transaction; a partial event that would reach or exceed the immutable order total reconciles without mutating financial truth.
  - A legacy partially_refunded state without durable refund-value history fails closed into refund_integrity_mismatch reconciliation.
  - Exact duplicate provider-event IDs remain idempotent before refund accounting; distinct event IDs are processed independently.
  - The additive migration can reverse an empty repository/test schema but checks refund-value columns before rollback and fails closed if authenticated refund evidence is populated.
  - Deterministic sequential, replay, over-refund, full-refund, mismatch and legacy-gap regressions are implemented.
  - MariaDB concurrency coverage uses two distinct legitimate partial refunds whose total remains below the order amount and requires both verified values to survive serialization.
  - Main advanced by unrelated Wallet audit PR #823; repair/issue-797 merged main 92d887372a1961251b9ec8ad7803549d28f1054b without overlap.
  - Static-analysis findings from CI run 31189161059 were repaired without changing financial semantics.
derived:
  - The locked cumulative calculation prevents lost-update accounting and arithmetic over-refund; the sequential over-refund regression exercises the bound while the concurrency regression proves both legitimate values survive.
unknown: []
conflicts: []
first_failure:
  marker: unconditional-forward-only-rollback-incompatible
  evidence: Game Auth Ticket Concurrency run 31189161038 failed because the unconditional RuntimeException in this migration blocked an existing repository migration rollback path even when no refund evidence existed.
rejected_hypotheses:
  - Provider-event idempotency alone could preserve repeated refund value; distinct provider event IDs intentionally bypass exact replay deduplication.
  - A mutable refunded accumulator on payment_orders alone would provide sufficient financial history; append-oriented transition evidence is required to reconstruct each accepted refund delta.
  - An unconditional throwing migration down path protects financial evidence without compatibility cost; existing repository rollback workflows require empty-schema reversibility, so rollback now refuses only when refund evidence is populated.
  - One concurrent over-refund race alone proves no legitimate refund value is lost; final concurrency coverage instead requires two valid concurrent deltas to both persist and reach the exact cumulative total.
changed_paths:
  - app/Payments/Actions/ProcessPaymentProviderEvent.php
  - app/Payments/Data/VerifiedProviderEvent.php
  - app/Payments/Models/PaymentOrderTransition.php
  - app/Payments/PaymentOrderStateMachine.php
  - database/migrations/2026_08_07_143000_add_refund_truth_to_payment_order_transitions.php
  - tests/Feature/Payments/PaymentPartialRefundIntegrityTest.php
  - tests/Feature/Payments/PaymentPartialRefundConcurrencyMariaDbTest.php
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/active/OTERYN-20260807-payment-partial-refund-integrity.md
validation:
  - command: GitHub Actions CI run 31189161059 on cf4da073b4e7f77e3e9bd7de6e6f7031f6276479
    result: FAIL
    evidence: PHPStan reported one production nullsafe access and test-only collection/pcntl type findings; all were repaired on the same branch.
  - command: GitHub Actions Game Auth Ticket Concurrency run 31189161038 on cf4da073b4e7f77e3e9bd7de6e6f7031f6276479
    result: FAIL
    evidence: Existing migration rollback was blocked by an unconditional refund-history RuntimeException; rollback logic was narrowed to fail closed only when refund evidence is populated.
  - command: GitHub Actions Agent Governance run 31189160020 on cf4da073b4e7f77e3e9bd7de6e6f7031f6276479
    result: FAIL
    evidence: This task checkpoint validated; failure is an unrelated main-lane liveness defect for terminal Wallet audit PR #823 and is outside Issue #797 ownership.
blockers: []
next_action: Validate the next exact PR head after the migration-rollback compatibility repair; fix only material Issue #797 failures, then complete HEIGHTENED self-review and terminal closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 4
  session_id: OTERYN-20260807T1622+0200-issue-797
  session_started_at: 2026-08-07T14:22:13Z
  checkpointed_at: 2026-08-07T14:54:16Z
  last_progress_at: 2026-08-07T14:54:16Z
  phase: validation
  exact_head: e3bc5ec5670383d5f87e4b025129d9f6c0bb7de3
  pull_request: 826
  active_operation: repair exact-head static findings and preserve migration rollback compatibility without allowing populated refund evidence deletion
  external_run_ids:
    - 31189161059
    - 31189161038
    - 31189160020
    - 31189782396
    - 31189782330
    - 31189782377
  operation_started_at: 2026-08-07T14:54:16Z
  wait_deadline_at: none
  check_generation: validation-4
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: Issue #797 remains open and PR #826 remains unmerged until terminal exact-head validation completes.
  next_action: Inspect the new head workflow results; repair only a material Issue #797 failure on repair/issue-797, otherwise perform HEIGHTENED self-review and merge.
```

## Notes

No production, provider sandbox, customer charge, secret, destructive external action or external-repository mutation is authorized.
