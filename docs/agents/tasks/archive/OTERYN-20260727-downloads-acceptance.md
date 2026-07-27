---
task_id: OTERYN-20260727-downloads-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - open pull requests and active tasks owning Downloads, localization or acceptance paths
  - existing Download feature, URL-policy, localization and administrator tests
  - current Download routes, controllers and views
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-downloads-acceptance

## Goal

Close the `downloads.public-admin-localization` ledger record through a bounded composed browser lifecycle while preserving the existing URL allowlist, immutable publication, MFA and exact-permission contracts.

## Acceptance criteria

- [x] Public Download Center proves empty, current release, platform filtering, approved metadata and unavailable/fail-closed states.
- [x] Administrator lifecycle proves guest, no-MFA, no-permission and authorized create/publish boundaries.
- [x] Browser validation rejects an unapproved executable URL and never uploads, fetches or proxies executable content.
- [x] English and Polish public behavior proves no English release-note fallback and a published Polish translation.
- [x] Desktop, tablet and mobile layouts have no horizontal page overflow; scrollable tables remain usable.
- [x] Bounded read-only Firefox and WebKit public rendering passes after the Chromium lifecycle creates deterministic data.
- [x] The public artifact table has a stable accessible name.
- [x] The Downloads ledger record becomes `covered` only after stable exact evidence exists.
- [x] Target Downloads browser execution, route classification, account lifecycle and all required repository checks pass on the exact implementation head.

## Ownership

```yaml
owned_paths:
  - .github/workflows/downloads-acceptance.yml
  - resources/views/downloads/index.blade.php
  - scripts/acceptance/package.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/downloads-public-portability.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-exhaustive-portal-acceptance.md
modules:
  - Downloads
  - Localization
  - Testing / Acceptance E2E
  - Agent governance
dependencies:
  - PR 247 merged as 4e8a11a9b76aeaaa59a5dcc38bcd8a8e2fa54b39
  - Issue 240
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T20:00:00+02:00
head: 01ec013db342cdc355918f3cc50d3d387da3cf3e
branch: test/OTERYN-20260727-downloads-acceptance
pr: 253
status: ready
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/downloads-acceptance.yml
  - resources/views/downloads/index.blade.php
  - scripts/acceptance/package.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/downloads-public-portability.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-exhaustive-portal-acceptance.md
proven:
  - complete zero-retry Chromium Downloads lifecycle passed on head 01ec013db342cdc355918f3cc50d3d387da3cf3e
  - bounded Firefox and WebKit public portability passed after the localized Polish heading contract was corrected
  - the Downloads table has an accessible name on the table element itself
  - public empty, current, platform filter, fail-closed, restored, English and Polish states are exercised
  - administrator guest, no-MFA, no-permission, validation, create, publish and immutable released-metadata boundaries are exercised
  - the canonical portal ledger records Downloads as covered with stable feature, unit and browser evidence markers
  - the PR 247 task lifecycle is archived and removed from active work
  - no executable upload, fetch or proxy capability was introduced
  - no Canary, login-server, OTClient or production write occurred
derived:
  - the product and browser package are complete for the declared Downloads surface contract
  - the next bounded Issue 240 package is Events
unknown:
  - final production CDN and approved artifact-host availability remain outside repository and staging evidence
conflicts: []
first_failure:
  marker: downloads-accessibility-and-portability-contract
  evidence: initial runs exposed an exact-permission assumption, an ambiguous navigation/download locator, an unnamed table, an unselected portability spec and an English assertion on the Polish route; each root cause was corrected without retries or weakened coverage, and run 30289922496 passed Chromium, Firefox and WebKit
rejected_hypotheses:
  - existing feature tests alone prove the composed public and administrator browser lifecycle
  - platform_admin should automatically gain every newly introduced module permission
  - executable upload or proxy behavior should be added to make the Download Center complete
  - the Polish route should be tested with English presentation text
  - cross-browser proof requires repeating secret-bearing MFA and mutations in every engine
changed_paths:
  - .github/workflows/downloads-acceptance.yml
  - resources/views/downloads/index.blade.php
  - scripts/acceptance/package.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/downloads-public-portability.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-exhaustive-portal-acceptance.md
validation:
  - command: Downloads Acceptance run 30289922496
    result: PASS
    evidence: exact-head Chromium lifecycle plus Firefox and WebKit portability succeeded with zero retries
  - command: Portal Acceptance Contract on 01ec013db342cdc355918f3cc50d3d387da3cf3e
    result: PASS
    evidence: live route classification and complete account lifecycle remained green
  - command: CI and production-like required checks on 01ec013db342cdc355918f3cc50d3d387da3cf3e
    result: PASS
    evidence: CI, governance, outage, edge, game-auth, image build and Synology preflight succeeded before ledger closure
blockers:
  - none
next_action: Mark PR 253 ready and squash-merge after this docs-only exact head receives all required checks, then begin the bounded Events package.
```

## Notes

This package closes only the delivered Download Center contract. It does not add executable hosting, production deployment or `PRODUCTION_PROVEN` evidence.
