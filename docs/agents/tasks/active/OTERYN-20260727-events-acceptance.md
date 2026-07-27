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
  - active tasks and open PRs touching Events, scripts/acceptance, Playwright configuration or the portal coverage manifest
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
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/package.json
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
updated_at: 2026-07-27T17:54:00Z
head: 7b6ab892988958e49334883f8052f0aa7ede5799
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
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/package.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
proven:
  - Main at task start is 17f1acd430c27c8c441b59b16b440b8763f03312.
  - PR #253 owns Downloads acceptance and does not overlap Events paths.
  - Existing Events feature tests prove UTC boundaries, validation, exact permissions, audit redaction and optimistic-lock conflicts.
  - The portal ledger currently classifies events.public-admin as planned.
  - PR #254 is open as a mergeable draft and contains deterministic public/admin Events fixtures and browser scenarios.
  - Public Events coverage is included in bounded Chromium, Firefox and WebKit portability projects.
  - Public and administrator Events coverage is included in desktop, tablet and mobile Chromium projects.
  - PHP fixture syntax and JavaScript administrator-spec syntax pass local parser checks.
derived:
  - The package adds composed browser evidence without replacing stronger lower-level transaction and locking proofs.
unknown:
  - Exact-head outcomes for browser, coverage-contract and ordinary repository workflows after the governance metadata correction.
conflicts: []
first_failure:
  marker: Agent Governance / Validate active task checkpoints
  evidence: checkpoint used unsupported validation result PENDING; governance contract allows PASS, FAIL, BLOCKED or NOT_RUN
rejected_hypotheses:
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
changed_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - scripts/acceptance/package.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
validation:
  - command: repository and PR diff inspection through GitHub connector
    result: PASS
    evidence: route/controller/request/view/model/audit boundaries and all changed paths reviewed
  - command: php -l scripts/acceptance/seed-browser-events.php
    result: PASS
    evidence: no syntax errors detected under local PHP 8.4 parser
  - command: node --check scripts/acceptance/tests/events-admin-acceptance.spec.mjs
    result: PASS
    evidence: no JavaScript syntax errors under Node 22 parser
  - command: exact-head GitHub workflows after governance correction
    result: NOT_RUN
    evidence: new synchronize event is pending after this checkpoint correction
blockers:
  - none
next_action: Inspect exact-head workflow runs after the governance correction and fix the first remaining concrete failure before changing the ledger to covered.
```

## Notes

The ledger moves to `covered` only when the browser package and ordinary route-classification validation pass. Production and live Synology verification remain separate Issue #240/#91 boundaries.
