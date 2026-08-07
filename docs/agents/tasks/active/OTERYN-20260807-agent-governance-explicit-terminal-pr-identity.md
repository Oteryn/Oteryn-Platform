---
task_id: OTERYN-20260807-agent-governance-explicit-terminal-pr-identity
issue: 811
programme_id: OTERYN_PLATFORM_REMEDIATION
status: implementing
risk: high
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
branch: repair/issue-811
base_branch: main
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
search_first:
  - Issue #811
  - deterministic branch repair/issue-811
  - related open pull requests
optional_reads: []
---

# OTERYN-20260807-agent-governance-explicit-terminal-pr-identity

## Goal

Repair Issue #811 so an explicit numeric terminal PR can release task ownership only after the task branch is proven to match the PR head branch identity.

## Acceptance criteria

- [ ] Numeric terminal PRs fail closed when `task.branch` differs from PR `head.ref`.
- [ ] Unrelated merged or closed-unmerged PRs cannot release ownership for another task branch.
- [ ] Existing open/draft numeric-PR and branch-only reconciliation behavior remains valid.
- [ ] Matching terminal PR behavior remains valid, including retained-branch advisory handling.
- [ ] Missing, foreign or malformed PR identity remains fail-closed.
- [ ] Deterministic tests cover merged mismatch, closed-unmerged mismatch and matching terminal behavior.
- [ ] Exact-head focused validation, Agent Governance and repository-selected CI pass with no unresolved material findings or review threads.

## Ownership

```yaml
owned_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-explicit-terminal-pr-identity.md
modules:
  - agent-governance
  - task-liveness
dependencies:
  - Issue #811
  - related Issues #558 and #788
blockers:
  - none
cross_repository_tasks:
  - none
coordination_key: workflow:agent-governance-explicit-terminal-pr-identity
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: OTERYN-20260807-agent-governance-explicit-terminal-pr-identity
  classified_at: 2026-08-07T13:34:00Z
  risk: high
  triggers:
    - ownership collision prevention
    - live GitHub state reconciliation
    - terminal task lifecycle
  unknown_or_conflict: []
  rationale: A false terminal identity can release ownership for unrelated active work.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T13:34:00Z
head: e93b11fd9671400a52ae135db1564ad77b700393
branch: repair/issue-811
pr: none
status: implementing
phase: implement
session_id: OTERYN-20260807T1533+0200-issue-811
session_role: implementer
execution_mode: github-only
execution_reason: bounded two-file governance repair with GitHub Actions validation
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-explicit-terminal-pr-identity.md
proven:
  - Issue #811 is open, implementation-authorized, high-risk P1 work with agent:ready and no blocker.
  - The deterministic branch repair/issue-811 did not exist and was created from main e93b11fd9671400a52ae135db1564ad77b700393.
  - Current numeric open/draft PR reconciliation checks task branch against PR head.ref.
  - Current numeric terminal PR reconciliation releases ownership before checking task branch against PR head.ref.
derived:
  - The smallest repair is to enforce branch/head-ref identity before either open or terminal numeric PR state can affect ownership.
unknown: []
conflicts: []
first_failure:
  marker: terminal-explicit-pr-branch-identity-not-checked
  evidence: tools/agents/task_liveness.py numeric terminal path enters TERMINAL_ARCHIVE_PENDING without comparing task.branch to head_ref.
rejected_hypotheses:
  - A new terminal-only ownership state is required; the existing branch_pr_mismatch finding can fail closed before terminal state handling.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-explicit-terminal-pr-identity.md
validation:
  - command: focused task-liveness tests
    result: NOT_RUN
    evidence: implementation not yet committed
blockers:
  - none
next_action: Implement the common numeric-PR branch identity gate and deterministic terminal mismatch regressions, then run focused validation.
```

## Notes

Repository mutation is limited to `blakinio/Oteryn-Platform`; no production or external-repository operation is authorized.
