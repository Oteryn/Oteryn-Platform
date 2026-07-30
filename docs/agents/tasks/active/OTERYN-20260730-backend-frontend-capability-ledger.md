---
task_id: OTERYN-20260730-backend-frontend-capability-ledger
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/testing/product-completeness-benchmark.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue #326, Issue #340 and open PRs touching product or portal coverage validation
  - existing product and portal coverage validators before adding a new contract
  - active task ownership for scripts/acceptance/coverage and docs/testing
optional_reads:
  - docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260730-backend-frontend-capability-ledger

## Goal

Deliver the first bounded remediation slice of #326 through Issue #340: a machine-readable, fail-closed backend–frontend capability ledger linked to the existing 43-capability product benchmark and portal surface manifest.

## Acceptance criteria

- [ ] Every canonical product capability has exactly one backend/frontend record.
- [ ] Every user-facing `implemented` capability requires implemented backend, reachable frontend, integrated real-route behavior and browser evidence.
- [ ] Backend-only user-facing capability cannot validate as `implemented`.
- [ ] Frontend without reliable integrated browser evidence is classified `untested` or `partial`, never inferred as implemented.
- [ ] Referenced portal surface IDs exist and are `covered` for integrated capabilities.
- [ ] Supporting/API-only exceptions require a bounded rationale.
- [ ] Missing, partial and not-applicable product statuses remain consistent across ledgers.
- [ ] Strict Portal Acceptance executes the new validator with retries unaffected.
- [ ] Focused negative fixtures prove the validator fails on backend-only and unknown-surface claims.
- [ ] Documentation states that this slice does not close the remaining every-screen/browser/state matrix in #326.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/product-backend-frontend-completeness.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs
  - scripts/acceptance/coverage/test-backend-frontend-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T07:18:00Z
head: 1513bacbea7f2af8b773517a3321be4d4e404862
branch: test/OTERYN-20260730-backend-frontend-capability-ledger
pr: 341
status: validating
context_routes:
  - agent-governance
  - testing
  - architecture
  - public-portal
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/product-backend-frontend-completeness.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs
  - scripts/acceptance/coverage/test-backend-frontend-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - PR #315 merged the mandatory backend/frontend/browser implementation rule.
  - Existing portal validation classifies every named route and verifies surface evidence markers.
  - Existing product validation did not require explicit backend/frontend/integration records for all 43 capabilities.
  - The new ledger contains 43 records and the validator cross-checks canonical IDs, layer statuses, exact covered surface IDs and stable Playwright markers.
  - Negative fixtures cover backend-only promotion, unknown surfaces, missing non-UI rationale, product/layer contradiction and missing records.
derived:
  - A separate cross-ledger validator is the smallest safe slice and avoids rewriting runtime modules.
  - This slice must leave the remaining every-screen/browser/state matrix open in parent #326.
unknown:
  - Whether the corrected exact surface IDs and checkpoint pass the next exact-head CI suite.
conflicts: []
first_failure:
  marker: backend-frontend-unknown-surface-ids
  evidence: Portal Acceptance run 30521949405 on head d0416600deeca89261d9ea038baeab5f326c2489 rejected three descriptive surface names; commit 4069d96724534ced45c9817467a873af21f2e494 replaced them with exact manifest IDs.
rejected_hypotheses:
  - Treat feature or API evidence alone as frontend implementation proof.
  - Treat descriptive labels as exact portal surface IDs.
  - Close #326 with this first ledger slice.
changed_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/testing/product-backend-frontend-completeness.json
  - scripts/acceptance/coverage/test-backend-frontend-completeness.mjs
  - scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs
  - scripts/acceptance/package.json
validation:
  - command: Portal Acceptance strict coverage run 30521949405
    result: FAIL
    evidence: validator correctly rejected three unknown surface IDs on head d0416600deeca89261d9ea038baeab5f326c2489.
  - command: corrected exact-head GitHub Actions suite
    result: NOT_RUN
    evidence: queued after exact surface and checkpoint reconciliation.
blockers: []
next_action: Confirm the corrected strict validator and negative fixtures pass, then reconcile final exact-head evidence.
```