---
task_id: OTERYN-20260730-long-content-large-results
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue #362, parent #326 and open PRs touching portal coverage, pagination, overflow or long-value evidence
  - existing Playwright evidence for long localized values, large result sets, tables, cards and pagination
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/testing/PORTAL_EVIDENCE_DIMENSIONS.json
---

# OTERYN-20260730-long-content-large-results

## Goal

Deliver Issue #362 as a bounded fail-closed audit and evidence contract for applicable long-content and large-result rendered states without claiming unrelated closure under parent #326.

## Acceptance criteria

- [x] Every delivered rendered surface has an explicit applicability classification.
- [ ] Applicable long-content and large-collection states map to exact executable evidence.
- [ ] Deterministic fixtures exercise long EN/PL values and bounded multi-page collections through real routes and data paths.
- [ ] Evidence verifies readable wrapping, table/card containment, stable pagination and no document-level horizontal overflow.
- [ ] Referenced evidence files, stable markers, Playwright projects and npm profiles exist.
- [ ] Missing mappings, unknown consumers, orphan evidence and unjustified exclusions fail deterministically.
- [ ] Strict Portal Acceptance executes the validator and negative fixtures.
- [ ] Parent #326 and production nonclaims remain open.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-long-content-large-results.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
  - bounded acceptance fixtures and browser specs selected after inventory
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
dependencies:
  - Issue #362
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T19:20:00Z
head: 8f5cf4746b22158647925a41d12bc63c30b3b489
branch: test/OTERYN-20260730-long-content-large-results
pr: 363
status: implementing
context_routes:
  - agent-governance
  - testing
  - architecture
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-long-content-large-results.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Current main remains 55ba8840a7de6556b6b173f587179f986a5a68e1.
  - The canonical manifest explicitly declares long-content on public.news-and-managed-pages.
  - The canonical manifest explicitly declares pagination on public.game-data across public highscores, character, guild, online and server routes.
  - All 18 delivered manifest surfaces now have an explicit inventory classification in PORTAL_CONTENT_SCALE_EVIDENCE.json.
  - The inventory ledger deliberately leaves executable evidence mappings empty.
  - Exact-SHA CI, Phase 7, DB outage, Game Auth and Edge Security passed on 8f5cf4746b22158647925a41d12bc63c30b3b489.
  - Agent Governance on 8f5cf4746b22158647925a41d12bc63c30b3b489 failed only because validation result PARTIAL is not a supported checkpoint value.
derived:
  - Wiki, CMS, localization, support/legal, events, announcements and downloads require bounded long-content review because they render managed localized text.
  - Public game data, Wiki indexes/search, administrator indexes, events, downloads and EditorialMedia require bounded collection review.
unknown:
  - Which candidate surfaces already contain sufficient deterministic scale fixtures and exact viewport assertions.
  - Whether inventory will expose runtime wrapping or pagination defects requiring bounded repair.
conflicts: []
first_failure:
  marker: executable evidence contract
  evidence: mapped_surfaces is intentionally empty pending direct spec inspection
rejected_hypotheses:
  - Treat broad responsive smoke evidence as proof of long localized values and large paginated datasets on every surface.
  - Treat candidate applicability classification as executable evidence.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-long-content-large-results.md
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
validation:
  - command: repository manifest inventory review
    result: PASS
    evidence: all 18 delivered manifest surfaces classified exactly once in the new inventory ledger
  - command: exact-SHA CI on 8f5cf4746b22158647925a41d12bc63c30b3b489
    result: FAIL
    evidence: CI and non-governance workflows passed; Agent Governance rejected unsupported validation result PARTIAL, now corrected to FAIL
blockers:
  - New exact-SHA workflow results are required after the checkpoint result correction.
next_action: Inspect the highest-value existing specs for public.news-and-managed-pages and public.game-data, then bind only exact markers that exercise deterministic long content or multi-page collections.
```