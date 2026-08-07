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

- [x] A task with `pr: none` does not remain authoritative BRANCH_ONLY when the same live branch/head already has an open matching PR.
- [x] A retained branch cannot preserve active ownership after a matching merged or closed PR for the same branch/head.
- [x] Branch reuse with new commits is distinguished from the terminal PR head.
- [x] Multiple or ambiguous matching PR histories fail closed instead of granting ownership.
- [x] Genuine pre-PR branches with no matching PR remain supported.
- [x] Deterministic tests cover genuine branch-only, omitted-PR open PR, retained merged PR, retained closed-unmerged PR, branch reuse, ambiguous history and GitHub API failure.
- [x] Control Room reflects reconciled ownership through the live-validity report without a separate Control Room code path.
- [ ] Exact-head Agent Governance and repository-selected CI pass with zero unresolved review findings.

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
updated_at: 2026-08-07T11:22:00Z
head: d087fe9b1581ad79fdd63b6449c104d2f1367feb
branch: repair/issue-788
pr: 808
status: validating
context_routes:
  - ci-repair
  - agent-governance
owned_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation.md
proven:
  - Issue #788 is open, implementation-authorized, risk:high and priority:P1; deterministic branch repair/issue-788 was claimed from main 6b0efc015812d699c20424c4048e2fdba570c2dd with no pre-existing claim or open Issue #788 PR.
  - PR #808 is the single authoritative Issue #788 delivery and its source branch is repair/issue-788.
  - The original branch-only path granted ownership from branch existence alone and could not discover open or terminal PR history when task metadata omitted the PR number.
  - GitHubState and GitHubClient now support fail-closed pull-request history lookup for the claimed source branch, and branch-only reconciliation compares the current branch SHA with exact PR head repo, ref and SHA identity.
  - An exact-current-head open or draft PR with omitted task PR identity fails live validity; an exact-current-head terminal PR also fails live validity and cannot preserve ownership through a retained branch.
  - Multiple exact-current-head PR identities and unavailable or malformed GitHub state fail closed.
  - A branch with no exact-current-head PR remains legitimate BRANCH_ONLY ownership, including branch reuse after a terminal PR whose recorded head SHA differs from the current branch SHA.
  - Twenty deterministic task-liveness tests pass for numeric PR behavior, genuine branch-only work, omitted open/draft/terminal PRs, branch reuse, ambiguity, malformed state and API failures.
derived:
  - Control Room already consumes task_liveness report validity and surfaces live contradictions while keeping schema/local task state separate, so corrected liveness truth satisfies the Control Room acceptance boundary without modifying control_room.py.
  - Agent Governance already executes task-liveness tests, live ownership validation and live-aware Control Room enforcement, so no workflow change is required.
unknown: []
conflicts: []
first_failure:
  marker: Issue #788 proven omitted-PR branch-only bypass.
  evidence: The pre-repair evaluate_task branch-only path called get_branch only and set ownership_active true whenever the ref existed.
rejected_hypotheses:
  - Treat every historical PR on the same branch name as terminal ownership evidence; that would falsely invalidate a deliberately reused branch with new commits.
  - Query only open pull requests; that would leave retained merged or closed-unmerged source branches able to preserve stale ownership.
changed_paths:
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - docs/agents/tasks/active/OTERYN-20260807-agent-governance-branch-pr-reconciliation.md
validation:
  - command: python tools/agents/test_task_liveness.py
    result: PASS
    evidence: Twenty deterministic tests passed, including omitted open/draft/merged/closed PR, branch reuse, ambiguity and API/shape failure cases.
blockers: []
next_action: Perform exact-head full-diff self-review on PR #808, mark it ready, then require exact-head Agent Governance and repository-selected CI before merge.
```

## Notes

No application runtime, production, protected environment or external repository mutation is required by this repair. The implementation deliberately leaves Control Room, workflow and governance-contract bytes unchanged because their existing report/enforcement interfaces already consume the corrected liveness result.
