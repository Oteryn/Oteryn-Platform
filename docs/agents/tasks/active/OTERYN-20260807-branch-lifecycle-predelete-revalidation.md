---
task_id: OTERYN-20260807-branch-lifecycle-predelete-revalidation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
search_first:
  - Issue #780 and deterministic branch repair/issue-780
optional_reads:
  - .github/workflows/branch-lifecycle.yml
---

# OTERYN-20260807-branch-lifecycle-predelete-revalidation

## Goal

Repair Issue #780 so every destructive branch deletion is guarded by immediate live SHA, open-PR, active-claim/task, protection and retention revalidation instead of relying on the earlier inventory snapshot.

## Acceptance criteria

- [ ] Re-resolve the exact live ref SHA immediately before each deletion and fail closed on drift.
- [ ] Re-resolve open PR and active claim/task state immediately before each deletion and fail closed when the branch became active.
- [ ] Re-resolve protection and retention state immediately before each deletion and fail closed on change.
- [ ] Preserve reviewed-manifest hashing, exact merged-PR evidence, default-branch protection and recovery guarantees.
- [ ] Add deterministic regression tests for SHA, PR, claim/task, protection and retention races.
- [ ] Complete HEIGHTENED exact-head self-review, focused validation and required CI before merge.

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
policy_version: 2
task_kind: implementation
phase: implement
session_id: chatgpt-remediator-20260807-780
session_role: implementer
execution_mode: github-only
execution_reason: bounded repository-owned Python and workflow repair with GitHub Actions validation
updated_at: 2026-08-07T09:05:14Z
head: 8478b627609f9d82799bc5866c8ba504d5751f19
branch: repair/issue-780
pr: none
status: implementing
context_routes:
  - agent-governance
  - ci-repair
  - testing
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
proven:
  - current apply_manifest validates the manifest once against a process-local report and then performs name-only deletion without per-entry live revalidation
  - Issue #780 is implementation-authorized, high risk, agent:ready and had no deterministic claim branch before this claim
  - repair/issue-780 was created from main@8478b627609f9d82799bc5866c8ba504d5751f19
  - ADR 0024 forbids deleting open-PR, active-claim, protected, retention or changed/ambiguous branches
derived:
  - the repair requires a fail-closed per-entry live safety gate immediately before delete
unknown: []
conflicts: []
first_failure:
  marker: stale pre-delete snapshot
  evidence: tools/agents/branch_lifecycle.py apply_manifest deletion loop
rejected_hypotheses:
  - workflow concurrency alone prevents the race
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-predelete-revalidation.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet committed
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-remediator-20260807-780
  classified_at: 2026-08-07T09:05:14Z
  risk: high
  triggers:
    - destructive Git ref deletion
    - race/concurrency safety
    - repository governance
  unknown_or_conflict: []
  rationale: stale state can delete newly active or changed branches
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive branch-lifecycle safety defect with one deterministic acceptance contract
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
invocation_started_at: 2026-08-07T09:05:14Z
last_progress_at: 2026-08-07T09:05:14Z
ci_checks_for_current_head: 0
ci_check_generation: 0
terminal_ci_wait_started_at: none
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
blockers:
  - none
next_action: implement per-entry live pre-delete revalidation and deterministic race tests
```

## Notes

Issue #780 remains bounded to Oteryn-Platform branch-lifecycle automation. Production, staging and external-repository mutation are out of scope.
