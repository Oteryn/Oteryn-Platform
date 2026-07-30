---
task_id: OTERYN-20260730-viewport-browser-evidence
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue #347, parent #326 and open PRs touching portal coverage or Playwright profiles
  - existing portal coverage validator, Playwright projects and acceptance npm profiles
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/testing/E2E_COVERAGE_ROADMAP.md
---

# OTERYN-20260730-viewport-browser-evidence

## Goal

Deliver Issue #347 as a bounded fail-closed evidence-dimension contract that binds every declared portal viewport and browser/profile requirement to exact executable Playwright evidence without claiming the remaining full state matrix in #326.

## Acceptance criteria

- [ ] Canonical viewport and browser/profile identifiers are machine validated.
- [ ] Every covered rendered surface has an explicit dimension policy or bounded non-rendered rationale.
- [ ] Every declared viewport/browser dimension maps to an existing evidence file and stable marker.
- [ ] Referenced Playwright projects and npm profiles exist.
- [ ] Missing, unknown or orphan dimension mappings fail deterministically.
- [ ] Strict Portal Acceptance executes the dimension validator and negative fixtures.
- [ ] Documentation preserves #326 and production nonclaims.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_EVIDENCE_DIMENSIONS.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
dependencies:
  - Issue #347
  - parent Issue #326
  - completed Issue #340 / PR #341
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T08:30:00Z
head: 8e613c00503c0874e69e2085c740f87f4a87e002
branch: test/OTERYN-20260730-viewport-browser-evidence
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
  - architecture
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_EVIDENCE_DIMENSIONS.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Current portal validator requires non-empty viewport/browser arrays and generic stable markers but does not bind each declared dimension to exact evidence.
  - Current main at task start is 8e613c00503c0874e69e2085c740f87f4a87e002.
derived:
  - A separate dimension ledger can close this bounded contract gap without rewriting runtime screens or multiplying secret-sensitive suites across every browser.
unknown:
  - Exact baseline count of covered rendered surfaces and the minimum truthful risk-based portability policies for each.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treat any generic Playwright marker as proof for every viewport and browser declared by a surface.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not started
blockers:
  - none
next_action: Inventory all covered rendered surfaces, Playwright projects and npm profiles, then define the smallest truthful dimension schema.
```
