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
- Normalized findings: **0 HIGH / 6 MEDIUM / 1 LOW**

## Precedence

The following files are authoritative for the current Issue #365 mechanism classification and supersede inconsistent older wording:

1. `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
2. `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
3. corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`;
4. corrected `VALIDATOR_PACKET_ADDENDUM.md`;
5. `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit-mechanism-correction.md`.

Current root-cause state is `UNKNOWN`. The old-document lazy-thumbnail race is `DERIVED / LOW confidence`.

## Environment and source boundary

| Evidence | State | Exact identity |
|---|---|---|
| Frozen source | `PROVEN / REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` |
| Strict/current browser source | `PROVEN / CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` |
| Post-serialization original-flow source | `PROVEN / CI_PROVEN` | `6c1e910d36771f50da5eded93cc50274a90c62d2` |
| Post-serialization result | `REPRODUCED_INTERMITTENT` | 1 PASS / 2 mobile reproductions |
| Current remediation state | `NOT_PROVEN_REMEDIATED` | session serialization is nondeterministic for this defect |
| Root cause | `UNKNOWN` | no matching request/session chain preserved |
| Old-document lazy-thumbnail race | `DERIVED / LOW confidence` | weakened by source-faithful 18-sample counterevidence |
| Exact execution procedure | `READY / NOT_EXECUTED` | fail-closed 12-sample frozen-target runbook |
| Current execution environment | `PROVEN / BLOCKED_ENVIRONMENT` | no mutable checkout-capable production-like validator |
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

## Strict repository and browser validation

Portal Acceptance Contract:

- source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`, job `91164376176`, artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

Fresh current critical execution:

- run `30633216753`, attempt 2, job `91339118796`;
- artifact `8814897157`;
- smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2, accessibility 9/9;
- total 96/96 PASS with retries zero.

This proves the delivered critical profile, but the current original-administration scenario no longer contains the historical transient publication assertion.

## Post-serialization original-flow reruns

Exact source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

| Attempt | Job | Artifact | Digest | Responsive mobile |
|---:|---:|---:|---|---|
| 2 | `91342520692` | `8815321615` | `sha256:5b2168f4952ba52f0a737b47d3a195a061c8ffc023d07cbfa115b643358d623a` | PASS |
| 3 | `91343023604` | `8815383351` | `sha256:7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | `sha256:790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | REPRODUCED |

Attempts 3 and 4 failed the transient status assertion while preserving durable `Published`, version 3 and `Unpublish to draft`. Desktop, tablet and all portability browsers passed.

Detailed evidence: `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`.

## Embedded browser diagnostics

| Attempt | Desktop | Tablet | Mobile |
|---:|---|---|---|
| 3 | PASS; 9×500 on `1/3/5`; 6 aborted | PASS; 12×500 on `1/3/5/7`; 8 aborted | REPRODUCED; 16×500 on `1/3/5/7/9`; 0 aborted |
| 4 | PASS; 9×500 on `1/3/5`; 6 aborted | PASS; 12×500 on `1/3/5/7`; 8 aborted | REPRODUCED; 14×500 on `1/3/5/7/9`; 0 aborted |

Proven conclusions:

- stale-media expansion follows deterministic acceptance project ordering;
- desktop and tablet retain publication feedback despite contaminated thumbnail traffic;
- HTTP 500 presence alone is insufficient to remove the flash;
- viewport changes thumbnail completion/cancellation behavior;
- existing diagnostics do not preserve request timing, initiator document, `Referer`, correlation ID or session state.

Detailed extraction and hashes: `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`.

## Generic lazy-scroll feasibility probe

`ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md` used a simplified responsive grid with `Publish` immediately below the images.

| Profile | Initially loaded | New loads after direct click | New loads after pre-scroll |
|---|---:|---:|---:|
| desktop | 12/12 | 0 | 0 |
| tablet | 8/12 | 4 | 0 |
| mobile | 3/12 | 4 | 0 |

Classification: `CONTROLLED_SYNTHETIC / GENERIC_FEASIBILITY`.

It proves that actionability scrolling can activate native-lazy images in some geometries. It does not model the real Wiki form sufficiently to support app-specific causal confidence.

## Source-faithful layout probe

`ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md` copied the real frozen form ordering, relevant CSS geometry, 12 native-lazy media cards and exact Playwright viewports.

Three immediate and three pre-scroll samples were executed per profile, 18 total.

| Profile | Initially started | New starts from Publish action start |
|---|---:|---:|
| desktop | 12 | 0 in 6/6 samples |
| tablet | 10 | 0 in 6/6 samples |
| mobile | 4 | 0 in 6/6 samples |

This directly weakens the specific old-document action-induced thumbnail hypothesis. It does not reproduce Laravel HTTP, authentication, session locks or the defect itself.

Corrected mechanism state:

```yaml
root_cause: UNKNOWN
old_document_lazy_thumbnail_race:
  classification: DERIVED
  confidence: LOW
```

## Flash lifecycle facts

Proven source facts:

- publication feedback exists only as Laravel session flash;
- the administrator layout renders it only from `session('status')`;
- media index and thumbnails use authenticated administrator routes and the same web session;
- Laravel ages flash data during session save;
- session blocking supplies mutual exclusion but does not itself prove redirect priority.

Not proven:

- which request or framework path removes `status`;
- exact browser request-start order in attempts 3 and 4;
- exact session-lock acquisition and save order;
- whether damaged media contributes causally;
- exact frozen clean and exactly-one-corrupt behavior.

Canonical analysis: corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`.

## Exact frozen execution runbook

`ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md` remains `READY / NOT_EXECUTED`.

Mandatory 12-sample matrix:

| Fixture | Immediate Publish | Pre-scroll + settle |
|---|---:|---:|
| clean after EditorialMedia reset | 3 | 3 |
| exactly one corrupt row | 3 | 3 |

The differential is hypothesis-neutral. Every sample must use one worker and retries zero.

Required correlated evidence:

- browser request start, frame/initiator, navigation identity and sanitized `Referer`;
- publish response, redirect chain and `X-Request-ID`;
- server entry and route identity;
- session-lock attempt/acquire/release;
- session load/save with sanitized `_flash.new`, `_flash.old` and `status` presence;
- exact fixture row/object proof;
- first redirected document status text and durable publication state;
- evidence hashes;
- restored framework hash and empty final Git status.

Causal promotion requires a complete matching-session chain. Temporal coexistence is insufficient.

## Execution environment preflight

`ISSUE_365_EXECUTION_ENVIRONMENT_PREFLIGHT.md` records:

- direct GitHub and package/CDN DNS unavailable in the sandbox;
- no Composer, Docker/Podman, MariaDB, Redis or Codex CLI;
- no connector action for archive/zipball, arbitrary runner commands, Codespaces or connected Codex Cloud execution;
- existing workflow reruns cannot inject untracked observers;
- Phase 7 artifacts contain summaries only, not a reusable checkout/runtime;
- committing temporary observers or workflows is forbidden.

The blocker is environmental rather than a missing procedure or unresolved repository instruction.

## Normalized findings

| Finding | Severity | State |
|---|---|---|
| `OTERYN-AUDIT-P35-006` | MEDIUM | damaged EditorialMedia fixture leakage proven |
| `OTERYN-AUDIT-P35-001` | MEDIUM | nine content-scale fragment surfaces omitted |
| `OTERYN-AUDIT-P35-002` | MEDIUM | dedicated HTTP 503 matrix missing |
| `OTERYN-AUDIT-P35-003` | MEDIUM | accessibility evidence not fail-closed per surface |
| `OTERYN-AUDIT-P35-005` | MEDIUM | intermittent mobile flash loss; root cause unknown |
| `OTERYN-AUDIT-P35-007` | MEDIUM | invalid native HTML pattern proven |
| `OTERYN-AUDIT-P1-001` | LOW | active-work ownership conflict |

Totals: **0 HIGH / 6 MEDIUM / 1 LOW**.

## Durable artifacts

Machine-readable inventory and state:

- `baseline.json`;
- `phase-1-surface-inventory.json`;
- `phase-2-capability-reconciliation.json`;
- `phase-3-5-state-browser-evidence.json`;
- `phase-3-5-addendum.json`.

Issue #365 evidence:

- `ISSUE_365_HISTORICAL_ARTIFACT_REVIEW.md`;
- `ISSUE_365_STATIC_CAUSE_ANALYSIS.md`;
- `ISSUE_365_FLASH_REMEDIATION_EVIDENCE.md` — superseded where inconsistent;
- `ISSUE_365_POST_FIX_RERUN_EVIDENCE.md`;
- `ISSUE_365_EMBEDDED_BROWSER_DIAGNOSTICS.md`;
- corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`;
- `ISSUE_365_LAZY_SCROLL_SYNTHETIC_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
- `ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md`;
- `ISSUE_365_EXECUTION_ENVIRONMENT_PREFLIGHT.md`;
- `VALIDATOR_PACKET.md`;
- corrected `VALIDATOR_PACKET_ADDENDUM.md`;
- `VALIDATOR_VERDICT.md`.

Reports:

- reports under `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit*.md`;
- `OTERYN-20260731-portal-backend-frontend-audit-mechanism-correction.md` supersedes inconsistent mechanism wording in the original consolidated report.

## Residual completion gate

`VALIDATED` remains forbidden until a mutable checkout-capable validator executes the complete exact frozen 12-sample runbook and returns a sanitized, hash-complete, clean-restoration evidence package.

No merge, deployment or production action is authorized.
