---
task_id: OTERYN-20260807-payment-partial-refund-integrity
issue: 797
programme_id: OTERYN_PLATFORM_REMEDIATION
status: investigating
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

- [ ] Provider-neutral partial-refund amount semantics are explicit and unambiguous.
- [ ] Durable payment truth records reconstructable refunded minor units without raw provider payloads or unnecessary personal data.
- [ ] Distinct sequential partial refunds are independently accounted for after the order is already `partially_refunded`.
- [ ] Cumulative verified refunds cannot exceed the immutable order amount and fail closed into reconciliation on mismatch.
- [ ] Full-refund handling is consistent with accumulated refund truth.
- [ ] Duplicate provider-event IDs remain idempotent while distinct event IDs remain independent.
- [ ] Order locking/transaction boundaries prevent concurrent partial refunds from losing value or over-refunding.
- [ ] Forward-only migration safely upgrades deployed schemas without deleting immutable payment history.
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
updated_at: 2026-08-07T14:22:13Z
head: 51208defaa9ccf03c9e14489e0c7095685361f30
branch: repair/issue-797
pr: none
status: investigating
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
  - Issue #797 is implementation-authorized, P1/high, agent-ready and unblocked.
  - Deterministic branch repair/issue-797 was created from trusted main 51208defaa9ccf03c9e14489e0c7095685361f30.
  - PaymentOrderStateMachine currently returns duplicate_state when a second partial-refund event targets an already partially_refunded order.
  - ProcessPaymentProviderEvent currently validates each partial amount independently and does not persist cumulative refunded minor units.
derived:
  - Durable refunded-value accounting must be updated under the existing locked-order transaction before an event is acknowledged processed.
unknown:
  - Exact current payment model, migration and focused-test shapes needed for the smallest compatible repair.
conflicts: []
first_failure:
  marker: repeated-partial-refund-durable-value-missing
  evidence: Issue #797 and current payment state-machine/event-processing implementation.
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payment-partial-refund-integrity.md
validation: []
blockers: []
next_action: Inspect payment models, migrations, verified-event contract and focused tests; define the smallest forward-compatible durable refund ledger/accumulator and implement one coherent repair package.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: OTERYN-20260807T1622+0200-issue-797
  session_started_at: 2026-08-07T14:22:13Z
  checkpointed_at: 2026-08-07T14:22:13Z
  last_progress_at: 2026-08-07T14:22:13Z
  phase: investigation
  exact_head: 51208defaa9ccf03c9e14489e0c7095685361f30
  pull_request: none
  active_operation: inspect current payment persistence and focused regressions
  external_run_ids: []
  operation_started_at: 2026-08-07T14:22:13Z
  wait_deadline_at: none
  check_generation: claim-1
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: Issue #797 remains open and deterministic branch repair/issue-797 remains owned by this task.
  next_action: Continue root-cause inspection and implement the bounded repair on repair/issue-797.
```

## Notes

No production, provider sandbox, customer charge, secret, destructive external action or external-repository mutation is authorized.
