---
task_id: OTERYN-20260807-branch-lifecycle-predelete-revalidation
issue: 780
status: validating
agent: ChatGPT
project_lane: oteryn-platform-core
task_kind: implementation
phase: validate
branch: repair/issue-780
base_branch: main
created: 2026-08-07T09:05:14Z
updated: 2026-08-07T09:18:22Z
risk: high
validation_intensity: HEIGHTENED
execution_mode: github-only
implementation_authorized: true
production_activation_authorized: false
terminal_pr_policy: archive_pending
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
---

# OTERYN-20260807 branch lifecycle pre-delete revalidation

## Goal

Repair Issue #780 so every destructive branch deletion is guarded by immediate live SHA, open-PR, active-claim/task, protection and retention revalidation instead of relying on the earlier inventory snapshot.

## Acceptance criteria

- [x] Re-resolve the exact live ref SHA immediately before each deletion and fail closed on drift.
- [x] Re-resolve open PR and active claim/task state immediately before each deletion and fail closed when the branch became active.
- [x] Re-resolve protection and retention state immediately before each deletion and fail closed on change.
- [x] Preserve reviewed-manifest hashing, exact merged-PR evidence, default-branch protection and recovery guarantees.
- [x] Add deterministic regression tests for SHA, PR, claim/task, protection and retention races.
- [ ] Complete HEIGHTENED exact-head self-review and required CI, merge PR #789, archive the task and release ownership.

## Ownership

```yaml
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-predelete-revalidation.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-predelete-revalidation.md
modules:
  - branch-lifecycle governance automation
dependencies:
  - Issue #780
  - ADR 0024
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
blockers:
  - none
cross_repository_tasks:
  - none
```

## Repair claim

```yaml
repair_claim:
  protocol_version: 5
  issue: 780
  owner: chatgpt-remediator-20260807-780
  task_id: OTERYN-20260807-branch-lifecycle-predelete-revalidation
  branch: repair/issue-780
  base_sha: 8478b627609f9d82799bc5866c8ba504d5751f19
  claimed_at: 2026-08-07T09:05:14Z
  owned_paths:
    - tools/agents/branch_lifecycle.py
    - tools/agents/test_branch_lifecycle.py
    - .github/workflows/branch-lifecycle.yml
  coordination_key: workflow:branch-lifecycle-deletion-cas
  validation_intensity: HEIGHTENED
  status: active
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T09:18:22Z
head: 288770582e9b7b1e3ff358383748f5ca4b19f049
branch: repair/issue-780
pr: 789
status: validating
context_routes:
  - agent-governance
  - ci-repair
  - testing
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-predelete-revalidation.md
proven:
  - Issue #780 is implementation-authorized, high risk and owned by this deterministic repair branch.
  - PR #789 contains per-entry live branch SHA, protection, open-PR, active-task and remediation-Issue revalidation before destructive deletion.
  - PR #789 adds an expected-SHA guard immediately at the delete call and preserves post-delete ref verification.
  - Branch Lifecycle run 31165056373 passed focused regression tests, policy validation and the non-destructive live dry-run on head 288770582e9b7b1e3ff358383748f5ca4b19f049.
  - CI run 31165056560 passed classify-changes and test on head 288770582e9b7b1e3ff358383748f5ca4b19f049.
  - BRANCH_DELETION_APPROVAL.json is absent on main, so merging this repair does not activate the reviewed-deletion apply path by itself.
derived:
  - A targeted live revalidation gate plus a second expected-SHA check at DELETE is the strongest fail-closed guard available with GitHub's name-based ref deletion API.
unknown: []
conflicts: []
first_failure:
  marker: invalid-task-checkpoint-extra-keys
  evidence: Agent Governance run 31165056314 rejected the initial task checkpoint because validation_gate was an unsupported nested key; implementation and Branch Lifecycle checks passed.
rejected_hypotheses:
  - Workflow concurrency alone prevents a branch from changing or becoming active after inventory.
  - The failed Agent Governance generation represents a branch-lifecycle implementation failure.
changed_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-predelete-revalidation.md
validation:
  - command: python3 tools/agents/test_branch_lifecycle.py and branch lifecycle policy validation
    result: PASS
    evidence: Branch Lifecycle run 31165056373 validate job passed on implementation head 288770582e9b7b1e3ff358383748f5ca4b19f049.
  - command: Branch Lifecycle live dry-run
    result: PASS
    evidence: Branch Lifecycle run 31165056373 completed successfully without destructive apply on the pull request.
  - command: repository CI classify-changes and test
    result: PASS
    evidence: CI run 31165056560 completed successfully on implementation head 288770582e9b7b1e3ff358383748f5ca4b19f049.
  - command: production or staging E2E
    result: NOT_APPLICABLE
    evidence: This repair changes repository-governance branch deletion safety only and performs no product runtime, staging or production mutation.
  - command: Agent Governance initial generation
    result: FAIL
    evidence: Run 31165056314 failed only because this task record contained unsupported checkpoint keys; this checkpoint update removes those keys for the next exact-head generation.
blockers: []
next_action: Validate this exact task-record head, complete whole-diff self-review and required CI, merge PR #789, then archive the task and release Issue #780 ownership.
```

## HEIGHTENED self-review gate

Current state: `PENDING` until the final exact PR head completes all required checks. Review scope includes all changed files, Issue #780 acceptance, negative race paths, rollback, compatibility, related PRs and review-thread state.

Rollback is bounded: reverting the squash merge restores the previous branch-lifecycle implementation. No branch-deletion approval file exists on current main, so this repair does not independently authorize or trigger a deletion candidate set.

## Notes

Issue #780 remains bounded to Oteryn-Platform branch-lifecycle automation. Production, staging and external-repository mutation are out of scope.
