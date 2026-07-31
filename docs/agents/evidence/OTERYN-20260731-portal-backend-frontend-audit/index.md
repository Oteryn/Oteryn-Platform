# OTERYN-20260731 portal backend/frontend audit evidence index

## Audit identity

- Task: `OTERYN-20260731-portal-backend-frontend-audit`
- Frozen target: `REPO_MAIN` SHA `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`
- Baseline: `2026-07-31T16:43:00Z`
- Branch: `audit/OTERYN-20260731-portal-backend-frontend-audit`
- Draft PR: `#381`
- Parent: Issue `#326`
- Related defect evidence: Issue `#365`
- Status: `BLOCKED` pending exact-target focused execution and independent validation

## Evidence-state rules

- `PROVEN`: direct code, runtime, test, CI, artifact or live GitHub evidence.
- `DERIVED`: explicit inference from proven facts.
- `UNKNOWN`: evidence is insufficient.
- `CONFLICT`: authoritative sources disagree.

Environment identity is independent from evidence state: `REPO_MAIN`, `OPEN_PR_ONLY`, `CI_PROVEN`, `STAGING_PROVEN`, `PRODUCTION_PROVEN`, `UNKNOWN`.

## Environment boundary

| Evidence | State | Environment | Exact identity |
|---|---|---|---|
| Frozen source target | PROVEN | REPO_MAIN | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` |
| Direct strict/browser source | PROVEN | CI_PROVEN | `fdb45a4325949d3ab1c4860e3a4527553f11c789` |
| Runtime equivalence to frozen target | DERIVED | REPO_MAIN | comparison changes documentation and byte-identical Marketplace config |
| Latest exact staging evidence | PROVEN | STAGING_PROVEN | source `717977f252b09b9b2e979f8110b7f48b88682223`; run `30633745660`; job `91166065335`; artifact `8794683627` |
| Frozen target deployed to staging | UNKNOWN | UNKNOWN | no exact deployment proof |
| Production release/availability | UNKNOWN | UNKNOWN | no direct exact-release evidence established |
| Exact-target Issue #365 reproduction | UNKNOWN | UNKNOWN | focused run not executed |
| Independent validator verdict | UNKNOWN | UNKNOWN | separate validator artifact pending |

## Canonical inventory

- Surface groups: `27`.
- Manifest named-route assignments: `228`.
- Base-manifest surfaces: `18`.
- Fragment surfaces: `9` across six sorted fragments.
- Rendered or rendered-with-resource groups: `26`.
- Supporting endpoint groups: `1`.
- Marketplace conditional surface groups: `3`.
- Capability records: `43` — 23 implemented, 3 partial, 14 missing, 3 not applicable.
- User-facing backend-only implemented promotions: `0`.
- User-facing frontend-only implemented promotions: `0`.

## Recovered direct CI/runtime evidence

### Portal Acceptance Contract

- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`;
- strict job `91164376176`;
- artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

Observed closure:

- 240 discovered named routes;
- 228 classified routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 121 Blade views, 26 structural views, two exclusions and zero orphan views;
- 400 navigation references and 30 bounded direct-entry routes;
- all 27 surfaces classified for media applicability;
- content-scale validator classified only 18 base-manifest surfaces.

### Critical browser evidence

- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216753`;
- job `91164367653`;
- artifact `8794373786`;
- digest `sha256:3dd06aeee7436d4eb9ba3ec23b5e3b8684e987d7f58dcc4a247b54df48f0adeb`;
- real Laravel HTTP, isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog;
- Playwright retries `0`.

Results:

- smoke `7/7` PASS;
- portability `36/36` PASS across Chromium, Firefox and WebKit;
- responsive `42/42` PASS across desktop, tablet and mobile;
- resilience `2/2` PASS;
- accessibility `9/9` PASS.

The artifact explicitly records `FULL_ACCEPTANCE_NOT_EXECUTED`, `VISUAL_UX_NOT_EXECUTED` and `PRODUCTION_SMOKE_PENDING`.

## Issue #365 corrected historical analysis

Reviewed exact artifacts:

| Run | SHA | Job | Artifact | Verified ZIP SHA-256 |
|---|---|---:|---:|---|
| `30562698853` | `35f39b48233b186502cbdcc05aec7ffc40e78fc7` | `90939481510` | `8767657461` | `8af4dedd1e213108a2599df303f45de7bf22caf603c180f7607ad5d8395a85c6` |
| `30578806660` | `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb` | `90993603962` | `8773887288` | `4a514e4a53d427599f07e0a22ad7cd918a154187e6ddd837e707ece2c14e96f2` |

### Publication flash

Both runs prove:

- desktop and tablet completed the Wiki flow;
- mobile alone lost the expected accessible publication flash;
- the redirected page showed `Published`, version 3 and `Unpublish to draft`;
- durable publication succeeded.

### Thumbnail traffic

Both runs recorded the same GET `/admin/wiki/media/{id}/thumbnail` pattern:

- desktop: 9 HTTP 500 responses across IDs 1, 3, 5;
- tablet: 12 across IDs 1, 3, 5, 7;
- mobile: 16 across IDs 1, 3, 5, 7, 9.

Static source and exact report ordering explain these responses:

1. `admin-wiki-editorial-media.spec.mjs` seeds media, intentionally corrupts and removes stored files, but leaves rows and performs no reset.
2. Portability projects leave damaged odd-numbered rows 1, 3 and 5.
3. Responsive administration runs before the corresponding Wiki media mutator in each viewport.
4. The stale set therefore grows to 1/3/5, then 1/3/5/7, then 1/3/5/7/9.
5. `WikiEditorialMediaFileResponse` detects the missing/integrity-failed object.
6. The dedicated Editorial Media fallback test explicitly expects HTTP 500 for an intentionally corrupt thumbnail and verifies accessible fallback rendering.

The historical 500s are thus `PROVEN` acceptance fixture leakage and expected integrity-failure traffic. They do not prove valid production media failure.

### Invalid HTML pattern

Both artifacts also recorded two Chromium console errors per viewport from `[a-z0-9]+([._-][a-z0-9]+)*`. Frozen source retains this pattern on category stable key and article content type fields. Laravel request validation independently retains the intended regex.

No shared cause among fixture leakage, flash loss and invalid pattern errors is proven.

## Normalized finding index

| Finding | Severity | State | Summary |
|---|---|---|---|
| `OTERYN-AUDIT-P35-006` | MEDIUM | PROVEN source + historical CI | Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later tests |
| `OTERYN-AUDIT-P35-001` | MEDIUM | PROVEN | content-scale strict closure omits nine canonical fragment surfaces |
| `OTERYN-AUDIT-P35-002` | MEDIUM | PROVEN | dedicated global error matrix omits HTTP 503 |
| `OTERYN-AUDIT-P35-003` | MEDIUM | DERIVED | accessibility evidence is not fail-closed per rendered surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | PROVEN historical CI | durable Wiki publication lacked transient accessible success feedback on mobile |
| `OTERYN-AUDIT-P35-007` | MEDIUM | PROVEN source + historical CI | invalid HTML pattern weakens native Wiki form validation and emits console errors |
| `OTERYN-AUDIT-P1-001` | LOW | CONFLICT | `ACTIVE_WORK.md` conflicts with live PR/task state |

Totals: **0 HIGH, 6 MEDIUM, 1 LOW**.

## Durable artifacts

### Machine-readable

- `baseline.json` — frozen baseline, tooling constraints, open-PR and deployment boundaries.
- `phase-1-surface-inventory.json` — canonical 27-surface inventory.
- `phase-2-capability-reconciliation.json` — 43-capability backend/frontend/integration matrix.
- `phase-3-5-state-browser-evidence.json` — original state/browser matrix.
- `phase-3-5-addendum.json` — normalized severity override, P35-007 and corrected totals.

### Human-readable reports

- `../../reports/OTERYN-20260731-portal-backend-frontend-audit.md` — corrected consolidated report.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-addendum.md` — detailed P35-007 source/history analysis.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-baseline.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-1-inventory.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-2-capabilities.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-3-5-states-browser.md`.

### Issue #365 and validator handoff

- `ISSUE_365_HISTORICAL_ARTIFACT_REVIEW.md` — exact hashes, per-viewport counts and original evidence boundaries.
- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md` — proven stale-fixture execution chain and severity correction.
- `VALIDATOR_PACKET.md` — exact frozen-target checkout, strict contract, focused Wiki probe, adversarial review and verdict contract.
- `INDEPENDENT_VALIDATION.md` — pending a fresh checkout-capable validator session.

## Open-PR boundary

At baseline, PR #338 Game Catalog/Wiki acceptance changes and PR #328 character-rename contract were `OPEN_PR_ONLY`; they are not promoted into the frozen target. The audit PR contains only task, report and evidence paths under its authorized ownership.

## First relevant failure

`git ls-remote https://github.com/blakinio/Oteryn-Platform.git refs/heads/main` failed in the current sandbox with `Could not resolve host: github.com`.

GitHub API and preserved CI artifacts enabled source, runtime-contract and historical browser reconciliation. They do not provide a checkout-capable Laravel/Playwright environment or an independent validator identity.

## Completion gate

The audit remains `BLOCKED` until a fresh validator:

- independently verifies the stale-fixture chain;
- runs at least three clean isolated zero-retry Issue #365 probes;
- runs one controlled polluted comparison;
- captures sanitized application/server logs;
- publishes exactly one verdict: `VALIDATED`, `VALIDATED_WITH_CORRECTIONS`, or `REJECTED`.

`VALIDATED` is forbidden when exact-target focused execution is absent or inconclusive. No merge, deployment or production action is authorized by this audit.
