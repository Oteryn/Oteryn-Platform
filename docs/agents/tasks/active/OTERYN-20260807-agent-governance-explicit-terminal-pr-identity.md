---
task_id: OTERYN-20260807-agent-governance-explicit-terminal-pr-identity
issue: 811
programme_id: OTERYN_PLATFORM_REMEDIATION
status: validating
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
  - pull request #819
optional_reads: []
---

# OTERYN-20260807-agent-governance-explicit-terminal-pr-identity

## Goal

Repair Issue #811 so an explicit numeric terminal PR can release task ownership only after the task branch is proven to match the PR head branch identity.

## Acceptance criteria

- [x] Numeric terminal PRs fail closed when `task.branch` differs from PR `head.ref`.
- [x] Unrelated merged or closed-unmerged PRs cannot release ownership for another task branch.
- [x] Existing open/draft numeric-PR and branch-only reconciliation behavior remains valid.
- [x] Matching terminal PR behavior remains valid, including retained-branch advisory handling.
- [x] Missing, foreign or malformed PR identity remains fail-closed.
- [x] Deterministic tests cover merged mismatch, closed-unmerged mismatch and matching terminal behavior.
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
    evidence:
      - PR #819 full diff inspected after implementation; final exact-head review remains pending after this checkpoint update.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T13:40:00Z
head: 87faa31a67860626e31a5ea7853653dbb5418d3e
branch: repair/issue-811
pr: 819
status: validating
phase: final_ci
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
  - Issue #811 was claimed through deterministic branch repair/issue-811 from main e93b11fd9671400a52ae135db1564ad77b700393.
  - PR #819 is the single authoritative delivery PR for Issue #811.
  - Terminal merged and closed-unmerged branch mismatch paths now fail with branch_pr_mismatch before archive-pending handling.
  - Missing terminal branch identity and foreign terminal PR heads fail before archive-pending handling.
  - Matching terminal identity retains existing archive-pending and retained-branch advisory behavior.
  - Agent Governance run 31183622144 executed 25 task-liveness tests successfully, including the new regressions.
  - CI run 31183621671 passed on implementation head 87faa31a67860626e31a5ea7853653dbb5418d3e.
derived:
  - The Agent Governance failure on implementation head is lifecycle metadata only: live validation correctly detected that the newly created PR #819 had not yet been recorded in this task checkpoint.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-branch-pr-identity-omitted
  evidence: run 31183622144 reported branch_pr_identity_omitted for repair/issue-811 because checkpoint pr was none after PR #819 was created.
rejected_hypotheses:
  - The focused task-liveness regression suite failed; it passed all 25 tests.
  - The implementation introduced an application CI failure; CI run 31183621671 passed.
changed_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-explicit-terminal-pr-identity.md
validation:
  - command: python tools/agents/test_task_liveness.py
    result: PASS
    evidence: Agent Governance run 31183622144 step Run live task liveness tests; 25 tests passed.
  - command: repository CI
    result: PASS
    evidence: CI run 31183621671 passed on implementation head 87faa31a67860626e31a5ea7853653dbb5418d3e; exact post-checkpoint head rerun pending.
  - command: Agent Governance live ownership validation
    result: FAIL
    evidence: run 31183622144 correctly rejected omitted PR identity; this checkpoint records PR #819 and creates the new exact head to validate.
blockers:
  - none
next_action: Observe the exact-head PR #819 workflow generation; inspect any failure, then perform exact-head self-review and merge if all required checks pass.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: OTERYN-20260807T1533+0200-issue-811
  session_started_at: 2026-08-07T13:33:52Z
  checkpointed_at: 2026-08-07T13:40:00Z
  last_progress_at: 2026-08-07T13:40:00Z
  phase: final_ci
  exact_head: pending-checkpoint-commit
  pull_request: 819
  active_operation: observe exact-head GitHub Actions after recording PR identity
  external_run_ids: []
  operation_started_at: 2026-08-07T13:40:00Z
  wait_deadline_at: 2026-08-07T14:10:00Z
  check_generation: post-pr-identity-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #819 exact-head workflows reach a terminal state or a material failure appears.
  next_action: Query aggregate workflow state for PR #819 exact head once and inspect only failed jobs if any.
```

## Notes

Repository mutation is limited to `blakinio/Oteryn-Platform`; no production or external-repository operation is authorized.
