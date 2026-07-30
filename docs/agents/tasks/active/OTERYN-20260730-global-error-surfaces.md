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
  - resources/views/identity/login.blade.php
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
updated_at: 2026-07-30T10:31:00Z
head: 12303ef044582a58054293c25354d6fadaa5d250
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
  - resources/views/identity/login.blade.php
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
  - A real cross-site browser form now produces HTTP 419; same-origin form submissions are intentionally admitted by Laravel 13 request-origin verification before token comparison.
  - The global locale detector resolves only normalized en/pl values and renders an early TokenMismatchException through the branded 419 view with an explicit Content-Language header.
  - Existing identity login rate limiting provides a real bounded 429 path without introducing a test-only endpoint.
  - The login limiter can execute before route locale middleware, so the login POST action must carry the normalized locale explicitly for a localized 429.
  - The 500 harness restores the exact Blade source and clears compiled views in a guaranteed finally path; limiter cache is cleared before and after every bounded 429 sequence.
  - Active PRs #348/#349 own viewport/browser dimension-ledger paths, not the paths claimed here.
derived:
  - A dedicated error-state acceptance profile is safer and more truthful than adding global error flows to a domain-specific Community Data profile.
unknown:
  - Exact-head result after the login form preserves locale on its POST action.
conflicts: []
first_failure:
  marker: browser-error-state-matrix
  evidence: Runs 30533677029, 30533912431 and 30534134012 exposed Laravel 13 same-origin request-forgery behavior; run 30534749960 then proved 419 passed and isolated the remaining Polish 429 Content-Language mismatch.
rejected_hypotheses:
  - Add test-only routes that directly return or abort with target statuses.
  - Treat status-only HTTP client assertions as proof of the rendered browser UX.
  - A missing or invalid CSRF token on a same-origin Laravel 13 browser form necessarily yields 419; origin verification admits the request first.
  - Session-only locale is sufficient for a throttle response that may execute before route locale middleware.
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
    result: FAIL
    evidence: exact head 12303ef044582a58054293c25354d6fadaa5d250 passed real localized 419 and failed only the Polish 429 Content-Language assertion before the login form POST carried locale.
blockers:
  - none
next_action: Preserve the normalized locale in the real login form POST action, cover it in focused tests and rerun exact-head validation.
```
