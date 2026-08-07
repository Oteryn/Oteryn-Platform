---
task_id: OTERYN-20260807-branch-lifecycle-atomic-delete
issue: 793
status: implementing
agent: ChatGPT
branch: repair/issue-793
base_branch: main
created: 2026-08-07T09:50:18Z
updated: 2026-08-07T09:50:18Z
risk: high
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
execution_mode: github-only
execution_budget_minutes: 60
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - Issue #793 and related Issue #780 / PR #789
  - open PRs and active tasks for branch lifecycle ownership
optional_reads: []
---

# OTERYN-20260807-branch-lifecycle-atomic-delete

## Goal

Repair Issue #793 by replacing the final name-only branch deletion boundary with a server-enforced expected-old-SHA deletion guard while preserving all existing fail-closed lifecycle checks.

## Acceptance criteria

- [ ] Remote destructive deletion carries the reviewed expected branch SHA as an atomic precondition.
- [ ] A ref advance after the final client-side read but before deletion is rejected remotely and the advanced branch remains present.
- [ ] Existing live SHA, open-PR, active-task, issue, protection, policy and default-branch revalidation remains intact.
- [ ] Reviewed manifest hashing, dry-run behavior and recovery evidence remain intact.
- [ ] Focused deterministic tests cover success, remote lease rejection and existing negative paths.
- [ ] Exact-head Branch Lifecycle, Agent Governance and repository-selected CI pass with no unresolved review threads.

## Ownership

```yaml
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete.md
modules:
  - branch-lifecycle-governance
dependencies:
  - Issue #793
  - ADR 0024
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T09:50:18Z
head: 993b3561feb75644d4a07f3e3377020be051eed6
branch: repair/issue-793
pr: none
status: implementing
phase: implement
session_id: chatgpt-remediator-20260807-793
session_role: implementer
execution_mode: github-only
execution_reason: repository connector supports bounded multi-file repair and Actions validation
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete.md
proven:
  - Issue #793 is open, agent-ready, implementation-authorized and unclaimed.
  - main is 993b3561feb75644d4a07f3e3377020be051eed6 at claim preflight.
  - No related PR or repair/issue-793 branch existed before claim.
  - Current apply path performs final client-side SHA reads but destructive REST DELETE carries only the branch name.
  - Branch Lifecycle apply job has contents:write and actions/checkout persisted Git credentials are available to the job checkout.
derived:
  - A Git receive-pack force-with-lease deletion can carry the expected old object ID to the remote ref update boundary without exposing a token in command arguments.
unknown:
  - exact final CI run identities until the repair PR exists
conflicts: []
first_failure:
  marker: non-atomic-rest-delete-boundary
  evidence: Issue #793 and current tools/agents/branch_lifecycle.py
rejected_hypotheses:
  - Another client-side GET before REST DELETE can make the operation atomic.
  - GitHub REST Delete a reference accepts an expected-old-SHA precondition.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete.md
validation:
  - command: live issue/branch/PR/task ownership preflight
    result: PASS
    evidence: Issue #793 unclaimed; deterministic branch absent; no related PR; main exact SHA verified
blockers:
  - none
next_action: Implement server-enforced expected-SHA deletion and deterministic remote-boundary race regression tests.
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-remediator-20260807-793
  classified_at: 2026-08-07T09:50:18Z
  risk: high
  triggers:
    - destructive Git ref deletion
    - race/concurrency safety
    - repository governance
  unknown_or_conflict: []
  rationale: The repair changes the final destructive branch-ref update boundary and must prove fail-closed behavior under a last-instruction race.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-remediator-20260807-793
  session_started_at: 2026-08-07T09:50:18Z
  checkpointed_at: 2026-08-07T09:50:18Z
  last_progress_at: 2026-08-07T09:50:18Z
  phase: implement
  exact_head: 993b3561feb75644d4a07f3e3377020be051eed6
  pull_request: none
  active_operation: implement atomic branch deletion guard
  external_run_ids: []
  operation_started_at: 2026-08-07T09:50:18Z
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: repair/issue-793 remains exclusive owner of Issue #793 paths
  next_action: Implement server-enforced expected-SHA deletion and deterministic remote-boundary race regression tests.
```

## Notes

This repair is repository-governance only. It authorizes no production, staging, secret, protected-environment or external-repository mutation.
