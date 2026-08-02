# Independent validator verdict

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Policy overlay inspected on main: `39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c`  
Verdict: `VALIDATED_WITH_CORRECTIONS`

## Independent conclusion

The repository, CI, browser, recovered-artifact and live GitHub evidence supports this audit only after four interpretation boundaries:

1. session serialization is **not proven to remediate** the intermittent responsive-mobile Wiki publication-feedback loss;
2. the 43-capability backend/frontend ledger proves a bounded repository-integration contract, not complete delivery under policy v2;
3. the current Issue/PR/task ownership graph cannot be inferred from `ACTIVE_WORK.md` or stale branch checkpoints and requires the Phase 7 live reconciliation;
4. current-main change-routing workflows cannot provide exact-head product validation for the frozen-base audit branch because classifier files are loaded from the older PR head.

No product implementation, workflow change, deployment, production mutation, issue lifecycle change or external-repository action was performed.

## Issue #365 correction

Exact post-serialization source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

| Attempt | Job | Artifact | Responsive mobile |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

Both reproductions lacked the accessible transient `Wiki article published.` status while preserving durable `Published`, version 3 and `Unpublish to draft`. Desktop, tablet and Chromium/Firefox/WebKit portability passed.

```yaml
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause: UNKNOWN
old_document_lazy_thumbnail_race:
  classification: DERIVED
  confidence: LOW
samples:
  pass: 1
  reproduced: 2
```

Recovered diagnostics prove deterministic stale EditorialMedia fixture expansion and show that desktop/tablet can retain publication feedback despite thumbnail HTTP 500 traffic. They do not prove a causal request/session chain.

The source-faithful 18-sample layout probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in every desktop, tablet and mobile sample. It weakens the old-document lazy-thumbnail hypothesis but does not reproduce Laravel HTTP or session behavior.

## Strict backend/frontend validation correction

The strict Portal Acceptance Contract passed:

- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run `30633216358`, job `91164376176`, artifact `8794204786`;
- result `PASS`.

This supersedes the historical Phase 2 `UNKNOWN_NOT_EXECUTED` statement. Relationship to the frozen target remains `DERIVED_NOT_EXACT_HEAD`.

A separate critical browser run passed 96/96 tests with retries zero: smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2 and accessibility 9/9. This is broad critical evidence, not exhaustive every-screen/every-state proof.

## Delivery-completeness correction

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

Repository integration remains proven for the legacy boundary. Full delivery is not proven until all applicable persistence, backend, authorization, transport, frontend, states, localization, accessibility, tests, zero-retry E2E, independent audit, exact-head CI and terminal task/PR gates pass.

### OTERYN-AUDIT-P6-001 — MEDIUM / OPEN

The 43-capability ledger is a benchmark subset, not an exhaustive module completion ledger. Explicit legacy records are absent for CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E.

## Live work-graph and CI correction

Phase 7 directly mapped all 21 open Issues and all 6 open PRs and executed the current exact-head workflow set.

### OTERYN-AUDIT-P7-001 — MEDIUM / OPEN

The current coordination index and task checkpoints do not match live Git state:

- `ACTIVE_WORK.md` declares no active tasks while PRs `#338`, `#381`, `#391`, `#405`, `#471` and `#476` are open;
- task checkpoints for PRs `#338`, `#391`, `#405` and `#471` lag their current PR heads or PR identity.

The exact head mismatches and dispositions are recorded in `phase-7-issue-pr-coverage.json`.

Impact: an implementation agent can select stale ownership, blocker, validation or next-action state.

### OTERYN-AUDIT-P7-002 — MEDIUM / OPEN

PR `#391` and its official-client live-reference task have no explicit parent Issue and no first-class module ownership in the 18-module production-completion ledger.

The work must receive intentional programme ownership under an external-client interoperability boundary or an explicit existing-module mapping before it can be considered part of a fail-closed programme inventory.

### OTERYN-AUDIT-P7-003 — MEDIUM / OPEN

The current change-routing workflow rollout is not backward-compatible with pre-rollout PR heads.

On exact audit head `475013aa05a44a24d83cea09b0237147216c8d1f`:

- Agent Governance run `30767823565` passed;
- CI `30767823552`, Edge Security `30767823563`, Platform DB Outage `30767823557` and Game Auth Ticket Concurrency `30767823549` failed in classification because `tests/ci/test_classify_changes.py` is absent from the exact PR head;
- Phase 7 Production-Like run `30767823551` failed in classification because `scripts/ci/classify_changes.py` is absent from the exact PR head.

Both files exist on current main. All five heavy workflows skipped application setup and product testing. This is not product-failure evidence; it is proven workflow/branch compatibility failure.

Impact: the audit cannot obtain current exact-head heavy CI without a backward-compatible fallback, a controlled rebase or intentional import of CI support files. Those changes belong to the separate CI/governance implementation agent.

## Findings summary

Frozen portal/product findings:

- HIGH: `0`;
- MEDIUM: `7`;
- LOW: `1`.

Additional live work-graph/CI findings:

- HIGH: `0`;
- MEDIUM: `3`;
- LOW: `0`.

Open frozen MEDIUM findings:

- `OTERYN-AUDIT-P35-006` — damaged EditorialMedia fixture leakage;
- `OTERYN-AUDIT-P35-001` — nine content-scale fragment surfaces omitted;
- `OTERYN-AUDIT-P35-002` — dedicated HTTP 503 matrix missing;
- `OTERYN-AUDIT-P35-003` — accessibility evidence not fail-closed per surface;
- `OTERYN-AUDIT-P35-005` — intermittent mobile publication-feedback loss;
- `OTERYN-AUDIT-P35-007` — invalid native HTML pattern;
- `OTERYN-AUDIT-P6-001` — non-exhaustive benchmark capability ledger.

Open frozen LOW finding:

- `OTERYN-AUDIT-P1-001` — frozen active-work ownership conflict.

Open live work-graph/CI MEDIUM findings:

- `OTERYN-AUDIT-P7-001` — current active-work/checkpoint graph is stale;
- `OTERYN-AUDIT-P7-002` — official-client interoperability work lacks issue/module ownership;
- `OTERYN-AUDIT-P7-003` — current workflow routing cannot validate pre-rollout exact PR heads.

Corrected but not open:

- `OTERYN-AUDIT-P6-002` — stale Phase 2 validator status.

## Active exact-frozen validation

- control head `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- run `30763456046`;
- job `91537990755`;
- temporary observation PR `#476`;
- workers `1`; retries `0`.

Preparation, exact frozen checkout and validator generation passed. At the first check of the current invocation, the corrected 12-sample matrix remained `in_progress`.

Do not rerun it. One later unchanged-state check remains permitted. When terminal, inspect once, verify artifacts, synchronize Issue `#365` and PR `#381` evidence, and close PR `#476` without merge.

## Verdict boundary

`VALIDATED` is forbidden while:

- the exact-frozen correlated matrix is non-terminal or inconclusive;
- frozen portal/product material findings remain open;
- live work-graph/CI material findings remain open;
- the exhaustive Issue `#326` module/capability completion contract is absent;
- exact-head heavy product validation cannot execute on the frozen-base audit branch;
- related PRs and tasks are not intentionally terminal.

Verdict remains `VALIDATED_WITH_CORRECTIONS`. Task status remains `waiting`.
