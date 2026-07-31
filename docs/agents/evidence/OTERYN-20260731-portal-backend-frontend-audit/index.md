# OTERYN-20260731 portal backend/frontend audit evidence index

## Audit identity

- Task: `OTERYN-20260731-portal-backend-frontend-audit`
- Audit target: `REPO_MAIN` SHA `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`
- Baseline time: `2026-07-31T16:43:00Z`
- Branch: `audit/OTERYN-20260731-portal-backend-frontend-audit`
- Draft PR: `#381`
- Parent: Issue `#326`
- Related defect evidence: Issue `#365`

## Evidence-state rules

- `PROVEN`: direct code, runtime, test, CI, artifact or live GitHub evidence.
- `DERIVED`: explicit inference from `PROVEN` facts.
- `UNKNOWN`: evidence is insufficient.
- `CONFLICT`: authoritative sources disagree.

Environment classes remain independent: `REPO_MAIN`, `OPEN_PR_ONLY`, `CI_PROVEN`, `STAGING_PROVEN`, `PRODUCTION_PROVEN`, `UNKNOWN`.

## Baseline evidence

| Evidence | State | Environment | Exact identity |
|---|---|---|---|
| Current audit target | PROVEN | REPO_MAIN | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` |
| Issue #326 scope | PROVEN | REPO_MAIN | open issue; exhaustive integrated completeness matrix required |
| Issue #365 prior reproduction | PROVEN | CI_PROVEN | runs `30562698853` / `30578806660`; current-main reproduction not yet run |
| Latest exact staging deployment | PROVEN | STAGING_PROVEN | source `717977f252b09b9b2e979f8110b7f48b88682223`; run `30633745660`; job `91166065335`; artifact `8794683627` |
| Audit-target deployment to staging | UNKNOWN | UNKNOWN | staging source predates audit target |
| Production deployment and availability | UNKNOWN | UNKNOWN | no direct exact-release proof inspected under Issue #91 |
| Local checkout/runtime execution | UNKNOWN | UNKNOWN | current sandbox cannot resolve `github.com` |

## Audit PR exact-head CI

Audit PR head `220ae3d231d4269bf80fc51409f5b3b95a7975be` passed:

- Phase 7 Production-Like Validation `30648697391`;
- Agent Governance `30648697395`;
- Game Auth Ticket Concurrency `30648697408`;
- Edge Security Emulation `30648697440`;
- Platform DB Outage Validation `30648697392`;
- CI `30648697401`.

The branch changed audit documentation only. These runs prove the checked application code plus audit documents at that PR head; they do not replace a fresh browser/route inventory run and do not prove deployment.

## Open PR delta inventory at baseline

| PR | State | Relevance to portal audit | Classification |
|---|---|---|---|
| #338 | draft, open | Game Catalog consumer/admin view and Wiki acceptance synchronization | OPEN_PR_ONLY |
| #335 | open | Synology boot repair and runner recovery | OPEN_PR_ONLY |
| #328 | draft, open | character rename contract discovery | OPEN_PR_ONLY |
| #218 | draft, open | Tibia client static analysis | OPEN_PR_ONLY, outside portal implementation |
| #189 | draft, open | Liquid20 evidence/retry docs | OPEN_PR_ONLY, outside portal implementation |
| #182 | draft, open | Liquid20 retry operation | OPEN_PR_ONLY, outside portal implementation |
| #116 | draft, open | scheduled E2E evidence collection task | OPEN_PR_ONLY |
| #381 | draft, open | this audit record, reports and evidence only | OPEN_PR_ONLY audit artifact |

PR #338 owns `scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs` and `resources/views/game-catalog/admin/snapshot.blade.php`; this audit does not edit those paths.

## Phase 1 inventory evidence

- Canonical surface count: `27`.
- Manifest named-route assignments: `228`.
- Base manifest surfaces: `18`.
- Sorted module-fragment surfaces: `9` across six fragment files.
- Rendered or rendered-with-resource groups: `26`.
- Supporting endpoint groups: `1`.
- Marketplace conditional surface groups: `3`.
- Route/view policy: strict closure enabled, 30 bounded direct-entry routes, two explicit view exclusions.
- Runtime `php artisan route:list --json`: `NOT_RUN`.
- Exact-target route/view/navigation validator: `NOT_RUN`.

Primary Phase 1 sources:

- `routes/web.php`, `routes/localization.php`, `routes/api.php`, `routes/internal.php` and inspected module route files;
- `app/Localization/LocalizedPublicRouteRegistrar.php`;
- `scripts/acceptance/coverage/portal-coverage-manifest.json` plus six `surfaces/*.json` fragments;
- `docs/testing/product-backend-frontend-completeness.json`;
- `scripts/acceptance/coverage/portal-evidence-dimensions.json` plus four fragments;
- `docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json` and endpoint/delegated exception files.

## Manifest/tool evidence

- `composer.json`: PHP `^8.5`; Laravel `^13.8`; scripts `analyse`, `format`, `format:check`, `test`.
- `.github/workflows/ci.yml`: PHP `8.5`; Composer `v2`; MariaDB `11.8`.
- `scripts/acceptance/package.json`: Playwright `1.60.0`; strict coverage, backend/frontend completeness, dimensions, media, route/view/navigation, responsive, portability, resilience, accessibility and account-lifecycle scripts.
- Installed Node and npm versions remain `UNKNOWN` until a full checkout/runtime session records them.

## Artifacts

- `baseline.json` — machine-readable baseline and environment classifications.
- `phase-1-surface-inventory.json` — machine-readable 27-surface inventory and Phase 1 findings.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-baseline.md` — human-readable baseline report.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-1-inventory.md` — Phase 1 route/surface reconciliation report.
- Final report — pending.
- Independent validator artifact — pending.

## First relevant failure

`git ls-remote https://github.com/blakinio/Oteryn-Platform.git refs/heads/main` failed in the current sandbox with `Could not resolve host: github.com`. The GitHub connector remained available for live inspection and allowed audit-only documentation writes. Runtime route dumps, local validators and Playwright execution were not claimed.
