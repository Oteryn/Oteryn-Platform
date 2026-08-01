# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent: Issue `#326`  
Related evidence: Issue `#365`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Executive conclusion

The repository contains a broad, internally consistent portal implementation and a mature machine-enforced validation architecture.

The audit established:

- 27 canonical surface groups;
- 228 classified route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 400 navigation references and zero orphan views;
- 43 capabilities: 23 implemented, 3 partial, 14 missing and 3 not applicable;
- no user-facing backend-only or frontend-only capability falsely promoted to implemented.

Normalized findings: **0 HIGH / 6 MEDIUM / 1 LOW**.  
Independent verdict: **`VALIDATED_WITH_CORRECTIONS`**.

The original responsive-mobile Wiki publication-flash defect reproduced in two of three independent zero-retry attempts on the exact session-serialization source. Durable publication succeeded in both failures. Session serialization is therefore **not proven to remediate the defect deterministically**.

Recovered diagnostics prove that desktop and tablet retain publication feedback despite contaminated thumbnail HTTP 500 traffic. HTTP 500 presence alone is insufficient to explain the mobile loss.

The root cause remains **`UNKNOWN`**. A new source-faithful 18-sample Chromium probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in every desktop, tablet and mobile sample. The previously proposed old-document lazy-thumbnail race is therefore corrected from `DERIVED / HIGH confidence` to **`DERIVED / LOW confidence`**.

The task remains `BLOCKED`, not `DONE`, because the exact frozen clean/one-corrupt 12-sample matrix and matching browser/request/session evidence require a mutable checkout-capable production-like validator.

## Evidence classifications

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen audit source |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and fresh critical profile passed |
| `CI_PROVEN` | `6c1e910d36771f50da5eded93cc50274a90c62d2` | original transient assertion: 1 PASS / 2 mobile reproductions |
| `DERIVED` | `6c1e...` → frozen Wiki runtime | relevant Wiki runtime unchanged; exact frozen execution absent |
| `CONTROLLED_SYNTHETIC` | generic responsive probe | browser feasibility only |
| `CONTROLLED_SOURCE_FAITHFUL` | source-faithful layout probe | zero Publish-action thumbnail starts in 18 samples |
| `DERIVED / LOW confidence` | old-document thumbnail race | possible but not leading or proven |
| `STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` | separate staging source |
| `PRODUCTION_PROVEN` | none | production remains `UNKNOWN` |

Open PR code remains `OPEN_PR_ONLY`. CI evidence does not imply deployment.

## Canonical inventory

The canonical inventory contains:

- 27 surface groups;
- 228 manifest route assignments;
- 240 named routes;
- 126 rendered routes;
- 76 form actions;
- 16 redirects;
- 10 supporting resources;
- 95 bound views;
- 121 Blade views;
- 26 structural views;
- zero orphan views;
- 400 navigation references.

Capability reconciliation contains 43 records:

- 23 implemented;
- 3 partial;
- 14 missing;
- 3 not applicable.

Detailed machine-readable evidence:

- `baseline.json`;
- `phase-1-surface-inventory.json`;
- `phase-2-capability-reconciliation.json`;
- `phase-3-5-state-browser-evidence.json`;
- `phase-3-5-addendum.json`.

## Strict and current browser validation

### Portal Acceptance Contract

- source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`;
- job `91164376176`;
- artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

The content-scale sub-validator loads 18 base-manifest surfaces and omits nine canonical fragment surfaces.

### Fresh current critical profile

- run `30633216753`, attempt 2;
- job `91339118796`;
- artifact `8814897157`;
- digest `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`;
- smoke 7/7;
- portability 36/36;
- responsive 42/42;
- resilience 2/2;
- accessibility 9/9;
- total 96/96 PASS with retries zero.

This proves the delivered critical profile. It does not directly retest the historical transient assertion in the original administration scenario because that assertion was later removed.

## Issue #365 post-serialization execution

Exact source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

This source:

- applies `->block()` to administrator Wiki routes;
- retains the original `Wiki article published.` assertion;
- uses Playwright retries zero.

| Attempt | Job | Artifact | Responsive mobile |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

In attempts 3 and 4:

- the accessible transient publication status was absent;
- durable `Published` state was present;
- version 3 was present;
- `Unpublish to draft` was present;
- desktop and tablet passed;
- Chromium, Firefox and WebKit portability passed.

Classification:

```yaml
id: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause: UNKNOWN
samples:
  pass: 1
  reproduced: 2
```

Detailed evidence: `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

## Recovered embedded browser diagnostics

The Playwright HTML reports contain embedded report ZIPs with complete sanitized `browser-diagnostics` attachments.

| Attempt | Project | Result | Thumbnail 500 | Stale IDs | Aborted requests | Page errors |
|---:|---|---|---:|---|---:|---:|
| 3 | desktop | PASS | 9 | `1/3/5` | 6 | 0 |
| 3 | tablet | PASS | 12 | `1/3/5/7` | 8 | 0 |
| 3 | mobile | REPRODUCED | 16 | `1/3/5/7/9` | 0 | 0 |
| 4 | desktop | PASS | 9 | `1/3/5` | 6 | 0 |
| 4 | tablet | PASS | 12 | `1/3/5/7` | 8 | 0 |
| 4 | mobile | REPRODUCED | 14 | `1/3/5/7/9` | 0 | 0 |

Every original-flow project records exactly two invalid-pattern console errors.

These diagnostics prove:

1. acceptance fixture leakage follows deterministic project ordering;
2. desktop/tablet pass despite contaminated thumbnail traffic;
3. HTTP 500 presence alone does not remove publication feedback;
4. mobile completes more contaminated responses and aborts none;
5. viewport affects request completion/cancellation.

They do not preserve:

- request timestamps;
- initiator document or frame;
- `Referer`;
- request/correlation ID;
- redirect navigation identity;
- session identity;
- session-lock or session-save order.

Therefore no thumbnail request can be assigned safely as the consuming request.

Detailed evidence: `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`.

## Flash lifecycle facts

Proven source facts:

1. publication redirects with `->with('status', 'Wiki article published.')`;
2. the administrator layout renders the success element only from `session('status')`;
3. the Wiki form creates authenticated same-origin media-index and native-lazy thumbnail requests;
4. media and lifecycle routes use the administrator web session;
5. Laravel ages flash data during session save;
6. session blocking supplies mutual exclusion but not a proven redirect-priority guarantee.

A request created only after the first redirected HTML arrives cannot explain why that same server-rendered HTML already lacks the alert. This limits the candidate window but does not identify the responsible request or framework path.

## Generic responsive probe

The first controlled probe used:

- 12 native-lazy images;
- responsive 3/2/1-column grid;
- `Publish` immediately below the grid;
- direct click versus explicit pre-scroll;
- three samples per viewport and mode.

| Profile | Initially loaded | New loads after direct action | New loads after pre-scroll |
|---|---:|---:|---:|
| desktop | 12 | 0 | 0 |
| tablet | 8 | 4 | 0 |
| mobile | 3 | 4 | 0 |

Classification: `CONTROLLED_SYNTHETIC / GENERIC_FEASIBILITY`.

This proves browser feasibility in that simplified geometry only.

Detailed evidence: `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`.

## Source-faithful layout probe

The corrected probe copied the actual frozen-source structure relevant to scroll and lazy loading:

- media picker first;
- article settings;
- two translation panels with real Markdown-field height;
- categories;
- change-note/save section;
- Lifecycle and `Publish` last;
- 12 media cards with native lazy thumbnails;
- exact Playwright desktop, tablet and mobile viewports.

Three immediate and three pre-scroll samples were executed per viewport, 18 total.

| Profile | Initially started thumbnails | New starts from Publish action start | Final scroll Y | Document height |
|---|---:|---:|---:|---:|
| desktop | 12 | 0 in 6/6 samples | 2989 | 3989 |
| tablet | 10 | 0 in 6/6 samples | 4584 | 5764 |
| mobile | 4 | 0 in 6/6 samples | 7158 | 8002 |

Classification: `CONTROLLED_SOURCE_FAITHFUL / DERIVED`.

The result does not reproduce Laravel HTTP, session locking or the defect. It does directly invalidate `HIGH confidence` for the specific claim that the actual form's Publish action normally activates a deferred old-document thumbnail.

Corrected mechanism state:

```yaml
root_cause: UNKNOWN
old_document_lazy_thumbnail_race:
  classification: DERIVED
  confidence: LOW
```

Detailed evidence:

- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
- corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`.

## Findings

### MEDIUM — OTERYN-AUDIT-P35-006

**Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later projects.**

The media scenario corrupts/removes stored objects while preserving rows. Later projects request stale rows. Historical ordering predicts the observed ID expansion exactly. This is an acceptance isolation/evidence defect, not proof that valid production media fails.

### MEDIUM — OTERYN-AUDIT-P35-001

**Strict content-scale closure omits nine canonical fragment surfaces.**

### MEDIUM — OTERYN-AUDIT-P35-002

**The dedicated global error matrix omits HTTP 503.**

### MEDIUM — OTERYN-AUDIT-P35-003

**Accessibility evidence is bounded rather than fail-closed per delivered surface.**

### MEDIUM — OTERYN-AUDIT-P35-005

**Responsive-mobile Wiki publication intermittently loses accessible transient success feedback after durable success.**

The defect is historically proven and reproduced after session serialization. Root cause is unknown. The old-document lazy-thumbnail race is a low-confidence hypothesis only.

### MEDIUM — OTERYN-AUDIT-P35-007

**An invalid HTML pattern weakens native validation on two Wiki administrator fields.**

Backend Laravel validation still enforces the intended grammar.

### LOW — OTERYN-AUDIT-P1-001

**`ACTIVE_WORK.md` conflicts with live task/PR ownership.**

Live task and PR state is authoritative.

## Causality and nonclaims

The audit does not claim:

- stale damaged media rows cause the flash loss;
- any thumbnail HTTP 500 response removes publication feedback;
- session serialization remediates the defect deterministically;
- the source-faithful probe reproduces Oteryn runtime behavior;
- the old-document race is disproven in every possible real timing;
- preserving flash across media responses is already the correct repair;
- exact frozen-target staging deployment;
- production availability.

The full critical profile can accumulate deliberately damaged rows inside one attempt. The post-serialization runs prove reproduction under delivered suite ordering, not clean-versus-one-corrupt causality.

## Remediation boundary

No implementation is authorized in this audit.

A later implementation task must first identify the request or lifecycle path that removes or ages `status`. Candidate solutions may include:

- preventing a proven read-only request from aging pending publication feedback;
- decoupling a proven media authorization path from the mutable page session;
- using redirect-bound feedback instead of one-request session flash;
- repairing another session lifecycle path identified by exact instrumentation.

Preserving `status` specifically across media responses is a candidate requiring proof, not the smallest proven repair.

Client retries, delayed clicks, `networkidle` and pre-scroll are diagnostic controls, not production fixes.

## Exact frozen remaining gate

The normative runbook is `ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md`.

Required matrix:

| Fixture | Immediate Publish | Pre-scroll + settle |
|---|---:|---:|
| clean after EditorialMedia reset | 3 | 3 |
| exactly one corrupt row | 3 | 3 |

Every sample must use:

- exact frozen SHA;
- one worker;
- retries zero;
- original administration flow with only ephemeral observers;
- production-like Laravel HTTP and dependencies.

Required evidence:

- browser request start, frame, resource type, initiator and sanitized `Referer`;
- publish response and redirect navigation identity;
- response `X-Request-ID`;
- server request entry and route identity;
- session-lock attempt/acquire/release;
- session load/save with sanitized `_flash.new`, `_flash.old` and `status` presence;
- exact media row/object state;
- first redirected document feedback and durable publication state;
- evidence hashes;
- restored installed framework hash;
- empty final Git status.

The immediate/pre-scroll differential is hypothesis-neutral. Causal promotion requires the complete matching-session chain. Temporal coexistence is insufficient.

## Deployment boundary

Latest directly proven staging source remains `717977f252b09b9b2e979f8110b7f48b88682223`. No production operation was performed.

## Verdict

Verdict: `VALIDATED_WITH_CORRECTIONS`.

`VALIDATED` remains forbidden until one mutable checkout-capable validator executes the complete exact frozen 12-sample package and returns sanitized, hash-complete, clean-restoration evidence.

No merge, deployment or production action is authorized.
