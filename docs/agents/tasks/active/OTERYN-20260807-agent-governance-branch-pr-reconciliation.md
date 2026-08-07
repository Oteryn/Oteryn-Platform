---
task_id: OTERYN-20260807-agent-governance-branch-pr-reconciliation
programme_id: OTERYN_PLATFORM_REMEDIATION
issue: 788
task_kind: implementation
implementation_authorized: true
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
search_first:
  - Issue #788 and related PRs
  - active tasks and overlapping ownership
---

# OTERYN-20260807 agent-governance branch/PR reconciliation

## Goal

Repair Issue #788 so branch-only active tasks are reconciled against live pull-request history for the exact branch/head identity, preventing retained terminal branches or omitted PR metadata from falsely preserving ownership while retaining legitimate pre-PR branch-only work.

## Acceptance criteria

- [ ] A task with `pr: none` does not remain authoritative BRANCH_ONLY when the same live branch/head already has an open matching PR.
- [ ] A retained branch cannot preserve active ownership after a matching merged or closed PR for the same branch/head.
- [ ] Branch reuse with new commits is distinguished from the terminal PR head.
- [ ] Multiple or ambiguous matching PR histories fail closed instead of granting ownership.
- [ ] Genuine pre-PR branches with no matching PR remain supported.
- [ ] Deterministic tests cover genuine branch-only, omitted-PR open PR, retained merged PR, retained closed-unmerged PR, branch reuse, ambiguous history and GitHub API failure.
- [ ] Control Room reflects reconciled ownership.
- [ ] Exact-head focused tests, Agent Governance and repository-selected CI pass with zero unresolved review findings.

## Ownership

```yaml
owned_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - tools/agents/control_room.py
  - tools/agents/test_control_room.py
  - .github/workflows/agent-governance.yml
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation.md
coordination_key: workflow:agent-governance-branch-pr-reconciliation
validation_intensity: HEIGHTENED
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T11:14:00Z
status: investigating
branch: repair/issue-788
pr: none
phase: investigate
execution_mode: github-only
context_pressure: medium
context_growth: stable
decomposition_decision: single
owned_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - tools/agents/control_room.py
  - tools/agents/test_control_room.py
  - .github/workflows/agent-governance.yml
  - docs/agents/GOVERNANCE_CONTRACT.json
proven:
  - Issue #788 is open, implementation-authorized, risk:high, priority:P1 and agent:ready.
  - Deterministic claim branch repair/issue-788 was created from main at 6b0efc015812d699c20424c4048e2fdba570c2dd.
  - No pre-existing repair/issue-788 branch or open PR for Issue #788 was found before the claim.
unknown:
  - smallest backward-compatible GitHub API shape for exact branch/head PR-history reconciliation
  - whether control_room.py requires direct changes or inherits corrected task_liveness results
conflicts: []
validation:
  - pending focused deterministic tests
blockers: []
next_action: Inspect the current liveness client, tests, governance contract and Control Room consumption, then implement the smallest fail-closed branch-to-PR reconciliation with deterministic fixtures.
```
