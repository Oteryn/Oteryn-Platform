# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent acceptance issue: `#326`  
Programme overlay: `#451`  
Related evidence: Issue `#365`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Scope

This is an audit-only task. It may update reports, evidence and the audit checkpoint. It does not authorize product implementation, workflow changes, merge of temporary validator infrastructure, deployment, production mutation or work in another repository.

## Executive conclusion

The repository contains a broad, internally consistent portal implementation and a mature validation architecture. The legacy backend/frontend reconciliation proves that 23 benchmark capabilities have backend, reachable frontend and real-route integration without one-sided promotion.

Under the current delivery-completeness policy, that is not sufficient to call a capability complete. The authoritative result is:

```yaml
legacy_backend_frontend_result:
  implemented: 23
  partial: 3
  missing: 14
  not_applicable: 3
policy_v2_result:
  complete: 0
  repository_integrated_evidence_open: 23
  partial: 3
  missing: 14
  not_applicable: 3
```

The legacy word `implemented` remains a valid repository integration fact. It does not prove all persistence, authorization, state, locale, accessibility, test, zero-retry E2E, independent-audit, exact-final-head and terminal-PR/task gates.

The merged production-completion baseline identifies 18 modules. The earlier 43-capability ledger is a benchmark subset and lacks explicit records for several delivered or cross-cutting modules. This is new finding `OTERYN-AUDIT-P6-001`.

Open normalized findings: **0 HIGH / 7 MEDIUM / 1 LOW**.  
Independent verdict remains **`VALIDATED_WITH_CORRECTIONS`**.  
Task status is **`waiting`**, because the final bounded Issue #365 matrix is still active.

## Canonical inventory

The audit established:

- 27 canonical surface groups;
- 228 classified route assignments;
- 240 discovered named routes;
- 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources;
- 95 bound views, 121 Blade views, 26 structural views and zero orphan views;
- 400 navigation references;
- 43 legacy benchmark capabilities: 23 implemented, 3 partial, 14 missing and 3 not applicable;
- zero user-facing backend-only or frontend-only promotions to legacy implemented.

Policy-v2 module and capability details are in:

- `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/phase-6-delivery-completeness-crosswalk.json`;
- `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit-phase-6-delivery-completeness.md`.

## Strict backend/frontend and browser validation

Portal Acceptance Contract:

- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`;
- job `91164376176`;
- artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

This corrects the historical Phase 2 `UNKNOWN_NOT_EXECUTED` statement. Relationship to the frozen target is `DERIVED_NOT_EXACT_HEAD`.

Fresh critical browser execution:

- run `30633216753`, attempt 2, job `91339118796`;
- artifact `8814897157`;
- smoke 7/7;
- portability 36/36;
- responsive 42/42;
- resilience 2/2;
- accessibility 9/9;
- total 96/96 PASS with Playwright retries zero.

This proves a broad delivered critical profile, not every screen/state permutation.

## Delivery-completeness policy-v2 finding

### MEDIUM — OTERYN-AUDIT-P6-001

**The 43-capability backend/frontend ledger is a benchmark subset, not an exhaustive portal/module completion ledger.**

Explicit legacy capability records are absent for CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E. Route/surface and programme records exist for some of these, but the strict validator cannot fail closed across every module and all 13 delivery/closeout gates.

Disposition: open under Issue `#326`, coordinated with programme `#451`. Another agent owns implementation. PR `#381` records evidence only.

## Issue #365 authoritative state

```yaml
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause: UNKNOWN
old_document_lazy_thumbnail_race:
  classification: DERIVED
  confidence: LOW
```

Post-serialization source `6c1e910d36771f50da5eded93cc50274a90c62d2` retained the original transient status assertion and Playwright retries zero.

| Attempt | Job | Artifact | Responsive mobile |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

Both reproduced failures preserved durable `Published`, version 3 and `Unpublish to draft` state.

Recovered embedded diagnostics prove deterministic stale EditorialMedia fixture expansion. Desktop and tablet retain publication feedback despite contaminated thumbnail HTTP 500 traffic; HTTP 500 presence alone is insufficient to explain mobile flash loss. Existing artifacts do not preserve a complete browser/request/session causal chain.

The source-faithful 18-sample layout probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in all desktop, tablet and mobile samples. The old-document lazy-thumbnail race therefore remains a low-confidence hypothesis only.

## Active exact-frozen matrix

- control head `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- run `30763456046`;
- job `91537990755`;
- temporary observation PR `#476`;
- exact frozen target checked out separately;
- workers `1`;
- retries `0`.

Preparation, exact checkout and validator generation passed. At the second and final permitted state check, the 12-sample matrix remained `in_progress`.

The run must not be polled further or rerun in this invocation. When terminal, inspect once, verify artifacts, update Issue #365 evidence and close PR #476 without merge.

## Open findings

### MEDIUM

- `OTERYN-AUDIT-P35-006` — Wiki media fixtures leak deliberately damaged rows into later projects.
- `OTERYN-AUDIT-P35-001` — strict content-scale closure omits nine canonical fragment surfaces.
- `OTERYN-AUDIT-P35-002` — the dedicated global error matrix omits HTTP 503.
- `OTERYN-AUDIT-P35-003` — accessibility evidence is representative rather than fail-closed per surface.
- `OTERYN-AUDIT-P35-005` — responsive-mobile Wiki publication intermittently loses accessible transient success feedback after durable success.
- `OTERYN-AUDIT-P35-007` — two Wiki administrator fields use an invalid native HTML pattern.
- `OTERYN-AUDIT-P6-001` — the legacy capability ledger is not an exhaustive module/13-layer completion ledger.

### LOW

- `OTERYN-AUDIT-P1-001` — frozen `ACTIVE_WORK.md` ownership evidence conflicts with live task/PR state.

Corrected but not open: `OTERYN-AUDIT-P6-002`, the stale Phase 2 validator status.

## Product-gap boundary

The audit truthfully preserves partial or missing classifications:

- character deletion/restore, rename, world transfer and achievement display;
- real payments, products, entitlements, premium/VIP, vouchers and full customer histories;
- broader Game Catalog/knowledge capabilities;
- Platform API;
- Poland/EU commerce legal/privacy boundaries;
- operations, observability and public-edge evidence.

No implementation is performed here. The implementation agent should use existing Issues `#326`, `#317`, `#319`, `#320`, `#323`, `#321`, `#322` and programme `#451` rather than create a competing scope.

## Deployment boundary

Repository, CI, staging and production evidence remain separate.

- latest directly recorded staging source in this audit: `717977f252b09b9b2e979f8110b7f48b88682223`;
- frozen target deployed: `UNKNOWN`;
- exact private-production release: `UNKNOWN` in this audit;
- public exposure and real-payment activation: separate gates.

No production operation was performed.

## Verdict and residual gate

Verdict: `VALIDATED_WITH_CORRECTIONS`.

The task is not terminal while:

1. run `30763456046` is non-terminal;
2. Issue #365 lacks a valid exact-frozen correlated result;
3. seven medium findings remain open;
4. Issue #326 lacks an exhaustive machine-enforced 18-module/13-layer completion matrix;
5. related PRs/tasks are not intentionally terminal.

No merge, deployment or product implementation is authorized.
