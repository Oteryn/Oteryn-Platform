# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent: Issue `#326`  
Related evidence: Issue `#365`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Executive conclusion

The repository contains a broad and internally consistent portal implementation plus a mature machine-enforced acceptance architecture. The audit established:

- 27 canonical surface groups and 228 classified route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 400 navigation references and zero orphan views;
- 43 capabilities: 23 implemented, 3 partial, 14 missing and 3 not applicable;
- no user-facing backend-only or frontend-only capability falsely promoted to implemented;
- strict route/view/navigation/media closure;
- fresh zero-retry critical browser execution across Chromium, Firefox, WebKit and desktop/tablet/mobile.

The corrected finding set is **0 HIGH, 6 MEDIUM and 1 LOW**. The portal is not proven product-complete, deployed on the frozen target or production-complete.

Independent validator verdict: **`VALIDATED_WITH_CORRECTIONS`**.

The task remains `BLOCKED`, not `DONE`, because the exact custom Issue #365 probe package still lacks three isolated original-flow samples and one controlled polluted comparison.

## Evidence boundaries

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen source audit target |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and fresh critical browser execution passed |
| `DERIVED` | direct CI source → frozen target | runtime code equivalent; not relabelled as direct frozen-target execution |
| `STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` | exact staging evidence for a different source |
| `PRODUCTION_PROVEN` | none | production release and availability remain `UNKNOWN` |

Open PR code remains `OPEN_PR_ONLY`. Marketplace routes depend on `MARKETPLACE_ENABLED`; source presence does not prove deployment reachability.

## Canonical inventory

The 27 surface groups cover identity and account lifecycle, character/account management, public content and game data, downloads/events/announcements/support, Wiki and Game Catalog, administrative RBAC/CMS/audit/moderation, Editorial Media, Marketplace and supporting browser resource endpoints.

Authoritative detailed matrices:

- `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/phase-1-surface-inventory.json`;
- `phase-2-capability-reconciliation.json`;
- `phase-3-5-state-browser-evidence.json`;
- `phase-3-5-addendum.json`.

## Strict repository validation

Portal Acceptance Contract:

- source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run: `30633216358`;
- job: `91164376176`;
- artifact: `8794204786`;
- digest: `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result: `PASS`.

The strict route/view/navigation/media contracts close the canonical inventory. The content-scale sub-validator loads only 18 base-manifest surfaces and omits nine fragment surfaces.

## Fresh independent validator execution

A fresh attempt of the existing Acceptance E2E critical profile was initiated and reviewed on `2026-08-01`:

- run: `30633216753`, attempt `2`;
- job: `91339118796`;
- source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- profile: `critical`;
- retries: `0`;
- runtime: PHP 8.5, real Laravel HTTP, isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog;
- conclusion: `SUCCESS`;
- artifact: `8814897157`, `acceptance-e2e-critical-30633216753-2-direct`;
- GitHub artifact digest: `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`;
- locally downloaded ZIP SHA-256: `6b18d56738cad108180e20f99a22a82249ab564b6c234d12e19625d521b20f33`.

The two hashes describe different artifact representations and are intentionally recorded separately.

Results:

| Profile | Result |
|---|---:|
| smoke | 7/7 PASS |
| portability | 36/36 PASS |
| responsive | 42/42 PASS |
| resilience | 2/2 PASS |
| accessibility | 9/9 PASS |
| total | 96/96 PASS |

Full, exploratory visual and soak execution were intentionally outside the critical profile. Production smoke remains pending.

The fresh run passed:

- the original Wiki administration lifecycle in Chromium, Firefox and WebKit;
- the original lifecycle at desktop, tablet and mobile;
- the Wiki Editorial Media lifecycle in all portability and responsive projects;
- the Wiki media lifecycle in accessibility Chromium;
- the image-free Wiki media path in all applicable projects.

The media-intensive scenario explicitly asserts accessible `Wiki article published.` feedback and durable `Published` state. The original administration scenario still does not contain its historical transient flash assertion.

Full validator rationale and nonclaims are recorded in `VALIDATOR_VERDICT.md`.

## Findings

### MEDIUM — OTERYN-AUDIT-P35-006

**Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later tests.**

The Wiki media acceptance spec intentionally corrupts/removes stored files but preserves rows and lacks the reset used by the dedicated Editorial Media spec. Later projects request those stale rows. Historical ordering exactly predicts the repeated 9/12/16 HTTP 500 pattern across IDs 1/3/5, then 1/3/5/7, then 1/3/5/7/9.

The integrity service correctly rejects missing/corrupt objects, and the dedicated fallback test explicitly expects HTTP 500 plus accessible fallback for deliberately corrupt media. This is an acceptance isolation/evidence defect, not proof that valid production media fails.

### MEDIUM — OTERYN-AUDIT-P35-001

**Strict content-scale closure omits nine canonical fragment surfaces.**

The sub-validator loads 18 base-manifest surfaces but not the six fragment files. Nine delivered surfaces can lack explicit long-content/large-collection applicability while the command remains green.

### MEDIUM — OTERYN-AUDIT-P35-002

**Dedicated global error matrix omits HTTP 503.**

404, 419, 429 and 500 have EN/PL desktop/tablet/mobile coverage. A 503 view and bounded dependency scenario exist, but 503 lacks the same dedicated global contract.

### MEDIUM — OTERYN-AUDIT-P35-003

**Accessibility evidence is bounded rather than fail-closed per delivered surface.**

Representative zero-retry scenarios pass, but no one-record-per-rendered-surface applicability ledger prevents silent omission. Reduced-motion applicability remains `UNKNOWN`.

### MEDIUM — OTERYN-AUDIT-P35-005

**Historical mobile Wiki publication lost transient success feedback after durable success.**

Two historical mobile runs showed durable `Published` state and version advancement but lacked the expected accessible status message. Historical routes lacked session blocking. Commit `6c1e910d36771f50da5eded93cc50274a90c62d2` serializes all administrator Wiki session requests, and the fresh related media-intensive flash assertion passes across engines and viewports.

Current classification: `PARTIALLY_PROVEN_REMEDIATED`; the original transient assertion itself remains absent.

### MEDIUM — OTERYN-AUDIT-P35-007

**Invalid HTML pattern weakens native validation on two Wiki administrator fields.**

`pattern="[a-z0-9]+([._-][a-z0-9]+)*"` remains in category-key and article-content-type fields and produced deterministic Chromium console errors historically. Laravel independently enforces the intended server regex, so no backend bypass is proven.

### LOW — OTERYN-AUDIT-P1-001

**`ACTIVE_WORK.md` conflicts with live task/PR ownership.**

Live GitHub task and PR state was treated as authoritative.

## Causality and nonclaims

The audit does not combine fixture leakage, flash loss and invalid pattern errors into one cause. It does not claim:

- direct browser execution on exact frozen SHA `b6f7...`;
- zero HTTP 500 or console errors merely because successful JUnit/HTML output lacks those strings;
- exhaustive every-screen visual acceptance;
- exact frozen-target staging deployment;
- production availability.

## Deployment

Latest directly proven staging source remains `717977f252b09b9b2e979f8110b7f48b88682223`, run `30633745660`, job `91166065335`, artifact `8794683627`. No production operation was performed.

## Minimum remediation set

1. One acceptance-evidence task under Issue #326 for fragment-aware content-scale closure, dedicated 503 coverage and fail-closed accessibility applicability.
2. Continue Issue #365 without duplication: reset Wiki EditorialMedia fixtures and execute the exact clean-versus-controlled-polluted probe package.
3. Correct the two Wiki HTML patterns and add native `patternMismatch` plus zero-console-error regression coverage after focused classification.

No implementation is authorized or performed by this audit.

## Independent validation verdict

Status: `VALIDATED_WITH_CORRECTIONS`.

Durable verdict:

- `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/VALIDATOR_VERDICT.md`.

The fresh zero-retry run independently reconfirmed all available critical profiles and relevant Wiki scenarios. `VALIDATED` remains forbidden because the validator packet's exact custom execution is incomplete.

## Residual completion gate

The task remains `BLOCKED` until one checkout-capable run performs:

1. at least three clean isolated responsive-mobile samples of the original administration flow with its transient flash assertion restored ephemerally;
2. one controlled comparison with exactly one missing or corrupt EditorialMedia row;
3. sanitized publish redirect/status, session/request, thumbnail and bounded application/server evidence.

No merge, deployment or production action is authorized.
