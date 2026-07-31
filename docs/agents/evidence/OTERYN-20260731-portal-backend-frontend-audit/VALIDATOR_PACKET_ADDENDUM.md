# Validator packet addendum — corrected Issue #365 thumbnail classification

This file is mandatory and supersedes conflicting thumbnail assumptions in `VALIDATOR_PACKET.md`.

## Corrected baseline

`OTERYN-AUDIT-P35-006` is `MEDIUM`, not `HIGH`.

The historical HTTP 500 responses are explained by a proven acceptance-state leak:

- the Wiki media spec intentionally corrupts/removes EditorialMedia files;
- it leaves the rows and has no reset hook;
- later browser projects request those stale rows;
- the integrity service rejects their missing/corrupt bytes;
- the dedicated fallback test explicitly expects HTTP 500 for a deliberately corrupt thumbnail and verifies accessible fallback rendering.

Read `ISSUE_365_STATIC_CAUSE_ANALYSIS.md` before execution.

## Required independent verification

The validator must independently confirm:

1. `admin-wiki-editorial-media.spec.mjs` mutates files through `corrupt-files` and `remove-files`.
2. The spec performs no EditorialMedia reset before or after its tests.
3. `seed-browser-editorial-media.php reset` is the only reviewed helper that removes the rows and stored files together.
4. `WikiEditorialMediaFileResponse` rejects missing/integrity-failed objects.
5. `editorial-media-acceptance.spec.mjs` explicitly expects HTTP 500 for the deliberately corrupt thumbnail and checks the accessible fallback.
6. Historical portability and responsive ordering predicts IDs 1/3/5, then 1/3/5/7, then 1/3/5/7/9.
7. The two preserved runs contain the corresponding 9/12/16 response counts.

A disagreement requires exact counter-evidence and a correction to the audit.

## Replacement focused procedure

Execute two separate classes of exact-target probes.

### A. Clean isolated flow

Before every sample:

```bash
php scripts/acceptance/seed-browser-editorial-media.php reset
php artisan cache:clear
```

Then run the unchanged mobile Wiki administration scenario with retries disabled. Run at least three independent samples.

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

Run enough samples to determine whether concurrent integrity-failure requests affect flash persistence. Do not use Playwright retries as samples.

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

Do not infer causality from temporal coexistence alone.

## Verdict rule

`VALIDATED` remains forbidden while:

- the clean isolated flow is not executed at least three times;
- the controlled polluted comparison is absent;
- application/server evidence is incomplete;
- the validator does not independently verify the source/order/count chain.

The final artifact must explicitly state that the normalized audit totals are expected to be:

- `HIGH: 0`;
- `MEDIUM: 6`;
- `LOW: 1`;

or provide exact evidence supporting a correction.
