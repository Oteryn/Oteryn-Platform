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
execution_reason: bounded repository repair with GitHub Actions validation and no local checkout requirement
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
branch: fix/platform-runner-default-labels-20260819
base_sha: 2b9637e813e9431c91656b9982032e43e9b8160a
owned_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260819-platform-runner-default-labels.md
---

# Platform runner default-label repair

## Objective

Restore the GitHub-hosted default self-hosted-runner labels while retaining the dedicated `oteryn-staging` custom label, so the post-transfer verification contract `runs-on: [self-hosted, oteryn-staging]` can schedule without weakening it to custom-label-only matching.

## Verified defect

- live repository runner ID `21`, name `oteryn-synology-staging`, is online/idle but exposes only `oteryn-staging` and reports OS `unknown`;
- `deploy/synology/runner/entrypoint.sh` registered with `--labels "$RUNNER_LABELS" --no-default-labels` on the trusted base;
- PR #1164 intentionally requires `[self-hosted, oteryn-staging]`, so its queued verification job cannot select the current registration;
- the Synology runner Compose still points at the historical-owner deploy-runner image; runtime cutover is a later privileged operation and is not performed by this repository repair.

## Acceptance

- remove `--no-default-labels` from the repository-owned first-registration path;
- keep the custom `RUNNER_LABELS` contract unchanged;
- required Synology release-identity CI proves the entrypoint supplies `--labels` and does not suppress GitHub default labels;
- exact-head required CI passes;
- full exact-head self-review has no material finding;
- repository-only repair is merged before any privileged Synology re-registration;
- PR #1164 remains authoritative for live post-transfer runner verification after the runtime consumes the repaired image.

## Rollback / compatibility

Rollback is the parent `main` before this repair. Existing already-registered runner state is not mutated merely by merging repository code. The behavioral change applies only when the repaired image performs a first registration/re-registration, preserving the custom staging label while allowing GitHub's standard default labels.

## E2E classification

Repository implementation E2E is `NOT_APPLICABLE`: merging this repair does not itself restart or re-register the protected self-hosted runner. The observable live E2E is explicitly owned by PR #1164 and must remain pending until an authorized privileged Synology operation consumes the repaired image.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-19T23:39:00+02:00
head: 7615798c12237f14906dfd15bb2370ab8646e9c6
branch: fix/platform-runner-default-labels-20260819
pr: 1174
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
  - Live repository runner 21 is online and idle with only the oteryn-staging label and OS unknown.
  - Trusted-base entrypoint suppressed GitHub default runner labels with --no-default-labels.
  - PR 1164 requires both self-hosted and oteryn-staging labels and its runner jobs remain queued.
  - The implementation removed only --no-default-labels while retaining the custom --labels argument.
derived:
  - A future first registration or re-registration with the repaired entrypoint will retain oteryn-staging while allowing GitHub standard default labels.
unknown:
  - Live runner labels and scheduling state after the later privileged Synology re-registration.
conflicts: []
first_failure:
  marker: incomplete-checkpoint-schema
  evidence: checkpoint-validation job 96233584560 rejected the initial task checkpoint because mandatory version-1 fields were absent.
rejected_hypotheses:
  - Weakening PR 1164 to custom-label-only scheduling is an acceptable repair.
changed_paths:
  - deploy/synology/runner/entrypoint.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260819-platform-runner-default-labels.md
validation:
  - command: initial PR 1174 classify-changes validation
    result: FAIL
    evidence: checkpoint schema failure only; existing Synology release-identity tests passed 15 of 15.
  - command: required Synology runner-label regression on repaired head
    result: NOT_RUN
    evidence: assertion was folded into the existing required test suite after the first CI attempt.
  - command: live Synology re-registration and PR 1164 runner verification
    result: NOT_APPLICABLE
    evidence: this repository-only PR does not authorize or perform a protected self-hosted-runner restart or re-registration.
blockers: []
next_action: Run exact-head required CI, inspect the full diff and merge only if all applicable gates pass with no material self-review finding.
```
