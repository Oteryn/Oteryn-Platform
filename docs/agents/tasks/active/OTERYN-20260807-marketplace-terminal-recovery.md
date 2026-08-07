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

- [ ] Recovery fallback reads and guards current persisted auction state atomically.
- [ ] `completed`, `cancelled`, and `expired` remain monotonic terminal states under stale reconciliation failure.
- [ ] A stale settlement failure after another reconciler completes settlement preserves `SAGA_DONE`, winning-bid state, wallet ledger truth, and final balances.
- [ ] A stale cancellation/no-bid-expiry failure after another reconciler completes return-to-seller preserves the terminal result.
- [ ] Genuine non-terminal reconciliation failures still enter explicit recovery where appropriate.
- [ ] Existing transfer and wallet idempotency semantics are preserved without a Marketplace redesign.
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
updated_at: 2026-08-07T12:52:28Z
invocation_started_at: 2026-08-07T12:46:00Z
last_progress_at: 2026-08-07T12:52:28Z
head: ae716e3b955808916cb203bb97b59df0b44070cf
branch: repair/issue-804
pr: none
status: implementing
phase: implement
execution_mode: github
execution_reason: GitHub connector can perform bounded repository edits and Actions validation without local terminal access.
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
  - Issue #804 is P1/high, implementation-authorized, agent-ready, and unclaimed at task start.
  - No open implementation PR or active task owns the Marketplace reconciliation/recovery paths.
  - Current markRecovery updates by auction id without locking or terminal-state guard.
  - Settlement and return-to-seller success paths already use lockForUpdate and recognize terminal state.
derived:
  - A lock-and-current-state guard in markRecovery is the smallest compatible repair.
unknown: []
conflicts: []
first_failure:
  marker: OPA-REC-0001
  evidence: Issue #804 audit evidence
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet persisted
blockers:
  - none
next_action: Implement an atomic current-state recovery guard and deterministic stale-worker regression tests.
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Notes

Audit evidence was delivered by merged PR #805 and archived by merged PR #809. Those PRs are evidence-only predecessors and do not own this remediation.
