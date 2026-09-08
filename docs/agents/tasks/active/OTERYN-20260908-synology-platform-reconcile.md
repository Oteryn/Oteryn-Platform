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

Governing GitHub Issue: #1345 — canonical lifecycle authority for this task.

Add a second fail-closed Synology staging profile for schema-stable Platform-only runtime changes. Keep the existing presentation fast path unchanged; broader Platform-only changes must recreate only Platform plus its existing Marketplace scheduler, validate the preserved stack with the existing health contract, and retain the current full path for mixed, schema, Gateway, deployment-control, or unknown changes.

## Acceptance criteria

- [ ] existing presentation-only `platform-fast` behavior remains intact;
- [ ] accumulated known Platform-only runtime changes can select `platform-reconcile`;
- [ ] database/schema, Gateway, deployment-control and unknown runtime changes remain full;
- [ ] Platform reconcile preserves MariaDB, Redis, Canary, internal proxy and Gateway container identities;
- [ ] Platform reconcile runs the existing full staging health contract before release-state promotion;
- [ ] Platform reconcile restores the prior proven Platform/release state on failure;
- [ ] exact-head Synology contracts, required CI and `platform-gate` pass before Merge Queue integration;
- [ ] resulting-main deployment of the control-plane change proves the existing full fallback remains healthy.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-core.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
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
updated_at: 2026-09-08T10:33:00Z
head: d35e7b164e936a87a8873bfff0389cbe243d8980
branch: perf/1345-synology-platform-reconcile
pr: 1349
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-core.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
proven:
  - protected main was 01a41276d3f345da94b35ad00f093a9ca66482a3 when the implementation branch was created
  - Issue 1341 presentation fast path is integrated and its implementation packet is archived
  - protected-main Deploy run 34212996824 selected full because the accumulated range included app/Admin/AdminAuthorization.php from portal PR 1334
  - the existing health-check.sh validates Platform, Gateway, Canary, internal TLS, Gateway identity, canonical public origin, MFA and LAN endpoint without itself recreating services
  - the former deploy.sh implementation is preserved byte-for-byte as deploy-core.sh using blob 0ab49b18416099fdf26db64090d49c4258fad776
  - PR 1349 is the canonical implementation PR for Issue 1345
  - the workflow change adds only deploy-core.sh to pull-request/push path routing and shell syntax validation
  - deploy-core.sh is classified as a runtime deployment input so future changes cannot bypass protected-main staging reconciliation
derived:
  - a second Platform-only reconcile profile can reuse the same immutable provenance and rollback preflight while running the broader existing health contract on preserved services
  - full-stack recreation provides no additional component identity when only the Platform image changes and schema/deployment/Gateway inputs are unchanged
  - control-plane changes in PR 1349 must deliberately fall back to the full resulting-main deployment path
unknown:
  - exact-head validation result for PR 1349
  - live wall-time of the first genuine Platform-reconcile product release
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - treating all runtime paths as fast without schema/component boundaries would not be fail-closed
  - restarting Gateway, Canary, MariaDB or Redis merely because Platform application code changed is unnecessary when their provenance and schema contracts are unchanged
  - rewriting the mature migration/recovery path inside the new entrypoint would increase regression risk; the existing core is preserved instead
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-core.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
validation:
  - command: PR 1349 exact-head GitHub Actions
    result: NOT_RUN
    evidence: task is now bound to PR 1349; inspect the new exact-head workflow generation after this checkpoint commit
blockers:
  - none
next_action: inspect PR 1349 exact-head Synology, Agent Governance and required CI checks; repair the first failing contract before Merge Queue
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation PR 1349 is still active
source_branch_evidence: pending
```

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, or weakening of Merge Queue / `platform-gate` is authorized.