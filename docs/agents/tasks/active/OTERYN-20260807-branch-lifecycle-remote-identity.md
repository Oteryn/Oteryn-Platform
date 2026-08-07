---
task_id: OTERYN-20260807-branch-lifecycle-remote-identity
issue: 815
programme_id: OTERYN_PLATFORM_REMEDIATION
status: implementing
risk: high
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
branch: repair/issue-815
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
  - Issue #815
  - deterministic branch repair/issue-815
optional_reads:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
---

# OTERYN-20260807-branch-lifecycle-remote-identity

## Goal

Repair Issue #815 so destructive Branch Lifecycle git operations are bound to the configured repository root and the selected git remote is proven to target the same owner/name repository as the GitHub API client before any push.

## Acceptance criteria

- [ ] Destructive branch deletion cannot execute until the git working tree used for push is bound to the configured repository root.
- [ ] Selected git remote identity is normalized and proven equal to `GitHubClient.repo` before destructive push.
- [ ] Missing, ambiguous, unparsable or mismatched remotes fail closed before `git push`.
- [ ] `--root` controls destructive git subprocess CWD rather than ambient process CWD.
- [ ] Supported SSH and HTTPS GitHub remote forms normalize correctly without accepting a foreign repository.
- [ ] Existing exact `--force-with-lease=<ref>:<expected_sha>` atomicity remains intact.
- [ ] Deterministic wrong-CWD and foreign-origin regressions pass.
- [ ] Exact-head Branch Lifecycle, Agent Governance and repository-selected CI pass with zero unresolved material findings or review threads.

## Ownership

```yaml
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity.md
modules:
  - branch-lifecycle
  - agent-governance
dependencies:
  - Issue #815
  - related Issues #793, #780 and #658
blockers:
  - none
cross_repository_tasks:
  - none
coordination_key: workflow:branch-lifecycle-remote-identity
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: OTERYN-20260807-branch-lifecycle-remote-identity
  classified_at: 2026-08-07T13:33:00Z
  risk: high
  triggers:
    - destructive Git ref deletion
    - repository identity boundary
    - external-repository mutation risk
  unknown_or_conflict: []
  rationale: A successful local git push is authoritative for the configured remote, so root and remote identity must be proven before destructive mutation.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T13:33:00Z
head: e93b11fd9671400a52ae135db1564ad77b700393
branch: repair/issue-815
pr: none
status: implementing
phase: implementation
session_id: OTERYN-20260807T1533+0200-issue-815
session_role: implementer
execution_mode: github-only
execution_reason: bounded branch-lifecycle destructive-remote identity repair with deterministic mocked validation
context_routes:
  - ci-repair
  - agent-governance
  - testing
owned_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity.md
proven:
  - Issue #815 is implementation-authorized, P1/high, unblocked and labelled agent:ready.
  - Deterministic branch repair/issue-815 was created from trusted main e93b11fd9671400a52ae135db1564ad77b700393.
  - Current deletion code uses ambient CWD and an unverified remote name for destructive `git push`.
derived:
  - The repair can be validated without production or external-repository mutation by mocking/localizing git remote queries and push execution.
unknown: []
conflicts: []
first_failure:
  marker: branch-lifecycle-remote-identity-unbound
  evidence: Issue #815 exact evidence and current main implementation.
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity.md
validation: []
blockers:
  - none
next_action: Implement root-bound git execution, remote repository identity verification, and deterministic negative-path tests; then create the single delivery PR.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: OTERYN-20260807T1533+0200-issue-815
  session_started_at: 2026-08-07T13:33:00Z
  checkpointed_at: 2026-08-07T13:33:00Z
  last_progress_at: 2026-08-07T13:33:00Z
  phase: implementation
  exact_head: e93b11fd9671400a52ae135db1564ad77b700393
  pull_request: none
  active_operation: implement branch lifecycle repository identity binding
  external_run_ids: []
  operation_started_at: 2026-08-07T13:33:00Z
  wait_deadline_at: none
  check_generation: implementation
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: deterministic branch and active task ownership remain exact and conflict-free.
  next_action: Implement root/remote identity binding and focused regressions.
```

## Notes

Repository mutation is limited to `blakinio/Oteryn-Platform`; destructive validation against external repositories is forbidden.
