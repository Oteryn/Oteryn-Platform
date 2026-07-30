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

- [x] Dedicated `419`, `429` and `500` views extend the existing error layout.
- [x] English and Polish title, heading and body copy is explicit and fail-safe.
- [x] Real browser behavior produces exact `404`, `419`, `429` and `500` responses without test-only routes.
- [x] Every page has noindex metadata, visible code/heading, recovery action, keyboard reachability and no document-level horizontal overflow.
- [x] The `429` flow proves the existing login limiter with bounded retry metadata and no key disclosure.
- [x] The `500` flow runs with `APP_DEBUG=false`, leaks no exception/path/SQL/database/credential data and restores the removed dependency in guaranteed cleanup.
- [x] A machine-readable evidence contract remains tied to exact executable markers, projects and retries `0`.
- [x] Focused feature, browser and repository-required checks pass on the exact runtime head.
- [x] Parent #326 remains open for other surfaces and permutations.

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
  - tests/Feature/Security/TrustedProxySchemeTest.php
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
updated_at: 2026-07-30T10:45:00Z
head: 35d3e5725c63aa108714241743e0d6f9f2622962
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
  - tests/Feature/Security/TrustedProxySchemeTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
proven:
  - Dedicated branded EN/PL 419, 429 and 500 views extend the dependency-light error layout and provide explicit non-success copy, noindex metadata and recovery actions.
  - A real cross-site browser form produces HTTP 419; same-origin form submissions are intentionally admitted by Laravel 13 request-origin verification before token comparison.
  - The global locale detector resolves only normalized en/pl values and renders an early TokenMismatchException through the branded 419 view with an explicit Content-Language header.
  - The real login form carries locale on its POST action, allowing the existing limiter to render localized 429 responses even when throttle middleware executes before route locale middleware.
  - Error State Acceptance run 30535407447 passed real 404, cross-site 419, existing login-limiter 429 and non-debug 500/restoration for EN/PL on Chromium desktop, tablet and mobile with retries fixed at zero.
  - The 429 response exposes only bounded Retry-After metadata and does not disclose the limiter email/key; its cache state is cleared before and after every sequence.
  - The 500 harness temporarily removes the exact highscore Blade dependency, proves no exception/path/SQL/database/credential disclosure, restores it in finally and proves the route returns to HTTP 200.
  - The machine-readable evidence contract binds exact statuses, triggers, assertions, locales, projects, viewports, marker, runtime, retry policy and nonclaims; the executable spec fails closed when it drifts.
  - CI, Agent Governance, Portal Acceptance Contract, Acceptance E2E and Visual UX, Error State Acceptance, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage Validation, Phase 7 Production-Like Validation and Build Synology Staging Images all passed on exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - Parent Issue #326 remains open; this slice is isolated/CI evidence and does not establish staging or production completeness.
derived:
  - A dedicated error-state acceptance profile is safer and more truthful than adding global error flows to a domain-specific Community Data profile.
unknown:
  - Final documentation-head workflow outcome and review state before merge.
conflicts: []
first_failure:
  marker: browser-and-regression-contracts
  evidence: Governance run 30533366864 rejected unsupported IN_PROGRESS; CI run 30533519881 rejected a dynamic view-string; runs 30533677029/30533912431/30534134012 exposed Laravel 13 same-origin request-forgery semantics; run 30534749960 isolated pre-route Polish 429 locale loss; CI run 30535139426 isolated the obsolete trusted-proxy login-action expectation. Every root cause was corrected on a changed head without retries or weakened assertions.
rejected_hypotheses:
  - Add test-only routes that directly return or abort with target statuses.
  - Treat status-only HTTP client assertions as proof of the rendered browser UX.
  - A missing or invalid CSRF token on a same-origin Laravel 13 browser form necessarily yields 419; origin verification admits the request first.
  - Session-only locale is sufficient for a throttle response that may execute before route locale middleware.
  - Preserve the old login action without locale and special-case only the test harness.
  - Attach global error states to an unrelated named-route surface in the portal route manifest.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-global-error-surfaces.md
  - app/Http/Middleware/DetectPublicLocaleFromPath.php
  - resources/views/identity/login.blade.php
  - resources/views/errors/419.blade.php
  - resources/views/errors/429.blade.php
  - resources/views/errors/500.blade.php
  - lang/en/errors.php
  - lang/pl/errors.php
  - tests/Feature/Errors/LocalizedErrorSurfaceTest.php
  - tests/Feature/Security/TrustedProxySchemeTest.php
  - scripts/acceptance/coverage/error-state-evidence.json
  - scripts/acceptance/playwright.error-states.config.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - .github/workflows/error-state-acceptance.yml
validation:
  - command: Error State Acceptance run 30535407447
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962; real Laravel HTTP, isolated MariaDB/Redis, EN/PL, Chromium desktop/tablet/mobile and retries 0.
  - command: CI run 30535407424
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962; formatting, static analysis and complete test suite passed.
  - command: Agent Governance run 30535407415
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - command: Portal Acceptance Contract run 30535407355
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962; strict coverage and complete account lifecycle passed.
  - command: Acceptance E2E and Visual UX run 30535407472
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - command: Edge Security Emulation run 30535407382
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - command: Game Auth Ticket Concurrency run 30535407359
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - command: Platform DB Outage Validation run 30535407383
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962.
  - command: Phase 7 Production-Like Validation run 30535407375
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962; staging-like evidence only, not production proof.
  - command: Build Synology Staging Images run 30535407357
    result: PASS
    evidence: exact runtime head 35d3e5725c63aa108714241743e0d6f9f2622962; images built only, not deployed.
blockers:
  - none
next_action: Apply the final-gate label, verify every required workflow on the documentation head, reconcile review state and squash-merge PR #354 without closing parent Issue #326.
```
