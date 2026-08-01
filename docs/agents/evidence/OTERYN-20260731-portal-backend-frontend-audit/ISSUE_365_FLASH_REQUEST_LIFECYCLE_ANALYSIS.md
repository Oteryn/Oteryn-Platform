# Issue #365 flash request-lifecycle analysis

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `DERIVED / LOW confidence mechanism hypothesis`; root cause remains `UNKNOWN`.

## Corrected conclusion

The intermittent responsive-mobile publication-flash loss remains directly reproduced after administrator Wiki session serialization, but the audit no longer assigns `HIGH confidence` to the old-document lazy-thumbnail race.

A source-faithful 18-sample Chromium probe copied the real Wiki form ordering, responsive geometry, media-card rules, large translation fields, exact Playwright viewports and native-lazy thumbnail attributes. It recorded **zero thumbnail request starts from the beginning of `Publish.click()` in every desktop, tablet and mobile sample**, both with direct action and explicit pre-scroll.

The earlier generic responsive probe placed the publication control immediately below the media grid. It proves only that actionability scrolling can activate lazy images in some layouts. The real form contains substantial content between the media picker and Lifecycle controls, so that generic result cannot support the causal weight previously assigned to it.

Detailed correction: `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md` and `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`.

## Proven source and runtime chain

### Publication feedback

`AdminWikiLifecycleController::publish()` redirects with session flash:

```php
return redirect()
    ->route('admin.wiki.articles.edit', $saved)
    ->with('status', $message);
```

`resources/views/admin/layout.blade.php` renders the accessible success message only when `session('status')` exists:

```blade
@if (session('status'))
    <div class="alert alert-success" role="status">{{ session('status') }}</div>
@endif
```

There is no database-backed fallback for the transient message.

### Media requests

The Wiki article form loads `public/js/wiki-admin-media.js`. The script performs an authenticated same-origin media-index fetch and creates thumbnail images with:

- `loading="lazy"`;
- `decoding="async"`;
- private administrator thumbnail URLs.

Article edit, media index, thumbnails and publication mutation use the administrator web session. The targeted source applies Laravel session blocking to these routes.

### Laravel flash lifecycle

The repository requires Laravel 13. Flash data is aged during session save. Session blocking provides mutual exclusion; it does not by itself prove redirect-document priority or deterministic preservation of `status`.

This establishes that a same-session request can be relevant to flash lifetime. It does not identify the request that caused the preserved failures.

## Direct post-serialization evidence

Exact source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

| Attempt | Job | Artifact | Responsive-mobile result |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

Both reproductions retained durable `Published`, version 3 and `Unpublish to draft` state. Desktop, tablet and Chromium/Firefox/WebKit portability passed.

Correct state:

```yaml
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
samples:
  pass: 1
  reproduced: 2
```

## Preserved timing

The original test executes:

1. `Submit for review`;
2. successful status assertion;
3. `page.waitForLoadState('networkidle')`;
4. visibility check for `Publish`;
5. `Publish.click()`;
6. publication status assertion.

Recovered Playwright API steps show:

| Attempt | Profile | `networkidle` | Publish click | Result |
|---:|---|---:|---:|---|
| 3 | desktop | 623 ms | 84 ms | PASS |
| 3 | tablet | 662 ms | 74 ms | PASS |
| 3 | mobile | 682 ms | 100 ms | REPRODUCED |
| 4 | desktop | 793 ms | 75 ms | PASS |
| 4 | tablet | 755 ms | 193 ms | PASS |
| 4 | mobile | 643 ms | 100 ms | REPRODUCED |

The visibility check begins 0–1 ms after `networkidle`; the click begins 4–6 ms later. These timings define the observation window but do not identify request initiation or session ordering.

## Embedded diagnostics boundary

The diagnostic collector records console errors, page errors, failed requests and HTTP responses with status at least 500. It does **not** record timestamps, initiator document, frame, `Referer`, request headers, navigation identity, correlation ID or session identity.

Recovered results:

| Attempt | Desktop | Tablet | Mobile |
|---:|---|---|---|
| 3 | PASS; 9×500; 6 aborted requests | PASS; 12×500; 8 aborted | REPRODUCED; 16×500; 0 aborted |
| 4 | PASS; 9×500; 6 aborted requests | PASS; 12×500; 8 aborted | REPRODUCED; 14×500; 0 aborted |

This proves viewport-dependent thumbnail completion/cancellation behavior. It also proves that HTTP 500 presence alone is insufficient to remove the publication flash. It does not show which document initiated any specific request or whether a thumbnail request consumed `status`.

## Probe reconciliation

### Generic responsive probe

`ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md` used a responsive image grid with the control immediately below it. Tablet and mobile direct action activated four deferred images; pre-scroll eliminated those starts.

Classification: `CONTROLLED_SYNTHETIC / GENERIC_FEASIBILITY`.

### Source-faithful layout probe

`ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md` used the real form ordering and relevant source geometry.

| Profile | Initially started | New starts from action start |
|---|---:|---:|
| desktop | 12/12 | 0/3 samples |
| tablet | 10/12 | 0/3 samples |
| mobile | 4/12 | 0/3 samples |

The same zero result occurred after explicit pre-scroll.

Classification: `CONTROLLED_SOURCE_FAITHFUL / DERIVED`.

The source-faithful result outweighs the generic geometry for assessing the actual page. It does not reproduce the application defect and cannot prove impossibility in the real runtime, but it invalidates `HIGH confidence` for the specific action-induced old-document thumbnail chain.

## Current classification

```yaml
finding: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause:
  classification: UNKNOWN
mechanism_hypotheses:
  old_document_media_request_ages_status:
    classification: DERIVED
    confidence: LOW
    supporting_evidence:
      - flash is session-only
      - media reads share the administrator session
      - mobile completes more contaminated thumbnail responses and aborts none
    counterevidence:
      - source-faithful 18-sample layout probe started zero thumbnails from Publish action start
      - embedded diagnostics contain no request timing or initiator identity
  other_session_request_or_flash_lifecycle_path:
    classification: UNKNOWN
  client_dom_removal_or_locator_error:
    classification: LOW_PROBABILITY
    reason: error context shows the first redirected document with durable Published state and no accessible status element
proven:
  - durable publication succeeds while transient accessible feedback can be absent
  - session serialization does not remediate the defect deterministically
  - thumbnail HTTP 500 presence alone is not sufficient
  - viewport changes thumbnail completion and cancellation behavior
not_proven:
  - the request that consumes or removes status
  - exact session-lock and save order
  - exact frozen clean-isolated outcome
  - exactly-one-corrupt-row behavior
  - causal contribution of integrity-failure responses
```

## Remediation boundary

No implementation is authorized by this audit.

The previous recommendation to preserve pending publication `status` specifically across Wiki media-index and thumbnail responses is now a **candidate requiring proof**, not the smallest proven repair. Implementing it before exact request/session correlation risks masking the symptom while leaving the actual lifecycle defect unchanged.

Any later implementation task must first identify the consuming request or framework path. Acceptable remedies may include:

- preventing proven read-only subrequests from aging pending publication feedback;
- decoupling proven media authorization from the mutable page session;
- replacing one-request session flash with redirect-bound feedback;
- correcting a different session lifecycle defect if instrumentation identifies one.

Client retries, delayed clicks, `networkidle` or pre-scroll remain diagnostic controls and are not production remediation.

## Required exact validation

The exact frozen runbook remains necessary and must be hypothesis-neutral. Preserve the immediate/pre-scroll cells as controls, but do not assume they should differ.

Required correlation:

1. browser request start with frame, initiator and `Referer`;
2. publish POST response and redirect navigation identity;
3. response `X-Request-ID`;
4. server entry and route identity;
5. session-lock attempt/acquire/release;
6. session load/save and sanitized `_flash.new`, `_flash.old`, `status` presence;
7. exact clean and exactly-one-corrupt fixture proof;
8. first redirected document status text and durable publication state;
9. original framework hash restoration and empty final Git status.

Causal promotion requires a complete matching-session chain. Temporal coexistence remains insufficient.
