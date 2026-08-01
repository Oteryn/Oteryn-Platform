# Issue #365 flash remediation evidence — corrected

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Corrected conclusion

The prior `PARTIALLY_PROVEN_REMEDIATED` conclusion is superseded by direct post-fix reruns recorded in `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

Current state:

```yaml
id: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
samples:
  pass: 1
  reproduced: 2
confidence: HIGH
```

## What remains proven

- Historical exact heads `35f39b48233b186502cbdcc05aec7ffc40e78fc7` and `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb` lost the accessible mobile publication status after durable success.
- Commit `6c1e910d36771f50da5eded93cc50274a90c62d2` added Laravel `->block()` to all administrator Wiki routes, including thumbnail, submit-review and publish requests.
- Later source `fdb45a4325949d3ab1c4860e3a4527553f11c789` passes a related media-intensive scenario that explicitly asserts `Wiki article published.`.
- The current original administration scenario passes durable publication but removed the historical transient assertion.

## New direct disproof of deterministic remediation

At exact commit `6c1e910d36771f50da5eded93cc50274a90c62d2`, the original administration spec still contains:

```js
await expect(page.getByRole('status')).toContainText('Wiki article published.');
await expect(page.getByText(/Status: Published/u)).toBeVisible();
```

Three independent zero-retry attempts of workflow run `30612399525` produced:

| Attempt | Job | Artifact | Original responsive-mobile result |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

In both reproductions:

- no `role=status` containing `Wiki article published.` appeared within 10 seconds;
- durable state showed `Published`, version 3 and `Unpublish to draft`;
- desktop and tablet passed;
- portability Chromium, Firefox and WebKit passed.

Therefore session serialization cannot be claimed as a deterministic remediation. It remains a plausible mitigation or relevant concurrency control, but the defect survives it under the delivered critical-suite execution order.

## Frozen-target boundary

The frozen target and `6c1e...` share the exact `routes/modules/wiki.php` blob `f4a16ac017fd075b54904455bc8b6f05af304053`. The compare range contains no changes under `app/**`, `resources/views/**`, or Wiki route runtime. Acceptance-suite composition changed later.

The three reruns are direct `CI_PROVEN` evidence for `6c1e...`. Their relevance to the frozen Wiki runtime is `DERIVED`; they are not relabelled as direct frozen-target execution.

## Causality boundary

The complete critical profile can accumulate deliberately damaged EditorialMedia rows inside an attempt. The reruns prove intermittent post-serialization reproduction under delivered suite ordering. They do not prove that damaged rows cause the flash loss.

Still required:

- exact frozen SHA with an ephemeral restored observer;
- clean EditorialMedia reset before each sample;
- one controlled comparison with exactly one damaged row;
- sanitized publish/session/request/thumbnail/application evidence.