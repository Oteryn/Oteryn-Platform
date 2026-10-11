---
task_id: OTERYN-20261011-native-only-e2e-del-1
governing_issue: 1474
required_reads:
  - docs/agents/programs/*_REMOVAL_PLAN.md
search_first:
  - tests/e2e/native_auth_ephemeral_cutover
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads: []
---

# OTERYN-20261011 E2E-DEL-1 delete legacy cutover E2E

## Goal

Governing GitHub Issue: #1474 (native-only migration) - canonical lifecycle authority for this task.

Owner decision: delete `tests/e2e/native_auth_ephemeral_cutover` (3 files). It drives the external legacy harness and no workflow, composer script or phpunit config references it. Lower the legacy-token baseline accordingly.

## Acceptance criteria

- [x] `tests/e2e/native_auth_ephemeral_cutover` is deleted; no live reference remains (only the retired workflow name in the lifecycle registry and trigger-economy test, which is a different artifact, and historical records).
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` lowered with the ratchet `--write`; report-only `history` kept at its previous value; no enforced group rises.
- [x] `python tools/validation/legacy_token_ratchet.py` and `python -m pytest tests/ci` pass.

## Ownership

```yaml
owned_paths:
  - tests/e2e/native_auth_ephemeral_cutover/**
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/programs/*_REMOVAL_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261011-native-only-e2e-del-1.md
modules:
  - E2E tests
  - Documentation
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
updated_at: 2026-10-11T17:00:00Z
head: eafe707697c962e5755f8375f510f46c42804daf
branch: claude/decanary-e2e-del-20261011
pr: 1545
status: completed
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261011-native-only-e2e-del-1.md
proven:
  - No workflow, composer script, phpunit config or test references the deleted directory.
  - The active login-native-only record mentions it as a follow-up note and stays unchanged (not owned).
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - tests/e2e/native_auth_ephemeral_cutover/acceptance_extensions.py
  - tests/e2e/native_auth_ephemeral_cutover/exact_runner.py
  - tests/e2e/native_auth_ephemeral_cutover/platform_runner.py
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/programs/*_REMOVAL_PLAN.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-only-e2e-del-1.md
  - docs/agents/tasks/active/OTERYN-20261011-native-only-e2e-del-1.md
validation:
  - command: PR 1545 exact-head CI and protected integration
    result: PASS
    evidence: PR 1545 merged as eafe707697c962e5755f8375f510f46c42804daf; ratchet passed, tests/ci 216 passed
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: enforced groups fall; none rise
  - command: python -m pytest tests/ci
    result: PASS
    evidence: 216 passed
  - command: product-runtime-e2e
    result: NOT_APPLICABLE
    evidence: Deletes an unreferenced legacy test harness; no runtime change.
blockers: []
next_action: No further action; the task is archived and its source branch deletes after merge.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1545 is terminal on protected main and the source branch has no continuing ownership purpose
source_branch_evidence: PR #1545 merged as protected main eafe707697c962e5755f8375f510f46c42804daf at 2026-10-11T16:40:18Z; source branch deletes after merge
```

## Notes

Review P2 (wording, not corrected retroactively): the task's `owned_paths` and the checkpoint's `changed_paths` describe different sets. `owned_paths` is the ownership grant for the whole task (including the active and archive record paths), while `changed_paths` lists only the paths the PR actually modified; neither is a claim about the other. This archive releases the completed task ownership and changes no runtime state.
