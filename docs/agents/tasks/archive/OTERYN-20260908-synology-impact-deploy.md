---
task_id: OTERYN-20260908-synology-impact-deploy
governing_issue: 1341
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Issue 1341 and overlapping active Synology/portal tasks
  - current Synology build/deploy workflows and rollback contracts
optional_reads: []
---

# OTERYN-20260908-synology-impact-deploy

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1341 — canonical lifecycle authority remains open for the first genuine protected-main fast-path proof and any explicitly bounded follow-up expansion.

Deliver the first fail-closed impact-based staging fast path so presentation-only Platform releases do not pay the full MariaDB/Canary/Gateway reconciliation cost, while preserving immutable provenance, rollback/recovery and the existing full deployment path for mixed or risky changes.

## Acceptance criteria

- [x] classify accumulated live deployed-release -> target-release impact, not only the latest commit;
- [x] permit fast deployment only for a narrow proven presentation allowlist and ignore only known non-runtime paths;
- [x] preserve exact Platform/Gateway/Canary provenance and known schema compatibility;
- [x] recreate only Platform plus an existing tightly coupled Marketplace runtime when required;
- [x] prove MariaDB, Redis, Canary, internal proxy and Gateway are not restarted by the fast path contract;
- [x] restore the previously proven Platform and release metadata on fast-path failure;
- [x] retain the existing full migration/health/rollback path for all non-fast candidates;
- [x] focused Synology contracts, exact-head required checks and Merge Queue `platform-gate` passed before protected-main integration;
- [x] resulting protected-main deployment selected the full profile for deployment-control changes and completed healthy without rollback.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-impact-deploy.md
modules:
  - Synology staging deployment routing
  - Synology staging validation
dependencies:
  - protected main 3cbcfbd79c00cfb1edd95fa81b74557788f88320
  - Issue 1333 remains separate portal product work
  - Issue 1339 remains separate runner/autostart work
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T09:52:00Z
head: 3cbcfbd79c00cfb1edd95fa81b74557788f88320
branch: perf/1341-synology-impact-deploy
pr: 1342
status: completed
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-impact-deploy.md
proven:
  - PR 1342 exact head 63ec74fd8b2d9d01e2dd9c3f01b1163c1f4115f9 completed the required exact-head workflows, including focused Synology contracts, Agent Governance, Phase 7 and CI platform-gate.
  - Merge Queue candidate 3cbcfbd79c00cfb1edd95fa81b74557788f88320 passed required platform-gate in CI run 34211040762 and became protected main through the normal queue.
  - Protected main is exactly 3cbcfbd79c00cfb1edd95fa81b74557788f88320 with required platform-gate protection after PR 1342 integration.
  - Resulting-main Build Synology Staging Images run 34211239918 succeeded with validation/classification, zero image-build jobs and exact-provenance staging dispatch for this deployment-package-only change.
  - Resulting-main Deploy Synology Staging run 34211275963 job 102012609714 completed success; rollback was skipped and the final staging health contract passed.
  - Resulting-main deployment selected `full` because the accumulated runtime impact included deploy/synology/scripts/deploy.sh, proving deployment-control changes fail closed to the existing full path.
  - The resulting-main `Deploy prebuilt images` step ran from 09:41:19 to 09:50:46 UTC, 9 minutes 27 seconds, confirming the cost the fast path is designed to avoid for eligible low-impact releases.
  - The fast path classifies the accumulated diff from persisted proven current-release SHA to exact target SHA, uses a narrow resources/lang/static-public allowlist, verifies unchanged Gateway/Canary/schema/world identity, recreates only Platform plus an existing Marketplace scheduler when needed, runs bounded smoke checks, preserves core service container identities and restores the previous Platform/release state on post-swap failure.
  - Source branch perf/1341-synology-impact-deploy was automatically removed after Merge Queue integration; a fresh branch search returned no retained source ref.
derived:
  - A genuine eligible presentation-only release should avoid the measured 9 minute 27 second full-stack deploy body, but its protected-main wall time remains unmeasured until a real qualifying release reaches main.
unknown:
  - representative protected-main fast-path wall time for the first genuine eligible release
  - safe explicit impact maps for isolated non-presentation Platform modules beyond the first narrow allowlist
conflicts: []
first_failure:
  marker: resulting-main Agent Governance terminal lifecycle mismatch after implementation merge
  evidence: Issue 1341 was automatically closed by the PR body despite the intended live-proof hold and the active packet still referenced merged PR 1342; Issue 1341 was reopened and this archive closeout releases terminal implementation ownership without changing runtime behavior
rejected_hypotheses:
  - copying individual files into a running container would break immutable release provenance
  - treating all public files as presentation-only would incorrectly include public/index.php
  - treating mixed portal PR 1334 as a fast-path proof would be unsafe because its diff includes app/Admin/AdminAuthorization.php
changed_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-impact-deploy.md
validation:
  - command: PR 1342 exact-head GitHub Actions on 63ec74fd8b2d9d01e2dd9c3f01b1163c1f4115f9
    result: PASS
    evidence: focused Synology contracts, Agent Governance, CI platform-gate and Phase 7 completed successfully before Merge Queue
  - command: Merge Queue CI run 34211040762 on candidate 3cbcfbd79c00cfb1edd95fa81b74557788f88320
    result: PASS
    evidence: required platform-gate job 102012390886 completed success
  - command: resulting-main Build Synology Staging Images run 34211239918
    result: PASS
    evidence: classifier and deployment validation passed, component image-build job skipped, exact-provenance staging dispatch succeeded
  - command: resulting-main Deploy Synology Staging run 34211275963 job 102012609714
    result: PASS
    evidence: full profile selected for deploy.sh impact, deploy completed healthy, rollback skipped, final World Registry and staging health probes passed
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR 1342 merged through protected Merge Queue and the dedicated implementation ref has no retention purpose
source_branch_evidence: fresh GitHub branch search after merge returned no perf/1341-synology-impact-deploy ref
```

## Notes

This archive closes implementation ownership only. Issue #1341 remains open for the first genuine fast-path timing proof and for separately governed expansion to additional explicitly bounded low-impact Platform modules. No production deployment authority was granted or exercised.