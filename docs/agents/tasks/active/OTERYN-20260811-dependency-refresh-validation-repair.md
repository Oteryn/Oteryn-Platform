---
task_id: OTERYN-20260811-dependency-refresh-validation-repair
mode: implementation
issue: 691
branch: fix/issue-691-portal-exhaustive-trigger-coupling
status: validating
project_lane: oteryn-platform-core
---

# Issue #691 post-merge validation repair

## Goal

Repair the deterministic Portal Exhaustive exact-head trigger-coupling defect exposed by Composer-wave final-head run `31490875188`, revalidate the dependency refresh, then correct the premature lifecycle archive/closure.

## Scope

```yaml
owned_paths:
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/portal-exhaustive-acceptance.yml
  - .github/workflows/portal-exhaustive-trigger-coupling.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
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
- `Acceptance E2E and Visual UX` ran and passed; Wiki and Editorial Media were never emitted because their path filters did not cover the dependency-only `composer.lock` change that triggered Portal Exhaustive.
- Therefore this is a trigger-coupling defect in the audit workflow contract, not a Composer/package regression.
- Issue #691 was reopened after PR #998 prematurely archived/closed it before this supplemental failure became terminal.

## Repair contract

1. Every PR/push path that can emit Portal Exhaustive must also emit the exact-head strictness companions that Portal Exhaustive waits for.
2. Preserve fail-closed exact-head execution evidence; do not weaken the audit by treating absent workflows as success.
3. Detect trigger drift both inside Portal Exhaustive and independently when any coupled workflow file changes.
4. Do not touch Issue #487-owned product/evidence paths in PR #986.

## Implementation

- Wiki Reconciliation and Editorial Media include the complete Portal Exhaustive PR/push path surface in addition to their own narrower triggers.
- New `Portal Exhaustive Acceptance E2E` is a same-trigger companion that calls the existing reusable `acceptance-validation.yml` critical profile with zero retries; the primary Acceptance workflow itself is unchanged.
- Portal Exhaustive validates trigger-set inclusion before polling and requires Wiki Reconciliation, Editorial Media, and the dedicated acceptance companion.
- New `Portal Exhaustive Trigger Coupling` independently validates the same inclusion contract whenever any coupled workflow definition changes, so companion-side drift cannot silently bypass the audit validator.

## Checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T15:12:00+02:00
base_main: 681455739a054f344dc0e9478ff79821ac4a401d
branch: fix/issue-691-portal-exhaustive-trigger-coupling
pr: 1000
status: validating
changed_paths:
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/portal-exhaustive-acceptance.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/portal-exhaustive-trigger-coupling.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260811-dependency-refresh-validation-repair.md
first_failure:
  marker: portal-exhaustive-missing-dependent-workflow-runs
  evidence: run 31490875188 job 93776909451 timed out with only Acceptance E2E present; Wiki Reconciliation and Editorial Media had no exact-head runs.
proven:
  - main 681455739a054f344dc0e9478ff79821ac4a401d contains the Composer resolution from merged PR #997 and the premature archive PR #998.
  - Issue #691 is reopened and PR #1000 is the dedicated corrective delivery PR.
  - Current branch diff excludes every path owned by open PR #986.
  - An earlier #1000 generation already proved the repaired trigger architecture emits Portal Exhaustive, Wiki Reconciliation, Editorial Media and Portal Exhaustive Acceptance on the same head.
  - Trigger coupling is fail-closed for both pull_request and push events and has an independent workflow-definition drift gate.
unknown:
  - terminal exact-head results of the final checkpoint generation
conflicts:
  - PR #986 owns PORTAL_STRICTNESS_EVIDENCE.json and related Issue #487 acceptance files; this repair intentionally excludes those paths.
next_action: Freeze this head, require exact-head Agent Governance/CI, Portal Exhaustive, Portal Exhaustive Trigger Coupling and all strictness companions to reach terminal success, perform whole-diff and independent review, then merge #1000 and correct the Issue #691 archive lifecycle.
```
