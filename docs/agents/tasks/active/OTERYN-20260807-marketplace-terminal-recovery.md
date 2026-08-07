---
task_id: OTERYN-20260807-marketplace-terminal-recovery
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - Issue #804 and related Marketplace recovery PRs
optional_reads: []
---

# OTERYN-20260807-marketplace-terminal-recovery

## Goal

Repair Issue #804 / OPA-REC-0001 so a stale reconciliation failure cannot regress a newer terminal Character Bazaar auction state into `recovery_required`.

## Acceptance criteria

- [x] Recovery fallback reads and guards current persisted auction state atomically.
- [x] `completed`, `cancelled`, and `expired` remain monotonic terminal states under stale reconciliation failure in deterministic regression coverage.
- [x] A stale settlement failure after another reconciler completes settlement preserves `SAGA_DONE`, winning-bid state, wallet ledger truth, and final balances in deterministic regression coverage.
- [x] A stale cancellation/no-bid-expiry failure after another reconciler completes return-to-seller preserves the terminal result in deterministic regression coverage.
- [x] Genuine non-terminal reconciliation failures still enter explicit recovery in deterministic regression coverage.
- [x] Existing transfer and wallet idempotency semantics are preserved without a Marketplace redesign.
- [ ] Exact-head self-review is PASS with no material findings.
- [ ] Focused Marketplace validation and repository-required exact-head CI pass with zero unresolved review threads.

## Ownership

```yaml
project_lane: oteryn-platform-bazaar
issue: 804
finding: OPA-REC-0001
validation_intensity: HEIGHTENED
owned_paths:
  - app/Marketplace/Actions/ReconcileCharacterAuctions.php
  - app/Marketplace/Actions/RecoverCharacterAuction.php
  - tests/Feature/Marketplace/MarketplaceSettlementRecoveryTest.php
  - tests/Feature/Marketplace/**Auction*Concurrency*.php
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery.md
modules:
  - Marketplace
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
forbidden_paths:
  - app/Payments/**
  - app/Wallet/**
  - app/GameAuth/**
  - .github/workflows/**
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T12:57:30Z
invocation_started_at: 2026-08-07T12:46:00Z
last_progress_at: 2026-08-07T12:57:30Z
head: c9205b12672d473ae2b2bcf321397f77e4c263f7
branch: repair/issue-804
pr: 812
status: validating
phase: validate
execution_mode: github
execution_reason: GitHub connector plus pull-request Actions provide the repository-approved exact-head validation path.
context_routes:
  - database
  - security
  - testing
owned_paths:
  - app/Marketplace/Actions/ReconcileCharacterAuctions.php
  - app/Marketplace/Actions/RecoverCharacterAuction.php
  - tests/Feature/Marketplace/MarketplaceSettlementRecoveryTest.php
  - tests/Feature/Marketplace/**Auction*Concurrency*.php
proven:
  - Issue #804 was P1/high, implementation-authorized, agent-ready, and unclaimed at task start.
  - PR #812 is the sole implementation PR for branch repair/issue-804.
  - markRecovery now uses a database transaction plus lockForUpdate and returns current terminal rows unchanged.
  - Deterministic regression coverage exercises stale settlement completion, cancelled/expired return-to-seller completion, and a genuine non-terminal failure.
  - The first PR CI generation exposed a task-liveness metadata failure because the task still recorded pr none after PR #812 was created.
derived:
  - Updating the durable task with PR #812 is the targeted repair for the governance failure; no runtime change is indicated by that failure.
unknown: []
conflicts: []
first_failure:
  marker: Agent Governance run 31180317069 / task liveness
  evidence: branch repair/issue-804 current head had draft PR #812 while active task recorded pr none
rejected_hypotheses:
  - Runtime Marketplace code caused the first Agent Governance failure; checkpoint validation and ownership validation structure passed before live task-liveness reconciliation rejected the omitted PR identity.
changed_paths:
  - app/Marketplace/Actions/ReconcileCharacterAuctions.php
  - tests/Feature/Marketplace/MarketplaceAuctionTerminalRecoveryConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery.md
validation:
  - command: PR #812 first Agent Governance generation
    result: FAIL
    evidence: run 31180317069; live task liveness rejected omitted PR identity
  - command: full exact-head PR validation
    result: NOT_RUN
    evidence: superseded candidate head after governance metadata repair
blockers:
  - none
next_action: Inspect the new exact-head PR #812 validation generation and repair only evidence-based failures.
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: repair-804-20260807T124600Z
  session_started_at: 2026-08-07T12:46:00Z
  checkpointed_at: 2026-08-07T12:57:30Z
  last_progress_at: 2026-08-07T12:57:30Z
  phase: exact-head validation
  exact_head: c9205b12672d473ae2b2bcf321397f77e4c263f7
  pull_request: 812
  active_operation: PR exact-head Actions validation
  external_run_ids: []
  operation_started_at: 2026-08-07T12:55:52Z
  wait_deadline_at: 2026-08-07T13:40:52Z
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: new PR #812 head emits required CI after durable PR identity repair
  next_action: Inspect the aggregate exact-head check state for the new PR #812 head.
```

## Notes

Audit evidence was delivered by merged PR #805 and archived by merged PR #809. Those PRs are evidence-only predecessors and do not own this remediation.
