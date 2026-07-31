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

- [x] Every delivered named route has exactly one route-kind classification.
- [x] Every rendered route maps to an existing page view and exact implementation marker.
- [x] Page-like Blade views are reachable or have a bounded exclusion/retirement record.
- [x] Declared navigation entries reference existing named routes and exact source markers.
- [x] Rendered screens are navigable or have a bounded direct-entry rationale.
- [x] Unknown routes/views, duplicate ownership, broken navigation and weak exceptions fail deterministic negative fixtures.
- [x] Strict Portal Acceptance executes the validator and fixture suite.
- [x] Confirmed defects are repaired or tracked; parent Issue #326 remains open.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/coverage/validate-route-view-navigation-*.mjs
  - scripts/acceptance/coverage/test-route-view-navigation-*.mjs
  - scripts/acceptance/package.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_*.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_CLOSURE_EVIDENCE_2026-07-30.md
  - .github/workflows/portal-acceptance-contract.yml
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
updated_at: 2026-07-30T21:31:00Z
head: a00b7331f317177f9540ee765f495df411ddce8a
branch: task/OTERYN-20260730-route-view-navigation-inventory
pr: 364
status: validating
context_routes:
  - testing
  - architecture
owned_paths:
  - scripts/acceptance/coverage/validate-route-view-navigation-*.mjs
  - scripts/acceptance/coverage/test-route-view-navigation-*.mjs
  - scripts/acceptance/package.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_*.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_CLOSURE_EVIDENCE_2026-07-30.md
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-inventory.md
proven:
  - Issue #360 is bounded under parent #326 and PR #364 owns this implementation.
  - Exact-head strict workflow run 30583555606 job 91009499982 passed on 0eb6780a79e4aecfe66ce54642f1541a87f0f31b.
  - The generated inventory classified all 228 runtime named routes: 126 rendered screens, 76 form actions, 16 redirects and 10 supporting resources.
  - The inventory bound 95 of 121 Blade views, classified 26 structural views, recorded 2 bounded exclusions and reported zero orphan views.
  - The validator verified 400 navigation references and 30 bounded direct-entry routes with zero errors and zero warnings.
  - Twelve deterministic negative fixtures passed, including unknown route/view, duplicate ownership, broken navigation and weak-exception failures.
  - No production, staging, Canary, schema, payment-provider or user-data mutation occurred.
derived:
  - Repository route/view/navigation closure for Issue #360 is proven at the exact tested implementation head.
  - Parent Issue #326 and any PRODUCT_COMPLETE declaration require independent evidence and remain open.
unknown:
  - Final aggregate CI result on the documentation-only head after this checkpoint commit.
conflicts: []
first_failure:
  marker: none
  evidence: strict workflow run 30583555606 passed after resolving the final delegated support renderer binding
rejected_hypotheses:
  - Replacing the established acceptance harness with a generic parallel Playwright hierarchy would duplicate existing evidence and violate repository architecture.
  - Treating all unlinked views as dead code was rejected because POST responses, supporting endpoints and delegated renderers own valid page-like templates.
  - A broad legacy-route prefix exception was rejected in favor of individually enumerated compatibility entries.
changed_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-inventory.md
  - docs/testing/ROUTE_VIEW_NAVIGATION_CLOSURE_EVIDENCE_2026-07-30.md
  - docs/testing/ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_ENDPOINT_EXCEPTIONS.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - scripts/acceptance/coverage/test-route-view-navigation-final.mjs
  - scripts/acceptance/coverage/test-route-view-navigation-inventory.mjs
  - scripts/acceptance/coverage/validate-route-view-navigation-complete.mjs
  - scripts/acceptance/coverage/validate-route-view-navigation-contract.mjs
  - scripts/acceptance/coverage/validate-route-view-navigation-final.mjs
  - scripts/acceptance/coverage/validate-route-view-navigation-inventory.mjs
  - scripts/acceptance/coverage/validate-route-view-navigation-repository.mjs
  - scripts/acceptance/package.json
validation:
  - command: npm --prefix scripts/acceptance run test:coverage-contract:strict
    result: PASS
    evidence: Portal Acceptance Contract run 30583555606, strict job 91009499982, artifact digest sha256:c6a3b52535410927b64bd6235b6961bceb98c3d92df38e46d12579e4ba558bef
  - command: route/view/navigation negative fixtures
    result: PASS
    evidence: 12 fixtures in the same strict artifact; classified_routes=228, orphan_views=0
blockers:
  - none
next_action: Verify aggregate CI on the final documentation/checkpoint head, then mark PR #364 ready and close only Issue #360.
```

## Notes

No production, staging, Canary, schema, payment-provider or user-data mutation is authorized. Action endpoints are not forced to have standalone pages; layouts, partials, components, mail and error templates must not be misclassified as dormant pages. The durable evidence is `docs/testing/ROUTE_VIEW_NAVIGATION_CLOSURE_EVIDENCE_2026-07-30.md`; parent Issue #326 remains open.
