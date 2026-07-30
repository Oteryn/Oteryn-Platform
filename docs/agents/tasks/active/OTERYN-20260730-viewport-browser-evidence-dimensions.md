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
- [x] Every critical rendered surface proves blocking zero-retry Chromium desktop, tablet and mobile execution through an exact project or test-controlled viewport marker.
- [x] Firefox/WebKit coverage or bounded risk-based exclusion rationale is explicit.
- [x] Missing files, markers, projects, profiles and orphan mappings fail closed.
- [x] Deterministic negative fixtures cover missing mobile/tablet evidence, unknown project/browser, missing rationale, missing marker, orphan record and non-blocking critical evidence.
- [x] Strict Portal Acceptance invokes the dimension validator and fixtures through the existing strict package entrypoint.
- [x] Responsive localization evidence asserts visible user-operable links rather than hidden desktop elements.
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
  - scripts/acceptance/coverage/validate-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/test-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
dependencies:
  - Issue #326
  - completed Issue #340 / PR #341
  - completed Issue #350 / PR #351 remains a separate state-evidence slice
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T10:50:00Z
head: efa34b63d3834e14e4b4f05333db80b5825b9892
branch: test/OTERYN-20260730-viewport-browser-evidence-dimensions
pr: 349
status: ready
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
  - scripts/acceptance/coverage/validate-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/test-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
proven:
  - Existing portal coverage validation accepted arbitrary non-empty viewport and browser strings and only required aggregate stable evidence markers.
  - The canonical dimension contract contains exactly 27 records split across four fragments, matching all delivered portal surfaces including supporting media endpoints.
  - Thirteen executable profile groups map exact configuration files, project names, browser engines, viewports, blocking invocations and zero-retry evidence.
  - All 23 critical rendered surfaces prove blocking zero-retry Chromium desktop 1440x1000, tablet 820x1180 and mobile 390x844 through direct projects, exact test-controlled viewport markers or project-selected shared evidence.
  - Secret-bearing, destructive and high-mutation flows use explicit risk-based Firefox/WebKit exclusions rather than invented portability claims.
  - Marketplace responsive Chromium evidence remains, while the unproven generic bounded-portability declaration was removed.
  - The pre-existing dimension validator scaffold referenced a missing portal-dimension-evidence.json file and was absent from the strict gate; its public command names now delegate to the canonical implementation.
  - Six general dimension fixtures plus two critical-viewport fixtures deterministically reject missing mappings, unknown projects/browser IDs, missing rationale/markers, orphan records, missing tablet execution and non-blocking critical evidence.
  - The first strengthened strict run rejected Account Security and Downloads test-controlled tablet/mobile evidence because the validator incorrectly required separate projects; the validator was corrected to require exact scenario and viewport markers without weakening the zero-retry Chromium requirement.
  - The responsive suite then exposed hidden desktop language/navigation locators on tablet/mobile. The test now requires visible language and Aktualności links through the real responsive menu.
  - Implementation evidence head 611b130fb50a1fb2661b890b7f80a70675dad58d passed all nine authoritative workflows, including strict dimension closure, eight negative fixtures, full Visual UX and complete zero-retry account lifecycle.
  - The branch was restacked onto current main eda893990dccca6ffe65549e224f908299d90750 without overwriting the separately merged Issue #350 / PR #351 public game-data stress and 500-recovery evidence.
  - Restacked documentation head efa34b63d3834e14e4b4f05333db80b5825b9892 passed all nine authoritative workflows, including strict Portal Acceptance, full Visual UX and complete zero-retry Account Lifecycle.
derived:
  - Exact dimension linkage can be closed without asserting the remaining every-state, long-data, 500 and media-failure Cartesian matrix.
  - Direct module projects, standard representative portability, test-controlled viewport loops and explicit risk exclusions must remain distinguishable in evidence.
unknown:
  - Remaining state/data/error/media permutations for other delivered surfaces under parent #326.
  - Direct exact-release production behavior remains unverified.
conflicts: []
first_failure:
  marker: critical-test-controlled-viewport-classification
  evidence: Portal Acceptance run 30532300578 on head 5180093e7401dded2b5b7cea9c2c0fb158fbd41a rejected four Account Security and Downloads tablet/mobile mappings because test-controlled evidence was evaluated as if each viewport required a separate Playwright project; commit 6dae0ede0c8b9d3a9f50bee34a456245a9be5746 corrected the distinction while preserving exact markers and blocking zero-retry Chromium.
rejected_hypotheses:
  - Treat a non-empty browsers or viewports array as proof that each dimension executed.
  - Expand every secret-sensitive lifecycle across every browser merely to satisfy a matrix.
  - Preserve Marketplace bounded-portability without an executable Firefox/WebKit mapping.
  - Maintain two independent dimension validators or ledgers.
  - Treat a hidden desktop navigation element as tablet/mobile frontend operability.
changed_paths:
  - docs/agents/PROJECT_STATE.md
  - docs/agents/tasks/active/OTERYN-20260730-viewport-browser-evidence-dimensions.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/identity.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/public-core.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/modules.json
  - scripts/acceptance/coverage/validate-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/test-portal-evidence-dimensions.mjs
  - scripts/acceptance/coverage/validate-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/test-critical-viewport-evidence.mjs
  - scripts/acceptance/coverage/validate-dimension-evidence.mjs
  - scripts/acceptance/coverage/test-dimension-evidence.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - scripts/acceptance/package.json
validation:
  - command: Agent Governance run 30534561641
    result: PASS
    evidence: exact restacked documentation head efa34b63d3834e14e4b4f05333db80b5825b9892.
  - command: CI run 30534561731
    result: PASS
    evidence: exact restacked documentation head; formatting, static analysis and full tests passed.
  - command: Portal Acceptance Contract run 30534561703
    result: PASS
    evidence: strict route/product/backend-frontend/dimension ledgers, eight dimension fixtures and complete zero-retry account lifecycle passed.
  - command: Acceptance E2E and Visual UX run 30534561639
    result: PASS
    evidence: Chromium smoke, Firefox/WebKit portability, responsive desktop/tablet/mobile including visible localization navigation, resilience and keyboard accessibility passed.
  - command: Downloads Acceptance run 30534561631
    result: PASS
    evidence: exact restacked documentation head.
  - command: Phase 7 Production-Like Validation run 30534561637
    result: PASS
    evidence: exact restacked documentation head; staging-like boundary only.
  - command: Platform DB Outage Validation run 30534561753
    result: PASS
    evidence: exact restacked documentation head.
  - command: Edge Security Emulation run 30534561724
    result: PASS
    evidence: exact restacked documentation head.
  - command: Game Auth Ticket Concurrency run 30534561649
    result: PASS
    evidence: exact restacked documentation head.
blockers:
  - none
next_action: Confirm the checkpoint-only final head checks, mark PR #349 ready, update its exact evidence summary and merge Issue #347 without closing parent #326.
```
