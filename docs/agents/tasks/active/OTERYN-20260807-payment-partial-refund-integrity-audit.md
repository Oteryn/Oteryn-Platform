---
task_id: OTERYN-20260807-payment-partial-refund-integrity-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
---

# OTERYN-20260807 payment partial-refund integrity audit

## Goal

Independently re-audit the payment settlement core after completed OPA-SEC-0001, focusing on repeated and concurrent partial refunds, durable refunded-value truth and provider-contract semantics without modifying payment implementation.

## Acceptance criteria

- [x] Current main, active tasks and open PRs were refreshed and no payment-core implementation owner overlaps this audit.
- [x] Issue #547, PR #595, ADR 0021 and the payment runbook were revalidated against current main.
- [x] Verified-event amount/currency/object matching, state-machine transitions, persistence and focused regression coverage were inspected.
- [x] Sequential repeated partial-refund and cumulative/over-refund behavior were falsified against the current model.
- [x] Existing Issues and PRs were searched for the same root cause.
- [x] One material finding was routed to Issue #797.
- [ ] Exact-head documentation/governance CI passes, audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/reports/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/evidence/OTERYN-20260807-payment-partial-refund-integrity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - payment-integrity audit records only
dependencies:
  - Issue #797 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T10:02:00Z
head: be38527d1876f3c88363474ac790eba95fcea5f6
branch: audit/OTERYN-20260807-payment-partial-refund-integrity
pr: 799
status: validating
context_routes:
  - security
  - database-persistence
  - api-contracts
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/reports/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/evidence/OTERYN-20260807-payment-partial-refund-integrity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - OPA-SEC-0001 is repaired on main: verified provider events now carry authenticated currency and amount, and success/refund/dispute/chargeback mismatches are reconciled.
  - `payment.partially_refunded` targets `partially_refunded`; once already in that state, a distinct later partial-refund event becomes a state-machine NOOP with reason duplicate_state.
  - Partial-refund settlement validation only requires same currency and an individual amount between zero and the immutable order total.
  - PaymentOrder stores no refunded-value accumulator and PaymentOrderTransition stores no refund amount.
  - Successfully processed event metadata intentionally does not persist authenticated amount/currency, so a later partial-refund NOOP does not preserve its refunded amount in durable Platform truth.
  - Current focused tests have no case for two distinct sequential partial refunds or concurrent distinct partial refunds.
  - OPA-SEC-0002 is recorded as Issue #797 with risk high, priority P1 and implementation authorization.
  - Concurrent main change PR #796 repaired OPA-GOV-0022 only and does not alter the audited payment-core paths.
derived:
  - The current model cannot reconstruct cumulative refunded value after repeated partial refunds and cannot enforce an aggregate over-refund boundary.
  - Provider-neutral partial-refund amount semantics are ambiguous because ADR 0021 defines only a positive amount below the total, not incremental-versus-cumulative meaning.
unknown: []
conflicts:
  - Parent Issue #321 requires partial-refund lifecycle plus immutable financial/audit records, while the current core can acknowledge a second verified partial refund as processed without durably preserving its amount.
first_failure:
  marker: OPA-SEC-0002
  evidence: a second distinct payment.partially_refunded event passes amount/currency checks but the state machine returns duplicate_state NOOP and there is no refunded-value persistence.
rejected_hypotheses:
  - Provider event ID idempotency makes the second partial refund a duplicate; rejected because the finding concerns a distinct authenticated event ID.
  - Payment status alone preserves refund truth; rejected because partially_refunded contains no minor-unit total and repeated partial events remain the same state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/reports/OTERYN-20260807-payment-partial-refund-integrity-audit.md
  - docs/agents/evidence/OTERYN-20260807-payment-partial-refund-integrity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source payment-core falsification on main@f7abc6096264aee890e0ab475087adeba7265397
    result: PASS
    evidence: current state machine, processor, models, ADR and tests establish the repeated-partial-refund gap.
  - command: current-main delta reconciliation through 1a72040b1ecb367090f21ec8a767294ff376ae5e
    result: PASS
    evidence: concurrent PR #796 changes branch-lifecycle governance only and does not invalidate payment evidence.
  - command: prior OPA-SEC-0001 repair/resulting-state review
    result: PASS
    evidence: Issue #547 is closed completed and PR #595 is merged; current code contains its amount/currency/object-integrity controls.
  - command: runtime/product E2E
    result: NOT_APPLICABLE
    evidence: this audit package changes documentation/evidence only and production payments remain disabled.
blockers: []
next_action: Complete exact-head checks on PR #799, merge it, then archive this task and release ownership.
```
