# Independent validator verdict

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Policy overlay inspected on main: `39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c`  
Verdict: `VALIDATED_WITH_CORRECTIONS`

## Independent conclusion

The evidence supports this audit only after four interpretation boundaries:

1. session serialization is **not proven to remediate** the intermittent responsive-mobile Wiki publication-feedback loss;
2. the 43-capability backend/frontend ledger proves a bounded repository-integration contract, not complete delivery under policy v2;
3. the current Issue/PR/task ownership graph requires the Phase 7 live reconciliation rather than `ACTIVE_WORK.md` or stale checkpoints alone;
4. current-main change-routing workflows cannot provide exact-head product validation for the frozen-base audit branch because classifier files are loaded from the older PR head.

The corrected exact-frozen run `30763456046` reached its matrix, but the matrix step ended `cancelled`. Immutable evidence upload succeeded and the job remained non-terminal during cleanup at the second and final permitted state check. Until terminal artifact inspection, the cancellation reason, sample completeness and correlation evidence are unknown.

No product implementation, workflow change, deployment, production mutation, issue lifecycle change or external-repository action was performed.

## Issue #365 correction

Exact post-serialization source: `6c1e910d36771f50da5eded93cc50274a90c62d2`.

| Attempt | Job | Artifact | Responsive mobile |
|---:|---:|---:|---|
| 2 | `91342520692` | `8815321615` | PASS |
| 3 | `91343023604` | `8815383351` | REPRODUCED |
| 4 | `91343514611` | `8815457044` | REPRODUCED |

Both reproductions lacked accessible transient `Wiki article published.` status while preserving durable `Published`, version 3 and `Unpublish to draft`.

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

Recovered diagnostics prove deterministic stale EditorialMedia fixture expansion but do not prove a causal request/session chain. The source-faithful 18-sample layout probe recorded zero thumbnail request starts from the beginning of `Publish.click()` in every sample; it weakens but does not eliminate the old-document lazy-thumbnail hypothesis.

## Strict validation correction

The strict Portal Acceptance Contract passed on exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789` in run `30633216358`, job `91164376176`, artifact `8794204786`.

A separate critical browser run passed 96/96 tests with retries zero: smoke 7/7, portability 36/36, responsive 42/42, resilience 2/2 and accessibility 9/9. This is broad critical evidence, not exhaustive every-screen/every-state proof.

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

### OTERYN-AUDIT-P6-001 — MEDIUM / OPEN

The 43-capability ledger is a benchmark subset, not an exhaustive module completion ledger. Explicit records are absent for several delivered or cross-cutting modules and the strict validator cannot fail closed over all 18 modules and all 13 delivery gates.

## Live work-graph and CI correction

Phase 7 directly mapped all 21 open Issues and all 6 open PRs and tested the exact-head workflow boundary.

### OTERYN-AUDIT-P7-001 — MEDIUM / OPEN

`ACTIVE_WORK.md` declares no active tasks while six PRs are open, and task checkpoints for PRs `#338`, `#391`, `#405` and `#471` lag live PR identity.

### OTERYN-AUDIT-P7-002 — MEDIUM / OPEN

PR `#391` and its official-client live-reference task have no explicit parent Issue and no first-class module ownership in the production-completion ledger.

### OTERYN-AUDIT-P7-003 — MEDIUM / OPEN

Current change-routing workflows are not backward-compatible with pre-rollout PR heads.

On exact audit head `475013aa05a44a24d83cea09b0237147216c8d1f`:

- Agent Governance run `30767823565` passed;
- CI `30767823552`, Edge Security `30767823563`, Platform DB Outage `30767823557` and Game Auth Ticket Concurrency `30767823549` stopped before product validation because `tests/ci/test_classify_changes.py` was absent;
- Phase 7 Production-Like `30767823551` stopped because `scripts/ci/classify_changes.py` was absent.

Both files exist on current main. These are workflow/branch compatibility failures, not portal product regressions.

## Corrected exact-frozen run

```yaml
control_head: 8c58035cacb9fd4675d898a1652036fc8b9d4357
run: 30763456046
job: 91537990755
temporary_pr: 476
workers: 1
retries: 0
matrix_step: CANCELLED
artifact_upload: PASS
job_terminal_state_at_last_allowed_check: UNKNOWN_NON_TERMINAL
```

Proven: exact checkout and preparation passed, the matrix entered runtime, the matrix step ended `cancelled`, artifact upload succeeded, and cleanup was still in progress.

Unknown: cancellation reason, artifact ID/digest/completeness, completed samples and whether request/session correlation survived.

A cancelled step or successful upload alone proves neither failure nor remediation. Do not rerun. In a later invocation, inspect the terminal run once, verify the artifact, update Issue `#365` and PR `#381`, and close PR `#476` without merge.

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

Corrected but not open: `OTERYN-AUDIT-P6-002`, stale Phase 2 validator status.

## Verdict boundary

`VALIDATED` is forbidden while:

- run `30763456046` lacks terminal artifact inspection and classification;
- frozen portal/product material findings remain open;
- live work-graph/CI material findings remain open;
- the exhaustive Issue `#326` module/capability completion contract is absent;
- exact-head heavy product validation cannot execute on the frozen-base audit branch;
- related PRs and tasks are not intentionally terminal.

Verdict remains `VALIDATED_WITH_CORRECTIONS`. Task status remains `waiting`.
