# OTERYN-20260731 portal backend/frontend audit evidence index

## Identity and status

- Task: `OTERYN-20260731-portal-backend-frontend-audit`
- Frozen target: `REPO_MAIN` SHA `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`
- Branch: `audit/OTERYN-20260731-portal-backend-frontend-audit`
- Draft PR: `#381`
- Parent: Issue `#326`
- Related evidence: Issue `#365`
- Audit status: `BLOCKED`
- Validator verdict: `VALIDATED_WITH_CORRECTIONS`

`PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` remain separate from environment labels `REPO_MAIN`, `OPEN_PR_ONLY`, `CI_PROVEN`, `STAGING_PROVEN`, `PRODUCTION_PROVEN` and `UNKNOWN`.

## Environment boundary

| Evidence | State | Exact identity |
|---|---|---|
| Frozen source | `PROVEN / REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` |
| Direct strict/browser source | `PROVEN / CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` |
| Runtime equivalence to frozen source | `DERIVED` | comparison changes documentation and byte-identical Marketplace configuration |
| Latest staging evidence | `PROVEN / STAGING_PROVEN` | source `717977f252b09b9b2e979f8110b7f48b88682223`, run `30633745660`, job `91166065335`, artifact `8794683627` |
| Frozen target deployed | `UNKNOWN` | no exact staging or production deployment proof |
| Production availability | `UNKNOWN` | no direct exact-release evidence |

## Canonical inventory

- 27 canonical surface groups.
- 228 classified manifest route assignments.
- 240 discovered named routes.
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources.
- 95 bound views, 121 Blade views, 26 structural views and zero orphan views.
- 400 navigation references.
- 43 capabilities: 23 implemented, 3 partial, 14 missing, 3 not applicable.
- Zero user-facing backend-only or frontend-only promotions to implemented.

## Strict repository contract

- source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run: `30633216358`;
- job: `91164376176`;
- artifact: `8794204786`;
- digest: `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result: `PASS`.

The contract covers all 27 canonical surfaces for route/view/media closure. The content-scale validator itself loads only 18 base-manifest surfaces and omits nine fragment surfaces.

## Fresh validator browser execution

A fresh rerun was initiated and independently reviewed on `2026-08-01`:

- workflow: `Acceptance E2E and Visual UX`;
- run: `30633216753`, attempt `2`;
- job: `91339118796`;
- direct source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- profile: `critical`;
- retries: `0`;
- conclusion: `SUCCESS`;
- artifact: `8814897157`, `acceptance-e2e-critical-30633216753-2-direct`;
- GitHub artifact digest: `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`;
- locally downloaded ZIP SHA-256: `6b18d56738cad108180e20f99a22a82249ab564b6c234d12e19625d521b20f33`.

The two hashes describe different artifact representations and are intentionally not asserted equal.

| Profile | Result |
|---|---:|
| smoke | 7/7 PASS |
| portability | 36/36 PASS |
| responsive | 42/42 PASS |
| resilience | 2/2 PASS |
| accessibility | 9/9 PASS |
| total | 96/96 PASS |

The run used PHP 8.5, real Laravel HTTP, isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog. Full, exploratory visual and soak profiles were intentionally outside the critical run; production smoke remains pending.

## Issue #365 classification

### Thumbnail traffic

Historical 9/12/16 HTTP 500 counts are explained by Wiki acceptance fixture leakage:

- the media spec intentionally corrupts/removes stored objects;
- rows survive without reset;
- later projects request stale rows;
- the integrity service rejects missing/corrupt bytes;
- the dedicated fallback scenario explicitly expects HTTP 500 and accessible fallback for deliberately corrupt media.

This is `MEDIUM` acceptance isolation/evidence failure, not a proven failure of valid production media.

### Publication flash

- historical mobile runs lost transient `role=status` feedback after durable publication;
- historical routes lacked session blocking;
- commit `6c1e910d36771f50da5eded93cc50274a90c62d2` serializes administrator Wiki session requests;
- the fresh rerun passed the original administration scenario across portability and desktop/tablet/mobile;
- the related media-intensive scenario explicitly asserts `Wiki article published.` plus durable `Published` state and passed all portability/responsive projects plus accessibility Chromium.

The remediation remains `PARTIALLY_PROVEN_REMEDIATED`: the original administration scenario no longer includes its historical transient flash assertion.

## Normalized findings

| Finding | Severity | Summary |
|---|---|---|
| `OTERYN-AUDIT-P35-006` | MEDIUM | damaged EditorialMedia fixtures leak into later acceptance projects |
| `OTERYN-AUDIT-P35-001` | MEDIUM | content-scale closure omits nine canonical fragment surfaces |
| `OTERYN-AUDIT-P35-002` | MEDIUM | dedicated global error matrix omits HTTP 503 |
| `OTERYN-AUDIT-P35-003` | MEDIUM | accessibility evidence is bounded rather than fail-closed per surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | historical mobile flash loss; remediation partially proven |
| `OTERYN-AUDIT-P35-007` | MEDIUM | invalid HTML pattern weakens native Wiki form validation |
| `OTERYN-AUDIT-P1-001` | LOW | `ACTIVE_WORK.md` conflicts with live task/PR state |

Totals: **0 HIGH, 6 MEDIUM, 1 LOW**.

## Durable artifacts

- `baseline.json`
- `phase-1-surface-inventory.json`
- `phase-2-capability-reconciliation.json`
- `phase-3-5-state-browser-evidence.json`
- `phase-3-5-addendum.json`
- `ISSUE_365_HISTORICAL_ARTIFACT_REVIEW.md`
- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md`
- `ISSUE_365_FLASH_REMEDIATION_EVIDENCE.md`
- `VALIDATOR_PACKET.md`
- `VALIDATOR_PACKET_ADDENDUM.md`
- `VALIDATOR_VERDICT.md`
- reports under `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit*.md`

## Residual completion gate

The validator verdict is `VALIDATED_WITH_CORRECTIONS`, not `VALIDATED`. The task remains blocked until one checkout-capable execution performs the exact custom package:

1. at least three clean isolated zero-retry samples of the original Wiki administration flow with the transient flash assertion restored ephemerally;
2. one controlled comparison with exactly one missing or corrupt EditorialMedia row;
3. sanitized publish, session/request, thumbnail and application/server evidence.

Absence of HTTP 500 or console strings from successful JUnit/HTML output is not treated as proof that those diagnostics were absent. No merge, deployment or production action is authorized.
