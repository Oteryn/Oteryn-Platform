---
task_id: OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
terminal_pr_policy: archive_pending
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
---

# OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit

## Goal

Independently falsify the live task-liveness implementation delivered for Issue #558, focusing on branch-only ownership versus live PR truth, without modifying governance runtime or repair code.

## Acceptance criteria

- [x] Current main and recent Issue #558 terminal state were refreshed before selection.
- [x] Open PRs and Issues were searched for overlapping branch/PR-liveness work.
- [x] `task_liveness.py`, its client protocol, focused tests and Agent Governance invocation were inspected on current main.
- [x] Branch-only, open-PR, terminal-PR, retained-branch, branch-reuse and API-failure boundaries were checked against the implemented proof model.
- [x] One material finding was deduplicated and routed to Issue #788.
- [ ] Exact-head documentation/governance CI passes, audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/reports/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/evidence/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - Agent Governance live-liveness audit records only
dependencies:
  - Issue #788 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T09:17:00Z
head: 6aa16e18fc866f5364feac6844fd89b175c694df
branch: audit/OTERYN-20260807-agent-governance-branch-pr-reconciliation
pr: 790
status: validating
context_routes:
  - agent-governance
  - ci
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/reports/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/evidence/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Issue #558 is terminal and its live-liveness implementation is present on current main.
  - Numeric-PR tasks resolve PR state, but `pr: none` plus an existing branch takes a branch-existence-only path and is marked BRANCH_ONLY with active ownership.
  - The GitHub state protocol exposes no branch-to-PR discovery operation, so omitted PR identity cannot be reconciled against open or terminal PR truth.
  - Focused tests explicitly accept branch-only `pr: none` and cover terminal behavior only when the task already declares a numeric PR.
  - The prior audit task associated with PR #784 remained `pr: none` after the PR existed and exact-head Agent Governance still passed, demonstrating the live omitted-PR condition.
  - OPA-GOV-0021 is recorded as Issue #788 with risk high, priority P1 and implementation authorization.
  - Audit package is bound to PR #790 and uses explicit archive-pending terminal policy for post-merge closeout.
derived:
  - A retained branch can continue to claim active ownership after its PR becomes terminal when the task omits the PR number.
  - A correct repair must preserve genuine pre-PR branch-only work while tying terminal classification to exact branch/head identity.
unknown: []
conflicts:
  - Issue #558 intended branch/PR/task reconciliation and retained-terminal-branch classification, while current branch-only evaluation cannot observe PR state at all.
first_failure:
  marker: OPA-GOV-0021
  evidence: tools/agents/task_liveness.py evaluate_task branch-only path queries only the branch ref when pr is none.
rejected_hypotheses:
  - Existing branch existence alone proves pre-PR work; rejected because the same branch can already own an open or terminal PR.
  - Terminal tests cover the omitted-PR path; rejected because all terminal fixtures declare numeric pr 12.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/reports/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit.md
  - docs/agents/evidence/OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source static falsification on main@a1b3690c85fe4fb585d5725474769a8aced2e686
    result: PASS
    evidence: Issue #788 records the exact evaluator branch, client capability gap and missing fixtures.
  - command: live repository observation from audit PR #784 lifecycle
    result: PASS
    evidence: branch-only audit task retained pr none while its PR existed and Agent Governance passed.
  - command: runtime/product E2E
    result: NOT_APPLICABLE
    evidence: this audit package changes documentation/evidence only.
blockers: []
next_action: Complete exact-head documentation/governance checks for PR #790, merge it, archive this task and release ownership.
```
