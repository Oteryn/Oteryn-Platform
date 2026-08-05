---
task_id: OTERYN-20260724-validation-cost-policy
coordination_id: OTS-20260724-validation-cost-policy
status: completed
branch: dudantas/validation-cost-policy
base_branch: main
created: 2026-07-24
completed: 2026-07-24
archived: 2026-08-05
related_pr: "129"
required_reads:
  - AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_ROUTING.md
search_first: []
optional_reads: []
owned_paths: []
modules_touched:
  - agent-governance
cross_repo_tasks:
  - CAN-20260724-validation-cost-policy
  - OTH-20260724-validation-cost-policy
  - OTC-20260724-validation-cost-policy
---

# Risk-based validation policy

## Goal

Make validation proportional to changed paths, risk and coherent project milestones. Agents perform focused checks during individual steps, defer application/container builds and broad suites until a phase is reviewable as a whole, and still validate early when dependencies, migrations, security or deployment prerequisites require it.

## Acceptance criteria

- [x] Every Platform agent loads the validation matrix during startup.
- [x] Multi-step work uses focused checks during individual steps.
- [x] Heavy application/container validation is deferred to coherent milestone completion when safe.
- [x] Dependency, migration, security and deployment exceptions remain explicit.
- [x] PR #129 merged from exact final head `39d5d345c9d2289a3a07bcf400971c944159a0a0`.
- [x] Merge commit `60b12fb2d1748fb016484eca521a6c61af505d37` is terminal implementation evidence.
- [x] Validation-policy and context-routing ownership is released.
- [x] The historical source branch is classified without continuation authority.

## Terminal scope

PR #129 changed only:

- `docs/agents/BUILD_TEST_MATRIX.md`;
- `docs/agents/CONTEXT_ROUTING.md`;
- the original task record.

The delivered policy remains authoritative through the current repository files. This archived task is historical evidence only and owns no current path, lease, branch, pull request, workflow, runtime, production state, or external repository.

## Ownership release

```yaml
owned_paths: []
shared_paths: []
leases: []
released_paths:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260724-validation-cost-policy.md
cross_repository_authority: none
```

The historical cross-repository task identifiers are references only. They do not authorize reads beyond normal repository access or any write in another repository.

## Branch and pull-request lifecycle

```yaml
pull_request:
  number: 129
  state: merged
  final_head: 39d5d345c9d2289a3a07bcf400971c944159a0a0
  merge_commit: 60b12fb2d1748fb016484eca521a6c61af505d37
  unresolved_review_threads: 0
source_branch:
  name: dudantas/validation-cost-policy
  live_state_at_reconciliation: present
  classification: retained_historical
  current_dependency: none found
  continuation_authority: false
  ownership_authority: false
```

The retained branch is not an active task, recovery branch, remediation lock, deployment rollback source, or authority to resume PR #129. Future changes require a new task and current ownership preflight.

## Terminal checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T20:51:00Z
head: 60b12fb2d1748fb016484eca521a6c61af505d37
branch: dudantas/validation-cost-policy
pr: 129
status: completed
context_routes:
  - agent-governance
  - testing
owned_paths: []
proven:
  - PR 129 is closed and merged from final head 39d5d345c9d2289a3a07bcf400971c944159a0a0.
  - Merge commit 60b12fb2d1748fb016484eca521a6c61af505d37 contains the completed validation-cost policy change.
  - The source branch remains present but has no open pull request or current task dependency.
  - The task acceptance criteria were delivered by the merged pull request.
derived:
  - The task can be archived while the policy remains current independently of this historical record.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: terminal merge superseded the stale pre-merge checkpoint
rejected_hypotheses:
  - PR 129 still requires review or merge; live GitHub state proves it merged on 2026-07-24.
  - The historical task must retain current policy ownership; current work is coordinated by new tasks and live claims.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260724-validation-cost-policy.md
  - docs/agents/tasks/archive/OTERYN-20260724-validation-cost-policy.md
validation:
  - command: GitHub PR 129 terminal-state verification
    result: PASS
    evidence: merged=true, final head 39d5d345c9d2289a3a07bcf400971c944159a0a0, merge commit 60b12fb2d1748fb016484eca521a6c61af505d37
  - command: retained source-branch and related open-PR search
    result: PASS
    evidence: branch remains present; no open PR or current task dependency was found
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: reconciliation changes task lifecycle documentation only and does not alter executable behavior
blockers: []
next_action: none
```

## Closeout

```yaml
implementation_complete: true
complete_feature_or_declared_partial: true
outcome_verified: true
e2e:
  result: NOT_APPLICABLE_WITH_REASON
  reason: documentation and ownership-only reconciliation changes no executable system boundary
task_archived_or_terminal: true
ownership_released: true
stale_branch_reconciled: true
```
