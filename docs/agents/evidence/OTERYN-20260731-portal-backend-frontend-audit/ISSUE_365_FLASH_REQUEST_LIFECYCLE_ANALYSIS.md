# Issue #365 flash request-lifecycle analysis

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `DERIVED / HIGH confidence`; not a replacement for the remaining clean-versus-controlled runtime probe.

## Conclusion

The strongest source-backed mechanism for the intermittent mobile publication-flash loss is **request-order consumption of Laravel flash data by authenticated Wiki media subrequests**.

Session blocking serializes requests sharing one session. It does not guarantee that the redirected article-edit GET is the first request that saves the session after the publish POST. A queued JSON media-index request or thumbnail request may acquire the session lock first and consume the one-request flash lifetime before the redirect page renders it.

This mechanism explains why:

- durable publication succeeds;
- the redirect page can render `Published`, version 3 and `Unpublish to draft`;
- the transient `Wiki article published.` status can be absent;
- `->block()` reduces concurrency risk but does not deterministically close the defect;
- `networkidle` before publishing improves timing but does not control requests scheduled after form submission or navigation;
- mobile can reproduce while desktop/tablet pass under the same contaminated media set.

## Repository source chain

### Publication response

`AdminWikiLifecycleController::publish()` delegates to `transition()`, which returns:

```php
return redirect()
    ->route('admin.wiki.articles.edit', $saved)
    ->with('status', $message);
```

The success message therefore exists only as session flash data.

### Status rendering

`resources/views/admin/layout.blade.php` renders the message only when `session('status')` is present:

```blade
@if (session('status'))
    <div class="alert alert-success" role="status">{{ session('status') }}</div>
@endif
```

There is no durable database-backed or redirect-parameter fallback for this transient status.

### Automatic session-bearing media requests

`resources/views/admin/wiki/articles/form.blade.php` always loads `public/js/wiki-admin-media.js` and exposes the authenticated media-index route.

The script immediately calls `load(indexUrl)`. That function performs a same-origin credentialed `fetch`, then creates lazy, asynchronously decoded images whose `src` values point at authenticated Wiki thumbnail routes.

Consequently, loading or reloading the article form creates this request family under the same browser session:

1. article-edit document request;
2. automatic media-index JSON request;
3. zero or more lazy thumbnail requests.

### Route and session boundary

The article-edit, media-index, thumbnail and publish routes are all administrator Wiki web routes. The frozen source applies `->block()` to those routes, so they share Laravel session locking rather than becoming stateless.

`AdminWikiMediaController::index()` returns up to 12 rows and a private thumbnail URL for every row. `thumbnail()` returns the private media response. Both requests use the authenticated web session.

## Laravel 13 flash lifecycle

The repository requires `laravel/framework ^13.8`.

Laravel documents flash data as available for the subsequent HTTP request and deleted after that request. In Laravel 13 `Illuminate\Session\Store`:

- `flash()` stores the value and records the key in `_flash.new`;
- every `save()` calls `ageFlashData()`;
- `ageFlashData()` deletes `_flash.old`, moves `_flash.new` to `_flash.old`, and clears `_flash.new`.

Therefore:

1. publish POST flashes `status` and saves it as old flash data;
2. the first later request using and saving that session may read the value;
3. when that request saves, the old flash key is forgotten;
4. a second later request no longer sees the status.

Session `block()` provides mutual exclusion only. It does not assign redirect navigation higher priority than pending JSON/image requests.

## Runtime evidence fit

Three post-serialization attempts of run `30612399525` produced one mobile pass and two exact mobile reproductions.

Recovered diagnostics for attempts 3 and 4 show:

- desktop PASS with 9 thumbnail HTTP 500 responses;
- tablet PASS with 12 thumbnail HTTP 500 responses;
- mobile REPRODUCED with 16 and 14 thumbnail HTTP 500 responses;
- no page errors in the original-flow projects;
- zero Playwright failed-request entries in both mobile reproductions;
- durable article publication in both failures.

This disproves the simple rule that any thumbnail 500 necessarily removes the flash. It remains consistent with request ordering: the relevant variable is which session-bearing request saves first after the publish response, not merely whether an integrity-failure response exists.

## Classification

```yaml
finding: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
mechanism:
  classification: DERIVED
  confidence: HIGH
  statement: authenticated media subrequests can consume one-request Laravel flash before the redirected edit document renders it
proven:
  - publication status is session flash only
  - the edit page automatically starts same-session JSON and thumbnail requests
  - all involved routes use the web session and session blocking
  - Laravel ages flash on every session save
  - the original mobile symptom reproduces after session blocking
not_proven:
  - exact request ordering in the preserved reproductions
  - whether one valid or corrupt thumbnail request is sufficient
  - whether integrity failure changes scheduling or lock timing
  - exact frozen clean-isolated outcome
```

## Smallest safe remediation direction

No implementation is authorized by this audit. The smallest server-side candidate for a later implementation task is to ensure Wiki media-index and thumbnail requests do **not consume the pending publication `status` flash**.

A narrowly scoped candidate is middleware or endpoint logic that, when a `status` flash exists, keeps only that key for the next request before the media response saves the session. This must be validated against Laravel's flash arrays and must not extend unrelated flash state.

Broader alternatives are:

- make authenticated media reads use a read-only/non-aging session path;
- decouple media authorization from the page session;
- replace one-request session flash with a redirect-bound status representation.

Client-only `networkidle`, delayed clicks or retries are not sufficient remediation because they do not establish server-side request ordering.

## Required implementation validation

A later authorized repair must include:

1. a focused server test proving media-index requests preserve a pending `status` flash for the following article-edit request;
2. the same test for a valid thumbnail response;
3. the same test for an integrity-failure thumbnail response;
4. proof that the article-edit document consumes the flash exactly once;
5. three zero-retry clean responsive-mobile publication samples;
6. controlled valid-media and exactly-one-damaged-row mobile samples;
7. sanitized request ordering and session-flash state evidence.

The exact validator packet remains required before promoting the audit verdict to `VALIDATED`.