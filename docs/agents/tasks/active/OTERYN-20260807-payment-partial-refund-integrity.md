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
updated_at: 2026-08-07T14:43:56Z
head: 7972bfd5c1f980b1dbf9e571c92596c77fcdc1f1
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
  - Provider-neutral contract now defines payment.partially_refunded amount_minor as an incremental authenticated refund delta and payment.refunded amount_minor as cumulative terminal truth equal to the immutable order total.
  - Accepted refund events persist verified_refund_amount_minor and resulting refunded_total_minor on append-oriented versioned payment-order transitions.
  - Repeated partial refunds on an already partially_refunded order intentionally create same-state versioned transitions instead of duplicate-state NOOPs.
  - Refund accumulation executes inside the existing payment-order row lock and transaction; a partial event that would reach or exceed the immutable order total reconciles without mutating financial truth.
  - A legacy partially_refunded state without durable refund-value history fails closed into refund_integrity_mismatch reconciliation.
  - Exact duplicate provider-event IDs remain idempotent before refund accounting; distinct event IDs are processed independently.
  - The additive migration is forward-only and its down path throws rather than allowing Laravel to mark a destructive refund-history rollback as completed.
  - Deterministic sequential, replay, over-refund, full-refund, mismatch and legacy-gap regressions are implemented.
  - MariaDB concurrency coverage now uses two distinct legitimate partial refunds whose total remains below the order amount and proves both verified values survive serialization.
  - Main advanced by unrelated Wallet audit PR #823; repair/issue-797 merged main 92d887372a1961251b9ec8ad7803549d28f1054b without overlap and is no longer behind the protected base.
derived:
  - The locked cumulative calculation prevents both lost-update accounting and arithmetic over-refund; the sequential over-refund regression exercises the fail-closed bound while the concurrent regression proves both legitimate values survive.
unknown: []
conflicts: []
first_failure:
  marker: stale-base-required-checks
  evidence: Main advanced after PR #826 opened; the repair branch was merged with current main before terminal exact-head validation rather than relying on stale-base checks.
rejected_hypotheses:
  - Provider-event idempotency alone could preserve repeated refund value; distinct provider event IDs intentionally bypass exact replay deduplication.
  - A mutable refunded accumulator on payment_orders alone would provide sufficient financial history; append-oriented transition evidence is required to reconstruct each accepted refund delta.
  - A no-op migration down method safely implements forward-only evidence preservation; Laravel could mark the migration rolled back while leaving columns behind, so down now fails closed instead.
  - One concurrent over-refund race alone proves no legitimate refund value is lost; the final concurrency regression instead requires two valid concurrent deltas to both persist and reach the exact cumulative total.
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
next_action: Validate the synchronized exact PR head with focused payment/concurrency tests, repository CI and Agent Governance; repair any material failure on the same branch, then complete HEIGHTENED self-review and terminal closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: OTERYN-20260807T1622+0200-issue-797
  session_started_at: 2026-08-07T14:22:13Z
  checkpointed_at: 2026-08-07T14:43:56Z
  last_progress_at: 2026-08-07T14:43:56Z
  phase: validation
  exact_head: 7972bfd5c1f980b1dbf9e571c92596c77fcdc1f1
  pull_request: 826
  active_operation: exact-head validation of cumulative partial-refund financial truth after synchronizing current main
  external_run_ids:
    - 31188869721
    - 31188869672
  operation_started_at: 2026-08-07T14:43:56Z
  wait_deadline_at: none
  check_generation: validation-3
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: Issue #797 remains open and PR #826 remains unmerged until terminal exact-head validation completes.
  next_action: Inspect the newest PR head workflow results; fix only a material failure on repair/issue-797, otherwise perform HEIGHTENED self-review and merge.
```

## Notes

No production, provider sandbox, customer charge, secret, destructive external action or external-repository mutation is authorized.
