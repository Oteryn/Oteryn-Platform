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
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-core.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
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
updated_at: 2026-09-08T10:20:00Z
head: 01a41276d3f345da94b35ad00f093a9ca66482a3
branch: perf/1345-synology-platform-reconcile
pr: none
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-core.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - scripts/ci/classify_synology_builds.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
proven:
  - protected main is 01a41276d3f345da94b35ad00f093a9ca66482a3 after PR 1343 Merge Queue integration
  - Issue 1341 presentation fast path is integrated and its implementation packet is archived
  - protected-main Deploy run 34212996824 selected full because the accumulated range included app/Admin/AdminAuthorization.php from portal PR 1334
  - the existing health-check.sh validates Platform, Gateway, Canary, internal TLS, Gateway identity, canonical public origin, MFA and LAN endpoint without itself recreating services
derived:
  - a second Platform-only reconcile profile can reuse the same immutable provenance and rollback preflight while running the broader existing health contract on preserved services
  - full-stack recreation provides no additional component identity when only the Platform image changes and schema/deployment/Gateway inputs are unchanged
unknown:
  - exact-head validation result for the new profile
  - live wall-time of the first genuine Platform-reconcile product release
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - treating all runtime paths as fast without schema/component boundaries would not be fail-closed
  - restarting Gateway, Canary, MariaDB or Redis merely because Platform application code changed is unnecessary when their provenance and schema contracts are unchanged
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-synology-platform-reconcile.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation has not yet been committed
blockers:
  - none
next_action: implement the bounded Platform-reconcile wrapper and focused contract tests, then open one PR for Issue 1345
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation is active
source_branch_evidence: pending
```

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, or weakening of Merge Queue / `platform-gate` is authorized.