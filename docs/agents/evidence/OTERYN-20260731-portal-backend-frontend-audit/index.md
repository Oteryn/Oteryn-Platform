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
| Issue #365 prior reproduction | PROVEN | CI_PROVEN | runs `30562698853` / `30578806660`; audit-target reproduction not run |
| Latest exact staging deployment | PROVEN | STAGING_PROVEN | source `717977f252b09b9b2e979f8110b7f48b88682223`; run `30633745660`; job `91166065335`; artifact `8794683627` |
| Audit-target deployment to staging | UNKNOWN | UNKNOWN | exact staging source differs from audit target |
| Production deployment and availability | UNKNOWN | UNKNOWN | no direct exact-release proof inspected under Issue #91 |
| Connector-only checkout state | PROVEN | UNKNOWN | local GitHub DNS unavailable; no repository checkout in this session |

## Audit PR exact-head CI

Audit PR head `220ae3d231d4269bf80fc51409f5b3b95a7975be` passed:

- Phase 7 Production-Like Validation `30648697391`;
- Agent Governance `30648697395`;
- Game Auth Ticket Concurrency `30648697408`;
- Edge Security Emulation `30648697440`;
- Platform DB Outage Validation `30648697392`;
- CI `30648697401`.

The branch changed audit documentation only. These runs do not establish deployment or fresh exhaustive browser closure.

## Recovered direct CI/runtime evidence

### Portal Acceptance Contract

- Exact source SHA: `fdb45a4325949d3ab1c4860e3a4527553f11c789`.
- Run `30633216358`, strict job `91164376176`.
- Artifact `8794204786`, digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`.
- Result: `PASS`.

Observed closure:

- 27 canonical surfaces and 228 classified manifest route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 121 Blade views, 26 structural views, two exclusions and zero orphan views;
- 400 navigation references and 30 direct-entry routes;
- 43 product/backend/frontend capability records with no validator error;
- 27 dimension and media applicability records with no validator error;
- content-scale validator classified only 18 base-manifest surfaces.

### Critical browser evidence

- Exact source SHA: `fdb45a4325949d3ab1c4860e3a4527553f11c789`.
- Run `30633216753`, job `91164367653`.
- Artifact `8794373786`, digest `sha256:3dd06aeee7436d4eb9ba3ec23b5e3b8684e987d7f58dcc4a247b54df48f0adeb`.
- Runtime: real Laravel HTTP, isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog.
- Playwright retries: `0`.

Results:

- smoke: `7/7` PASS;
- Chromium/Firefox/WebKit portability: `36/36` PASS;
- desktop/tablet/mobile responsive: `42/42` PASS;
- resilience: `2/2` PASS;
- accessibility: `9/9` PASS.

Artifact markers explicitly state `FULL_ACCEPTANCE_NOT_EXECUTED`, `VISUAL_UX_NOT_EXECUTED` and `PRODUCTION_SMOKE_PENDING`.

### Relation to frozen audit target

Comparison from `fdb45a4325949d3ab1c4860e3a4527553f11c789` to the audit target changes documentation and `config/marketplace.php`; the Marketplace config blob is byte-identical at both SHAs. Runtime equivalence is recorded as `DERIVED`. Direct CI remains `CI_PROVEN` only for its exact source SHA and is not promoted to audit-target deployment proof.

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
- Strict route/view policy: 30 direct-entry routes, two explicit view exclusions.

Primary sources include route modules, localization registration, the base coverage manifest and fragments, backend/frontend ledger, dimension fragments and route/view/navigation inventory.

## Phase 2 capability evidence

- Capability count: `43`.
- Implemented: `23`.
- Partial: `3`.
- Missing: `14`.
- Not applicable: `3`.
- User-facing backend-implemented/frontend-not-implemented records: `0`.
- User-facing frontend-implemented/backend-not-implemented records: `0`.

Open PR #338 does not change the `REPO_MAIN` missing classification for public NPC/shop knowledge. Open PR #328 does not change the missing character-rename classification.

## Phase 3–5 state/browser evidence

### Global errors

Dedicated browser evidence covers `404`, `419`, `429` and `500` in EN/PL at desktop, tablet and mobile with zero retries. The localized `503` view exists and a bounded online-dependency 503 recovery scenario exists, but `503` is absent from the dedicated global error matrix.

### Media

All 27 surfaces have media applicability classification. Three rendered consumers require normal, missing, broken/integrity-failed and no-image states; all 12 required evidence records are present.

### Content scale

The validator passes for 18 base-manifest surfaces but does not load the six fragment files. Nine canonical surfaces therefore lack explicit content-scale applicability/evidence classification in that strict contract.

### Accessibility

Nine bounded zero-retry scenarios pass. No one-record-per-rendered-surface fail-closed accessibility applicability matrix was found; reduced-motion applicability remains `UNKNOWN`.

## Issue #365 evidence

Historical exact runs:

- SHA `35f39b48233b186502cbdcc05aec7ffc40e78fc7`, run `30562698853`, job `90939481510`, artifact `8767657461`;
- SHA `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`, run `30578806660`, job `90993603962`, artifact `8773887288`.

Both mobile runs failed to find the expected publication `role=status` message while the redirected form showed `Published`, version 3 and `Unpublish to draft`. Issue #365 separately preserves multiple thumbnail HTTP 500 responses. No common cause is proven. Audit-target reproduction remains `NOT_RUN` / `UNKNOWN`.

## Finding index

| Finding | Severity | State | Summary |
|---|---|---|---|
| `OTERYN-AUDIT-P35-006` | HIGH | PROVEN historical CI | Wiki thumbnail requests returned HTTP 500 on two historical heads |
| `OTERYN-AUDIT-P35-001` | MEDIUM | PROVEN | content-scale strict closure omits nine canonical surfaces |
| `OTERYN-AUDIT-P35-002` | MEDIUM | PROVEN | global error matrix omits HTTP 503 |
| `OTERYN-AUDIT-P35-003` | MEDIUM | DERIVED | accessibility evidence is not fail-closed per delivered surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | PROVEN historical CI | durable Wiki publication lacked transient success feedback |
| `OTERYN-AUDIT-P1-001` | LOW | CONFLICT | ACTIVE_WORK conflicts with live PR/task state |
| Other audit records | INFO / UNKNOWN | mixed | environment gates, intentional gaps and unexecuted current-target checks |

## Manifest/tool evidence

- `composer.json`: PHP `^8.5`; Laravel `^13.8`; scripts `analyse`, `format`, `format:check`, `test`.
- `.github/workflows/ci.yml`: PHP `8.5`; Composer `v2`; MariaDB `11.8`.
- `scripts/acceptance/package.json`: Playwright `1.60.0`.
- Installed local PHP, Composer, Node and npm versions remain `UNKNOWN` because this session has no checkout/runtime shell.

## Artifacts

- `baseline.json` — machine-readable baseline and environments.
- `phase-1-surface-inventory.json` — 27-surface inventory and Phase 1 findings.
- `phase-2-capability-reconciliation.json` — 43-capability backend/frontend/integration matrix.
- `phase-3-5-state-browser-evidence.json` — state, browser, deployment and Issue #365 evidence matrix.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-baseline.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-1-inventory.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-2-capabilities.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-3-5-states-browser.md`.
- Final consolidated report — pending current-target validation boundary decision.
- Independent validator artifact — pending fresh checkout-capable validator session.

## First relevant failure

`git ls-remote https://github.com/blakinio/Oteryn-Platform.git refs/heads/main` failed in the current sandbox with `Could not resolve host: github.com`. GitHub API and preserved CI artifacts enabled static and historical evidence reconciliation, but not a focused audit-target Wiki execution or fresh independent validator run.
