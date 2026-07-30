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
  - Issue #353, parent #326 and open PRs touching error views, locale detection or acceptance workflow paths
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
  - app/Http/Middleware/DetectPublicLocaleFromPath.php
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/errors.php
  - lang/pl/errors.php
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
updated_at: 2026-07-30T10:01:00Z
head: 1516a2aa77fb621787268a39d0f6a863a70e921e
branch: feat/OTERYN-20260730-global-error-surfaces
pr: 354
status: validating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - identity
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-global-error-surfaces.md
  - app/Http/Middleware/DetectPublicLocaleFromPath.php
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/errors.php
  - lang/pl/errors.php
  - tests/Feature/Errors/LocalizedErrorSurfaceTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
proven:
  - Main has dedicated branded localized 404 and 503 views extending resources/views/errors/layout.blade.php.
  - Main had no dedicated 419, 429 or 500 view before this task, so those statuses used framework fallback rendering.
  - The existing smoke suite proved only the numeric 419 response through page.request and did not prove a browser-rendered localized error surface.
  - Existing identity login rate limiting provides a real bounded 429 path without introducing a test-only endpoint.
  - The global locale detector now accepts only normalized en/pl query values before route middleware, allowing pre-route CSRF errors to preserve an explicitly requested locale while invalid values fail closed.
  - The dedicated acceptance profile triggers real unmatched-route 404, missing-CSRF 419, login-limiter 429 and non-debug missing-view 500 behavior for EN/PL at desktop, tablet and mobile with retries fixed at zero.
  - The 500 harness restores the exact Blade source and clears compiled views in a guaranteed finally path; limiter cache is cleared before and after every bounded 429 sequence.
  - Active PRs #348/#349 own viewport/browser dimension-ledger paths, not the paths claimed here.
derived:
  - A dedicated error-state acceptance profile is safer and more truthful than adding global error flows to a domain-specific Community Data profile.
unknown:
  - First exact-head workflow outcome for the new Error State Acceptance profile.
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
  - app/Http/Middleware/DetectPublicLocaleFromPath.php
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/errors.php
  - lang/pl/errors.php
  - tests/Feature/Errors/LocalizedErrorSurfaceTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
validation:
  - command: Error State Acceptance and repository workflows
    result: IN_PROGRESS
    evidence: first exact-head runs started from PR #354.
blockers:
  - none
next_action: Inspect every failing exact-head workflow step, fix the root cause and repeat validation only on a changed head.
```
