---
task_id: OTERYN-20260807-branch-lifecycle-remote-identity
issue: 815
programme_id: OTERYN_PLATFORM_REMEDIATION
status: validating
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

- [x] Destructive branch deletion cannot execute until the git working tree used for push is bound to the configured repository root.
- [x] Selected git remote identity is normalized and proven equal to `GitHubClient.repo` before destructive push.
- [x] Missing, ambiguous, unparsable or mismatched remotes fail closed before `git push`.
- [x] `--root` controls destructive git subprocess CWD rather than ambient process CWD.
- [x] Supported SSH and HTTPS GitHub remote forms normalize correctly without accepting a foreign repository.
- [x] Existing exact `--force-with-lease=<ref>:<expected_sha>` atomicity remains intact.
- [x] Deterministic wrong-CWD and foreign-origin regressions are implemented.
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
updated_at: 2026-08-07T14:01:00Z
head: e4e832c23f683d245f922b9f2e7497356ba20c1b
branch: repair/issue-815
pr: 822
status: validating
phase: validation
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
  - Issue #815 is implementation-authorized, P1/high and unblocked.
  - Deterministic branch repair/issue-815 is claimed by this task.
  - PR #822 is the only open delivery PR for repair/issue-815.
  - Destructive git execution is root-bound and the selected push remote is repository-identity checked before push.
derived:
  - The repair is validated without production or external-repository mutation by deterministic mocked git remote and push behavior.
unknown: []
conflicts: []
first_failure:
  marker: branch-lifecycle-remote-identity-unbound
  evidence: Issue #815 exact evidence and pre-repair implementation.
rejected_hypotheses: []
changed_paths:
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity.md
validation:
  - PR #822 opened from repair/issue-815 to main.
  - Exact-head Agent Governance first run exposed missing task PR identity; this checkpoint records PR #822 and supersedes that head.
blockers:
  - none
next_action: Validate the new exact PR head with Branch Lifecycle, Agent Governance, repository-selected CI and HEIGHTENED self-review; repair any material failure on the same branch.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: OTERYN-20260807T1533+0200-issue-815
  session_started_at: 2026-08-07T13:33:00Z
  checkpointed_at: 2026-08-07T14:01:00Z
  last_progress_at: 2026-08-07T14:01:00Z
  phase: validation
  exact_head: e4e832c23f683d245f922b9f2e7497356ba20c1b
  pull_request: 822
  active_operation: exact-head validation and heightened self-review
  external_run_ids:
    - 31185209898
    - 31185209109
    - 31185209133
  operation_started_at: 2026-08-07T13:58:01Z
  wait_deadline_at: none
  check_generation: validation-2
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: PR #822 remains open on repair/issue-815 and exact-head validation is incomplete.
  next_action: Re-run exact-head governance and selected CI after this ownership checkpoint update.
```

## Notes

Repository mutation is limited to `blakinio/Oteryn-Platform`; destructive validation against external repositories is forbidden.
