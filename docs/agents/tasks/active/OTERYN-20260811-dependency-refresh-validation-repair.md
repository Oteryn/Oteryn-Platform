---
task_id: OTERYN-20260811-dependency-refresh-validation-repair
mode: implementation
issue: 691
branch: fix/issue-691-portal-exhaustive-trigger-coupling
status: implementing
project_lane: oteryn-platform-core
---

# Issue #691 post-merge validation repair

## Goal

Repair the deterministic Portal Exhaustive exact-head trigger-coupling defect exposed by Composer-wave final-head run `31490875188`, revalidate the dependency refresh, then correct the premature lifecycle archive/closure.

## Scope

```yaml
owned_paths:
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/acceptance-validation.yml
  - docs/agents/tasks/active/OTERYN-20260811-dependency-refresh-validation-repair.md
  - docs/agents/tasks/archive/OTERYN-20260811-dependency-refresh-validation-repair.md
risk: HEIGHTENED
production_mutation: forbidden
external_repository_writes: forbidden
```

## Verified failure

- PR #997 final head `3a66ffb583b93570249cbe0295ad44295fecf625` passed Composer install and all observed dependency/application gates except Portal Exhaustive Audit.
- Portal Exhaustive run `31490875188`, job `93776909451`, failed only in `Await exact-head strictness execution evidence` after 25 minutes.
- The step unconditionally required exact-head success for `Wiki Reconciliation Acceptance`, `Editorial Media Acceptance`, and `Acceptance E2E and Visual UX`.
- `Acceptance E2E and Visual UX` ran and passed; Wiki and Editorial Media were never emitted because their path filters do not cover the dependency-only `composer.lock` change that triggers Portal Exhaustive.
- Therefore this is a trigger-coupling defect in the audit workflow contract, not a Composer/package regression.
- Issue #691 was reopened after PR #998 prematurely archived/closed it before this supplemental failure became terminal.

## Repair contract

1. Every PR/push path that can emit Portal Exhaustive must also be able to emit all three workflows that Portal Exhaustive waits for.
2. Preserve fail-closed exact-head execution evidence; do not weaken the audit by treating absent workflows as success.
3. Add a deterministic validator so future trigger drift fails immediately instead of waiting 25 minutes.
4. Do not touch Issue #487-owned product/evidence paths in PR #986.

## Checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T14:58:00+02:00
base_main: 681455739a054f344dc0e9478ff79821ac4a401d
branch: fix/issue-691-portal-exhaustive-trigger-coupling
status: implementing
first_failure:
  marker: portal-exhaustive-missing-dependent-workflow-runs
  evidence: run 31490875188 job 93776909451 timed out with only Acceptance E2E present; Wiki Reconciliation and Editorial Media had no exact-head runs.
conflicts:
  - PR #986 owns PORTAL_STRICTNESS_EVIDENCE.json and related Issue #487 acceptance files; this repair intentionally excludes those paths.
next_action: Align dependent workflow PR/push triggers with the Portal Exhaustive trigger surface, add a static coupling validator, then open one corrective PR and require exact-head Portal Exhaustive plus dependent workflows and repository-selected CI to pass.
```
