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
  - Issue #326 and open PRs touching product or portal coverage validation
  - existing product and portal coverage validators before adding a new contract
  - active task ownership for scripts/acceptance/coverage and docs/testing
optional_reads:
  - docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260730-backend-frontend-capability-ledger

## Goal

Deliver the first bounded remediation slice of #326: a machine-readable, fail-closed backend–frontend capability ledger linked to the existing 43-capability product benchmark and portal surface manifest.

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
updated_at: 2026-07-30T07:00:00Z
head: d88db988f342d1c8b085244cf8d9d49d80d7d1c4
branch: test/OTERYN-20260730-backend-frontend-capability-ledger
pr: null
status: implementing
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
  - Existing product validation does not yet require explicit backend/frontend/integration records for all 43 capabilities.
derived:
  - A separate cross-ledger validator is the smallest safe slice and avoids rewriting runtime modules.
unknown:
  - Exact first failing invariant until the new validator is executed in CI.
conflicts: []
first_failure: null
rejected_hypotheses:
  - Treat feature or API evidence alone as frontend implementation proof.
  - Close #326 with this first ledger slice.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
validation:
  - command: GitHub Actions exact-head validation
    result: NOT_RUN
    evidence: implementation has not yet been committed.
blockers: []
next_action: Add the cross-ledger schema, validator and focused negative fixture tests, then run the strict Portal Acceptance contract.
```