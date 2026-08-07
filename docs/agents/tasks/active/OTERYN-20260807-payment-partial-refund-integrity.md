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
- [x] Forward-only migration safely upgrades deployed schemas without deleting immutable payment history.
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
updated_at: 2026-08-07T14:33:28Z
head: cd9d1cc51634689427532d3d1c4715a590de75fd
branch: repair/issue-797
pr: none
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
  - Issue #797 is implementation-authorized, P1/high and unblocked; deterministic branch repair/issue-797 is exclusively claimed by this task.
  - Root cause is the combination of duplicate-state handling for repeated partial refunds and absence of durable refund-value accounting.
  - Provider-neutral contract now defines payment.partially_refunded amount_minor as an incremental authenticated refund delta and payment.refunded amount_minor as cumulative terminal truth equal to the immutable order total.
  - Accepted refund events persist verified_refund_amount_minor and resulting refunded_total_minor on append-oriented versioned payment-order transitions.
  - Repeated partial refunds on an already partially_refunded order intentionally create same-state versioned transitions instead of duplicate-state NOOPs.
  - Refund accumulation executes inside the existing payment-order row lock and transaction; a partial event that would reach or exceed the immutable order total reconciles without mutating financial truth.
  - A legacy partially_refunded state without durable refund-value history fails closed into refund_integrity_mismatch reconciliation.
  - Exact duplicate provider-event IDs remain idempotent before refund accounting; distinct event IDs are processed independently.
  - A forward-only additive migration preserves authenticated refund settlement evidence and intentionally does not drop it in down().
  - Deterministic sequential, replay, over-refund, full-refund, mismatch, legacy-gap and MariaDB concurrent-distinct-partial regressions are implemented.
derived:
  - Two concurrent +600 partial refunds against a 1000 order serialize on the order lock: one can establish total 600 and the other must reconcile because 1200 would exceed the order total.
unknown: []
conflicts: []
first_failure:
  marker: repeated-partial-refund-durable-value-missing
  evidence: Issue #797 and the pre-repair state machine/event-processing path silently consumed a second distinct partial refund as duplicate_state and stored no cumulative refund amount.
rejected_hypotheses:
  - Provider-event idempotency alone could preserve repeated refund value; distinct provider event IDs intentionally bypass exact replay deduplication.
  - A mutable refunded accumulator on payment_orders alone would provide sufficient financial history; append-oriented transition evidence is required to reconstruct each accepted refund delta.
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
validation: []
blockers: []
next_action: Open the single authoritative Issue #797 PR, run exact-head focused payment/concurrency and repository-selected CI, repair any material failure on the same branch, then perform HEIGHTENED full-diff self-review and terminal closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: OTERYN-20260807T1622+0200-issue-797
  session_started_at: 2026-08-07T14:22:13Z
  checkpointed_at: 2026-08-07T14:33:28Z
  last_progress_at: 2026-08-07T14:33:28Z
  phase: validation
  exact_head: cd9d1cc51634689427532d3d1c4715a590de75fd
  pull_request: none
  active_operation: exact-head validation of cumulative partial-refund financial truth
  external_run_ids: []
  operation_started_at: 2026-08-07T14:33:28Z
  wait_deadline_at: none
  check_generation: implementation-2
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: Issue #797 remains open and repair/issue-797 remains owned by this task until terminal merge/closeout.
  next_action: Create the authoritative PR and validate the exact branch head before HEIGHTENED self-review and merge.
```

## Notes

No production, provider sandbox, customer charge, secret, destructive external action or external-repository mutation is authorized.
