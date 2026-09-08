---
task_id: OTERYN-20260908-synology-platform-reconcile
governing_issue: 1345
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Issue 1345 and current Synology impact-deploy implementation
  - active Synology and portal ownership
optional_reads: []
---

# OTERYN-20260908-synology-platform-reconcile

## Goal

Governing GitHub Issue: #1345 remains the canonical issue-level authority for the first genuine live `platform-reconcile` timing proof. This archived task records the completed implementation and protected-main integration only.

Add a second fail-closed Synology staging profile for schema-stable Platform-only runtime changes. Keep the existing presentation fast path unchanged; broader Platform-only changes recreate only Platform plus its existing Marketplace scheduler, validate the preserved stack with the existing health contract, and retain the current full path for mixed, schema, Gateway, deployment-control, or unknown changes.

## Acceptance criteria

- [x] existing presentation-only `platform-fast` behavior remains intact;
- [x] accumulated known Platform-only runtime changes can select `platform-reconcile`;
- [x] database/schema, Gateway, deployment-control and unknown runtime changes remain full;
- [x] Platform reconcile preserves MariaDB, Redis, Canary, internal proxy and Gateway container identities by contract;
- [x] Platform reconcile runs the existing full staging health contract before release-state promotion;
- [x] Platform reconcile restores the prior proven Platform/release state on failure;
- [x] exact-head Synology contracts, required CI and `platform-gate` passed before protected integration;
- [x] resulting-main deployment of the control-plane change selected the existing full fallback and completed healthy with rollback skipped.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-platform-reconcile.md
modules:
  - Synology staging deployment routing
  - Synology staging validation
dependencies:
  - Issue 1341 presentation-fast implementation on protected main
  - Issue 1339 remains separate runner/autostart work
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T11:20:00Z
head: 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b
branch: perf/1345-synology-platform-reconcile
pr: 1349
status: completed
phase: terminal_closeout
terminal_pr_policy: archive_pending
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-platform-reconcile.md
proven:
  - PR 1349 is the canonical implementation PR for this task and merged only through protected Merge Queue.
  - Initial exact-head cd449d0a735aa3218e4bce4e8b1c05b320eb4151 exposed a Synology Rollback Contract regression in run 34216148757 job 102028235933 because the first design renamed canonical deploy.sh.
  - The rename design was rejected; canonical deploy/synology/scripts/deploy.sh is restored exactly to pre-task blob 0ab49b18416099fdf26db64090d49c4258fad776 and no rollback/recovery assertion was weakened or redirected.
  - New impact routing lives in deploy/synology/scripts/deploy-impact.sh; presentation-only candidates delegate to canonical deploy.sh and therefore preserve the existing platform-fast profile.
  - Database/schema, Gateway, deployment-control and unknown runtime inputs are outside the Platform-only allowlist and delegate fail-closed to canonical deploy.sh full behavior.
  - Platform-reconcile contract recreates only Platform plus the existing Marketplace scheduler, then runs Platform smoke and the existing health-check.sh before release-state promotion.
  - Platform-reconcile contract requires MariaDB, Redis, Canary, internal proxy and Gateway container IDs to remain unchanged and restores the previously proven Platform/release state on failure.
  - Final lifecycle head e1882f1e5cb627746f90140211337861d5e31ad6 passed all nine triggered workflows, including CI run 34217776186 with platform-gate job 102034192836 success and Agent Governance success with archive_pending active-task state.
  - Merge Queue candidate 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b passed CI run 34218043488 and platform-gate job 102034958305 before integration.
  - PR 1349 merged as protected main 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b; branch protection still requires platform-gate.
  - Resulting-main Build Synology run 34218264210 succeeded; ordinary main did not publish deploy-runner.
  - Resulting-main Platform source SHA is 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b with immutable image ghcr.io/oteryn/oteryn-platform@sha256:590d8a4cd5321a9cd66bbaea3aaccbeda2d53ea65cbfcdd75d31fbcdc8630d70.
  - Resulting-main Gateway source SHA is 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b with immutable image ghcr.io/oteryn/oteryn-game-gateway@sha256:3c541cec71e084e4827350380ab00eddaae37e4f4250ae643ee04d9bae037054.
  - Canary remained ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - Resulting-main Deploy Synology Staging run 34218320510 job 102035271707 completed success; Deploy prebuilt images ran from 2026-09-08T11:02:04Z to 2026-09-08T11:15:43Z and the overall job completed at 2026-09-08T11:15:51Z.
  - The guarded deploy workflow invoked deploy-impact.sh and logged Synology staging deploy profile: full because the accumulated runtime/control impact included .github/workflows/build-synology-staging-images.yml.
  - The same resulting-main deployment verified internal TLS Gateway to Platform and Gateway to Canary session issuer, Gateway identity and isolation, MFA QR, canonical public HTTPS origins, LAN game endpoint 192.168.1.2:7172, and World Registry route 1 online with login enabled.
  - Resulting-main deployment ended with Oteryn Synology staging deployment is healthy; rollback step was skipped.
  - Initial resulting-main Agent Governance run 34218264203 failed only because GitHub had auto-closed governing Issue 1345 at merge; terminal PR archive_pending ownership itself was valid.
  - Issue 1345 was reopened and remains OPEN; rerun job 102038660174 completed success with live governing-Issue ownership, live active-task ownership and Control Room all green.
  - Source branch perf/1345-synology-platform-reconcile was auto-deleted after Merge Queue integration; branch search returned no retained source ref.
derived:
  - The implementation can reduce unrelated staging churn for schema-stable Platform-only runtime changes without weakening immutable component identity or the canonical full fallback.
  - The control-plane merge itself was intentionally not a platform-reconcile benchmark because its own workflow/router changes correctly forced full reconciliation.
unknown:
  - No genuine qualifying product release has yet exercised platform-reconcile on protected main, so live platform-reconcile wall-time and measured savings remain unknown.
  - Issue 1345 remains open specifically for that future genuine live proof; no synthetic or no-op product commit was created to manufacture timing evidence.
conflicts: []
first_failure:
  marker: Synology Rollback Contract initial exact-head rename regression repaired
  evidence: run 34216148757 job 102028235933 failed after the first design moved the canonical full implementation away from deploy.sh; the final design restores deploy.sh blob 0ab49b18416099fdf26db64090d49c4258fad776 and adds deploy-impact.sh as a separate router.
rejected_hypotheses:
  - treating all runtime paths as fast without schema/component boundaries would not be fail-closed
  - restarting Gateway, Canary, MariaDB or Redis merely because Platform application code changed is unnecessary when their provenance and schema contracts are unchanged
  - rewriting or renaming the mature deploy.sh migration/recovery path increases regression risk and was rejected after the exact-head contract failure
  - weakening or redirecting existing rollback/recovery assertions would hide a path-identity regression rather than fixing it
  - creating a no-op product commit to obtain timing evidence would not be representative and was explicitly rejected
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-platform-reconcile.md
validation:
  - command: PR 1349 final lifecycle exact-head GitHub Actions on e1882f1e5cb627746f90140211337861d5e31ad6
    result: PASS
    evidence: all nine triggered workflows completed success; CI run 34217776186 required platform-gate job 102034192836 success
  - command: Merge Queue CI on candidate 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b
    result: PASS
    evidence: run 34218043488 platform-gate job 102034958305 success
  - command: resulting-main Build Synology Staging Images
    result: PASS
    evidence: run 34218264210 completed success with exact immutable Platform and Gateway provenance and no ordinary-main deploy-runner publication
  - command: resulting-main Deploy Synology Staging full fallback
    result: PASS
    evidence: run 34218320510 job 102035271707 completed success with profile full, final healthy marker and rollback skipped
  - command: resulting-main Agent Governance after reopening governing Issue 1345
    result: PASS
    evidence: rerun job 102038660174 completed success with all live ownership checks green
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR 1349 merged through protected Merge Queue and the dedicated source branch has no retention purpose
source_branch_evidence: branch search after merge returned no perf/1345-synology-platform-reconcile ref; protected main is 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b
```

## Remaining issue-level observation

Issue #1345 intentionally remains open. A future genuine qualifying protected-main product change may provide the first `platform-reconcile` timing proof. When that occurs, create a separate active measurement/closeout packet if repository governance requires active ownership; do not reactivate this completed implementation task and do not manufacture a no-op product commit.

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, or weakening of Merge Queue / `platform-gate` occurred.