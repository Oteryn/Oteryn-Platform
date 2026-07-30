---
task_id: OTERYN-20260730-route-view-navigation-inventory
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
search_first:
  - scripts/acceptance/coverage
  - routes
  - resources/views
  - scripts/acceptance/tests
optional_reads:
  - docs/architecture/adr/0015-portal-delivered-surface-acceptance-ledger.md
---

# OTERYN-20260730-route-view-navigation-inventory

## Goal

Close Issue #360 with a bounded fail-closed inventory that binds delivered named routes to route kinds, rendered Blade page views and navigation/direct-entry evidence without closing unrelated parent Issue #326 gaps.

## Acceptance criteria

- [ ] Every delivered named route has exactly one route-kind classification.
- [ ] Every rendered route maps to an existing page view and exact implementation marker.
- [ ] Page-like Blade views are reachable or have a bounded exclusion/retirement record.
- [ ] Declared navigation entries reference existing named routes and exact source markers.
- [ ] Rendered screens are navigable or have a bounded direct-entry rationale.
- [ ] Unknown routes/views, duplicate ownership, broken navigation and weak exceptions fail deterministic negative fixtures.
- [ ] Strict Portal Acceptance executes the validator and fixture suite.
- [ ] Confirmed defects are repaired or tracked; parent Issue #326 remains open.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/coverage/route-view-navigation-inventory.json
  - scripts/acceptance/coverage/validate-route-view-navigation-inventory.mjs
  - scripts/acceptance/coverage/test-route-view-navigation-inventory.mjs
  - scripts/acceptance/package.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-inventory.md
modules:
  - acceptance-coverage
  - frontend-evidence
dependencies:
  - issue-360
  - issue-326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T20:55:00Z
head: 55ba8840a7de6556b6b173f587179f986a5a68e1
branch: task/OTERYN-20260730-route-view-navigation-inventory
pr: none
status: investigating
context_routes:
  - testing
  - architecture
owned_paths:
  - scripts/acceptance/coverage/route-view-navigation-inventory.json
  - scripts/acceptance/coverage/validate-route-view-navigation-inventory.mjs
  - scripts/acceptance/coverage/test-route-view-navigation-inventory.mjs
  - scripts/acceptance/package.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-inventory.md
proven:
  - Issue #360 is open under parent #326 and no open PR owns it.
  - The current strict acceptance profile already validates route coverage, product completeness, dimension evidence and media-state evidence.
  - Main head at task creation is 55ba8840a7de6556b6b173f587179f986a5a68e1.
derived:
  - The new contract should extend the existing acceptance coverage architecture instead of replacing it with a parallel E2E tree.
unknown:
  - Exact rendered-route, page-view and navigation-entry inventory and any confirmed reachability defects.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Replacing the established acceptance harness with a generic parallel Playwright hierarchy would duplicate existing evidence and violate repository architecture.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-inventory.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: discovery phase
blockers:
  - none
next_action: Inspect current route declarations, rendered views, navigation sources and existing validator conventions.
```

## Notes

No production, staging, Canary, schema, payment-provider or user-data mutation is authorized. Action endpoints are not forced to have standalone pages; layouts, partials, components, mail and error templates must not be misclassified as dormant pages.
