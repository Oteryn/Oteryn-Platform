# Independent validator verdict

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Validation date: `2026-08-01`  
Verdict: `VALIDATED_WITH_CORRECTIONS`

## Executive correction

Fresh validation disproved the earlier description of the historical Wiki flash defect as `PARTIALLY_PROVEN_REMEDIATED`.

The exact targeted session-serialization source `6c1e910d36771f50da5eded93cc50274a90c62d2` retained the original transient assertion. Three independent zero-retry workflow attempts produced:

- one responsive-mobile PASS;
- two exact responsive-mobile reproductions of the missing `Wiki article published.` status;
- durable `Published`, version 3 and `Unpublish to draft` evidence in both reproductions;
- desktop, tablet and portability Chromium/Firefox/WebKit PASS in all three attempts.

Corrected state:

```yaml
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause: UNKNOWN
samples:
  pass: 1
  reproduced: 2
```

## Direct post-serialization execution

Workflow run: `30612399525`  
Exact source: `6c1e910d36771f50da5eded93cc50274a90c62d2`  
Profile: `critical`  
Playwright retries: `0`  
Runtime: PHP 8.5, real Laravel HTTP, isolated MariaDB Platform/Canary, Redis ACL and MailHog.

| Attempt | Job | Artifact | Digest | Responsive mobile |
|---:|---:|---:|---|---|
| 2 | `91342520692` | `8815321615` | `sha256:5b2168f4952ba52f0a737b47d3a195a061c8ffc023d07cbfa115b643358d623a` | PASS |
| 3 | `91343023604` | `8815383351` | `sha256:7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | `sha256:790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | REPRODUCED |

Attempt 2 completed responsive 42/42. Its overall job later failed in a separate accessibility image-free draft test. Attempts 3 and 4 completed portability successfully and each failed responsive 41/42 only at the original mobile publication-flash assertion.

## Embedded diagnostic result

| Attempt | Desktop | Tablet | Mobile |
|---:|---|---|---|
| 3 | PASS; 9×500; 6 aborted requests | PASS; 12×500; 8 aborted | REPRODUCED; 16×500; 0 aborted |
| 4 | PASS; 9×500; 6 aborted requests | PASS; 12×500; 8 aborted | REPRODUCED; 14×500; 0 aborted |

This proves:

- contaminated thumbnail traffic can coexist with successful publication feedback;
- thumbnail HTTP 500 presence alone is insufficient to explain flash loss;
- viewport affects thumbnail completion and cancellation;
- existing diagnostics do not identify request timing, initiator document, correlation ID or session-lock/session-save order.

No causal relationship between damaged EditorialMedia rows and flash loss is claimed.

## Mechanism-confidence correction

The earlier generic browser probe showed that a Playwright action can activate deferred lazy images when the action control sits immediately below a responsive image grid.

A later source-faithful probe copied the real frozen-source Wiki form ordering and relevant geometry. It executed three immediate and three pre-scroll samples per exact viewport, 18 total:

| Profile | Initially started thumbnails | New starts from Publish action start |
|---|---:|---:|
| desktop | 12 | 0 in 6/6 samples |
| tablet | 10 | 0 in 6/6 samples |
| mobile | 4 | 0 in 6/6 samples |

Therefore the specific old-document lazy-thumbnail race is corrected from `DERIVED / HIGH confidence` to `DERIVED / LOW confidence`. The root cause remains `UNKNOWN`.

This controlled evidence does not prove that an old-document request was impossible in the real application runtime. It proves that the generic simplified geometry cannot support the earlier app-specific confidence.

Canonical correction:

- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
- corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`;
- corrected `VALIDATOR_PACKET_ADDENDUM.md`.

## Frozen-target relation

The reruns are direct `CI_PROVEN` evidence for `6c1e...`, not direct execution of the frozen SHA.

The frozen target and `6c1e...` have the identical `routes/modules/wiki.php` blob `f4a16ac017fd075b54904455bc8b6f05af304053`. The compare range contains no changes under the relevant Wiki application views or route runtime. Relevance to frozen runtime is strong but remains `DERIVED` until exact frozen execution.

## Other retained validation

A separate fresh run on direct source `fdb45a4325949d3ab1c4860e3a4527553f11c789` remains valid:

- run `30633216753`, attempt 2, job `91339118796`;
- critical profile 96/96 PASS with zero retries;
- smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2, accessibility 9/9;
- artifact `8814897157`, digest `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`.

That run proves the delivered critical profile. It does not directly test the historical transient assertion in the original administration scenario because that assertion was removed.

## Findings and severity

Normalized totals remain:

- 0 HIGH;
- 6 MEDIUM;
- 1 LOW.

`OTERYN-AUDIT-P35-005` remains MEDIUM because durable publication succeeds and the defect affects transient accessible feedback.

`OTERYN-AUDIT-P35-006` remains MEDIUM because the stale thumbnail pattern is a proven acceptance isolation/evidence defect, not a valid-production-media failure.

## Remediation boundary

No repair is proven yet.

Preserving pending publication `status` specifically across Wiki media-index or thumbnail responses is a candidate requiring exact correlation, not the smallest proven remediation. A later implementation task must first identify the request or framework path that removes or ages `status`.

Client retries, delayed actions, `networkidle` and pre-scroll are diagnostic controls, not production fixes.

## Residual gate

Still required:

- direct exact-frozen browser execution with an ephemeral restored observer;
- three clean immediate and three clean pre-scroll samples;
- three exactly-one-corrupt immediate and three exactly-one-corrupt pre-scroll samples;
- browser request start and initiator evidence;
- redirect and `X-Request-ID` correlation;
- server entry, session-lock and session load/save evidence;
- sanitized flash-state snapshots;
- exact fixture and evidence hashes;
- restored framework hash and empty Git status.

The immediate/pre-scroll differential is hypothesis-neutral. `VALIDATED` remains forbidden until the complete exact frozen package is executed.

No implementation, merge, deployment, staging, production or Canary action is authorized.
