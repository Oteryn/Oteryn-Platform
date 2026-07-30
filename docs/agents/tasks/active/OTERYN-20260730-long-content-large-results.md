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

- [ ] Every delivered rendered surface has an explicit applicability classification.
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
updated_at: 2026-07-30T17:08:00Z
head: 55ba8840a7de6556b6b173f587179f986a5a68e1
branch: test/OTERYN-20260730-long-content-large-results
pr: none
status: investigating
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
  - Parent Issue #326 still requires explicit long-value, large bounded pagination and horizontal-overflow evidence.
  - No separate open issue or PR was found for this bounded slice before Issue #362 was created.
  - Current main at task start is 55ba8840a7de6556b6b173f587179f986a5a68e1.
derived:
  - Applicability must be inventoried before runtime or fixture changes to avoid forcing synthetic scale states onto surfaces that cannot consume them.
unknown:
  - Exact rendered consumer set for long content and large collections.
  - Whether inventory will expose runtime wrapping or pagination defects requiring bounded repair.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treat broad responsive smoke evidence as proof of long localized values and large paginated datasets on every surface.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-long-content-large-results.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: inventory and implementation not started
blockers:
  - none
next_action: Inventory actual rendered long-content and paginated collection consumers, then define the smallest truthful evidence schema and fixture set.
```
