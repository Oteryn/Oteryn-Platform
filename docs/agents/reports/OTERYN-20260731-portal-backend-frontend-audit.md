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

Under the current delivery-completeness policy, that is not sufficient to call a capability complete:

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

The merged production-completion baseline identifies 18 modules. The earlier 43-capability ledger is a benchmark subset and lacks explicit records for several delivered or cross-cutting modules. This is finding `OTERYN-AUDIT-P6-001`.

The live GitHub reconciliation mapped all 21 open Issues and all 6 open pull requests. It found that the main active-work index declares zero active tasks while five PRs contain active task records, four records lag their live PR identity, and PR `#391` has neither a parent Issue nor explicit module ownership. Exact-head execution also proved that the current change-routing workflows cannot validate a pre-rollout PR head because their classifier files are read from that older head. These are findings `OTERYN-AUDIT-P7-001`, `OTERYN-AUDIT-P7-002` and `OTERYN-AUDIT-P7-003`.

Open frozen portal/product findings: **0 HIGH / 7 MEDIUM / 1 LOW**.  
Additional open live work-graph/CI findings: **0 HIGH / 3 MEDIUM / 0 LOW**.  
Independent verdict remains **`VALIDATED_WITH_CORRECTIONS`**.  
Task status remains **`waiting`** on the bounded Issue `#365` matrix and externally owned CI compatibility remediation.

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

Policy-v2 details are in:

- `phase-6-delivery-completeness-crosswalk.json`;
- `OTERYN-20260731-portal-backend-frontend-audit-phase-6-delivery-completeness.md`.

The current live work and CI graph is in:

- `phase-7-issue-pr-coverage.json`;
- `OTERYN-20260731-portal-backend-frontend-audit-phase-7-issue-pr-coverage.md`.

## Strict backend/frontend and browser validation

Portal Acceptance Contract:

- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`, job `91164376176`, artifact `8794204786`;
- digest `sha256:82daac38363f959c21019d3e570eff987366774886cf1e2f9b1afdf2e889a385`;
- result `PASS`.

This corrects the historical Phase 2 `UNKNOWN_NOT_EXECUTED` statement. Relationship to the frozen target is `DERIVED_NOT_EXACT_HEAD`.

Fresh critical browser execution:

- run `30633216753`, attempt 2, job `91339118796`, artifact `8814897157`;
- smoke 7/7;
- portability 36/36;
- responsive 42/42;
- resilience 2/2;
- accessibility 9/9;
- total 96/96 PASS with Playwright retries zero.

This proves a broad delivered critical profile, not every screen/state permutation.

## Delivery-completeness finding

### MEDIUM — OTERYN-AUDIT-P6-001

The 43-capability backend/frontend ledger is a benchmark subset, not an exhaustive portal/module completion ledger.

Explicit legacy capability records are absent for CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E. Route/surface and programme records exist for some of these, but the strict validator cannot fail closed across every module and all 13 delivery/closeout gates.

Disposition: open under Issue `#326`, coordinated with programme `#451`. Another agent owns implementation.

## Live Issue, PR and CI reconciliation

Phase 7 observed:

- 21 open Issues and 6 open PRs;
- all 21 Issues and all 6 PRs mapped to role, module and disposition;
- 5 PRs with active task records;
- temporary validator PR `#476` without a separate task record;
- `ACTIVE_WORK.md` declaring no active tasks;
- Agent Governance passing on exact audit head `475013aa05a44a24d83cea09b0237147216c8d1f`;
- five heavy validation workflows stopping before product validation because current-main classifier files are absent from the older PR head.

Current PR dispositions:

- `#338` — blocked required Game Catalog consumer; keep draft;
- `#381` — this audit; keep draft;
- `#391` — blocked official-client interoperability research; keep draft but classify ownership;
- `#405` — blocked production gate evidence; keep draft;
- `#471` — active payment backend producer; keep draft and refresh checkpoint;
- `#476` — temporary validator; close without merge after evidence persistence.

### MEDIUM — OTERYN-AUDIT-P7-001

The live coordination index and task checkpoints do not match the current PR graph.

- `ACTIVE_WORK.md` says no active tasks;
- PR `#338`: live `8baec8d66c1bab0b618684096300ab491dacacb4`, checkpoint `b1adb5355871cc7ede579799669d38ca323e3dcc`;
- PR `#391`: live `630ed73c09242cf3d37f3652b06fa252c6b0f10d`, checkpoint `cabad487a139aaf0983dfc55cfb18d9f43720633`;
- PR `#405`: live `6357fce7d68cfaa16452e7d71719a5c0ea886717`, checkpoint `90f367963ddaee6fa6884319fc8cc54e23ca8ec4`;
- PR `#471`: live `cda564d4072f8ddac9f258a106b660a3558c50d5`, checkpoint `head: UNKNOWN`, `pr: none`.

Impact: another autonomous agent can read stale ownership, validation, blocker or next-action state and duplicate or mis-sequence work.

Disposition: programme `#451` coordination remediation. No implementation in PR `#381`.

### MEDIUM — OTERYN-AUDIT-P7-002

PR `#391` and task `OTERYN-20260801-official-linux-client-live-reference` have no explicit parent Issue and no first-class module ownership in the production-completion ledger.

This work creates an external-client interoperability research capability and plans cross-stack requirements. It must be adopted under a dedicated module boundary or intentionally mapped to existing modules by programme `#451`.

Disposition: programme classification and ownership correction. No implementation or issue mutation in PR `#381`.

### MEDIUM — OTERYN-AUDIT-P7-003

Current change-routing workflows are not backward-compatible with pre-rollout PR heads.

On exact audit head `475013aa05a44a24d83cea09b0237147216c8d1f`:

- Agent Governance `30767823565` passed after the checkpoint repair;
- CI `30767823552`, Edge Security `30767823563`, Platform DB Outage `30767823557` and Game Auth Ticket Concurrency `30767823549` failed in classification because `tests/ci/test_classify_changes.py` is absent from the exact PR head;
- Phase 7 Production-Like `30767823551` failed because `scripts/ci/classify_changes.py` is absent from the exact PR head.

The files exist on current main but not on the frozen-base audit branch. All five heavy workflows skipped application setup and product validation. A backward-compatible workflow fallback, controlled rebase or intentional import of CI support files is required.

Disposition: programme `#451` CI/governance remediation. This audit does not modify workflows or committed tests and does not silently alter the frozen product baseline.

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

Recovered diagnostics prove deterministic stale EditorialMedia fixture expansion. Desktop and tablet retain publication feedback despite contaminated thumbnail HTTP 500 traffic; HTTP 500 presence alone is insufficient to explain mobile flash loss. Existing artifacts do not preserve a complete browser/request/session causal chain.

The source-faithful 18-sample layout probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in all desktop, tablet and mobile samples. The old-document lazy-thumbnail race therefore remains a low-confidence hypothesis only.

## Active exact-frozen matrix

- control head `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- run `30763456046`;
- job `91537990755`;
- temporary observation PR `#476`;
- workers `1`;
- retries `0`.

Preparation, exact checkout and validator generation passed. At the first state check in the current invocation, the 12-sample matrix remained `in_progress`.

The run must not be rerun. One later unchanged-state check remains permitted in this invocation. When terminal, inspect once, verify artifacts, update Issue `#365` and PR `#381` evidence, and close PR `#476` without merge.

## Open frozen portal/product findings

### MEDIUM

- `OTERYN-AUDIT-P35-006` — Wiki media fixtures leak deliberately damaged rows into later projects.
- `OTERYN-AUDIT-P35-001` — strict content-scale closure omits nine canonical fragment surfaces.
- `OTERYN-AUDIT-P35-002` — dedicated global error matrix omits HTTP 503.
- `OTERYN-AUDIT-P35-003` — accessibility evidence is representative rather than fail-closed per surface.
- `OTERYN-AUDIT-P35-005` — responsive-mobile Wiki publication intermittently loses accessible transient success feedback after durable success.
- `OTERYN-AUDIT-P35-007` — two Wiki administrator fields use an invalid native HTML pattern.
- `OTERYN-AUDIT-P6-001` — the legacy capability ledger is not an exhaustive module/13-layer completion ledger.

### LOW

- `OTERYN-AUDIT-P1-001` — frozen `ACTIVE_WORK.md` ownership evidence conflicts with live task/PR state.

Additional live work-graph/CI findings: `P7-001`, `P7-002` and `P7-003`, all MEDIUM.

Corrected but not open: `OTERYN-AUDIT-P6-002`, the stale Phase 2 validator state.

## Product-gap boundary

The audit preserves partial or missing classifications:

- character deletion/restore, rename, world transfer and achievement display;
- real payments, products, entitlements, premium/VIP, vouchers and complete customer histories;
- broader Game Catalog/knowledge capabilities;
- Platform API;
- Poland/EU commerce legal/privacy boundaries;
- operations, observability and public-edge evidence;
- external-client interoperability ownership and acceptance classification;
- backward-compatible exact-head validation for pre-routing-rollout branches.

No implementation is performed here. The implementation agent must use the Phase 6 module crosswalk and Phase 7 live work graph rather than relying only on the selected issue list in older reports.

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
2. Issue `#365` lacks a valid exact-frozen correlated result;
3. frozen portal/product material findings remain open;
4. Phase 7 coordination, taxonomy and CI-compatibility findings remain open;
5. Issue `#326` lacks an exhaustive machine-enforced 18-module/13-layer completion matrix;
6. exact-head heavy product validation cannot execute on the frozen-base audit branch;
7. related PRs/tasks are not intentionally terminal.

No merge, deployment or product implementation is authorized.
