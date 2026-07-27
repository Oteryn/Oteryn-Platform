---
task_id: OTERYN-20260727-events-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - active tasks and open PRs touching Events, scripts/acceptance or the portal coverage manifest
  - current Events routes, controllers, views, validation, lifecycle actions and feature tests
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-events-acceptance

## Goal

Close the `events.public-admin` record from the machine-enforced portal acceptance ledger through one bounded, deterministic and zero-retry browser package without duplicating lower-level transaction proofs.

## Acceptance criteria

- [x] Public Events proves empty, active, upcoming, archived, cancelled, detail, not-found, English and Polish states.
- [x] Guest, authenticated without confirmed MFA, confirmed-MFA without permission, manage-only and manage-plus-publish boundaries fail closed or succeed as designed.
- [x] Administrator create, validation, edit-to-draft, publish/status transition and browser-visible stale-conflict behavior are covered.
- [x] Desktop, tablet and mobile views have no document-level horizontal overflow and remain keyboard/focus usable.
- [x] Audit evidence and secret-safe diagnostics are preserved without duplicating full content in artifacts.
- [x] The exact Events evidence markers are present and `events.public-admin` is `covered` after exact-head validation.
- [x] No production action or `PRODUCTION_PROVEN` claim is made.

## Ownership

```yaml
owned_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
modules:
  - Events
  - Testing / Acceptance E2E
dependencies:
  - Issue #240
  - ADR 0015
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T18:25:00Z
head: 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
branch: test/OTERYN-20260727-events-acceptance
pr: 254
status: ready_for_review
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
owned_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
proven:
  - Main at task start was 17f1acd430c27c8c441b59b16b440b8763f03312.
  - PR #253 owns Downloads acceptance; Events does not modify its shared package or primary Playwright configuration paths.
  - Existing Events feature tests remain authoritative for UTC boundaries, validation, exact permissions, audit redaction and optimistic-lock correctness.
  - Dedicated Events Acceptance run 30293402575 passed on exact runtime/browser head 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e with zero retries.
  - The Events matrix covers Chromium desktop, tablet and mobile plus public Firefox and WebKit.
  - Public scenarios prove empty, active, upcoming, archived, cancelled, detail, not-found and locale-isolated English/Polish behavior.
  - Administrator scenarios prove guest, no-MFA, no-permission, manage-only and manage-plus-publish boundaries, validation, draft, publish, edit-to-draft, visible 409 conflict and audit redaction.
  - Events scenarios restore the shared homepage fixture after every test, preventing cross-spec contamination.
  - Portal Acceptance Contract run 30293402858 passed live route classification, evidence-file and marker validation on the exact runtime/browser head.
  - CI run 30293402905 passed Composer validation/audit, Pint, PHPStan level 10 and PHPUnit on the exact runtime/browser head.
  - Acceptance E2E and Visual UX run 30293402551 passed smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, resilience and keyboard accessibility profiles.
  - Phase 7, Platform DB outage, Edge Security, Game Auth concurrency, Synology preflight and Agent Governance also passed on the same exact runtime/browser head.
  - The manifest classifies events.public-admin as covered and the human-readable matrix records no remaining Events gap.
derived:
  - The dedicated module workflow provides the required composed browser evidence without replacing stronger lower-level transaction and locking proofs.
unknown:
  - Production behavior remains UNKNOWN until the separate production verification boundary is executed.
conflicts: []
first_failure:
  marker: Acceptance E2E and Visual UX / bounded browser portability / homepage-navigation-seo
  evidence: the initial Events fixture removed Acceptance tournament; deterministic shared-fixture restoration fixed the root cause and the exact-head rerun passed
rejected_hypotheses:
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
  - Expanding the shared primary Playwright matrix was rejected because PR #253 owned overlapping shared files and a module-specific gate provides a cleaner proof boundary.
changed_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
validation:
  - command: CI run 30293402905 on 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
    result: PASS
    evidence: Composer validation/audit, formatting, PHPStan level 10 and PHPUnit succeeded
  - command: Events Acceptance run 30293402575 on 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
    result: PASS
    evidence: complete zero-retry Events matrix succeeded on five browser/viewport projects
  - command: Portal Acceptance Contract run 30293402858 on 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
    result: PASS
    evidence: coverage classification and complete account lifecycle both succeeded
  - command: Acceptance E2E and Visual UX run 30293402551 on 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
    result: PASS
    evidence: required critical browser profiles succeeded
  - command: Phase 7 and focused system workflows on 3a51bc945f13fd7c9b3fe8e37ea5d644fb2f312e
    result: PASS
    evidence: production-like, DB outage, edge, game-auth concurrency and Synology preflight checks succeeded
blockers:
  - none
next_action: Mark PR #254 ready for review and squash-merge it after the docs-only checkpoint check passes.
```

## Notes

The Events result is repository/controlled-runtime evidence only. Production and live go-live verification remain separate Issue #240/#91 boundaries.
