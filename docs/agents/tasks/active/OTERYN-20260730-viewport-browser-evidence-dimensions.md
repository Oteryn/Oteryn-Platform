---
task_id: OTERYN-20260730-viewport-browser-evidence-dimensions
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/coverage/validate-portal-coverage.mjs
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/package.json
search_first:
  - Issue #326, Issue #347 and open PRs touching scripts/acceptance/coverage, Playwright profiles or portal acceptance workflows
  - existing viewport/browser declarations and stable Playwright evidence markers
  - reusable coverage validators and negative-fixture harnesses
optional_reads:
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
---

# OTERYN-20260730-viewport-browser-evidence-dimensions

## Goal

Deliver Issue #347 as the next bounded slice of parent #326: fail-closed machine linkage from every declared viewport and browser/profile requirement to exact executable Playwright evidence, while preserving risk-based browser scope and production nonclaims.

## Acceptance criteria

- [x] Every covered rendered surface has an explicit dimension policy or a bounded supporting-endpoint exclusion.
- [x] Canonical viewport and browser/profile identifiers are validated against an allowlist.
- [x] Every declared viewport mapping requires an exact evidence file, stable marker and executable Playwright project.
- [x] Critical rendered surfaces require blocking zero-retry profiles for their declared dimensions.
- [x] Firefox/WebKit coverage or bounded risk-based exclusion rationale is explicit.
- [x] Missing files, markers, projects, profiles and orphan mappings fail closed.
- [x] Deterministic negative fixtures cover missing mobile evidence, unknown project/browser, missing rationale, missing evidence and orphan records.
- [x] Strict Portal Acceptance invokes the dimension validator and fixtures through the existing strict package entrypoint.
- [x] Parent #326 remains open for the still-unproven full state/data/error/media Cartesian matrix.
- [x] No staging or production proof is claimed.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence-dimensions.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/**
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
dependencies:
  - Issue #326
  - completed Issue #340 / PR #341
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T09:45:00Z
head: e78947f1808e9cca08ca26a2962b1f25e3cfccba
branch: test/OTERYN-20260730-viewport-browser-evidence-dimensions
pr: 349
status: validating
context_routes:
  - agent-governance
  - testing
  - architecture
  - web-cms
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence-dimensions.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/**
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Existing portal coverage validation accepted arbitrary non-empty viewport and browser strings and only required aggregate stable evidence markers.
  - Playwright defines canonical Chromium desktop/tablet/mobile, bounded Firefox/WebKit portability and specialized zero-retry module profiles.
  - The canonical dimension contract now contains exactly 27 records split across four fragments, matching every delivered portal surface including the supporting media endpoints.
  - Thirteen executable profile groups map exact config files, project names, browsers, viewports, blocking invocations and zero-retry evidence.
  - Secret-bearing, destructive and high-mutation flows use explicit risk-based Firefox/WebKit exclusions rather than invented portability claims.
  - Marketplace responsive Chromium evidence is retained while its unproven bounded-portability declaration has been removed.
  - The pre-existing dimension validator scaffolding referenced a missing portal-dimension-evidence.json file and was not part of the strict gate; its public command names now delegate to the canonical implementation.
  - Six deterministic negative fixtures cover missing mobile mapping, unknown project, missing rationale, orphan record, missing marker and unknown browser declaration.
derived:
  - This slice can close exact dimension linkage without asserting the remaining every-state, long-data, 500 and media-failure Cartesian matrix.
  - Direct module projects, standard representative portability, test-controlled viewport loops and explicit risk exclusions must remain distinguishable in evidence.
unknown:
  - Exact-head CI may still reject stale markers, incorrect profile references or a browser scenario whose rendered heading differs from the asserted copy.
conflicts: []
first_failure:
  marker: dimension-contract-missing-source
  evidence: The pre-existing validate-dimension-evidence.mjs defaulted to scripts/acceptance/coverage/portal-dimension-evidence.json, but that contract file did not exist and the strict package command did not invoke the dimension checks.
rejected_hypotheses:
  - Treat a non-empty browsers or viewports array as proof that each dimension executed.
  - Expand every secret-sensitive lifecycle across every browser merely to satisfy a matrix.
  - Preserve Marketplace bounded-portability without an executable Firefox/WebKit mapping.
  - Maintain two independent dimension validators or ledgers.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence-dimensions.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/identity.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/public-core.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/modules.json
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
validation:
  - command: GitHub Actions exact-head workflows for PR #349
    result: NOT_RUN
    evidence: final strict integration head has just been assembled; a new exact-head workflow set is required
blockers:
  - none
next_action: Run the full exact-head workflow set, fix the first fail-closed invariant without weakening the validator, then update project and frontend audit evidence before merge.
```
