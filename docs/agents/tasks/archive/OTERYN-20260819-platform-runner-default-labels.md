---
task_id: OTERYN-20260819-platform-runner-default-labels
status: completed
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: closeout
session_id: chatgpt-20260819-platform-runner-default-labels
session_role: implementer
execution_mode: chat-github
execution_reason: bounded repository repair with GitHub Actions validation
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one runner registration defect and its focused regression contract
validation_level: focused
heavy_validation_runs: 1
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 1155
pull_request: 1174
branch: none
base_sha: 2b9637e813e9431c91656b9982032e43e9b8160a
merged_sha: 3d07ac95c69250eea15095976e08e083ae66abcb
owned_paths: []
---

# Platform runner default-label repair

## Result

PR #1174 removed only `--no-default-labels` from the repository-owned Synology runner registration path, preserving `--labels "$RUNNER_LABELS"`. This allows a future first registration/re-registration to retain the custom `oteryn-staging` label while restoring GitHub's standard self-hosted runner labels.

The required regression was added to `tests/ci/test_synology_deploy_release_identity.py`. No live Synology restart, re-registration, protected-environment mutation, secret mutation, staging deployment or production operation was performed by this task; those observable live cutover steps remain owned by PR #1164 / Issue #1155.

## Validation

Exact candidate head `af62650f927ff52d1bd2ccf87bf92c3f9789072a` passed checkpoint validation, static validation, all required change-classifier lanes, runtime tests, repository `test`, stable `platform-gate`, concurrency proof, Platform DB outage validation, Phase 7 production-like validation, Synology deployment-package validation, and all three Synology image builds. Full-diff self-review found no material issue and no review thread remained open.

Squash merge: `3d07ac95c69250eea15095976e08e083ae66abcb`.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-19T23:47:00+02:00
head: 3d07ac95c69250eea15095976e08e083ae66abcb
branch: none
pr: 1174
status: completed
context_routes:
  - agent-governance
  - deployment-operations
  - testing
owned_paths: []
proven:
  - PR 1174 merged as 3d07ac95c69250eea15095976e08e083ae66abcb.
  - The repository runner entrypoint no longer suppresses GitHub default labels.
  - The custom oteryn-staging label remains configured through RUNNER_LABELS.
  - All applicable exact-head repository checks and full-diff self-review passed before merge.
derived:
  - Future registration with the repaired entrypoint can satisfy the intended self-hosted plus oteryn-staging scheduling contract.
unknown:
  - Live runner labels and scheduling state after the separately authorized privileged Synology re-registration.
conflicts: []
first_failure:
  marker: incomplete-checkpoint-schema
  evidence: The initial task checkpoint was repaired before final validation; no product-code failure remained.
rejected_hypotheses:
  - Weakening PR 1164 to custom-label-only scheduling is an acceptable repair.
changed_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/archive/OTERYN-20260819-platform-runner-default-labels.md
validation:
  - command: exact-head PR 1174 required and applicable CI
    result: PASS
    evidence: candidate af62650f927ff52d1bd2ccf87bf92c3f9789072a
  - command: full exact-head diff self-review
    result: PASS
    evidence: exactly three scoped files and zero unresolved review threads
  - command: live Synology re-registration and PR 1164 runner verification
    result: NOT_APPLICABLE
    evidence: live protected-runner mutation was outside this repository repair task and remains owned by PR 1164 / Issue 1155
blockers: []
next_action: No action for this archived task; ownership is released and live cutover continues under PR 1164 / Issue 1155.
```
