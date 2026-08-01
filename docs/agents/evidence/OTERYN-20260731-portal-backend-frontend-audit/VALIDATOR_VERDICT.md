# Independent validator verdict

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Validation date: `2026-08-01`  
Verdict: `VALIDATED_WITH_CORRECTIONS`

## Executive correction

Fresh validation disproved the prior description of the historical Wiki flash defect as `PARTIALLY_PROVEN_REMEDIATED`.

The exact targeted session-serialization commit `6c1e910d36771f50da5eded93cc50274a90c62d2` still contained the original transient assertion in `admin-wiki-administration.spec.mjs`. Three independent zero-retry GitHub Actions attempts produced:

- one responsive-mobile PASS;
- two exact responsive-mobile reproductions of the missing `Wiki article published.` status message;
- durable `Published`, version 3 and `Unpublish to draft` evidence in both reproductions;
- desktop and tablet PASS in all three attempts;
- portability Chromium, Firefox and WebKit PASS in all three attempts.

Corrected state: `REPRODUCED_INTERMITTENT` after session serialization; current remediation state: `NOT_PROVEN_REMEDIATED`.

## Direct post-fix execution

Workflow run: `30612399525`  
Exact direct source: `6c1e910d36771f50da5eded93cc50274a90c62d2`  
Profile: `critical`  
Playwright retries: `0`  
Runtime: PHP 8.5, real Laravel HTTP, isolated MariaDB Platform/Canary, Redis ACL and MailHog.

| Attempt | Job | Artifact | Digest | Original responsive-mobile flow |
|---:|---:|---:|---|---|
| 2 | `91342520692` | `8815321615` | `sha256:5b2168f4952ba52f0a737b47d3a195a061c8ffc023d07cbfa115b643358d623a` | PASS |
| 3 | `91343023604` | `8815383351` | `sha256:7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | `sha256:790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | REPRODUCED |

Attempt 2 completed responsive 42/42. Its overall job later failed in a separate accessibility image-free draft test. Attempts 3 and 4 completed portability successfully and each failed responsive 41/42 only at the original mobile publication-flash assertion.

Detailed evidence is in `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

## Frozen-target relation

The reruns are direct `CI_PROVEN` evidence for `6c1e...`, not direct execution of the frozen SHA.

The frozen target and `6c1e...` have the identical `routes/modules/wiki.php` blob `f4a16ac017fd075b54904455bc8b6f05af304053`. The compare range contains no changes under `app/**`, `resources/views/**`, or Wiki route runtime. Later changes include acceptance tests/tooling, Marketplace/deployment configuration and documentation.

Therefore relevance to the frozen Wiki runtime is strong but `DERIVED`. Exact frozen-target classification remains unresolved because the current original test removed the transient assertion.

## Other fresh validation retained

A separate fresh rerun on direct source `fdb45a4325949d3ab1c4860e3a4527553f11c789` remains valid:

- run `30633216753`, attempt 2, job `91339118796`;
- critical profile 96/96 PASS, zero retries;
- smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2, accessibility 9/9;
- artifact `8814897157`, digest `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`.

That run proves current delivered critical profiles pass, but the original administration scenario no longer asserts its transient publication flash. The related media-intensive scenario does assert the flash and passes; it does not negate the direct intermittent reproduction in the original flow.

## Findings and severity

Normalized totals remain:

- 0 HIGH;
- 6 MEDIUM;
- 1 LOW.

`OTERYN-AUDIT-P35-005` remains MEDIUM because durable publication succeeds and the defect affects transient accessible feedback. Its evidence state changes from `PARTIALLY_PROVEN_REMEDIATED` to:

```yaml
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
samples:
  pass: 1
  reproduced: 2
```

No causal relationship between damaged EditorialMedia rows and flash loss is claimed.

## Residual boundaries

Still not executed:

- direct browser execution on exact frozen SHA with an ephemeral restored observer;
- clean isolation that resets EditorialMedia before each sample;
- a controlled comparison with exactly one missing/corrupt row;
- sanitized request/session/application logs for that comparison.

The three post-fix attempts used fresh runners and service containers, but the complete critical profile can accumulate deliberately damaged media rows inside an attempt. They prove reproduction under the delivered suite order, not causality.

## Verdict rationale

`VALIDATED_WITH_CORRECTIONS` remains the only supported verdict. The audit inventory, severity normalization and fixture-leak correction remain valid, while the remediation statement required a material correction based on direct new evidence.

`VALIDATED` remains forbidden until the exact frozen clean-versus-one-row package is executed. No implementation, merge, deployment, staging, production or Canary action is authorized.