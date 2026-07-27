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
- [x] The Downloads ledger record becomes `covered` only after stable exact evidence exists.
- [ ] Target Downloads browser execution, route classification, account lifecycle and all required repository checks pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - .github/workflows/downloads-acceptance.yml
  - .github/workflows/prepare-downloads-ledger.yml
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
updated_at: 2026-07-27T19:45:00+02:00
head: 3c9e4fe9218f5460298477e02cd5da239ceda851
branch: test/OTERYN-20260727-downloads-acceptance
pr: 253
status: implementing
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/downloads-acceptance.yml
  - .github/workflows/prepare-downloads-ledger.yml
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
proven:
  - complete zero-retry Chromium Downloads lifecycle passed on head 01ec013db342cdc355918f3cc50d3d387da3cf3e
  - bounded Firefox and WebKit public portability passed after the localized Polish heading contract was corrected
  - the Downloads table now has an accessible name on the table element itself
  - public empty, current, platform filter, fail-closed, English and Polish states are exercised
  - administrator guest, no-MFA, no-permission, validation, create, publish and immutable released-metadata boundaries are exercised
  - the canonical portal ledger records Downloads as covered with stable browser evidence markers
  - no executable upload, fetch or proxy capability was introduced
  - no Canary, login-server, OTClient or production write occurred
derived:
  - the product and browser package are complete for the declared Downloads surface contract
  - the workflow-authored ledger commit must be followed by this trusted checkpoint commit so required checks can execute without action-required approval
unknown:
  - final exact-head required workflow conclusions for the trusted checkpoint commit
conflicts: []
first_failure:
  marker: downloads-accessibility-and-portability-contract
  evidence: initial runs exposed an unnamed table, then an unselected portability spec, then an English-only assertion on the Polish route; each root cause was corrected without retries or weakened coverage
rejected_hypotheses:
  - existing feature tests alone prove the composed public and administrator browser lifecycle
  - executable upload or proxy behavior should be added to make the Download Center complete
  - the Polish route should be tested with English presentation text
changed_paths:
  - .github/workflows/downloads-acceptance.yml
  - .github/workflows/prepare-downloads-ledger.yml
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
validation:
  - command: Downloads Acceptance run 30289922496
    result: PASS
    evidence: exact-head Chromium lifecycle plus Firefox and WebKit portability succeeded with zero retries
  - command: CI and production-like required checks on 01ec013db342cdc355918f3cc50d3d387da3cf3e
    result: PASS
    evidence: CI, governance, outage, edge, game-auth, image build and Synology preflight succeeded before ledger closure
blockers:
  - none
next_action: Run every required workflow on this trusted exact head, mark PR 253 ready and squash-merge when all checks pass.
```

## Notes

This package closes only the delivered Download Center contract. It does not add executable hosting, production deployment or `PRODUCTION_PROVEN` evidence.