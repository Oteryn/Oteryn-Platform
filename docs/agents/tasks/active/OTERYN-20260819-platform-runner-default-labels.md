---
task_id: OTERYN-20260819-platform-runner-default-labels
status: validating
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: validate
session_id: chatgpt-20260819-platform-runner-default-labels
session_role: implementer
execution_mode: chat-github
execution_reason: correct a discovered security contradiction in the runner-label boundary with exact-head CI
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one security-boundary correction plus its existing required regression contract
validation_level: focused
heavy_validation_runs: 2
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 1155
pull_request: 1176
branch: fix/reinstate-runner-no-default-labels-20260819
base_sha: 3d07ac95c69250eea15095976e08e083ae66abcb
owned_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260819-platform-runner-default-labels.md
---

# Platform staging-runner label boundary correction

## Objective

Restore and prove the intended repository-scoped custom-label boundary for the Docker-socket-capable Synology staging runner. Generic `self-hosted` jobs must not be eligible for this privileged host; the runner is intentionally registered with `--no-default-labels` and the custom `oteryn-staging` label.

## Corrected source of truth

- live repository runner ID `21`, name `oteryn-synology-staging`, exposes only `oteryn-staging` and reports OS `unknown`;
- PR #1164's one-off post-transfer workflow intentionally routes its no-side-effect runner job using only `oteryn-staging`;
- that workflow explicitly states that `--no-default-labels` prevents generic `self-hosted` jobs from landing on the Docker-socket-capable host;
- PR #1174 removed that flag based on an incorrect inference and therefore weakened the intended scheduling isolation despite passing deterministic CI;
- PR #1175 was closed unmerged as a premature archive after the contradiction was discovered.

## Acceptance

- restore `--no-default-labels` in the repository-owned first-registration path;
- retain `RUNNER_LABELS` / `--labels` custom-label behavior;
- required Synology release-identity CI requires the isolation flag rather than forbidding it;
- exact-head required CI passes;
- full exact-head self-review confirms alignment with #1164 and no generic self-hosted eligibility is introduced;
- no live Synology restart/re-registration or protected-environment mutation is performed by this correction;
- task is archived only after the correction merges and the erroneous #1174 behavior is fully superseded.

## Rollback / compatibility

The correction restores the trusted pre-#1174 entrypoint behavior. Existing live runner state already matches the custom-label-only model, so repository merge does not require a live runner mutation to regain the intended isolation. Any future re-registration remains scoped to `oteryn-staging` with default labels suppressed.

## E2E classification

Repository implementation E2E is `NOT_APPLICABLE`: this correction restores registration-policy code and its deterministic contract without mutating the live protected runner. PR #1164's existing no-side-effect custom-label runner job is the observable live attachment proof.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-19T23:58:00+02:00
head: 057f431119787b0f7bbc5d234ce18206b4e09411
branch: fix/reinstate-runner-no-default-labels-20260819
pr: 1176
status: validating
context_routes:
  - agent-governance
  - deployment-operations
  - testing
owned_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260819-platform-runner-default-labels.md
proven:
  - PR 1164 explicitly documents --no-default-labels as a security isolation boundary for the Docker-socket-capable staging runner.
  - PR 1164 routes runner verification using only the custom oteryn-staging label.
  - Live runner 21 currently exposes only oteryn-staging, consistent with that model.
  - PR 1174 removed --no-default-labels and therefore contradicted the live verification contract.
  - PR 1175 was closed without merge after the contradiction was discovered.
derived:
  - Restoring --no-default-labels returns repository code to the intended scheduling-isolation model without requiring a live runner mutation.
unknown: []
conflicts: []
first_failure:
  marker: incorrect-runner-label-assumption
  evidence: The post-merge #1164 workflow explicitly contradicted PR 1174's assumption that generic self-hosted labeling was required; its security comment states the opposite.
rejected_hypotheses:
  - PR 1164 requires both self-hosted and oteryn-staging labels.
  - Adding generic self-hosted eligibility is harmless on a Docker-socket-capable runner.
changed_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260819-platform-runner-default-labels.md
validation:
  - command: source-of-truth inspection of PR 1164 one-off post-transfer workflow
    result: PASS
    evidence: custom-label-only runs-on plus explicit no-default-labels security rationale
  - command: exact-head PR 1176 required and applicable CI
    result: NOT_RUN
    evidence: final validation generation pending after checkpoint repair
  - command: live Synology runner mutation
    result: NOT_APPLICABLE
    evidence: live runner already uses the intended custom-label-only registration and this repository correction performs no protected-host mutation
blockers: []
next_action: Run exact-head CI, inspect the complete diff and merge only if all applicable checks pass with no material self-review finding.
```
