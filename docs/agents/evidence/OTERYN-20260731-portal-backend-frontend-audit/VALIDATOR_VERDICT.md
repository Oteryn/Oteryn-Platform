# Independent validator verdict

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Policy overlay inspected on current main: `39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c`  
Verdict: `VALIDATED_WITH_CORRECTIONS`

## Independent conclusion

The available repository, CI, browser and recovered-artifact evidence supports the audit only after two material interpretation corrections:

1. session serialization is **not proven to remediate** the intermittent responsive-mobile Wiki publication-feedback loss;
2. the historical 43-capability backend/frontend ledger proves a bounded repository-integration contract, not full completion under the current delivery-completeness policy.

No product implementation, workflow change, deployment, production mutation or external-repository action was performed by this audit.

## Issue #365 correction

Exact post-serialization source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

Three independent zero-retry attempts produced:

| Attempt | Job | Artifact | Responsive mobile |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

Both reproductions lacked the accessible transient `Wiki article published.` status while preserving durable `Published`, version 3 and `Unpublish to draft` state. Desktop, tablet and Chromium/Firefox/WebKit portability passed.

Authoritative state:

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

Recovered embedded diagnostics prove deterministic stale EditorialMedia fixture expansion and show that desktop/tablet can retain publication feedback despite thumbnail HTTP 500 traffic. They do not prove a causal request/session chain.

The source-faithful 18-sample responsive layout probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in every desktop, tablet and mobile sample. It weakens the old-document lazy-thumbnail hypothesis but does not reproduce Laravel HTTP or session behavior.

## Strict backend/frontend validation correction

The strict Portal Acceptance Contract did execute successfully:

- exact source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run: `30633216358`;
- job: `91164376176`;
- artifact: `8794204786`;
- result: `PASS`.

This supersedes the historical Phase 2 `UNKNOWN_NOT_EXECUTED` statement. Relationship to the frozen target remains `DERIVED_NOT_EXACT_HEAD`.

A separate critical browser run on that source passed 96/96 tests with retries zero: smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2 and accessibility 9/9. This is broad critical evidence, not exhaustive every-screen/every-state proof.

## Delivery-completeness policy-v2 correction

The legacy 43-record result remains factually valid for its original contract:

```yaml
legacy_backend_frontend_result:
  implemented: 23
  partial: 3
  missing: 14
  not_applicable: 3
```

Under the current completion contract, an integrated capability is not complete until all applicable persistence, backend, authorization, transport, real frontend, states, localization, responsive/accessibility, focused/integration tests, real zero-retry E2E, independent audit, exact-final-head CI and terminal PR/task gates are proven together.

The independent policy-v2 result is therefore:

```yaml
policy_v2_result:
  complete: 0
  repository_integrated_evidence_open: 23
  partial: 3
  missing: 14
  not_applicable: 3
```

This does not retract repository integration. It prevents the legacy word `implemented` from being promoted into a full-delivery or production-complete claim.

### OTERYN-AUDIT-P6-001 — MEDIUM

The 43-capability ledger is a benchmark subset, not an exhaustive module completion ledger. Explicit legacy capability records are absent for CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E.

The authoritative 18-module and 43-capability overlay is:

- `phase-6-delivery-completeness-crosswalk.json`;
- `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit-phase-6-delivery-completeness.md`.

Implementation belongs to Issue `#326`, coordinated with programme `#451`, and is excluded from this audit PR.

## Normalized findings

Open findings after the policy-v2 extension:

- HIGH: `0`;
- MEDIUM: `7`;
- LOW: `1`.

Open MEDIUM findings:

- `OTERYN-AUDIT-P35-006` — damaged EditorialMedia fixture leakage;
- `OTERYN-AUDIT-P35-001` — nine content-scale fragment surfaces omitted;
- `OTERYN-AUDIT-P35-002` — dedicated HTTP 503 matrix missing;
- `OTERYN-AUDIT-P35-003` — accessibility evidence not fail-closed per surface;
- `OTERYN-AUDIT-P35-005` — intermittent mobile publication-feedback loss;
- `OTERYN-AUDIT-P35-007` — invalid native HTML pattern;
- `OTERYN-AUDIT-P6-001` — non-exhaustive benchmark capability ledger.

Open LOW finding:

- `OTERYN-AUDIT-P1-001` — frozen active-work ownership conflict.

Corrected but not open:

- `OTERYN-AUDIT-P6-002` — stale Phase 2 validator status.

## Active exact-frozen validation

- control head: `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- run: `30763456046`;
- job: `91537990755`;
- temporary observation PR: `#476`;
- workers: `1`;
- retries: `0`.

Preparation, exact frozen checkout and validator generation passed. At the second and final allowed state check in this invocation, the corrected 12-sample matrix remained `in_progress`.

The run must not be polled again or rerun in this invocation. When terminal, inspect it once, verify any artifact, synchronize Issue #365 evidence and close PR #476 without merge.

## Verdict boundary

`VALIDATED` is forbidden while:

- the exact-frozen correlated matrix is non-terminal or inconclusive;
- seven material medium findings remain open;
- the exhaustive Issue #326 module/capability completion contract is absent;
- related PRs and tasks are not intentionally terminal.

Verdict remains `VALIDATED_WITH_CORRECTIONS`. Task status remains `waiting` on the external run.
