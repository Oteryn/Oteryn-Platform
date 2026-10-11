---
task_id: OTERYN-20261011-decanary-rename-1
governing_issue: 1474
required_reads:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
search_first:
  - docs/maintenance
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
optional_reads: []
---

# OTERYN-20261011 DECANARY-RENAME-1 pilot naming

## Goal

Governing GitHub Issue: #1474 (native-only migration) - canonical lifecycle authority for this task.

Packet F-5 of `docs/agents/programs/DECANARY_REMOVAL_PLAN.md`: rename the merge-queue rollout name collisions to `pilot` naming (two maintenance runbooks; the retired native-auth cache-build workflow keeps its historical filename in the registry), and reconcile the ownership-block blocker of the merged native-only baseline record.

## Acceptance criteria

- [x] `docs/maintenance/*MQ*` runbooks renamed to `*_PILOT_*` with content updated; no live reference to the old paths remains outside historical records.
- [x] The retired workflow's historical filename stays unchanged in `docs/agents/CI_WORKFLOW_LIFECYCLE.json` and `tests/ci/test_workflow_trigger_economy.py` (review: `pilot` naming is reserved for live artifacts).
- [x] The ownership block of `OTERYN-20261008-native-only-migration-baseline` states the same blocker as its checkpoint.
- [x] Legacy-token ratchet does not rise; `python -m pytest tests/ci` and workflow inventory validators pass.

## Ownership

```yaml
owned_paths:
  - docs/maintenance/DELEGATED_MQ_CANARY_20260913.md
  - docs/maintenance/DELEGATED_MQ_PILOT_20260913.md
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/maintenance/OTERYN_CI_MQ_PILOT_V2.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20261008-native-only-migration-baseline.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-rename-1.md
modules:
  - Documentation
  - CI contract tests
dependencies:
  - Issue #1474
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T12:00:00Z
head: UNKNOWN
branch: claude/decanary-rename-1-20261011
pr: 1527
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
owned_paths:
  - docs/maintenance/DELEGATED_MQ_CANARY_20260913.md
  - docs/maintenance/DELEGATED_MQ_PILOT_20260913.md
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/maintenance/OTERYN_CI_MQ_PILOT_V2.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20261008-native-only-migration-baseline.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-rename-1.md
proven:
  - The native-auth cache-build workflow is already retired; only its name remains in the lifecycle registry and the trigger-economy test.
  - Archived task records still cite the old runbook paths and stay unchanged as historical records.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/maintenance/DELEGATED_MQ_PILOT_20260913.md
  - docs/maintenance/OTERYN_CI_MQ_PILOT_V2.md
  - docs/agents/tasks/active/OTERYN-20261008-native-only-migration-baseline.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: counts fall in ci_deploy, docs, paths, runtime and tests; none rise
  - command: product-runtime-e2e
    result: NOT_APPLICABLE
    evidence: Documentation and CI-contract naming only; no runtime behavior changes.
blockers:
  - none
next_action: archive complete; await exact-head CI and lead integration.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: same-repository PR; source ref deletes after protected merge
source_branch_evidence: PR #1527 pending merge; verify source ref absent after merge
```
