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

## Issue #365 historical evidence

Reviewed exact artifacts:

| Run | SHA | Job | Artifact | Verified ZIP SHA-256 |
|---|---|---:|---:|---|
| `30562698853` | `35f39b48233b186502cbdcc05aec7ffc40e78fc7` | `90939481510` | `8767657461` | `8af4dedd1e213108a2599df303f45de7bf22caf603c180f7607ad5d8395a85c6` |
| `30578806660` | `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb` | `90993603962` | `8773887288` | `4a514e4a53d427599f07e0a22ad7cd918a154187e6ddd837e707ece2c14e96f2` |

Both runs prove:

- desktop and tablet completed the Wiki flow;
- mobile alone lost the expected accessible publication flash while the redirected page showed `Published`, version 3 and `Unpublish to draft`;
- repeated GET `/admin/wiki/media/{id}/thumbnail` HTTP 500 responses followed the same pattern in both runs:
  - desktop 9 responses across IDs 1, 3, 5;
  - tablet 12 across IDs 1, 3, 5, 7;
  - mobile 16 across IDs 1, 3, 5, 7, 9;
- two Chromium console errors per viewport came from invalid HTML pattern `[a-z0-9]+([._-][a-z0-9]+)*` on category-create and article-create pages.

Frozen source retains the same invalid HTML pattern in the two administrator Wiki forms. Laravel request validation independently retains the intended regex. No shared cause among thumbnail failure, flash loss and pattern errors is proven.

## Normalized finding index

| Finding | Severity | State | Summary |
|---|---|---|---|
| `OTERYN-AUDIT-P35-006` | HIGH | PROVEN historical CI | Wiki editorial thumbnail requests returned repeated HTTP 500 responses |
| `OTERYN-AUDIT-P35-001` | MEDIUM | PROVEN | content-scale strict closure omits nine canonical fragment surfaces |
| `OTERYN-AUDIT-P35-002` | MEDIUM | PROVEN | dedicated global error matrix omits HTTP 503 |
| `OTERYN-AUDIT-P35-003` | MEDIUM | DERIVED | accessibility evidence is not fail-closed per rendered surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | PROVEN historical CI | durable Wiki publication lacked transient accessible success feedback on mobile |
| `OTERYN-AUDIT-P35-007` | MEDIUM | PROVEN source + historical CI | invalid HTML pattern weakens native Wiki form validation and emits console errors |
| `OTERYN-AUDIT-P1-001` | LOW | CONFLICT | `ACTIVE_WORK.md` conflicts with live PR/task state |

Totals: **1 HIGH, 5 MEDIUM, 1 LOW**.

## Durable artifacts

### Machine-readable

- `baseline.json` — frozen baseline, tooling constraints, open-PR and deployment boundaries.
- `phase-1-surface-inventory.json` — canonical 27-surface inventory.
- `phase-2-capability-reconciliation.json` — 43-capability backend/frontend/integration matrix.
- `phase-3-5-state-browser-evidence.json` — state, browser, deployment and Issue #365 matrix.
- `phase-3-5-addendum.json` — P35-007 and normalized totals.

### Human-readable reports

- `../../reports/OTERYN-20260731-portal-backend-frontend-audit.md` — consolidated current report.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-addendum.md` — detailed P35-007 source/history analysis.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-baseline.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-1-inventory.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-2-capabilities.md`.
- `../../reports/OTERYN-20260731-portal-backend-frontend-audit-phase-3-5-states-browser.md`.

### Validator handoff

- `ISSUE_365_HISTORICAL_ARTIFACT_REVIEW.md` — exact hashes, per-viewport counts and causality boundaries.
- `VALIDATOR_PACKET.md` — exact frozen-target checkout, strict contract, three-run focused Wiki probe, adversarial finding review, unauthorized-change check and final verdict contract.
- `INDEPENDENT_VALIDATION.md` — pending a fresh checkout-capable validator session.

## Open-PR boundary

At baseline, PR #338 Game Catalog/Wiki acceptance changes and PR #328 character-rename contract were `OPEN_PR_ONLY`; they are not promoted into the frozen target. The audit PR contains only task, report and evidence paths under its authorized ownership.

## First relevant failure

`git ls-remote https://github.com/blakinio/Oteryn-Platform.git refs/heads/main` failed in the current sandbox with `Could not resolve host: github.com`.

GitHub API and preserved CI artifacts enabled source, runtime-contract and historical browser reconciliation. They do not provide a checkout-capable Laravel/Playwright environment or an independent validator identity.

## Completion gate

The audit remains `BLOCKED` until a fresh validator follows `VALIDATOR_PACKET.md`, executes at least three independent zero-retry focused Issue #365 probes on the exact frozen target with sanitized application/server logs, and publishes exactly one verdict:

- `VALIDATED`;
- `VALIDATED_WITH_CORRECTIONS`;
- `REJECTED`.

`VALIDATED` is forbidden when the exact-target focused execution is absent or inconclusive. No merge, deployment or production action is authorized by this audit.
