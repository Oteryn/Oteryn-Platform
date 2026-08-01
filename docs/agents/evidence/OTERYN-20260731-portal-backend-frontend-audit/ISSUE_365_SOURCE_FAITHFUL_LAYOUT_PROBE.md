# Issue #365 source-faithful layout probe

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen source basis: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `CONTROLLED_SOURCE_FAITHFUL / DERIVED`  
Browser: Chromium `144.0.7559.96`

## Purpose

This probe tests the narrow hypothesis that Playwright's far-down `Publish` action on the old Wiki article-edit document starts deferred native-lazy thumbnail requests after the preceding `networkidle` boundary.

The earlier generic lazy-scroll probe placed the publication control immediately below a responsive image grid. The real Oteryn form does not. Between the media picker and Lifecycle controls it contains article settings, two large translation panels, categories and the change-note/save section. That structural difference materially changes which elements enter the lazy-loading margin during the actionability scroll.

## Source-faithful inputs

The harness copied the frozen-source structure relevant to geometry and lazy loading:

- section order from `resources/views/admin/wiki/articles/form.blade.php`;
- 12 media cards created as in `public/js/wiki-admin-media.js`;
- `loading="lazy"` and `decoding="async"` thumbnail attributes;
- `.wiki-media-results`, `.wiki-media-card`, `.wiki-code-field`, translation-grid and responsive rules from `public/css/wiki-admin.css`;
- administrator shell geometry from `public/css/app.css`;
- exact Playwright viewports from `scripts/acceptance/playwright.config.mjs`:
  - desktop `1440×1000`;
  - tablet `820×1180`;
  - mobile `390×844`.

The browser network was fulfilled at the routing layer to record request start without external DNS. The harness did not run Laravel, authentication, session storage or session locks.

## Matrix

Three independent samples were executed for each profile and mode, 18 total:

- immediate Playwright `Publish.click()` after initial request quiet;
- explicit `scroll_into_view_if_needed()`, request quiet, then `Publish.click()`.

Request classification begins at the monotonic timestamp immediately before Playwright starts the click action. This includes actionability scrolling before the DOM `click` event.

## Results

| Profile | Mode | Initially started thumbnails | New thumbnail starts from action start | Final scroll Y | Document height |
|---|---|---|---|---:|---:|
| desktop | immediate | `12, 12, 12` | `0, 0, 0` | 2989 | 3989 |
| desktop | pre-scroll | `12, 12, 12` | `0, 0, 0` | 2989 | 3989 |
| tablet | immediate | `10, 10, 10` | `0, 0, 0` | 4584 | 5764 |
| tablet | pre-scroll | `10, 10, 10` | `0, 0, 0` | 4584 | 5764 |
| mobile | immediate | `4, 4, 4` | `0, 0, 0` | 7158 | 8002 |
| mobile | pre-scroll | `4, 4, 4` | `0, 0, 0` | 7158 | 8002 |

No sample started a thumbnail request at or after the Playwright action began.

Machine-readable summary: `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`.

## Preserved artifact reconciliation

Attempts 3 and 4 still prove the mobile flash defect after session serialization. Their Playwright API steps show:

| Attempt | Profile | `networkidle` duration | Gap to visibility check | Publish click duration | Result |
|---:|---|---:|---:|---:|---|
| 3 | desktop | 623 ms | 1 ms | 84 ms | PASS |
| 3 | tablet | 662 ms | 0 ms | 74 ms | PASS |
| 3 | mobile | 682 ms | 1 ms | 100 ms | REPRODUCED |
| 4 | desktop | 793 ms | 1 ms | 75 ms | PASS |
| 4 | tablet | 755 ms | 0 ms | 193 ms | PASS |
| 4 | mobile | 643 ms | 1 ms | 100 ms | REPRODUCED |

The embedded `browser-diagnostics` collector records HTTP responses in event order but does not preserve timestamps, initiator document, `Referer`, request headers, navigation identity or session correlation. Therefore the repeated thumbnail groups cannot be assigned safely to the pre-publish old document versus one of the redirected edit documents.

A separate viewport distinction is directly preserved:

- desktop and tablet record aborted thumbnail requests while still passing;
- both mobile reproductions record zero Playwright failed requests while recording more completed HTTP 500 responses.

This proves viewport-dependent completion/cancellation behavior, not flash causality.

## Correction to the earlier synthetic interpretation

The generic probe remains valid only for the proposition that Playwright actionability scrolling can, in some page geometries, activate native-lazy images. It is not representative enough to support `HIGH confidence` for the actual Oteryn Wiki form.

The source-faithful result directly weakens the proposed old-document activation chain:

```yaml
old_classification: DERIVED / HIGH confidence
corrected_classification: DERIVED / LOW confidence
reason: source-faithful form geometry produced zero action-start thumbnail requests in 18 of 18 samples
```

This does not prove that an old-document request was impossible in the historical runtime. Real response timing, image dimensions, layout timing, fallback replacement and application execution may differ. It does prove that the earlier generic probe cannot carry the causal weight previously assigned to it.

## Current bounded conclusion

Proven:

- publication feedback is session flash only;
- the defect reproduced intermittently after session serialization;
- the Wiki form creates same-session authenticated thumbnail requests;
- mobile completes more contaminated thumbnail responses and aborts fewer requests than desktop/tablet in the preserved reproductions;
- source-faithful actionability scrolling did not create new thumbnail requests in the controlled 18-sample matrix.

Unknown:

- which request, if any, consumed or removed publication `status`;
- whether the loss occurs before publish save, between publish and redirect GET, during redirect handling or through a separate session lifecycle path;
- exact request/session-lock order;
- causal contribution of damaged media rows;
- exact frozen clean and exactly-one-corrupt behavior.

The root cause is therefore `UNKNOWN`. The old-document media-request chain remains a low-confidence hypothesis, not the leading high-confidence explanation.

## Validator impact

The exact frozen runbook remains necessary but must be interpreted hypothesis-neutrally. Immediate versus pre-scroll remains a useful control, but causal acceptance must not assume that pre-scroll should change the result.

Required evidence remains:

- browser request-start timestamp, initiator/frame and `Referer`;
- publish response and redirect navigation identity;
- `X-Request-ID` correlation;
- session-lock acquire/release and session load/save;
- sanitized `_flash.new`, `_flash.old` and `status` presence;
- exact clean and exactly-one-corrupt fixture proof;
- restored framework hash and empty final Git status.

No application, test, workflow, dependency, deployment or production change was made.
