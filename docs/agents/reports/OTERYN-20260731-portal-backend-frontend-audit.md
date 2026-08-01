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

A new three-attempt post-fix execution materially corrects the Wiki conclusion: the original mobile publication-flash defect reproduced in two of three zero-retry attempts on the exact targeted session-serialization commit. Session blocking is therefore **not proven to remediate the defect deterministically**.

The task remains `BLOCKED`, not `DONE`, because exact frozen-target clean isolation, the exactly-one-damaged-row comparison and sanitized request/session/application evidence are still unavailable.

## Evidence boundaries

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen source audit target |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and fresh current critical profile passed |
| `CI_PROVEN` | `6c1e910d36771f50da5eded93cc50274a90c62d2` | original transient assertion: 1 PASS / 2 mobile reproductions after session serialization |
| `DERIVED` | `6c1e...` → frozen Wiki runtime | identical Wiki application/view/route runtime; acceptance-suite composition differs |
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

## New post-serialization original-flow execution

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

## Findings

### MEDIUM — OTERYN-AUDIT-P35-006

**Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later tests.**

The Wiki media spec corrupts/removes stored files while preserving rows. Later projects request stale rows. Historical ordering exactly predicts the repeated 9/12/16 HTTP 500 pattern. This is an acceptance isolation/evidence defect, not proof that valid production media fails.

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

The defect is historically proven and reproduced after session serialization in two of three independent zero-retry attempts. Session serialization alone is insufficient to claim remediation.

### MEDIUM — OTERYN-AUDIT-P35-007

**Invalid HTML pattern weakens native validation on two Wiki administrator fields.**

The invalid browser pattern remains, while backend Laravel regex validation still enforces the intended grammar.

### LOW — OTERYN-AUDIT-P1-001

**`ACTIVE_WORK.md` conflicts with live task/PR ownership.**

Live GitHub task and PR state was treated as authoritative.

## Causality and nonclaims

The audit does not claim that stale damaged media rows cause the flash loss. The new attempts were freshly bootstrapped, but the full critical profile can accumulate damaged rows inside each attempt. They prove post-fix reproduction under delivered suite ordering, not a clean-versus-one-row causal result.

The audit also does not claim:

- direct execution of the custom observer on exact frozen SHA;
- exact frozen-target staging deployment;
- production availability;
- exhaustive every-screen visual acceptance.

## Deployment

Latest directly proven staging source remains `717977f252b09b9b2e979f8110b7f48b88682223`, run `30633745660`, job `91166065335`, artifact `8794683627`. No production operation was performed.

## Minimum remediation set

1. Fragment-aware content-scale closure, dedicated 503 coverage and fail-closed accessibility applicability under Issue #326.
2. Continue Issue #365: reset EditorialMedia between samples, execute exact frozen-target clean isolation and one exactly controlled damaged-row comparison, and capture sanitized request/session/application evidence.
3. Correct the two Wiki HTML patterns and add native validation plus zero-console-error regression coverage after focused classification.

No implementation was authorized or performed by this audit.

## Verdict and residual gate

Verdict: `VALIDATED_WITH_CORRECTIONS`.

`VALIDATED` remains forbidden until one mutable checkout-capable execution performs:

1. exact frozen SHA with an ephemeral restored transient observer;
2. clean isolated samples with EditorialMedia reset before each sample;
3. one controlled comparison with exactly one missing/corrupt row;
4. sanitized publish redirect, session/request, thumbnail and application/server evidence.

No merge, deployment or production action is authorized.