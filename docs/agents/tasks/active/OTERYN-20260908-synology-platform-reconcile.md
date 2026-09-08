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

- [x] existing presentation-only `platform-fast` behavior remains intact;
- [x] accumulated known Platform-only runtime changes can select `platform-reconcile`;
- [x] database/schema, Gateway, deployment-control and unknown runtime changes remain full;
- [x] Platform reconcile preserves MariaDB, Redis, Canary, internal proxy and Gateway container identities;
- [x] Platform reconcile runs the existing full staging health contract before release-state promotion;
- [x] Platform reconcile restores the prior proven Platform/release state on failure;
- [x] exact-head Synology contracts, required CI and `platform-gate` pass before protected integration;
- [ ] resulting-main deployment of the control-plane change proves the existing full fallback remains healthy.

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
updated_at: 2026-09-08T10:52:00Z
head: 20f8eef3c01cbe3110dfbddb0a9d11ac5953aa8c
branch: perf/1345-synology-platform-reconcile
pr: 1349
status: ready
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
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
proven:
  - protected main is 01a41276d3f345da94b35ad00f093a9ca66482a3 before protected integration of PR 1349
  - Issue 1341 presentation fast path is integrated and its implementation packet is archived
  - protected-main Deploy run 34212996824 selected full because the accumulated range included app/Admin/AdminAuthorization.php from portal PR 1334
  - the existing health-check.sh validates Platform, Gateway, Canary, internal TLS, Gateway identity, canonical public origin, MFA and LAN endpoint without itself recreating services
  - PR 1349 is the canonical implementation PR for Issue 1345
  - initial exact-head cd449d0a735aa3218e4bce4e8b1c05b320eb4151 exposed one Synology Rollback Contract failure in run 34216148757 job 102028235933 because the first design had renamed the canonical deploy.sh implementation
  - canonical deploy.sh is restored exactly to pre-task blob 0ab49b18416099fdf26db64090d49c4258fad776; no rollback/recovery test is redirected or weakened
  - new Platform reconcile routing lives only in deploy/synology/scripts/deploy-impact.sh and guarded Deploy Synology Staging invokes that entrypoint for deploy actions
  - build routing classifies deploy-impact.sh as a runtime deployment input and both Synology validation workflows syntax-check it
  - database/schema, Gateway, deployment-control and unknown runtime paths remain outside the Platform-only allowlist and therefore delegate to canonical deploy.sh full behavior
  - presentation-only accumulated candidates are delegated to canonical deploy.sh so existing platform-fast behavior remains unchanged
  - final implementation exact head 20f8eef3c01cbe3110dfbddb0a9d11ac5953aa8c has Build Synology Staging Images 34217283791 success, Synology Rollback Contract 34217283680 success, Agent Governance 34217283679 success, CodeQL 34217283709 success, Edge Security Emulation 34217283692 success, Game Auth Ticket Concurrency 34217283821 success, Platform DB Outage Validation 34217283732 success, and Phase 7 Production-Like Validation 34217283689 success
  - final implementation CI run 34217283688 completed success and required platform-gate job 102033064988 completed success
  - PR 1349 has zero inline review threads and zero PR discussion comments at final implementation exact-head review
derived:
  - a second Platform-only reconcile profile can reuse the same immutable provenance and rollback preflight while running the broader existing health contract on preserved services
  - full-stack recreation provides no additional component identity when only the Platform image changes and schema/deployment/Gateway inputs are unchanged
  - keeping mature deploy.sh at its historical path minimizes regression surface because existing rollback/recovery tests and explicit operator workflows continue to exercise the same canonical implementation
  - control-plane changes in PR 1349 must deliberately fall back to the full resulting-main deployment path
unknown:
  - resulting-main full-fallback deployment result after protected integration
  - live wall-time of the first genuine Platform-reconcile product release
conflicts: []
first_failure:
  marker: Synology Rollback Contract initial exact-head rename regression repaired
  evidence: run 34216148757 job 102028235933 failed test_deploy_does_not_pull_all_images_twice_and_forces_tls_bootstrap_refresh because deploy.sh no longer contained the canonical full-path marker; the repair restores deploy.sh blob 0ab49b18416099fdf26db64090d49c4258fad776 and moves only the new router to deploy-impact.sh
rejected_hypotheses:
  - treating all runtime paths as fast without schema/component boundaries would not be fail-closed
  - restarting Gateway, Canary, MariaDB or Redis merely because Platform application code changed is unnecessary when their provenance and schema contracts are unchanged
  - rewriting or renaming the mature deploy.sh migration/recovery path increases regression risk and was rejected after the exact-head contract failure
  - weakening or redirecting existing rollback/recovery assertions would hide a path-identity regression rather than fixing it
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-rollback-contract.yml
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_deployment_package_routing.py
validation:
  - command: PR 1349 initial exact-head GitHub Actions on cd449d0a735aa3218e4bce4e8b1c05b320eb4151
    result: FAIL
    evidence: Synology Rollback Contract run 34216148757 job 102028235933 isolated the canonical deploy.sh path-identity regression; the design was repaired rather than suppressing the contract
  - command: PR 1349 final implementation exact-head GitHub Actions on 20f8eef3c01cbe3110dfbddb0a9d11ac5953aa8c
    result: PASS
    evidence: all nine triggered workflows completed success, including Build Synology 34217283791, Rollback 34217283680, Governance 34217283679, Phase 7 34217283689, DB Outage 34217283732 and CI 34217283688 with platform-gate job 102033064988 success
blockers:
  - none
next_action: archive task after PR 1349 reaches protected main and the resulting-main full-fallback staging deployment succeeds
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation PR 1349 has not yet reached protected main
source_branch_evidence: final implementation exact-head 20f8eef3c01cbe3110dfbddb0a9d11ac5953aa8c is validated and ready for protected integration
```

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, or weakening of Merge Queue / `platform-gate` is authorized.