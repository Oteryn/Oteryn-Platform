---
task_id: OTERYN-20260730-global-error-surfaces
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
  - resources/views/errors/layout.blade.php
  - resources/views/errors/404.blade.php
  - resources/views/errors/503.blade.php
search_first:
  - Issue #353, parent #326 and open PRs touching error views, public translations or acceptance workflow paths
  - existing CSRF, rate-limiter and non-debug internal-error behavior
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/tests/smoke.spec.mjs
---

# OTERYN-20260730-global-error-surfaces

## Goal

Deliver Issue #353 as one bounded application-error UX slice: dedicated localized `419`, `429` and `500` views plus real zero-retry browser evidence for `404`, `419`, `429` and `500` on Chromium desktop, tablet and mobile.

## Acceptance criteria

- [ ] Dedicated `419`, `429` and `500` views extend the existing error layout.
- [ ] English and Polish title, heading and body copy is explicit and fail-safe.
- [ ] Real browser behavior produces exact `404`, `419`, `429` and `500` responses without test-only routes.
- [ ] Every page has noindex metadata, visible code/heading, recovery action, keyboard reachability and no document-level horizontal overflow.
- [ ] The `429` flow proves the existing login limiter with bounded retry metadata and no key disclosure.
- [ ] The `500` flow runs with `APP_DEBUG=false`, leaks no exception/path/SQL/database/credential data and restores the removed dependency in guaranteed cleanup.
- [ ] A machine-readable evidence contract remains tied to exact executable markers, projects and retries `0`.
- [ ] Focused feature, browser and repository-required checks pass on the exact final head.
- [ ] Parent #326 remains open for other surfaces and permutations.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-global-error-surfaces.md
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/public.php
  - lang/pl/public.php
  - tests/Feature/Errors/LocalizedErrorSurfaceTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
modules:
  - PublicPortal
  - Identity
  - Testing
  - AgentGovernance
dependencies:
  - Issue #353
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T09:53:00Z
head: eda893990dccca6ffe65549e224f908299d90750
branch: feat/OTERYN-20260730-global-error-surfaces
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
  - web-cms
  - identity
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-global-error-surfaces.md
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/public.php
  - lang/pl/public.php
  - tests/Feature/Errors/LocalizedErrorSurfaceTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
proven:
  - Main has dedicated branded localized 404 and 503 views extending resources/views/errors/layout.blade.php.
  - Main has no resources/views/errors/419.blade.php, 429.blade.php or 500.blade.php, so those statuses use framework fallback rendering.
  - The existing smoke suite proves only the numeric 419 response through page.request and does not prove a browser-rendered localized error surface.
  - Existing identity login rate limiting provides a real bounded 429 path without introducing a test-only endpoint.
  - Temporarily removing an exact Blade dependency after clearing compiled views is already proven to generate a genuine non-debug 500 in the isolated acceptance runtime.
  - Active PRs #348/#349 own viewport/browser dimension-ledger paths, not the paths claimed here.
derived:
  - A dedicated error-state acceptance profile is safer and more truthful than adding global error flows to a domain-specific Community Data profile.
unknown:
  - Whether locale session persistence is sufficient for Polish 419/429/500 rendering without exception-level locale routing changes.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Add test-only routes that directly return or abort with target statuses.
  - Treat status-only HTTP client assertions as proof of the rendered browser UX.
  - Attach global error states to an unrelated named-route surface in the portal route manifest.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-global-error-surfaces.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet committed
blockers:
  - none
next_action: Open a draft PR, implement localized error views and a dedicated real-browser error-state acceptance profile.
```
