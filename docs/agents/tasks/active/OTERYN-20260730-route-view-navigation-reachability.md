---
task_id: OTERYN-20260730-route-view-navigation-reachability
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/coverage/validate-portal-coverage.mjs
search_first:
  - Issue #360, parent #326 and open PRs touching route coverage, Blade page inventory or navigation validation
  - route files, controllers, layouts and browser specs that establish rendered-screen reachability
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
---

# OTERYN-20260730-route-view-navigation-reachability

## Goal

Deliver Issue #360 as a bounded fail-closed route/view/navigation inventory for every delivered portal surface, repairing or explicitly tracking confirmed unreachable, broken-navigation or dormant-page defects without closing parent #326.

## Acceptance criteria

- [ ] Every delivered named route has exactly one route-kind classification.
- [ ] Every rendered route maps to an existing page view and exact implementation marker.
- [ ] Every page-like Blade view is reachable or has a bounded exclusion/retirement record.
- [ ] Every declared navigation entry references an existing named route and exact source marker.
- [ ] Every rendered screen is globally/contextually reachable or has a bounded direct-entry rationale.
- [ ] Unknown routes/views, duplicate ownership, broken navigation and weak exceptions fail deterministically.
- [ ] Strict Portal Acceptance executes the validator and negative fixtures.
- [ ] Confirmed defects are repaired or tracked without closing parent #326.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-reachability.md
  - docs/testing/PORTAL_ROUTE_VIEW_NAVIGATION_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-route-view-navigation.mjs
  - scripts/acceptance/coverage/test-route-view-navigation.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
  - WebPortal
  - Admin
  - Identity
dependencies:
  - Issue #360
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T16:30:00Z
head: 8c20e2abfb84b557a95c75fb139be264d7ed0e4d
branch: test/OTERYN-20260730-route-view-navigation-reachability
pr: 361
status: auditing
context_routes:
  - agent-governance
  - testing
  - architecture
  - web-portal
  - admin
  - identity
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-reachability.md
  - docs/testing/PORTAL_ROUTE_VIEW_NAVIGATION_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-route-view-navigation.mjs
  - scripts/acceptance/coverage/test-route-view-navigation.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Strict portal coverage classifies every delivered named route and verifies stable browser evidence markers.
  - The current manifest does not declare route kind, exact rendered page view or navigation reachability.
  - The current validator does not enumerate page-like Blade files or prove that declared navigation route names exist.
  - A source-derived inventory now enumerates the exact Laravel route table, literal production route/view references and page-like Blade candidates.
  - The initial gate fails only on literal named-route or Blade-view references that do not exist; dormant-page candidates remain report-only until explicitly classified.
  - Main at task start is 55ba8840a7de6556b6b173f587179f986a5a68e1.
derived:
  - Route existence and browser evidence are insufficient to prove that a rendered page is reachable or that no page-like template is dormant.
unknown:
  - Exact inventory output and whether any current literal navigation/view reference is broken.
  - Which unreferenced page-like candidates are true dormant pages versus bounded framework or convention-driven templates.
conflicts: []
first_failure:
  marker: route coverage lacks route-kind, page-view and navigation-entry fields
  evidence: scripts/acceptance/coverage/portal-coverage-manifest.json and validate-portal-coverage.mjs
rejected_hypotheses:
  - Infer reachability from a browser marker alone.
  - Require action endpoints, layouts, partials, components, mail or error templates to have standalone navigation.
  - Fail immediately on every unreferenced Blade candidate before classifying framework/convention-driven templates.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-route-view-navigation-reachability.md
  - scripts/acceptance/coverage/validate-route-view-navigation.mjs
  - scripts/acceptance/package.json
validation:
  - command: GitHub Actions on initial inventory checkpoint
    result: NOT_RUN
    evidence: exact workflow set will run on the updated draft PR head
blockers:
  - none
next_action: Inspect the exact inventory report, isolate the first true mismatch and then introduce the strict route-kind/page-view/navigation contract plus negative fixtures.
```