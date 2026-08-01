# OTERYN-20260731 portal backend/frontend audit evidence index

## Identity and status

- Task: `OTERYN-20260731-portal-backend-frontend-audit`
- Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`
- Branch: `audit/OTERYN-20260731-portal-backend-frontend-audit`
- Draft PR: `#381`
- Parent: Issue `#326`
- Related evidence: Issue `#365`
- Audit status: `BLOCKED`
- Validator verdict: `VALIDATED_WITH_CORRECTIONS`

## Environment boundary

| Evidence | State | Exact identity |
|---|---|---|
| Frozen source | `PROVEN / REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` |
| Strict/current browser source | `PROVEN / CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` |
| Post-serialization original-flow source | `PROVEN / CI_PROVEN` | `6c1e910d36771f50da5eded93cc50274a90c62d2` |
| Post-serialization result | `REPRODUCED_INTERMITTENT` | 1 PASS / 2 mobile reproductions |
| Relation to frozen Wiki runtime | `DERIVED` | identical Wiki application/view/route runtime; test-suite composition differs |
| Request-lifecycle mechanism family | `DERIVED / HIGH confidence` | old-document media request may age `status` before redirect GET |
| Responsive lazy-scroll probe | `CONTROLLED_SYNTHETIC / DERIVED` | action-induced old-document lazy work is feasible and responsive |
| Exact execution procedure | `READY / NOT_EXECUTED` | fail-closed 12-sample frozen-target runbook |
| Current execution environment | `PROVEN / BLOCKED_ENVIRONMENT` | no checkout/archive/dispatch path and no complete acceptance runtime |
| Latest staging evidence | `PROVEN / STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` |
| Frozen target deployed | `UNKNOWN` | no exact deployment proof |
| Production | `UNKNOWN` | no exact-release evidence |

## Canonical inventory

- 27 canonical surface groups.
- 228 classified manifest route assignments.
- 240 discovered named routes.
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources.
- 95 bound views, 121 Blade views, 26 structural views and zero orphan views.
- 400 navigation references.
- 43 capabilities: 23 implemented, 3 partial, 14 missing, 3 not applicable.
- Zero user-facing backend-only or frontend-only promotions to implemented.

## Strict repository contract

- source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`, job `91164376176`, artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

The content-scale validator loads only 18 base-manifest surfaces and omits nine fragment surfaces.

## Fresh current critical execution

- run `30633216753`, attempt 2, job `91339118796`;
- direct source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- artifact `8814897157`;
- digest `sha256:552d545260bad87d98f999568091c2ade84a5dce739130fbbe4e4c4e71def24f`;
- smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2, accessibility 9/9;
- total 96/96 PASS, retries 0.

This proves the delivered critical profile, but the original administration scenario no longer contains its historical transient flash assertion.

## Post-serialization original-flow reruns

Exact source: `6c1e910d36771f50da5eded93cc50274a90c62d2`, the targeted `fix(wiki): serialize admin session requests` commit. At this source, the original administration spec still asserts `Wiki article published.` and uses zero retries.

| Attempt | Job | Artifact | Digest | Original mobile result |
|---:|---:|---:|---|---|
| 2 | `91342520692` | `8815321615` | `sha256:5b2168f4952ba52f0a737b47d3a195a061c8ffc023d07cbfa115b643358d623a` | PASS |
| 3 | `91343023604` | `8815383351` | `sha256:7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | `sha256:790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | REPRODUCED |

Attempts 3 and 4 failed only the responsive-mobile transient status assertion; durable `Published`, version 3 and `Unpublish to draft` were present. Desktop, tablet and all portability browsers passed.

Corrected remediation state: `NOT_PROVEN_REMEDIATED`.

Detailed evidence: `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

## Embedded browser diagnostics

The Playwright HTML artifacts for attempts 3 and 4 contain embedded report ZIPs with full sanitized `browser-diagnostics` attachments.

| Attempt | Desktop | Tablet | Mobile |
|---:|---|---|---|
| 3 | PASS; 9×500 on `1/3/5` | PASS; 12×500 on `1/3/5/7` | REPRODUCED; 16×500 on `1/3/5/7/9` |
| 4 | PASS; 9×500 on `1/3/5` | PASS; 12×500 on `1/3/5/7` | REPRODUCED; 14×500 on `1/3/5/7/9` |

Every project records exactly two invalid-pattern console errors. Mobile records zero Playwright failed-request entries and zero page errors in both reproductions; desktop/tablet pass despite contaminated thumbnail traffic.

Therefore thumbnail HTTP 500 presence alone is not sufficient to remove publication feedback. Coexistence with mobile failure remains non-causal evidence until the exact clean-versus-one-damaged-row comparison is executed.

Detailed hashes, per-ID counts and extraction boundaries: `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`.

## Corrected flash request-lifecycle analysis

Strongest source-backed mechanism family: `DERIVED / HIGH confidence`.

- publication success exists only as Laravel session flash;
- the old Wiki edit form contains authenticated native-lazy thumbnails before the publication controls;
- Laravel ages flash data during session save;
- `->block()` serializes same-session requests but does not assign redirect-document priority;
- a request created only by the newly redirected page cannot explain why its first server-rendered HTML already lacks the alert.

The viable sequence is narrower: Playwright's far-down publication action may activate an old-document lazy thumbnail request; after the publish POST writes `status`, that queued request may acquire and save the session before the redirect GET, aging the flash.

This mechanism is not promoted to `PROVEN`: exact browser request-start, session-lock acquisition and session-save order were not preserved in attempts 3 and 4.

Detailed source chain and corrected boundary: `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`.

## Controlled responsive lazy-scroll probe

A local Chromium synthetic probe used 12 native-lazy images in a responsive 3/2/1-column grid with publication controls below the grid. Three samples per viewport and mode produced:

| Profile | Initially loaded | New loads after direct click | New loads after pre-scroll + settle |
|---|---:|---:|---:|
| desktop | 12/12 | 0 | 0 |
| tablet | 8/12 | 4 (`9–12`) | 0 |
| mobile | 3/12 | 4 (`9–12`) | 0 |

The mobile action moved the old document from the top to `scrollY=5437`; the four deferred images completed 12.9–17.9 ms after the click event. The pre-scroll control eliminated post-click lazy loads in every sample.

This proves action-induced responsive lazy work is technically feasible after an earlier settled boundary. It does not prove Oteryn request or session ordering.

Detailed method, limitations and required C1/C2 differential: `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`.

## Exact frozen execution runbook

`ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md` is the normative command-level procedure for the remaining gate. It is `READY / NOT_EXECUTED` and requires:

- exact detached frozen SHA and clean checkout proof;
- an ephemeral copy of the original Wiki scenario with only the historical status observer, request tracing and C1/C2 control added;
- direct, ephemeral instrumentation of the lockfile-resolved Laravel `StartSession` lifecycle;
- three zero-retry samples for each clean/corrupt × immediate/pre-scroll cell, 12 samples total;
- exact row-count and object-state proof before every sample;
- browser request, `X-Request-ID`, lock, session load/save and flash-aging correlation;
- SHA-256 evidence manifest;
- restoration of the original installed framework hash and empty Git status.

The runbook defines causal proof only when a matching-session old-document media request loads and ages publication `status` after the publish save but before redirect GET. Temporal coexistence alone is explicitly insufficient.

## Current execution-environment preflight

`ISSUE_365_EXECUTION_ENVIRONMENT_PREFLIGHT.md` records the fresh takeover preflight.

Proven boundaries:

- direct GitHub DNS/HTTPS is unavailable from the sandbox;
- Composer, Docker and Codex CLI are absent;
- the connected GitHub actions expose repository and workflow inspection but no repository archive, custom workflow dispatch, arbitrary runner command execution or Codex Cloud execution;
- the exact-head Phase 7 artifact contains only two summary JSON files and no reusable checkout/runtime;
- rerunning committed workflows cannot inject the untracked observer or custom 12-sample matrix;
- committing a temporary probe or workflow would violate the task and runbook.

The remaining blocker is therefore environmental rather than an unresolved instruction, CI failure or missing command specification.

## Issue #365 fixture boundary

Historical 9/12/16 thumbnail HTTP 500 counts remain explained by intentionally damaged EditorialMedia rows leaking across acceptance projects. This remains a MEDIUM isolation/evidence defect and not proof that valid production media fails.

The post-fix attempts used fresh runners and service containers, but complete critical-profile ordering can accumulate damaged rows inside an attempt. They prove reproduction under delivered suite ordering, not causality. A controlled exactly-one-row comparison remains missing.

## Normalized findings

| Finding | Severity | State |
|---|---|---|
| `OTERYN-AUDIT-P35-006` | MEDIUM | damaged EditorialMedia fixture leakage proven |
| `OTERYN-AUDIT-P35-001` | MEDIUM | nine content-scale fragment surfaces omitted |
| `OTERYN-AUDIT-P35-002` | MEDIUM | dedicated HTTP 503 matrix missing |
| `OTERYN-AUDIT-P35-003` | MEDIUM | accessibility evidence not fail-closed per surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | `REPRODUCED_INTERMITTENT`; `NOT_PROVEN_REMEDIATED` |
| `OTERYN-AUDIT-P35-007` | MEDIUM | invalid native HTML pattern proven |
| `OTERYN-AUDIT-P1-001` | LOW | active-work ownership conflict |

Totals: **0 HIGH, 6 MEDIUM, 1 LOW**.

## Durable artifacts

- `baseline.json`
- `phase-1-surface-inventory.json`
- `phase-2-capability-reconciliation.json`
- `phase-3-5-state-browser-evidence.json`
- `phase-3-5-addendum.json`
- `ISSUE_365_HISTORICAL_ARTIFACT_REVIEW.md`
- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md`
- `ISSUE_365_FLASH_REMEDIATION_EVIDENCE.md` — superseded where inconsistent by later evidence
- `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`
- `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`
- `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`
- `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`
- `ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md`
- `ISSUE_365_EXECUTION_ENVIRONMENT_PREFLIGHT.md`
- `VALIDATOR_PACKET.md`
- `VALIDATOR_PACKET_ADDENDUM.md`
- `VALIDATOR_VERDICT.md`
- reports under `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit*.md`

## Residual completion gate

`VALIDATED` remains forbidden until a mutable checkout-capable execution completes the normative 12-sample runbook and returns a hash-complete, sanitized, clean-restoration evidence package.

No merge, deployment or production action is authorized.
