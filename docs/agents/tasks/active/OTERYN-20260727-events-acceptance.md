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

- [ ] Public Events proves empty, active, upcoming, archived, cancelled, detail, not-found, English and Polish states.
- [ ] Guest, authenticated without confirmed MFA, confirmed-MFA without permission, manage-only and manage-plus-publish boundaries fail closed or succeed as designed.
- [ ] Administrator create, validation, edit-to-draft, publish/status transition and browser-visible stale-conflict behavior are covered.
- [ ] Desktop, tablet and mobile views have no document-level horizontal overflow and remain keyboard/focus usable.
- [ ] Audit evidence and secret-safe diagnostics are preserved without duplicating full content in artifacts.
- [ ] The exact Events evidence marker is added to the manifest and `events.public-admin` changes to `covered` only after required exact-head checks pass.
- [ ] No production action or `PRODUCTION_PROVEN` claim is made.

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
updated_at: 2026-07-27T18:10:00Z
head: 1b385a7b6afd7708d14322deb25adfb18b14de51
branch: test/OTERYN-20260727-events-acceptance
pr: 254
status: validating
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
  - Main at task start is 17f1acd430c27c8c441b59b16b440b8763f03312.
  - PR #253 owns Downloads acceptance; Events no longer modifies its shared package or primary Playwright configuration paths.
  - Existing Events feature tests prove UTC boundaries, validation, exact permissions, audit redaction and optimistic-lock conflicts.
  - The portal ledger currently classifies events.public-admin as planned.
  - CI on a99632f7212ec54ffd863e030d391ccd2345edec passed Composer validation/audit, Pint, level-10 static analysis and PHPUnit.
  - The first portability run executed the Events public scenario successfully in Chromium, Firefox and WebKit before later homepage assertions exposed shared fixture contamination.
  - Public and administrator Events now have an isolated zero-retry browser matrix with Chromium desktop/tablet/mobile and bounded public Firefox/WebKit projects.
  - Events scenarios restore the shared homepage event fixture after every test.
derived:
  - A dedicated module workflow removes active path ownership conflict while preserving the existing full Chromium baseline and adding unique bounded Events evidence.
unknown:
  - Exact-head outcome of the isolated Events workflow and ordinary required workflows after ownership isolation.
conflicts: []
first_failure:
  marker: Acceptance E2E and Visual UX / bounded browser portability / homepage-navigation-seo
  evidence: Events public fixture deleted Acceptance tournament; restored deterministically after each Events scenario before rerun
rejected_hypotheses:
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
  - Expanding the shared primary Playwright matrix was rejected because PR #253 already owns those shared files and a module-specific gate provides a cleaner proof boundary.
changed_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
validation:
  - command: repository and PR diff inspection through GitHub connector
    result: PASS
    evidence: route/controller/request/view/model/audit boundaries and final isolated changed paths reviewed
  - command: CI on a99632f7212ec54ffd863e030d391ccd2345edec
    result: PASS
    evidence: Composer validation and audit, Pint, PHPStan level 10 and PHPUnit all succeeded
  - command: Acceptance E2E and Visual UX on a99632f7212ec54ffd863e030d391ccd2345edec
    result: FAIL
    evidence: Events public scenarios passed; three portability failures were later homepage assertions caused by non-restored shared event data
  - command: exact-head Events and ordinary workflows after ownership isolation
    result: NOT_RUN
    evidence: new workflow/configuration head has just been published
blockers:
  - none
next_action: Inspect the isolated Events workflow on the exact head and fix the first concrete browser failure before promoting the ledger record.
```

## Notes

The ledger moves to `covered` only when the browser package and ordinary route-classification validation pass. Production and live Synology verification remain separate Issue #240/#91 boundaries.
