# Issue #365 bounded flash remediation evidence

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Conclusion

The historical publication-flash loss has stronger remediation evidence than `UNKNOWN`, but it is not fully closed.

- `PROVEN`: both historical failure heads lacked session blocking on admin Wiki routes.
- `PROVEN`: commit `6c1e910d36771f50da5eded93cc50274a90c62d2` added Laravel session blocking to all administrator Wiki GET/POST routes, including media thumbnails and publication lifecycle transitions.
- `PROVEN`: the direct later critical run on `fdb45a4325949d3ab1c4860e3a4527553f11c789` included the blocking routes and passed the Wiki media publication flash assertion across Chromium, Firefox, WebKit, desktop, tablet and mobile with zero retries.
- `PROVEN`: the original Wiki administration scenario also passed across those projects after the change, but its publication-flash assertion had been removed; it proves durable publication only.
- `DERIVED`: session serialization is a strong candidate remediation for the historical race.
- `UNKNOWN`: the exact original Wiki administration flow has not reasserted the transient publication message on the frozen target.

Current classification: `PARTIALLY_PROVEN_REMEDIATED`; focused exact-target confirmation remains required.

## Historical failure boundary

Historical exact heads:

- `35f39b48233b186502cbdcc05aec7ffc40e78fc7`;
- `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`.

At the first head, `routes/modules/wiki.php` contains no `->block()` on:

- administrator Wiki pages;
- media index/thumbnail routes;
- article create/edit/update routes;
- submit-review and publish lifecycle POSTs;
- preview and revision routes.

Both historical mobile runs lost the expected publication `role=status` message while durable state showed `Published`, version 3 and `Unpublish to draft`.

## Targeted remediation commit

Commit:

- SHA `6c1e910d36771f50da5eded93cc50274a90c62d2`;
- title `fix(wiki): serialize admin session requests`;
- created `2026-07-31T07:18:39Z`.

It added `->block()` to every administrator Wiki route in the module, including:

- `admin.wiki.media.thumbnail`;
- `admin.wiki.articles.submit-review`;
- `admin.wiki.articles.publish`;
- all relevant page, preview and mutation routes.

The commit's own Acceptance E2E and Portal Acceptance Contract runs were cancelled, so that commit alone is source evidence, not full browser proof.

## Subsequent test composition

Two later test commits refined the evidence:

- `9b0c7b527413f60ac4f795bc5c4cc35f7a5d8904` — `test(wiki): compose durable and flash publication checks`;
- `2e713b4bb0bdd9777a8d75cb56c9f60999fef2eb` — `test(wiki): retain exact publication flash evidence`.

On the direct CI source `fdb45a4325949d3ab1c4860e3a4527553f11c789`:

### General Wiki administration scenario

`scripts/acceptance/tests/admin-wiki-administration.spec.mjs`:

- waits for `networkidle` after submit-review because the responsive editor issues authenticated thumbnail requests;
- publishes the article;
- asserts durable `Status: Published` and `Unpublish to draft`;
- no longer asserts the publication `role=status` message.

Therefore a pass of this scenario does not directly close the historical flash defect.

### Wiki Editorial Media scenario

`scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`:

- tracks session-bearing thumbnail requests;
- waits until they are idle before publishing;
- clicks Publish;
- explicitly asserts `role=status` contains `Wiki article published.`;
- explicitly asserts durable `Status: Published`.

This is direct flash evidence for a closely related and more media-intensive publication path.

## Direct later zero-retry results

Critical browser artifact:

- source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216753`;
- job `91164367653`;
- artifact `8794373786`;
- retries `0`.

Relevant JUnit results:

### General administration scenario

- portability Chromium: PASS;
- portability Firefox: PASS;
- portability WebKit: PASS;
- responsive desktop: PASS;
- responsive tablet: PASS;
- responsive mobile: PASS.

These passes prove durable lifecycle success but not the transient publication flash because that assertion is absent.

### Wiki media publication scenario with explicit flash assertion

- portability Chromium: PASS;
- portability Firefox: PASS;
- portability WebKit: PASS;
- responsive desktop: PASS;
- responsive tablet: PASS;
- responsive mobile: PASS;
- accessibility Chromium: PASS.

This directly proves the flash survived publication in the serialized, thumbnail-aware scenario across all representative engines and dimensions in the later direct run.

## Frozen-target relation

The frozen target and `fdb45a...` have the same blobs for:

- `routes/modules/wiki.php`;
- `scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`.

The audit already records their runtime boundary as equivalent by source comparison. The direct run remains `CI_PROVEN` for `fdb45a...`; frozen-target interpretation is `DERIVED`, not relabelled as exact-target execution.

## Finding status

```yaml
id: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
current_remediation_state: PARTIALLY_PROVEN_REMEDIATED
confidence: HIGH
proven:
  - historical heads lacked route session blocking
  - targeted serialization commit exists in frozen source
  - later direct zero-retry flash assertion passes in the thumbnail-aware Wiki media publication scenario across engines and viewports
unknown:
  - exact original administration scenario publication flash on frozen target because its assertion was removed
recommended_confirmation:
  - restore a focused non-flaky flash assertion to the original scenario
  - execute at least three clean isolated responsive-mobile samples on the frozen target
  - retain durable-state assertions and sanitized session/request logs
```

## Causality boundary

The evidence strongly supports session-request serialization as the intended remedy. It does not prove that stale damaged thumbnail rows were the sole historical cause. The validator must still compare clean isolated and controlled polluted flows without inferring causality from coexistence alone.
