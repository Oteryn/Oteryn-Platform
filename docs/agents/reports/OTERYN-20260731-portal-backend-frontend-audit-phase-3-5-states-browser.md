# Portal backend/frontend audit — Phases 3–5 states and browser evidence

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Evidence boundary

Direct CI/browser evidence was recovered for source SHA `fdb45a4325949d3ab1c4860e3a4527553f11c789`. Comparison to the frozen audit target changes only documentation and `config/marketplace.php`; that configuration file has the same blob at both SHAs. Runtime equivalence is therefore `DERIVED`, while the direct run remains `CI_PROVEN` only for its exact source SHA.

This evidence does not prove deployment of the audit target. The latest exact staging source remains `717977f252b09b9b2e979f8110b7f48b88682223`; production remains `UNKNOWN`.

## Strict machine closure

Portal Acceptance Contract run `30633216358`, job `91164376176`, artifact `8794204786` passed with:

- 27 canonical surfaces;
- 228 classified manifest route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 121 Blade views and 26 structural views;
- two explicit view exclusions and zero orphan views;
- 400 navigation references and 30 direct-entry routes;
- 43 reconciled product/backend/frontend capabilities;
- 27 dimension records, 13 execution profiles and 23 critical surfaces;
- 27 media applicability records with 12 state-evidence records and zero media gaps.

## Critical browser run

Acceptance E2E run `30633216753`, job `91164367653`, artifact `8794373786` used real Laravel HTTP with isolated MariaDB Platform/Canary schemas, Redis ACL and MailHog. Playwright retries were zero.

| Profile | Tests | Failures | Boundary |
|---|---:|---:|---|
| Chromium smoke | 7 | 0 | critical integration smoke |
| Chromium/Firefox/WebKit portability | 36 | 0 | 12 scenarios per browser |
| Responsive desktop/tablet/mobile | 42 | 0 | 14 scenarios per viewport |
| Dependency resilience | 2 | 0 | public dependency failure and restoration |
| Accessibility | 9 | 0 | bounded representative keyboard/focus/semantic scenarios |

The artifact explicitly records `FULL_ACCEPTANCE_NOT_EXECUTED`, `VISUAL_UX_NOT_EXECUTED` and `PRODUCTION_SMOKE_PENDING`. It is broad critical evidence, not exhaustive every-screen visual acceptance.

## State coverage

### Global application errors

The dedicated error evidence contract covers HTTP `404`, `419`, `429` and `500` in EN and PL at desktop `1440x1000`, tablet `820x1180` and mobile `390x844`, with zero retries. Assertions include noindex, safe recovery, no document-level horizontal overflow and absence of sensitive disclosure.

A localized `503` view exists, and public online dependency failure/restoration has bounded resilience coverage. However, `503` is absent from the dedicated global EN/PL three-viewport error matrix.

### Media

All 27 surfaces are explicitly classified for media applicability. Rendered consumers requiring the complete media-state set are:

- `wiki.public`;
- `wiki.admin-editorial-lifecycle`;
- `editorial-media.admin`.

The supporting media/preview endpoint group is classified separately. Normal, missing, broken/integrity-failed and no-image states have 12 evidence records and no declared gap.

### Content scale

The strict content-scale validator reports 18 classified base-manifest surfaces, while the canonical portal inventory contains 27. It does not load the six sorted surface fragments and therefore omits nine delivered surface groups:

- account-security lifecycle;
- support/moderation lifecycle;
- community deaths/policy;
- three Marketplace groups;
- two Game Catalog groups;
- character profile preferences.

The validator can therefore pass without one explicit content-scale applicability/evidence record for those surfaces.

### Accessibility

Nine zero-retry accessibility tests passed. They cover representative keyboard navigation, visible focus, semantics and selected recovery paths. No fail-closed one-record-per-rendered-surface applicability/evidence matrix was found, and reduced-motion applicability remains `UNKNOWN`.

## Issue #365 corrected analysis

Historical artifacts:

- SHA `35f39b48233b186502cbdcc05aec7ffc40e78fc7`, run `30562698853`, job `90939481510`, artifact `8767657461`;
- SHA `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`, run `30578806660`, job `90993603962`, artifact `8773887288`.

### Publication feedback

In both runs the responsive-mobile administration test could not find the expected `role=status` publication message. The redirected form nevertheless displayed `Published`, version 3 and `Unpublish to draft`, proving durable publication succeeded while transient feedback was absent.

### Thumbnail integrity-failure traffic

Both runs recorded the same pattern during later Wiki administration tests:

| Project | Damaged media IDs visible | HTTP 500 responses |
|---|---|---:|
| responsive desktop | 1, 3, 5 | 9 |
| responsive tablet | 1, 3, 5, 7 | 12 |
| responsive mobile | 1, 3, 5, 7, 9 | 16 |

The source and exact report ordering explain the pattern:

1. `admin-wiki-editorial-media.spec.mjs` seeds a media row.
2. It intentionally corrupts and then removes stored objects for that row.
3. It performs no EditorialMedia reset, so the row remains queryable.
4. Portability projects leave damaged rows 1, 3 and 5.
5. Responsive administration executes before the corresponding Wiki media mutator in each viewport, so the stale set grows by one odd ID after each project.
6. `WikiEditorialMediaFileResponse` rejects missing/integrity-failed bytes.
7. The dedicated Editorial Media fallback test explicitly expects HTTP 500 for a deliberately corrupt thumbnail and verifies accessible fallback rendering.

Therefore the historical HTTP 500 traffic is a proven acceptance fixture-isolation defect and expected integrity-failure response, not proof that valid production thumbnails fail.

### Invalid HTML pattern

Both artifacts also recorded two Chromium console errors per viewport from the HTML pattern `[a-z0-9]+([._-][a-z0-9]+)*`. The identical literal remains in frozen source on the category stable-key and article content-type fields. Laravel request validation independently enforces the intended grammar.

### Current-target boundary

A clean isolated frozen-target Wiki run and a controlled polluted comparison were not executed in the current connector-only environment. Flash behavior remains `UNKNOWN` on the frozen target. The stale-fixture source defect is directly present on frozen source.

No shared cause among fixture leakage, flash loss and invalid pattern errors is proven.

## Findings

### MEDIUM — OTERYN-AUDIT-P35-006: Wiki media fixture isolation leak

The Wiki media spec leaves intentionally damaged media rows for later projects. This causes order-dependent diagnostics and repeated expected integrity-failure responses in unrelated lifecycle tests. Reset media fixtures before and after every Wiki media scenario, then test missing/corrupt fallback in a scoped controlled case.

### MEDIUM — OTERYN-AUDIT-P35-001: Content-scale closure omits nine surfaces

The strict evidence validator covers 18 rather than all 27 canonical surfaces. This is an acceptance-contract defect, not a proven frontend runtime defect.

### MEDIUM — OTERYN-AUDIT-P35-002: Global error matrix omits 503

A 503 view and bounded dependency scenario exist, but the required localized three-viewport global error contract does not include HTTP 503.

### MEDIUM — OTERYN-AUDIT-P35-003: Accessibility closure is not fail-closed per surface

Representative accessibility evidence passes, but no complete per-surface applicability ledger prevents silent omission of a new or less-representative screen.

### MEDIUM — OTERYN-AUDIT-P35-005: Historical Wiki publication success feedback loss

Publication completed, but the accessible transient success announcement was absent in two zero-retry mobile runs.

### MEDIUM — OTERYN-AUDIT-P35-007: Invalid Wiki HTML pattern

Two administrator inputs contain an HTML pattern Chromium treats as an invalid regular expression. Native browser validation can be unreliable and the forms emit console errors, while server-side validation remains intact.

### INFO — OTERYN-AUDIT-P35-004: Critical browser evidence is not exhaustive visual acceptance

The artifact states that full and visual profiles were skipped. Its claim must remain bounded.

### INFO — Exact-target focused execution pending

Clean isolated and controlled polluted exact-target probes remain unexecuted. This is an evidence boundary, not a separate numbered defect.

## Corrected totals

- HIGH: `0`;
- MEDIUM: `6`;
- LOW: `1` in Phase 1 governance evidence.

## Recommended remediation shape

Use the smallest safe number of follow-ups:

1. One acceptance-evidence remediation task under #326 for canonical fragment-aware content-scale closure, 503 inclusion and a fail-closed per-surface accessibility applicability matrix.
2. Keep Issue #365 as the Wiki reproduction/defect task:
   - add deterministic EditorialMedia reset around Wiki media tests;
   - run three clean isolated zero-retry administration flows;
   - run a separate controlled polluted comparison;
   - classify publication feedback independently from integrity-failure traffic.
3. Repair the two invalid HTML patterns with focused native validation and zero-console-error browser regression coverage.

No implementation is authorized or performed by this audit.
