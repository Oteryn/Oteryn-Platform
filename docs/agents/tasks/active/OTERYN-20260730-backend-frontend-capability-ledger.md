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

- [x] Every canonical product capability has exactly one backend/frontend record.
- [x] Every user-facing `implemented` capability requires implemented backend, reachable frontend, integrated real-route behavior and browser evidence.
- [x] Backend-only user-facing capability cannot validate as `implemented`.
- [x] Frontend without reliable integrated browser evidence is classified `untested` or `partial`, never inferred as implemented.
- [x] Referenced portal surface IDs exist and are `covered` for integrated capabilities.
- [x] Supporting/API-only exceptions require a bounded rationale.
- [x] Missing, partial and not-applicable product statuses remain consistent across ledgers.
- [x] Strict Portal Acceptance executes the new validator with retries unaffected.
- [x] Focused negative fixtures prove the validator fails on backend-only and unknown-surface claims.
- [x] Documentation states that this slice does not close the remaining every-screen/browser/state matrix in #326.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
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
updated_at: 2026-07-30T07:29:00Z
head: 8e6aa3ff7f5ca676e97bfb9f7d01f94d949116b0
branch: test/OTERYN-20260730-backend-frontend-capability-ledger
pr: 341
status: ready
context_routes:
  - agent-governance
  - testing
  - architecture
  - public-portal
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/product-backend-frontend-completeness.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs
  - scripts/acceptance/coverage/test-backend-frontend-completeness.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - PR #315 merged the mandatory backend/frontend/browser implementation rule.
  - The new ledger contains exactly 43 records and the validator cross-checks canonical IDs, layer statuses, exact covered surface IDs and stable Playwright markers.
  - A user-facing product capability marked implemented now fails strict CI unless backend, frontend and integration statuses are all implemented.
  - Non-UI machine/background capabilities require bounded exception rationale.
  - Five deterministic negative fixtures cover backend-only promotion, unknown surfaces, missing non-UI rationale, product/layer contradiction and missing records.
  - Strict Portal Acceptance run 30522494824 passed with 43 baseline capabilities, 27 portal surfaces, zero validation errors and five negative fixtures on evidence head 85a7a5174b2c67829388de4023dc3ee239a92d9c.
  - Complete zero-retry account lifecycle, full Visual UX, CI and all other required workflows passed on the same evidence head.
derived:
  - This cross-ledger slice closes the backend-only promotion risk without rewriting runtime modules.
  - The remaining every-rendered-screen/browser/state Cartesian matrix stays open in parent #326.
unknown:
  - Direct production behavior and the remaining exhaustive frontend-state matrix remain unverified.
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
  - docs/agents/PROJECT_STATE.md
  - docs/agents/tasks/active/OTERYN-20260730-backend-frontend-capability-ledger.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/testing/product-backend-frontend-completeness.json
  - scripts/acceptance/coverage/test-backend-frontend-completeness.mjs
  - scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs
  - scripts/acceptance/package.json
validation:
  - command: GitHub Actions Agent Governance run 30522494811
    result: PASS
    evidence: exact evidence head 85a7a5174b2c67829388de4023dc3ee239a92d9c.
  - command: GitHub Actions CI run 30522494844
    result: PASS
    evidence: exact evidence head; formatting, static analysis and full test suite passed.
  - command: GitHub Actions Portal Acceptance Contract run 30522494824
    result: PASS
    evidence: strict route/product/backend-frontend ledgers, five negative fixtures and complete zero-retry account lifecycle passed.
  - command: GitHub Actions Acceptance E2E and Visual UX run 30522494846
    result: PASS
    evidence: smoke, Chromium/Firefox/WebKit portability, responsive, resilience and keyboard accessibility profiles passed.
  - command: GitHub Actions Downloads Acceptance run 30522498890
    result: PASS
    evidence: exact evidence head.
  - command: GitHub Actions Phase 7 Production-Like Validation run 30522497002
    result: PASS
    evidence: exact evidence head; staging-like boundary only.
  - command: GitHub Actions Platform DB Outage Validation run 30522494901
    result: PASS
    evidence: exact evidence head.
  - command: GitHub Actions Edge Security Emulation run 30522497240
    result: PASS
    evidence: exact evidence head.
  - command: GitHub Actions Game Auth Ticket Concurrency run 30522497077
    result: PASS
    evidence: exact evidence head.
  - command: GitHub Actions Synology Production Target Preflight run 30522494895
    result: PASS
    evidence: exact evidence head; preflight is not production proof.
blockers: []
next_action: Confirm all required workflows on the final evidence-only head, mark PR #341 ready and merge Issue #340 without closing parent #326.
```