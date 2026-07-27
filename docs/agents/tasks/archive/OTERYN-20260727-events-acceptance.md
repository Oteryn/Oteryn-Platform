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
  - docs/agents/tasks/archive/OTERYN-20260727-events-acceptance.md
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
  - PR #253 merged as f9301792cfc82956aa4af792283c18bcbaf2c28e
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T20:55:00+02:00
head: 7e0fe46b57be4cf19900adefbada7133da47bb21
branch: main
pr: 255
status: closed
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
owned_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/tasks/archive/OTERYN-20260727-events-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
proven:
  - PR #255 merged as 7e0fe46b57be4cf19900adefbada7133da47bb21 and preserved the already-covered Downloads record.
  - Events Acceptance run 30295021986 passed the zero-retry Chromium desktop/tablet/mobile and public Firefox/WebKit matrix on exact PR head c4246b857bcf540a4a7338a1d34a6c6e8c10c199.
  - Portal Acceptance Contract run 30295021947 passed live route classification, evidence-file validation, evidence-marker validation and complete account lifecycle.
  - CI run 30295022169 passed Composer validation/audit, Pint, PHPStan level 10 and PHPUnit.
  - Acceptance E2E and Visual UX run 30295023083 passed required smoke, portability, responsive, resilience and accessibility profiles.
  - Phase 7, Platform DB outage, Edge Security, Game Auth concurrency, Downloads Acceptance, Synology preflight and Agent Governance also passed on the exact PR head.
  - The manifest classifies `events.public-admin` as `covered` while retaining `downloads.public-admin-localization` as `covered`.
  - Public scenarios prove empty, active, upcoming, archived, cancelled, detail, not-found and locale-isolated English/Polish behavior.
  - Administrator scenarios prove guest, no-MFA, no-permission, manage-only and manage-plus-publish boundaries, validation, draft, publish, edit-to-draft, visible 409 conflict and audit redaction.
  - Events scenarios restore the shared homepage fixture after every test, preventing cross-spec contamination.
derived:
  - The dedicated module workflow provides composed browser evidence without replacing stronger lower-level transaction and locking proofs.
unknown:
  - Production behavior remains UNKNOWN until the separate production verification boundary is executed.
conflicts: []
first_failure:
  marker: Acceptance E2E and Visual UX / bounded browser portability / homepage-navigation-seo
  evidence: the initial Events fixture removed Acceptance tournament; deterministic shared-fixture restoration fixed the root cause and exact-head reruns passed
rejected_hypotheses:
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
  - Forcing conflicted PR #254 or duplicate PR #256 was rejected after conflict-free PR #255 merged the same proven package.
changed_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/archive/OTERYN-20260727-events-acceptance.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
validation:
  - command: CI run 30295022169 on c4246b857bcf540a4a7338a1d34a6c6e8c10c199
    result: PASS
    evidence: Composer validation/audit, formatting, PHPStan level 10 and PHPUnit succeeded
  - command: Events Acceptance run 30295021986 on c4246b857bcf540a4a7338a1d34a6c6e8c10c199
    result: PASS
    evidence: complete zero-retry Events matrix succeeded on five browser/viewport projects
  - command: Portal Acceptance Contract run 30295021947 on c4246b857bcf540a4a7338a1d34a6c6e8c10c199
    result: PASS
    evidence: coverage classification and complete account lifecycle succeeded
  - command: Acceptance E2E and Visual UX run 30295023083 on c4246b857bcf540a4a7338a1d34a6c6e8c10c199
    result: PASS
    evidence: required critical browser profiles succeeded
  - command: focused system workflows on c4246b857bcf540a4a7338a1d34a6c6e8c10c199
    result: PASS
    evidence: Phase 7, DB outage, edge, game-auth concurrency, Downloads and Synology checks succeeded
blockers:
  - none
next_action: Start a bounded Announcements acceptance task under Issue #240.
```

## Notes

The Events result is repository/controlled-runtime evidence only. Production and live go-live verification remain separate Issue #240/#91 boundaries.
