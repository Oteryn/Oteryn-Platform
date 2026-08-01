# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent: Issue `#326`  
Related evidence: Issue `#365`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Executive conclusion

The repository contains a broad and internally consistent portal implementation plus a mature machine-enforced acceptance architecture. The audit established:

- 27 canonical surface groups and 228 classified route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 400 navigation references and zero orphan views;
- 43 capabilities: 23 implemented, 3 partial, 14 missing and 3 not applicable;
- no user-facing backend-only or frontend-only capability falsely promoted to implemented.

Normalized findings remain **0 HIGH, 6 MEDIUM and 1 LOW**. Independent verdict remains **`VALIDATED_WITH_CORRECTIONS`**.

A three-attempt post-fix execution materially corrects the Wiki conclusion: the original mobile publication-flash defect reproduced in two of three zero-retry attempts on the exact targeted session-serialization commit. Session blocking is therefore **not proven to remediate the defect deterministically**.

Recovered embedded Playwright diagnostics prove that contaminated desktop and tablet flows retain publication feedback despite 9 and 12 thumbnail HTTP 500 responses. Thumbnail 500 presence alone is insufficient to explain the mobile flash loss.

Source, framework and responsive-action analysis narrows the strongest mechanism family to `DERIVED / HIGH confidence`: a media request from the **old article-edit document** may be activated by the far-down publication action, queue behind the publish POST, then age the one-request flash before the redirect GET. Requests created only by the newly redirected page are excluded as the primary explanation for an alert already absent from its first server-rendered HTML.

A controlled Chromium probe confirms that a responsive Playwright click can activate deferred old-document lazy-image work after an earlier settled boundary. It does not directly prove Oteryn request or session-lock ordering.

The task remains `BLOCKED`, not `DONE`, because exact frozen-target clean isolation, the exactly-one-damaged-row comparison and sanitized browser/request/session evidence remain unavailable.

## Evidence boundaries

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen source audit target |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and fresh current critical profile passed |
| `CI_PROVEN` | `6c1e910d36771f50da5eded93cc50274a90c62d2` | original transient assertion: 1 PASS / 2 mobile reproductions after session serialization |
| `DERIVED` | `6c1e...` → frozen Wiki runtime | identical Wiki application/view/route runtime; acceptance-suite composition differs |
| `DERIVED / HIGH confidence` | corrected flash mechanism family | old-document media request may age pending `status` before redirect GET |
| `CONTROLLED_SYNTHETIC / DERIVED` | responsive lazy-scroll probe | action-induced old-document lazy work is feasible; app ordering unproven |
| `STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` | staging evidence for a different source |
| `PRODUCTION_PROVEN` | none | production remains `UNKNOWN` |

Open PR code remains `OPEN_PR_ONLY`. Marketplace route presence does not prove deployment reachability.

## Canonical inventory and strict validation

Portal Acceptance Contract:

- source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`, job `91164376176`, artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

The strict route/view/navigation/media contracts close the canonical inventory. The content-scale sub-validator loads only 18 base-manifest surfaces and omits nine fragment surfaces.

## Fresh current critical-profile validation

A fresh zero-retry run on direct source `fdb45a4325949d3ab1c4860e3a4527553f11c789` passed:

- run `30633216753`, attempt 2, job `91339118796`;
- artifact `8814897157`;
- digest `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`;
- smoke 7/7;
- portability 36/36;
- responsive 42/42;
- resilience 2/2;
- accessibility 9/9;
- total 96/96.

This proves the delivered critical profile passes. It does not directly test the historical transient assertion in the original administration scenario because that assertion was removed. A related media-intensive scenario still asserts the publication flash and passes, but it cannot negate direct reproduction in the original flow.

## Post-serialization original-flow execution

The exact targeted fix commit `6c1e910d36771f50da5eded93cc50274a90c62d2`:

- adds `->block()` to all administrator Wiki routes;
- retains the original `role=status` assertion for `Wiki article published.`;
- has the same `routes/modules/wiki.php` blob as the frozen target: `f4a16ac017fd075b54904455bc8b6f05af304053`.

Three independent workflow attempts used fresh runners/service containers, real Laravel HTTP and Playwright retries set to zero:

| Attempt | Job | Artifact | Original responsive-mobile result |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

In attempts 3 and 4, only responsive mobile failed the transient assertion. The error context showed durable `Published`, version 3 and `Unpublish to draft`, matching the historical symptom exactly. Desktop, tablet and all three portability browsers passed.

Corrected classification:

```yaml
id: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
samples:
  pass: 1
  reproduced: 2
```

Detailed evidence: `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

## Recovered embedded browser diagnostics

The HTML artifacts for attempts 3 and 4 contain base64-embedded Playwright report ZIPs. Their `browser-diagnostics` attachments provide complete sanitized response, console, failed-request and page-error observations for successful and failed projects.

| Attempt | Project | Result | Thumbnail HTTP 500 | Stale IDs | Failed requests | Pattern errors | Page errors |
|---:|---|---|---:|---|---:|---:|---:|
| 3 | desktop | PASS | 9 | `1/3/5` | 6 | 2 | 0 |
| 3 | tablet | PASS | 12 | `1/3/5/7` | 8 | 2 | 0 |
| 3 | mobile | REPRODUCED | 16 | `1/3/5/7/9` | 0 | 2 | 0 |
| 4 | desktop | PASS | 9 | `1/3/5` | 6 | 2 | 0 |
| 4 | tablet | PASS | 12 | `1/3/5/7` | 8 | 2 | 0 |
| 4 | mobile | REPRODUCED | 14 | `1/3/5/7/9` | 0 | 2 | 0 |

This strengthens three conclusions:

1. fixture leakage remains deterministic by project order;
2. invalid native pattern diagnostics remain deterministic at exactly two per original-flow project;
3. thumbnail HTTP 500 presence alone does not necessarily remove the publication flash because contaminated desktop/tablet flows pass.

The mobile 14-versus-16 response difference and causal contribution of damaged rows remain unknown. No shared cause is claimed.

Exact artifact/report hashes and per-ID distributions: `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`.

## Corrected flash request-lifecycle mechanism

The strongest source-backed mechanism family is `DERIVED / HIGH confidence`, not `PROVEN`.

Repository and framework chain:

1. publication redirects to the article edit route with `->with('status', 'Wiki article published.')`;
2. the administrator layout renders the message only from `session('status')`;
3. the old Wiki form creates authenticated native-lazy thumbnail requests before the far-down publication controls;
4. edit, media index, thumbnails and publication mutation all use the same web session and Laravel session blocking;
5. Laravel flash data is aged during session save;
6. session blocking provides mutual exclusion but does not prioritize the redirect GET over an already queued old-document media request.

The corrected boundary matters. A request started only after the redirected page's HTML arrives cannot explain why that first HTML already omitted the alert. The viable sequence is:

1. Playwright begins the publication action on the old document;
2. actionability scrolling activates a deferred old-document thumbnail request;
3. the publish POST writes `status` and releases the session lock;
4. the queued media request acquires and saves the session before the redirect GET, aging the flash;
5. the redirect GET renders durable state without the alert.

Preserved timings support a narrow action window: publication began only 5–7 ms after the pre-publication `networkidle` step. Click actions took 74–193 ms across the preserved desktop/tablet/mobile projects. `networkidle` does not prove that the action itself created no further lazy work.

Not proven:

- exact old-document request start in attempts 3 and 4;
- exact session-lock acquisition order;
- whether a valid, missing or corrupt thumbnail is sufficient;
- whether integrity failure changes scheduling or lock timing;
- exact frozen-target clean behavior.

Smallest later server-side candidate, not implemented here: preserve only a pending publication `status` across authenticated Wiki media-index and thumbnail responses, then prove that the redirect document consumes it exactly once. Explicit pre-scroll is a diagnostic control, not production remediation.

Detailed analysis: `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`.

## Controlled responsive lazy-scroll probe

A local Chromium `144.0.7559.96` probe exercised Playwright actionability scrolling and native lazy loading with:

- 12 images;
- responsive 3/2/1-column media grid;
- publication control below the grid;
- three samples per viewport and mode;
- direct click versus explicit pre-scroll plus settle.

| Profile | Initially loaded | New loads after direct click | New loads after pre-scroll + settle |
|---|---:|---:|---:|
| desktop | 12/12 | 0 | 0 |
| tablet | 8/12 | 4 (`9–12`) | 0 |
| mobile | 3/12 | 4 (`9–12`) | 0 |

The mobile action moved the old document from the top to `scrollY=5437`; the deferred images completed 12.9–17.9 ms after the click event. Explicit pre-scroll moved that work outside the click window and produced zero post-click loads in all samples.

Because container policy blocked network/file navigation, the probe used `page.set_content()` and data-URI images. It proves browser feasibility and responsive differentiation, not Oteryn HTTP or session causality.

Detailed method and limitations: `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`.

## Findings

### MEDIUM — OTERYN-AUDIT-P35-006

**Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later tests.**

The Wiki media spec corrupts/removes stored files while preserving rows. Later projects request stale rows. Historical ordering exactly predicts the repeated thumbnail HTTP 500 pattern. This is an acceptance isolation/evidence defect, not proof that valid production media fails.

### MEDIUM — OTERYN-AUDIT-P35-001

**Strict content-scale closure omits nine canonical fragment surfaces.**

The validator loads 18 base-manifest surfaces but not six fragment files.

### MEDIUM — OTERYN-AUDIT-P35-002

**Dedicated global error matrix omits HTTP 503.**

404, 419, 429 and 500 have the dedicated EN/PL viewport matrix; 503 does not.

### MEDIUM — OTERYN-AUDIT-P35-003

**Accessibility evidence is bounded rather than fail-closed per delivered surface.**

No one-record-per-rendered-surface applicability ledger prevents silent omission; reduced-motion applicability remains unknown.

### MEDIUM — OTERYN-AUDIT-P35-005

**Mobile Wiki publication intermittently loses accessible transient success feedback after durable success.**

The defect is historically proven and reproduced after session serialization in two of three independent zero-retry attempts. Session serialization alone is insufficient. The strongest mechanism family is old-document media-request consumption of one-request flash, `DERIVED / HIGH confidence` pending exact request and session ordering.

### MEDIUM — OTERYN-AUDIT-P35-007

**Invalid HTML pattern weakens native validation on two Wiki administrator fields.**

The invalid browser pattern remains, while backend Laravel regex validation still enforces the intended grammar.

### LOW — OTERYN-AUDIT-P1-001

**`ACTIVE_WORK.md` conflicts with live task/PR ownership.**

Live GitHub task and PR state was treated as authoritative.

## Causality and nonclaims

The audit does not claim that stale damaged media rows cause the flash loss. The full critical profile can accumulate damaged rows inside an attempt. The post-fix executions prove reproduction under delivered suite ordering, not a clean-versus-one-row causal result.

Desktop/tablet success under contamination rejects a simple deterministic relationship between any thumbnail HTTP 500 traffic and flash loss. It does not rule out timing, viewport or request-order interaction.

The synthetic responsive probe proves browser feasibility only. It does not relabel the old-document request race as direct Oteryn runtime proof.

The audit also does not claim:

- direct execution of the custom observer on exact frozen SHA;
- exact frozen-target staging deployment;
- production availability;
- exhaustive every-screen visual acceptance.

## Deployment

Latest directly proven staging source remains `717977f252b09b9b2e979f8110b7f48b88682223`, run `30633745660`, job `91166065335`, artifact `8794683627`. No production operation was performed.

## Minimum remediation set

1. Fragment-aware content-scale closure, dedicated 503 coverage and fail-closed accessibility applicability under Issue #326.
2. Continue Issue #365 with exact frozen clean isolation, exactly one controlled damaged row and sanitized request/session evidence.
3. Add the immediate-action versus pre-scroll C1/C2 differential to locate any old-document request in the publish window.
4. In a separately authorized implementation task, preserve only pending publication `status` across Wiki media-index/thumbnail session responses, with valid/corrupt/session-consumption tests.
5. Correct the two Wiki HTML patterns and add native validation plus zero-console-error regression coverage after focused classification.

No implementation was authorized or performed by this audit.

## Verdict and residual gate

Verdict: `VALIDATED_WITH_CORRECTIONS`.

`VALIDATED` remains forbidden until one mutable checkout-capable execution performs:

1. exact frozen SHA with an ephemeral restored transient observer;
2. at least three clean isolated responsive-mobile samples;
3. one comparison with exactly one missing/corrupt media row;
4. immediate-action versus pre-scroll request-order differential;
5. sanitized browser request, server request, session-lock, flash-state and application evidence.

No merge, deployment or production action is authorized.