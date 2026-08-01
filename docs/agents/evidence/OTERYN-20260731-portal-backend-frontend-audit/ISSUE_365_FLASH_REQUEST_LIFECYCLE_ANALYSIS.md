# Issue #365 flash request-lifecycle analysis

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `DERIVED / HIGH confidence mechanism family`; exact trigger and request order remain unproven.

## Corrected conclusion

The strongest source-backed mechanism for the intermittent mobile publication-flash loss is **request-order consumption of Laravel flash data by an authenticated Wiki media request originating from the old article-edit document**.

The previous broader wording allowed requests started by the newly redirected page. That boundary is too broad: the alert is absent from the first rendered article-edit page after the redirect, so requests created only after that HTML arrives cannot be the primary reason its server render omitted `session('status')`.

The viable race window is:

1. the old article-edit document is open and apparently idle at its current scroll position;
2. Playwright begins the far-down `Publish` action and scrolls that old document into position;
3. the scroll may activate deferred lazy thumbnail requests from the old document;
4. the publish POST acquires the session lock, writes the one-request `status` flash and releases the lock with the redirect response;
5. a queued old-document media request may acquire the serialized session before the redirect GET, save it and age the flash;
6. the redirect GET then renders durable publication state without the transient success alert.

Session blocking serializes same-session requests. It does not provide redirect-document priority or a proven fair queue order.

## Repository source chain

### Publication response

`AdminWikiLifecycleController::publish()` delegates to `transition()`, which redirects to the article edit route with a flashed status:

```php
return redirect()
    ->route('admin.wiki.articles.edit', $saved)
    ->with('status', $message);
```

The success message therefore exists only as Laravel session flash data.

### Status rendering

`resources/views/admin/layout.blade.php` renders the accessible alert only when `session('status')` exists:

```blade
@if (session('status'))
    <div class="alert alert-success" role="status">{{ session('status') }}</div>
@endif
```

There is no database-backed or redirect-parameter fallback for the transient message.

### Old-document media request source

`resources/views/admin/wiki/articles/form.blade.php` exposes the authenticated media-index route and loads `public/js/wiki-admin-media.js`.

The script immediately loads the index and creates thumbnail images with:

- `loading="lazy"`;
- `decoding="async"`;
- authenticated same-origin thumbnail URLs.

The responsive form places the media picker before the publication controls. Narrow layouts produce a taller one-column media grid, leaving more thumbnails deferred when the page is initially idle near the top.

### Route and session boundary

Article edit, media index, thumbnails and publication mutation are administrator Wiki web routes. The targeted source applies `->block()` to them, so they use the same Laravel session and session lock rather than a stateless media path.

`AdminWikiMediaController::index()` returns up to 12 rows and a private thumbnail URL for each row. `thumbnail()` returns the private media response. Both are session-bearing requests.

## Laravel 13 flash lifecycle

The repository requires `laravel/framework ^13.8`.

Laravel flash data is intended for the subsequent request. In Laravel 13 `Illuminate\Session\Store`:

- `flash()` stores the value and records the key in `_flash.new`;
- session `save()` calls `ageFlashData()`;
- `ageFlashData()` deletes `_flash.old`, moves `_flash.new` to `_flash.old`, and clears `_flash.new`.

Consequently, after the publish response writes `status`, the next same-session request that saves the session can age or remove it even if that request is a JSON or image response rather than the redirect document.

`->block()` supplies mutual exclusion only. It does not specify that the redirect GET must acquire the lock before an already queued media request.

## Preserved execution timing

In the two mobile reproductions, the test started the publication action only 6–7 ms after the recorded `networkidle` step ended. The desktop and tablet actions also began only 5–6 ms after that boundary.

The publication click itself took approximately:

| Attempt | Desktop | Tablet | Mobile |
|---:|---:|---:|---:|
| 3 | 84 ms | 74 ms | 100 ms |
| 4 | 75 ms | 193 ms | 100 ms |

`networkidle` therefore proves only that no qualifying request was active before the action began. It does not prove that Playwright's actionability scroll could not activate old-document lazy thumbnails during the click window.

## Controlled synthetic responsive probe

A local Chromium probe reproduced the relevant browser behavior without using Oteryn HTTP or Laravel:

- 12 native lazy images;
- responsive grid with 3 desktop, 2 tablet and 1 mobile column;
- publication control below the grid;
- three samples per viewport and mode.

Results:

| Profile | Initially loaded | New loads after direct click | New loads after pre-scroll + settle |
|---|---:|---:|---:|
| desktop | 12/12 | 0 | 0 |
| tablet | 8/12 | 4 (`9–12`) | 0 |
| mobile | 3/12 | 4 (`9–12`) | 0 |

The direct action moved the mobile viewport from the top to `scrollY=5437`; the four deferred images completed 12.9–17.9 ms after the click event. Explicitly pre-scrolling the control and settling before the click moved the lazy work out of the action window and eliminated post-click image loads in every sample.

This proves that the responsive Playwright action can create new old-document lazy-load work after an earlier settled boundary. It does **not** timestamp Oteryn request initiation, session-lock acquisition or flash consumption.

Detailed synthetic evidence: `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`.

## Runtime evidence fit

Three post-serialization attempts of run `30612399525` produced one mobile pass and two exact mobile reproductions.

Attempts 3 and 4 retained durable `Published`, version 3 and `Unpublish to draft`, while the accessible publication status was absent. Desktop, tablet and all portability projects passed.

Recovered diagnostics show:

- desktop PASS with 9 thumbnail HTTP 500 responses;
- tablet PASS with 12 thumbnail HTTP 500 responses;
- mobile REPRODUCED with 16 and 14 thumbnail HTTP 500 responses;
- no page errors in the original-flow projects;
- zero Playwright failed-request entries in both mobile reproductions.

This disproves the simple rule that any thumbnail 500 necessarily removes the flash. It remains consistent with request ordering: the relevant variable is whether an old-document session request is queued between the publish response and redirect GET, not merely whether a 500 exists.

## Classification

```yaml
finding: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
mechanism_family:
  classification: DERIVED
  confidence: HIGH
  statement: an old-document authenticated media request may age pending publication status before the redirect GET renders
proven:
  - publication status is session flash only
  - the old Wiki form creates authenticated lazy thumbnail requests
  - all involved routes use the same web session and session blocking
  - Laravel ages flash during session save
  - the original mobile symptom reproduces after session blocking
  - a controlled responsive Playwright action can activate deferred old-document lazy-image work after a prior settled boundary
not_proven:
  - exact old-document thumbnail request start in attempts 3 and 4
  - session-lock acquisition order in the reproductions
  - whether one valid missing or corrupt thumbnail is sufficient
  - whether integrity failure changes scheduling or lock timing
  - exact frozen clean-isolated outcome
```

## Smallest safe remediation direction

No implementation is authorized by this audit.

The smallest later server-side candidate remains to ensure Wiki media-index and thumbnail responses do **not consume a pending publication `status` flash**. Any middleware must preserve only that intended key and must prove that the redirected article-edit document consumes it exactly once.

Broader alternatives are:

- use a read-only or non-aging session path for authenticated media reads;
- decouple media authorization from the page session;
- replace one-request session flash with a redirect-bound status representation.

Client-only `networkidle`, delayed clicks or retries are not production remediation. Explicit pre-scroll is a diagnostic control, not a user-facing fix.

## Required exact validation

A mutable exact-target validator must pair the existing clean and exactly-one-damaged-row samples with:

1. immediate `Publish` action after the current idle boundary;
2. explicit pre-scroll of `Publish`, media settle, then publication;
3. monotonic browser request-start and server request-entry timestamps;
4. request route, media ID, `Referer`, correlation ID and originating document/navigation;
5. session-lock acquire/release and session-save timestamps;
6. sanitized `_flash.new`, `_flash.old` and `status` presence at session load/save boundaries;
7. proof that the redirected document consumes the status exactly once.

The exact validator packet remains required before promoting the audit verdict to `VALIDATED`.