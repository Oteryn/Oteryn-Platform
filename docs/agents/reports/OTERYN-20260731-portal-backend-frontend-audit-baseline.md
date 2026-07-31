# Portal backend/frontend audit baseline

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Captured: `2026-07-31T16:43:00Z`  
Audit target: `main` at `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Executive baseline

The audit is one cohesive phased task. A single canonical surface inventory is required to reconcile backend, frontend, integration, state, browser and deployment evidence; splitting before that reconciliation would create competing truth sources and duplicate severity normalization.

The live repository baseline does not equal the latest proven staging deployment. Staging is directly proven only for source `717977f252b09b9b2e979f8110b7f48b88682223` through workflow run `30633745660`, job `91166065335` and sanitized artifact `8794683627`. The audit target is newer, so its staging deployment is `UNKNOWN`. Production is also `UNKNOWN`; no direct exact-release evidence under Issue #91 was established in this baseline.

## Repository and coordination state

- `REPO_MAIN`: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`.
- Parent Issue #326: open; requires exhaustive integrated backend/frontend/state/browser classification.
- Related Issue #365: open; prior mobile zero-retry runs proved missing transient Wiki publication flash and concurrent thumbnail HTTP 500 responses, without proving one cause.
- `ACTIVE_WORK.md` states that no tasks are active, but live open PRs include task records and owned paths. This is a `CONFLICT`; live PRs and task records control.

## Open PR delta

Seven PRs were open at baseline: `#338`, `#335`, `#328`, `#218`, `#189`, `#182` and `#116`.

PR #338 is the material portal delta. It changes Game Catalog consumer/admin behavior, `resources/views/game-catalog/admin/snapshot.blade.php` and `scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`. Those changes are `OPEN_PR_ONLY` and must not be counted as `REPO_MAIN` or edited by this audit.

PR #335 may affect staging runner/container recovery but does not prove a deployment. PR #328 is a contract-only character rename discovery. The remaining PRs are unrelated to current portal implementation or represent separate evidence collection.

## Tool and manifest baseline

| Tool | Installed runtime | Manifest/workflow evidence | State |
|---|---|---|---|
| PHP | UNKNOWN | `^8.5`; CI uses `8.5` | PROVEN constraint, runtime UNKNOWN |
| Composer | UNKNOWN | CI uses `v2` | PROVEN workflow, runtime UNKNOWN |
| Node | UNKNOWN | pending checkout/runtime capture | UNKNOWN |
| npm | UNKNOWN | pending checkout/runtime capture | UNKNOWN |
| Playwright | UNKNOWN | package pins `1.60.0` | PROVEN manifest, runtime UNKNOWN |

The requested Composer and acceptance commands exist in the current manifests. They have not been run in this session.

## CI baseline

The connector returned no legacy combined-status contexts and no PR-triggered workflow runs for the audit-target commit. This is only the exact result of those inspected endpoints; it is not a claim that no push workflow ever ran.

## Environment baseline

| Environment | Exact SHA | Classification | Result |
|---|---|---|---|
| Repository main | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | REPO_MAIN | audit target |
| Open PR #338 | `8baec8d66c1bab0b618684096300ab491dacacb4` | OPEN_PR_ONLY | excluded from main implementation claims |
| Staging | `717977f252b09b9b2e979f8110b7f48b88682223` | STAGING_PROVEN | exact older deployment boundary |
| Audit target on staging | none proven | UNKNOWN | not promoted from repository evidence |
| Production | none proven | UNKNOWN | no production action performed |

## Execution limitation

The current sandbox cannot resolve `github.com`, so it cannot clone the repository or run route dumps, PHP/Composer/npm validators or Playwright. GitHub live-state inspection and audit-document writes were completed through the connected GitHub API. This is an infrastructure limitation, not a product finding.

## Phase decision

Phase 1 requires Codex or another full-checkout terminal/browser session on the existing task and branch. That session must preserve the immutable audit target, run Control Room/checkpoint validation, record installed tool versions and working-tree state, derive the runtime route inventory, reconcile all existing ledgers and write the first full machine-readable inventory matrix.

No implementation, merge, deployment or production action is authorized.