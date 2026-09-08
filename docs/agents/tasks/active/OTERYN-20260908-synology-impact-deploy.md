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

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1341 — canonical lifecycle authority for this task.

Add a fail-closed impact-based staging fast path so presentation-only Platform releases do not pay the full MariaDB/Canary/Gateway reconciliation cost, while preserving immutable provenance, rollback/recovery and the existing full deployment path for mixed or risky changes.

## Acceptance criteria

- [x] classify accumulated live deployed-release -> target-release impact, not only the latest commit;
- [x] permit fast deployment only for a narrow proven presentation allowlist and ignore only known non-runtime paths;
- [x] preserve exact Platform/Gateway/Canary provenance and known schema compatibility;
- [x] recreate only Platform plus an existing tightly coupled Marketplace runtime when required;
- [x] prove MariaDB, Redis, Canary, internal proxy and Gateway are not restarted by the fast path contract;
- [x] restore the previously proven Platform and release metadata on fast-path failure;
- [x] retain the existing full migration/health/rollback path for all non-fast candidates;
- [ ] focused Synology contracts and exact-head required checks pass before Merge Queue integration.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-impact-deploy.md
modules:
  - Synology staging deployment routing
  - Synology staging validation
dependencies:
  - protected main 7bf4d26efb8cd9e25a4090f283d701f4d3a304e0
  - Issue 1333 remains separate portal product work
  - Issue 1339 remains separate runner/autostart work
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T09:33:00Z
head: UNKNOWN
branch: perf/1341-synology-impact-deploy
pr: 1342
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-impact-deploy.md
proven:
  - protected main is 7bf4d26efb8cd9e25a4090f283d701f4d3a304e0 with required platform-gate
  - live run 34204735730 spent about 68 seconds resolving runtime provenance and about 7 minutes 50 seconds inside deploy.sh
  - PR 1342 owns the implementation branch and staging automation paths for Issue 1341
  - implementation keeps the existing full deployment body and inserts the fast decision before any MariaDB TLS Canary migration or Gateway reconciliation starts
derived:
  - the largest safe latency reduction is to bypass full-stack reconciliation only when accumulated runtime impact is proven presentation-only
  - Synology independently classifies from persisted current-release SHA to target SHA so skipped intermediate releases cannot be hidden by a latest-commit-only classification
unknown:
  - exact-head CI result for PR 1342
  - representative protected-main fast-path wall time after integration
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - copying individual files into a running container would break immutable release provenance
  - treating all public files as presentation-only would incorrectly include public/index.php
changed_paths:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-impact-deploy.md
validation:
  - command: PR 1342 exact-head GitHub Actions
    result: NOT_RUN
    evidence: checks started after PR creation and current checkpoint binding
blockers:
  - none
next_action: inspect PR 1342 exact-head checks and repair any first failing contract before Merge Queue
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The fast path is an optimization of staging reconciliation only. It grants no production authority and must not weaken protected main, Merge Queue, `platform-gate`, immutable image identity, rollback or schema recovery.