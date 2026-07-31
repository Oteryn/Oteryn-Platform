# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent: Issue `#326`  
Related evidence/defect: Issue `#365`  
Frozen audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Executive conclusion

The repository contains a broad, internally consistent portal implementation and a mature machine-enforced acceptance architecture. The audit identified:

- 27 canonical surface groups;
- 228 classified manifest route assignments;
- 43 canonical product capabilities: 23 implemented, 3 partial, 14 missing and 3 not applicable;
- no user-facing backend-only or frontend-only capability promoted to implemented;
- recovered strict route/view/navigation evidence with 240 discovered named routes, 126 rendered routes, 95 bound views, 400 navigation references and zero orphan views;
- recovered zero-retry critical browser evidence across Chromium, Firefox, WebKit and desktop/tablet/mobile viewports.

The portal is **not proven product-complete or production-complete**. The normalized finding set is:

- **1 HIGH**;
- **5 MEDIUM**;
- **1 LOW**.

The audit remains `BLOCKED`, not `DONE`, because exact frozen-target Issue #365 reproduction and a fresh independent validator verdict have not been executed.

## Evidence boundaries

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen source audit target |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and critical browser artifacts passed |
| `DERIVED` | direct CI source → frozen target | runtime code equivalent; direct CI is not relabelled as exact-target execution |
| `STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` | exact staging control evidence, not frozen-target deployment |
| `PRODUCTION_PROVEN` | none | production release and availability remain `UNKNOWN` |

The frozen target remains authoritative even if `main` advances. Open PR code is `OPEN_PR_ONLY` and is not counted as `REPO_MAIN` implementation.

## Canonical inventory

The 27 surface groups cover:

- registration, login, sessions, password and MFA lifecycle;
- Account Center, provisioning, account security and character ownership surfaces;
- public home, SEO, news, managed pages, localization, downloads, events, announcements and support/legal content;
- public game data, deaths/community policy, Wiki and Game Catalog;
- administration, RBAC, CMS, audit, moderation, Wiki editorial lifecycle and Editorial Media;
- Marketplace public, authenticated and administrator surfaces;
- supporting browser media/preview endpoints.

Machine-readable inventory and capability details are authoritative in:

- `phase-1-surface-inventory.json`;
- `phase-2-capability-reconciliation.json`;
- `phase-3-5-state-browser-evidence.json`;
- `phase-3-5-addendum.json`.

Marketplace route registration depends on `MARKETPLACE_ENABLED`; repository presence does not prove deployment reachability.

## Direct recovered validation

### Portal Acceptance Contract

- exact source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`;
- strict job `91164376176`;
- artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result: `PASS`.

Observed closure:

- 27 canonical surfaces;
- 228 classified manifest route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 121 Blade views, 26 structural views, two bounded exclusions and zero orphan views;
- 400 navigation references and 30 bounded direct-entry routes;
- 43 capability records with no backend/frontend validator error;
- all 27 surfaces classified for media applicability;
- content-scale validator classified only 18 base-manifest surfaces.

### Critical browser evidence

- exact source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216753`;
- job `91164367653`;
- artifact `8794373786`;
- digest `sha256:3dd06aeee7436d4eb9ba3ec23b5e3b8684e987d7f58dcc4a247b54df48f0adeb`;
- runtime: real Laravel HTTP, isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog;
- retries: `0`.

Results:

- smoke: `7/7` PASS;
- portability: `36/36` PASS across Chromium, Firefox and WebKit;
- responsive: `42/42` PASS across 1440×1000, 820×1180 and 390×844;
- resilience: `2/2` PASS;
- accessibility: `9/9` PASS.

The artifact explicitly states `FULL_ACCEPTANCE_NOT_EXECUTED`, `VISUAL_UX_NOT_EXECUTED` and `PRODUCTION_SMOKE_PENDING`. It cannot support an exhaustive every-screen visual or production claim.

## Findings

### HIGH — OTERYN-AUDIT-P35-006

**Historical Wiki editorial thumbnail requests returned HTTP 500.**

Two exact zero-retry historical CI heads recorded deterministic GET failures on `/admin/wiki/media/{id}/thumbnail` during the administrator Wiki flow:

| Project | HTTP 500 responses per run | Unique media IDs |
|---|---:|---|
| responsive desktop | 9 | 1, 3, 5 |
| responsive tablet | 12 | 1, 3, 5, 7 |
| responsive mobile | 16 | 1, 3, 5, 7, 9 |

The same 9/12/16 pattern occurred in both preserved artifacts:

- run `30562698853`, job `90939481510`, artifact `8767657461`, SHA `35f39b48233b186502cbdcc05aec7ffc40e78fc7`;
- run `30578806660`, job `90993603962`, artifact `8773887288`, SHA `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`.

Affected cards rendered `Preview unavailable`. Frozen-target reproduction remains `UNKNOWN`; no cause is assigned.

### MEDIUM — OTERYN-AUDIT-P35-001

**Strict content-scale closure omits nine canonical fragment surfaces.**

The validator loads the 18 base-manifest surfaces but not the six fragment files. Nine delivered surfaces can therefore lack explicit applicability/evidence for long values, large collections, pagination and wrapping while the strict command remains green.

### MEDIUM — OTERYN-AUDIT-P35-002

**Dedicated global error browser matrix omits HTTP 503.**

The matrix covers 404, 419, 429 and 500 in EN/PL at desktop/tablet/mobile with zero retries. A localized 503 view and bounded dependency-failure recovery scenario exist, but 503 lacks the same dedicated noindex, recovery, overflow and disclosure contract.

### MEDIUM — OTERYN-AUDIT-P35-003

**Accessibility evidence is bounded, not fail-closed per delivered surface.**

Nine representative zero-retry scenarios pass. No complete one-record-per-rendered-surface applicability/evidence ledger was found, and reduced-motion applicability remains `UNKNOWN`.

### MEDIUM — OTERYN-AUDIT-P35-005

**Historical mobile Wiki publication lost transient success feedback after durable success.**

On both historical heads, the publish POST succeeded and the redirected form showed `Published`, version 3 and `Unpublish to draft`; only the mobile project failed to find the expected accessible `role=status` publication message. This is a feedback defect, not evidence of publication failure.

### MEDIUM — OTERYN-AUDIT-P35-007

**Invalid HTML pattern weakens native validation on two Wiki administrator fields.**

Frozen source contains:

```html
pattern="[a-z0-9]+([._-][a-z0-9]+)*"
```

in:

- `resources/views/admin/wiki/categories/form.blade.php` for the stable key;
- `resources/views/admin/wiki/articles/form.blade.php` for the content type.

Both historical artifacts recorded deterministic Chromium console errors for this pattern in desktop, tablet and mobile execution. Laravel independently enforces the intended server regex in `AdminWikiCategoryRequest` and `AdminWikiArticleRequest`, so no backend validation bypass is proven.

### LOW — OTERYN-AUDIT-P1-001

**`ACTIVE_WORK.md` conflicts with live PR/task ownership.**

The file reported no active tasks while live open PRs contained active task records and owned paths. Live PR/task state was treated as authoritative.

## Causality boundaries

The audit does not claim a shared cause among:

- thumbnail HTTP 500 responses;
- missing publication flash;
- invalid HTML `pattern` console errors.

Historical artifacts prove coexistence, not causality. Each symptom requires independent exact-target classification.

## Backend-only, frontend-only and unreachable inventory

- User-facing backend-only capabilities promoted to implemented: none found.
- User-facing frontend-only capabilities promoted to implemented: none found.
- Supporting non-UI contracts include game-auth ticket issuance/redemption/login-context and browser media/preview resources.
- `home-preview` is an intentional noindex design reference, not a delivered route.
- `game-auth.oauth.authorize` is a bounded protocol/framework view exclusion.
- Recovered strict closure reported zero unclassified orphan views.
- PR #338 Game Catalog work and PR #328 character-rename contract remain `OPEN_PR_ONLY`.

## Deployment

### Staging

Latest directly proven staging source:

- SHA `717977f252b09b9b2e979f8110b7f48b88682223`;
- control run `30633745660`;
- job `91166065335`;
- artifact `8794683627`.

This does not prove deployment of the frozen audit target.

### Production

No production operation was performed. No direct exact-release evidence satisfying Issue #91 was established. Production is `UNKNOWN`, not failed.

## Recommended minimum remediation set

1. **One acceptance-evidence remediation task under Issue #326**
   - fragment-aware content-scale closure for all 27 surfaces;
   - HTTP 503 in the global error matrix;
   - fail-closed per-surface accessibility applicability/evidence.

2. **Continue existing Issue #365; do not create a duplicate**
   - run at least three focused zero-retry frozen-target Wiki probes;
   - capture sanitized publish redirect, session/application/server and thumbnail evidence;
   - classify flash and thumbnail symptoms independently;
   - repair only a proven cause.

3. **Small Wiki administrator validation repair after validator classification**
   - correct the two HTML patterns;
   - add native `patternMismatch` and zero-console-error browser regression coverage;
   - preserve backend validation semantics.

## Independent validation and completion gate

Status: `PENDING`.

The required procedure is fully specified in:

`docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/VALIDATOR_PACKET.md`

A fresh checkout-capable validator must:

- verify the frozen SHA and clean checkout;
- rerun the strict repository contracts;
- execute at least three independent zero-retry focused Wiki probes;
- capture sanitized application/server evidence;
- try to disprove every finding and sample lower-severity/nonclaim records;
- verify all PR paths are audit-only;
- publish a separate `INDEPENDENT_VALIDATION.md` with exactly one verdict: `VALIDATED`, `VALIDATED_WITH_CORRECTIONS`, or `REJECTED`.

`VALIDATED` is forbidden while focused Issue #365 execution is missing or inconclusive.

## Audit status

`BLOCKED`

The evidence package is validator-ready. The only unresolved completion gates are exact-target focused execution and independent validation. No implementation, merge, deployment or production action is authorized by this audit.
