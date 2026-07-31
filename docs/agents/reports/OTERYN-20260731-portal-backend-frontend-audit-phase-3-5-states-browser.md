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

All 27 surfaces are explicitly classified for media applicability. Only these rendered consumers require the complete media-state set:

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

## Issue #365

The two historical mobile reproductions remain independently proven:

- SHA `35f39b48233b186502cbdcc05aec7ffc40e78fc7`, run `30562698853`, job `90939481510`, artifact `8767657461`;
- SHA `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`, run `30578806660`, job `90993603962`, artifact `8773887288`.

In both runs the responsive-mobile test could not find the expected `role=status` publication message. The redirected form nevertheless displayed `Published`, version 3 and `Unpublish to draft`, proving durable publication succeeded while transient feedback was absent.

Issue #365 separately preserves multiple HTTP 500 thumbnail responses during the same historical page load. No shared cause between flash loss and thumbnail failure is proven.

Neither symptom was executed on the frozen audit target in the current connector-only environment. Current-target status is `UNKNOWN`, not fixed, not reproduced or still present.

## Findings

### HIGH — OTERYN-AUDIT-P35-006: Historical Wiki thumbnail HTTP 500 responses

A core administrator media workflow produced repeated server errors and unavailable previews on two historical exact heads. Current-target status is unknown. Continue under existing Issue #365 and investigate thumbnail logs independently from session/flash behavior.

### MEDIUM — OTERYN-AUDIT-P35-001: Content-scale closure omits nine surfaces

The strict evidence validator covers 18 rather than all 27 canonical surfaces. This is an acceptance-contract defect, not a proven frontend runtime defect.

### MEDIUM — OTERYN-AUDIT-P35-002: Global error matrix omits 503

A 503 view and bounded dependency scenario exist, but the required localized three-viewport global error contract does not include HTTP 503.

### MEDIUM — OTERYN-AUDIT-P35-003: Accessibility closure is not fail-closed per surface

Representative accessibility evidence passes, but no complete per-surface applicability ledger prevents silent omission of a new or less-representative screen.

### MEDIUM — OTERYN-AUDIT-P35-005: Historical Wiki publication success feedback loss

Publication completed, but the accessible transient success announcement was absent in two zero-retry mobile runs.

### INFO — OTERYN-AUDIT-P35-004: Critical browser evidence is not exhaustive visual acceptance

The artifact states that full and visual profiles were skipped. Its claim must remain bounded.

### INFO — OTERYN-AUDIT-P35-007: Current-target Wiki reproduction not run

The frozen audit target cannot be classified for either Issue #365 symptom without a checkout-capable Laravel/Playwright run and sanitized logs.

## Recommended remediation shape

Use the smallest safe number of follow-ups:

1. One acceptance-evidence remediation task under #326 for canonical fragment-aware content-scale closure, 503 inclusion and a fail-closed per-surface accessibility applicability matrix.
2. Keep Issue #365 as the independent Wiki reproduction/defect task; do not combine session flash and thumbnail causes unless evidence proves a shared cause.

No implementation is authorized or performed by this audit.
