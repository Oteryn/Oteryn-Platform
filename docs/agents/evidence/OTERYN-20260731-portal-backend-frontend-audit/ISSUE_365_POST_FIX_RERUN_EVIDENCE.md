# Issue #365 post-fix original-flow rerun evidence

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Direct rerun source: `6c1e910d36771f50da5eded93cc50274a90c62d2`

## Conclusion

The historical mobile publication-flash defect is **reproduced after the session-serialization change**. Session blocking is therefore not proven to remediate the defect deterministically.

Three independent GitHub Actions attempts used fresh production-like dependency bootstrap, real Laravel HTTP, MariaDB Platform/Canary schemas, Redis ACL, MailHog, PHP 8.5 and Playwright retries set to zero. The original `admin-wiki-administration.spec.mjs` at this source still asserted the transient accessible message `Wiki article published.` immediately after the publish redirect.

Result for the original flow:

- attempt 2: responsive mobile PASS;
- attempt 3: responsive mobile FAIL — publication flash absent, durable publication present;
- attempt 4: responsive mobile FAIL — publication flash absent, durable publication present.

Normalized result: **1 PASS / 2 REPRODUCED**. Desktop and tablet passed in every attempt. Portability Chromium, Firefox and WebKit passed in every attempt.

## Why this source is relevant

Commit `6c1e910d36771f50da5eded93cc50274a90c62d2` is the exact targeted fix `fix(wiki): serialize admin session requests`. It adds Laravel `->block()` to every administrator Wiki route, including thumbnail, submit-review and publish routes.

At that exact commit, the original administration spec contains both:

```js
await expect(page.getByRole('status')).toContainText('Wiki article published.');
await expect(page.getByText(/Status: Published/u)).toBeVisible();
```

The frozen target and this post-fix source have the same blob for `routes/modules/wiki.php`: `f4a16ac017fd075b54904455bc8b6f05af304053`. The compare range contains no changes under `app/**`, `resources/views/**`, or Wiki route runtime; later differences include acceptance tests/tooling, Marketplace configuration, deployment and documentation. Therefore the direct reruns remain `CI_PROVEN` for `6c1e...`; relevance to the frozen Wiki runtime is `DERIVED`, not relabelled as exact frozen-target execution.

## Attempts

| Attempt | Job | Artifact | GitHub artifact digest | Responsive result | Original mobile flow |
|---:|---:|---:|---|---|---|
| 2 | `91342520692` | `8815321615` | `sha256:5b2168f4952ba52f0a737b47d3a195a061c8ffc023d07cbfa115b643358d623a` | 42/42 PASS | PASS |
| 3 | `91343023604` | `8815383351` | `sha256:7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | 41/42; one failure | REPRODUCED |
| 4 | `91343514611` | `8815457044` | `sha256:790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | 41/42; one failure | REPRODUCED |

All attempts belong to workflow run `30612399525` and exact head SHA `6c1e910d36771f50da5eded93cc50274a90c62d2`. Each workflow attempt provisioned a new runner and fresh service containers. These are independent samples, not Playwright retries.

## Attempt 2

JUnit:

- portability: 36 tests, 0 failures;
- responsive: 42 tests, 0 failures;
- original Wiki administration flow passed in portability Chromium, Firefox and WebKit;
- original flow passed in responsive desktop, tablet and mobile.

The overall job later failed in a separate accessibility-only image-free Wiki draft test. That failure occurred after the required portability/responsive evidence and does not invalidate their completed JUnit results.

## Attempts 3 and 4

In both attempts:

- portability: PASS;
- responsive desktop original flow: PASS;
- responsive tablet original flow: PASS;
- responsive mobile original flow: FAIL at the exact transient flash assertion;
- expected: `Wiki article published.` in `role=status`;
- observed: no status element within 10 seconds;
- error context independently showed `Status: Published`, version 3 and `Unpublish to draft`.

Thus publication durably succeeded while accessible transient success feedback was lost, matching the historical Issue #365 symptom.

## Recovered embedded browser diagnostics

The Playwright HTML reports embed a base64-encoded report ZIP. Decoding the embedded reports recovered sanitized diagnostics for successful desktop/tablet tests and the failed mobile test.

Attempt 3:

- desktop PASS: 9 thumbnail HTTP 500 responses across IDs `1/3/5`, 6 aborted requests, 2 invalid-pattern console errors;
- tablet PASS: 12 thumbnail HTTP 500 responses across IDs `1/3/5/7`, 8 aborted requests, 2 invalid-pattern console errors;
- mobile REPRODUCED: 16 thumbnail HTTP 500 responses across IDs `1/3/5/7/9`, zero failed-request entries, 2 invalid-pattern console errors.

Attempt 4:

- desktop PASS: 9 thumbnail HTTP 500 responses across IDs `1/3/5`, 6 aborted requests, 2 invalid-pattern console errors;
- tablet PASS: 12 thumbnail HTTP 500 responses across IDs `1/3/5/7`, 8 aborted requests, 2 invalid-pattern console errors;
- mobile REPRODUCED: 14 thumbnail HTTP 500 responses across IDs `1/3/5/7/9`, zero failed-request entries, 2 invalid-pattern console errors.

Exact per-ID distributions, report hashes and extraction boundaries are preserved in `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`.

Desktop and tablet retain the publication flow despite contaminated thumbnail traffic. Therefore the presence of thumbnail HTTP 500 responses alone is not sufficient to remove the flash. Mobile failures coexist with the expanded stale-ID set, but the evidence does not prove causation or explain the 14-versus-16 response difference.

## Fixture boundary

These were complete critical-profile attempts. They were freshly bootstrapped per attempt, but the profile itself runs Wiki media scenarios before later responsive projects and can accumulate intentionally damaged EditorialMedia rows inside the same attempt. Therefore these runs prove post-serialization reproduction under the delivered critical-suite execution order. They are not the validator packet's still-missing controlled comparison with exactly one damaged row, and they do not prove that damaged rows cause the flash loss.

## Corrected finding state

```yaml
id: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
samples:
  pass: 1
  reproduced: 2
current_remediation_state: NOT_PROVEN_REMEDIATED
confidence: HIGH
proven:
  - durable publication succeeds when transient mobile success feedback is absent
  - the exact targeted session-serialization commit still reproduces the original mobile symptom in two of three independent zero-retry attempts
  - session serialization alone is insufficient to claim deterministic remediation
  - contaminated desktop and tablet samples preserve the flow despite 9 and 12 thumbnail HTTP 500 responses
unknown:
  - exact frozen-target result with the transient assertion restored ephemerally
  - causal contribution of damaged EditorialMedia requests
  - result of a controlled comparison with exactly one damaged row
  - clean valid-object thumbnail health
```

## Remaining exact boundary

A mutable checkout-capable validator is still required for:

1. the exact frozen SHA with an ephemeral observer/restored transient assertion;
2. a clean isolated sequence that resets EditorialMedia before each sample;
3. a controlled comparison containing exactly one missing or corrupt EditorialMedia row;
4. sanitized publish/session/request/thumbnail/application logs.

No application, test, workflow, deployment or Canary mutation was committed by this audit.