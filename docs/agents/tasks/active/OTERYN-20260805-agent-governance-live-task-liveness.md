---
task_id: OTERYN-20260805-agent-governance-live-task-liveness
issue: 558
status: validating
agent: ChatGPT
project_lane: oteryn-platform-core
task_kind: implementation
phase: validate
branch: repair/issue-558-agent-governance-live-task-liveness
base_branch: main
created: 2026-08-07T10:21:29+02:00
updated: 2026-08-07T10:28:06+02:00
risk: high
execution_mode: github-only
implementation_authorized: true
production_activation_authorized: false
terminal_pr_policy: archive_pending
context_pressure: medium
context_growth: stable
decomposition_decision: single
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/GOVERNANCE_CONTRACT.json
---

# OTERYN-20260805 agent governance live task liveness

## Goal

Repair Issue #558 by making Agent Governance validate live GitHub ownership truth in addition to local checkpoint shape, while preserving blocked/waiting external tasks and keeping terminal branch retention informational rather than active ownership.

## Acceptance

- [x] reject contradictory active/archive identities;
- [x] validate branch existence for claimed branch-only tasks and open PR tasks;
- [x] validate open PR head repository/branch identity;
- [x] reject terminal PR tasks unless they are in an explicit archive-pending transition;
- [x] reject stale terminal next actions such as merge or mark-ready instructions;
- [x] report retained terminal source branches without treating them as active ownership;
- [x] fail closed when required GitHub state cannot be resolved without leaking credentials;
- [x] expose schema-valid and live-valid state separately in Control Room;
- [x] cover open, draft, waiting external, branch-only, terminal closeout, stale terminal, missing branch, duplicate identity, API failure and prompt-injection fixtures;
- [ ] pass exact-head Agent Governance and all repository-selected checks;
- [ ] merge, archive this task and release ownership.

## Ownership

```yaml
owned_paths:
  - tools/agents/checkpoint.py
  - tools/agents/test_checkpoint.py
  - tools/agents/control_room.py
  - tools/agents/test_control_room.py
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - .github/workflows/agent-governance.yml
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/active/OTERYN-20260805-agent-governance-live-task-liveness.md
forbidden_paths:
  - docs/agents/tasks/active/OTERYN-20260722-game-gateway-mvp.md
  - docs/agents/tasks/active/OTERYN-20260724-announcements-events.md
  - docs/agents/tasks/active/OTERYN-20260724-download-center.md
  - services/game-gateway/**
  - app/GameAuth/**
  - docs/contracts/**
  - production systems
  - external repositories
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T10:28:06+02:00
head: bd12170bcc0702995b78715df0f7de5e290a3801
branch: repair/issue-558-agent-governance-live-task-liveness
pr: 779
status: validating
context_routes:
  - agent-governance
  - testing
  - workflows
owned_paths:
  - tools/agents/control_room.py
  - tools/agents/test_control_room.py
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - .github/workflows/agent-governance.yml
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/active/OTERYN-20260805-agent-governance-live-task-liveness.md
proven:
  - Issue #558 implementation is authorized and its former blocker PR #542 is merged and archived.
  - Current main active-task inventory contains five durable tasks; blocked verification-only tasks intentionally use pr none and branch none.
  - The current native-protocol programme coordinator uses pr none with an existing claimed branch and therefore requires a supported branch-only live state.
  - Current Agent Governance on main validates checkpoint shape only and does not reconcile GitHub PR branch or archive state.
  - PR #779 now contains the bounded live-state validator, Control Room integration, deterministic fixtures, workflow enforcement and additive governance policy revision.
derived:
  - Live liveness is a separate additive policy layer so checkpoint version 1 remains structurally compatible.
  - A merged or closed PR may remain briefly under tasks active only through an explicit archive-pending transition whose next action is archival.
unknown: []
conflicts: []
first_failure:
  marker: schema-valid-task-can-contradict-live-github-state
  evidence: Issue #558 proves merged PR and stale ownership records passed the existing local-only governance gate.
rejected_hypotheses:
  - Treat every retained terminal branch as active ownership.
  - Reject all pr none tasks, which would invalidate legitimate blocked external and pre-PR branch-only work.
  - Require a different-agent repair audit despite current remediation policy.
changed_paths:
  - tools/agents/control_room.py
  - tools/agents/test_control_room.py
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - .github/workflows/agent-governance.yml
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/active/OTERYN-20260805-agent-governance-live-task-liveness.md
validation:
  - command: current-main dependency and ownership preflight
    result: PASS
    evidence: PR #542 is terminal; Issue #558 has no competing open PR and its repair branch was identical to current main before implementation.
  - command: deterministic liveness and Control Room fixture suite
    result: PASS
    evidence: focused tests passed before publishing the repository change; exact-head GitHub Actions remains the merge authority.
blockers: []
next_action: Finish exact-head validation and self-review; after PR #779 reaches terminal merged state, archive this task and release ownership.
```

## Safety boundary

This task is repository governance infrastructure only. It does not authorize production deployment, protected-environment approval, secret mutation, external-repository changes, or cleanup of historical stale task records owned by separate Issues.
