# Validator packet addendum — corrected Issue #365 classification and hypothesis-neutral execution

This file is mandatory and supersedes conflicting thumbnail, request-origin or mechanism-confidence assumptions in `VALIDATOR_PACKET.md`, the older evidence index and the original consolidated report.

The normative command-level procedure is `ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md`. A mechanically equivalent implementation is acceptable only when it preserves the exact frozen source, 12-sample matrix, zero-retry rule, sanitized browser/server/session evidence, fail-closed checks and cleanup proof.

## Current baseline

```yaml
finding_totals:
  high: 0
  medium: 6
  low: 1
validator_verdict: VALIDATED_WITH_CORRECTIONS
issue_365:
  historical_state: PROVEN
  post_serialization_state: REPRODUCED_INTERMITTENT
  current_remediation_state: NOT_PROVEN_REMEDIATED
  root_cause: UNKNOWN
  old_document_lazy_thumbnail_race:
    classification: DERIVED
    confidence: LOW
```

`OTERYN-AUDIT-P35-006` remains `MEDIUM`, not `HIGH`. Historical thumbnail 500 responses are explained by intentionally damaged EditorialMedia rows leaking across acceptance projects. This is an acceptance isolation/evidence defect and not proof that valid production media fails.

Read before execution:

- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md`;
- `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`;
- `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`;
- corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`;
- `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
- `ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md`.

## Mechanism correction

The earlier generic responsive probe placed the publication control directly below an image grid. It proved that Playwright actionability scrolling can activate native-lazy images in some geometries.

The source-faithful probe copied the real Wiki form order and relevant responsive geometry. In 18 samples it recorded zero thumbnail request starts from the beginning of `Publish.click()` across desktop, tablet and mobile, in both immediate and explicit pre-scroll modes.

Therefore:

- the old-document media-request race remains possible in a real application runtime;
- it is not the leading `HIGH confidence` explanation;
- preserving pending `status` specifically across media responses is not yet a proven smallest repair;
- the validator must execute hypothesis-neutrally and identify the actual consuming request or lifecycle path.

A request created only after the redirected page HTML arrives still cannot explain why that first server-rendered HTML already lacks `session('status')`. The validator must distinguish all requests around the transition, but must not assume that an old-document thumbnail exists.

## Independent source verification

The validator must confirm:

1. `admin-wiki-editorial-media.spec.mjs` mutates stored objects through `corrupt-files` and `remove-files`.
2. The spec does not reset EditorialMedia before or after those tests.
3. `seed-browser-editorial-media.php reset` removes rows and stored files together.
4. `WikiEditorialMediaFileResponse` rejects missing or integrity-failed objects.
5. The dedicated editorial-media acceptance flow expects HTTP 500 for a deliberately corrupt thumbnail and verifies accessible fallback rendering.
6. Historical ordering predicts stale IDs `1/3/5`, then `1/3/5/7`, then `1/3/5/7/9`.
7. Preserved artifacts contain the corresponding response patterns.
8. The original Wiki administration test performs `networkidle`, checks `Publish` visibility, then clicks with retries zero.
9. The generic and source-faithful probes have different page geometry and must not be conflated.
10. The runbook restores only ephemeral observers and proves final Git/vendor cleanliness.

A disagreement requires exact counter-evidence and a correction to the audit.

## Mandatory matrix

Execute three independent zero-retry samples for every combination:

| Fixture | Immediate Publish | Explicit pre-scroll + settle |
|---|---:|---:|
| clean after EditorialMedia reset | 3 | 3 |
| exactly one corrupt row | 3 | 3 |

Total: 12 samples. A larger matrix is acceptable. A smaller matrix is incomplete.

### Clean samples

Before every sample:

```bash
php scripts/acceptance/seed-browser-editorial-media.php reset
php artisan cache:clear
```

Capture:

- exact tested SHA;
- initial and final EditorialMedia row/object state;
- publish POST and redirect chain;
- all accessible `role=status` texts;
- durable status/version and `Unpublish to draft`;
- every media-index and thumbnail request;
- browser diagnostics;
- sanitized server/session evidence.

### Exactly-one-corrupt samples

For each sample:

1. reset EditorialMedia;
2. seed exactly one valid row;
3. corrupt only that row's stored objects;
4. prove only that controlled row receives integrity-failure responses;
5. run the original mobile administration publication flow;
6. capture identical evidence;
7. reset after the sample.

### Immediate versus pre-scroll

The modes are controls, not predicted outcomes.

Immediate:

1. complete the existing pre-publication idle boundary;
2. start the existing `Publish` locator action immediately;
3. capture actionability scroll, request and session timing.

Pre-scroll:

1. explicitly scroll `Publish` into view;
2. wait for browser/media quiet;
3. start the same publication action;
4. capture identical evidence.

A difference may support an action-scroll trigger. Identical behavior may weaken it. Neither result identifies the consuming request without the matching session chain.

## Required correlation

For every relevant request record:

- monotonic browser request-start timestamp;
- frame, resource type, navigation identity, initiator when available and sanitized `Referer`;
- method, route, media ID, response status and `X-Request-ID`;
- monotonic server entry and response-end timestamps;
- session-lock attempt, acquire and release timestamps;
- session load/save snapshots containing only:
  - presence of `status`;
  - whether `status` is in `_flash.new`;
  - whether `status` is in `_flash.old`;
  - expected non-secret publication message or `[present-other]`;
  - hashed session ID.

Do not infer order from final response counts.

## Classification

### Fixture leakage

- `CONFIRMED`: source/order/count chain is independently verified.
- `CORRECTED`: any part is wrong; record exact correction.
- `REJECTED`: evidence proves a different source.

### Valid-object thumbnail health

- `PASS`: clean valid objects return no unexplained 5xx.
- `FAIL`: a valid object returns 5xx in clean isolation.
- `INCONCLUSIVE`: evidence is incomplete.

### Publication flash

- `REPRODUCED_CLEAN`;
- `REPRODUCED_POLLUTED_ONLY`;
- `REPRODUCED_BOTH`;
- `NOT_REPRODUCED`;
- `INCONCLUSIVE`.

### Consuming request or lifecycle path

- `PROVEN_CAUSAL`: complete matching-session chain identifies the request/path that removes or ages `status` before first redirected HTML rendering.
- `SUPPORTED`: strong correlated evidence exists but one causal link is incomplete.
- `PRESENT_NONCAUSAL`: candidate request/path exists but outcome does not track it.
- `REJECTED`: exact evidence contradicts the candidate.
- `INCONCLUSIVE`: correlation is incomplete.

The old-document thumbnail hypothesis must be classified using this same scale. It receives no preferred treatment.

## Verdict rule

`VALIDATED` remains forbidden while any of the following is missing:

- all 12 zero-retry samples;
- exact clean and exactly-one-corrupt fixture proof;
- browser request and redirect identity;
- request-ID correlation;
- session-lock and flash-state evidence;
- exact source, vendor and evidence hashes;
- original installed `StartSession` hash restoration;
- empty final Git status;
- independent source/order/count verification.

The final validator artifact must report the normalized totals as `0 HIGH / 6 MEDIUM / 1 LOW` or provide exact evidence supporting a correction.
