# Validator packet addendum — corrected Issue #365 thumbnail classification and request-order differential

This file is mandatory and supersedes conflicting thumbnail or request-origin assumptions in `VALIDATOR_PACKET.md`.

## Corrected baseline

`OTERYN-AUDIT-P35-006` is `MEDIUM`, not `HIGH`.

The historical HTTP 500 responses are explained by a proven acceptance-state leak:

- the Wiki media spec intentionally corrupts/removes EditorialMedia files;
- it leaves the rows and has no reset hook;
- later browser projects request those stale rows;
- the integrity service rejects their missing/corrupt bytes;
- the dedicated fallback test explicitly expects HTTP 500 for a deliberately corrupt thumbnail and verifies accessible fallback rendering.

Read these artifacts before execution:

- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md`;
- `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`;
- `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`;
- `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`.

## Corrected request boundary

The missing alert is absent from the first rendered article-edit page after the publish redirect. Requests created only after that page arrives cannot be the primary reason its server-rendered HTML omitted `session('status')`.

The focused validator must therefore distinguish:

- pending or newly activated media requests from the **old article-edit document**;
- the publish POST;
- the redirect GET;
- media requests created by the **new redirected document**.

The strongest current mechanism family is an old-document lazy thumbnail request queued during the far-down Playwright publication action. It is `DERIVED / HIGH confidence`, not direct causal proof.

## Required independent verification

The validator must independently confirm:

1. `admin-wiki-editorial-media.spec.mjs` mutates files through `corrupt-files` and `remove-files`.
2. The spec performs no EditorialMedia reset before or after its tests.
3. `seed-browser-editorial-media.php reset` is the reviewed helper that removes the rows and stored files together.
4. `WikiEditorialMediaFileResponse` rejects missing/integrity-failed objects.
5. `editorial-media-acceptance.spec.mjs` explicitly expects HTTP 500 for the deliberately corrupt thumbnail and checks the accessible fallback.
6. Historical portability and responsive ordering predicts IDs 1/3/5, then 1/3/5/7, then 1/3/5/7/9.
7. The preserved reports contain the corresponding historical response patterns.
8. The original administration flow begins the publication action only milliseconds after its pre-publication idle boundary.
9. The Wiki form uses native lazy thumbnails and places responsive media content before the publication controls.

A disagreement requires exact counter-evidence and a correction to the audit.

## Replacement focused procedure

Execute three linked classes of exact-target probes.

### A. Clean isolated flow

Before every sample:

```bash
php scripts/acceptance/seed-browser-editorial-media.php reset
php artisan cache:clear
```

Then run the original mobile Wiki administration scenario with retries disabled. Run at least three independent samples.

For every sample capture:

- exact tested SHA;
- initial and final EditorialMedia row count;
- publish POST status and redirect chain;
- all accessible `role=status` texts;
- durable status/version and `Unpublish to draft` action;
- every thumbnail request status and media ID;
- browser diagnostics;
- sanitized application/server logs.

Classify independently:

- valid-object thumbnail response;
- publication flash persistence;
- durable publication result.

A clean sample with no 500 response supports the fixture-leak diagnosis. It does not invalidate P35-006.

### B. Controlled polluted flow

For each controlled sample:

1. reset EditorialMedia;
2. seed exactly one media row;
3. deliberately corrupt or remove only that row's stored objects;
4. load the Wiki administration form;
5. prove that only the controlled row receives the integrity-failure response;
6. capture fallback rendering and publish-flash behavior;
7. reset after the sample.

Run enough independent zero-retry samples to determine whether controlled integrity-failure traffic changes publication-flash persistence.

### C. Old-document request-order differential

Run this pair for both clean media and exactly one controlled missing/corrupt row.

#### C1. Immediate action

1. load the original administration flow;
2. wait for the existing pre-publication idle boundary;
3. immediately invoke the existing `Publish` locator action;
4. capture the full request and session timeline.

#### C2. Pre-scroll control

Repeat the same flow, but before publication:

1. explicitly scroll `Publish` into view;
2. wait for old-document media work to settle;
3. then invoke the same publication action;
4. capture the identical timeline.

For every relevant request record:

- monotonic browser request-start timestamp;
- monotonic server request-entry and response-end timestamps;
- route, method, status, media ID and sanitized `Referer`;
- correlation/request ID;
- old-document versus redirected-document origin;
- session-lock wait, acquire and release timestamps;
- sanitized session-load and session-save state for `_flash.new`, `_flash.old` and presence of `status`.

The evidence must make the ordering of old thumbnail, publish POST and redirect GET mechanically inspectable. Do not infer order from final response counts.

A failure only in C1 supports an action-scroll request trigger. Identical C1/C2 behavior weakens or rejects that trigger. Neither result alone proves that integrity failure is causal.

## Classification

### P35-006

- `CONFIRMED`: the stale-row chain and historical order/count mapping are independently reproduced or fully verified from exact artifacts and source.
- `CORRECTED`: any part of the source/order/count chain is wrong; record exact corrections.
- `REJECTED`: evidence proves the historical 500 responses came from valid objects or a different source.

### Valid-object thumbnail health

- `PASS`: all clean samples return no unexplained 5xx for valid objects.
- `FAIL`: a valid object returns 5xx in a clean isolated sample.
- `INCONCLUSIVE`: environment or evidence is incomplete.

A clean valid-object failure is a separate application finding; do not merge it into the fixture-isolation finding.

### Publication flash

- `REPRODUCED_CLEAN`: missing flash occurs without stale thumbnail traffic.
- `REPRODUCED_POLLUTED_ONLY`: missing flash occurs only with controlled integrity-failure requests.
- `NOT_REPRODUCED`: all required clean and polluted samples retain correct accessible feedback.
- `INCONCLUSIVE`: evidence is incomplete.

### Old-document action-scroll trigger

- `SUPPORTED`: C1 activates an old-document media request in the publish window and C2 moves that request outside the window, with a corresponding flash result difference.
- `PRESENT_NONCAUSAL`: the old-document request exists but flash persistence does not track the differential.
- `REJECTED`: no old-document media request is activated in the relevant window or C1/C2 exact evidence contradicts the mechanism.
- `INCONCLUSIVE`: request/session ordering is incomplete.

Do not infer causality from temporal coexistence alone.

## Verdict rule

`VALIDATED` remains forbidden while:

- the clean isolated flow is not executed at least three times;
- the controlled polluted comparison is absent;
- the C1/C2 request-order differential is absent;
- session-lock and flash-state evidence is incomplete;
- the validator does not independently verify the source/order/count chain.

The final artifact must explicitly state that the normalized audit totals are expected to be:

- `HIGH: 0`;
- `MEDIUM: 6`;
- `LOW: 1`;

or provide exact evidence supporting a correction.