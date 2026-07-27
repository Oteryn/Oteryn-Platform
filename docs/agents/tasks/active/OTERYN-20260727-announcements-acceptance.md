---
task_id: OTERYN-20260727-announcements-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - open pull requests and active tasks owning Announcements, localization, homepage or acceptance paths
  - existing Announcement feature, translation, authorization and homepage tests
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-announcements-acceptance

## Goal

Close `announcements.admin-localization-home-composition` with deterministic browser evidence for public visibility windows, localization, exact MFA/RBAC administration, optimistic locking, audit redaction and responsive presentation.

## Acceptance criteria

- [x] Public homepage proves none-active, active, expired/future/draft hidden, escaped plain text and EN/PL locale-isolated states.
- [x] Guest, no-MFA, no-permission and exact `portal.announcements.manage` boundaries are browser-proven.
- [x] Validation rejects unsafe links; create, publish, source edit, stale translation recovery and 409 conflict are browser-proven.
- [x] Audit evidence is visible to an authorized operator and does not contain announcement body or credentials.
- [x] Chromium desktop/tablet/mobile and bounded public Firefox/WebKit execute with zero retries.
- [ ] The canonical ledger changes to `covered` only after exact-head browser and repository checks pass.
- [x] No production action or `PRODUCTION_PROVEN` claim is made.

## Ownership

```yaml
owned_paths:
  - .github/workflows/announcements-acceptance.yml
  - public/css/admin-translations.css
  - resources/views/admin/translations/form.blade.php
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/seed-account-overview-state.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
modules:
  - Announcements
  - Localization
  - Accounts acceptance harness
  - Testing / Acceptance E2E
  - Agent governance
dependencies:
  - PR #255 merged as 7e0fe46b57be4cf19900adefbada7133da47bb21
  - PR #257 archived Events on main as 05d08714a0b87ee8a453d01bded605ff42de8bbc
  - Issue #240
blockers:
  - exact-head account-lifecycle rerun after fixture command-contract repair
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T21:56:00+02:00
head: 067457a14caffef6068f22f185d75bd6ff6868c0
branch: test/OTERYN-20260727-announcements-acceptance-v2
pr: 259
status: validating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/announcements-acceptance.yml
  - public/css/admin-translations.css
  - resources/views/admin/translations/form.blade.php
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/seed-account-overview-state.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
proven:
  - lower-layer feature tests prove visibility boundaries, escaping, exact permission, MFA denial, unsafe-link rejection, optimistic locking and audit redaction
  - Events and Downloads are covered on current main
  - PR #257 already archived Events with exact final evidence
  - the clean successor archives the still-active completed Downloads task without replacing the Events archive
  - Announcements Acceptance run 30299581452 passed the complete zero-retry matrix on exact SHA 6d2a684780ffef2b6a8d0a7c435d00c1eacd895a
  - the Announcements matrix passed Chromium desktop, tablet and mobile plus public Firefox and WebKit
  - the shared translation form now proves an unobstructed mobile pointer target and keyboard activation without force or JavaScript submission
derived:
  - the Announcements product and composed browser contract are complete for the declared repository boundary
  - the remaining gate is the repaired pre-existing Account Overview acceptance fixture and final ledger promotion
unknown:
  - exact-head account-lifecycle result after the fixture command contract repair
  - final docs-inclusive workflow set after ledger promotion
conflicts: []
first_failure:
  marker: account-overview-fixture-command-contract
  evidence: final Account lifecycle run 30299581391 showed the requested missing state rendering Ready; the test invoked seed/binding commands while the fixture parsed email/password/state, so it mutated a different identity. The fixture now uses explicit commands without resetting the active browser identity password or session generation.
rejected_hypotheses:
  - lower-layer tests alone prove the composed public and administrator browser lifecycle
  - the mobile translation failure should be bypassed with force, JavaScript submit or weakened viewport coverage
  - the Account Overview failure was caused by Announcements product behavior or shared CSS
  - retrying the mismatched account fixture without fixing its argument contract would provide trustworthy evidence
changed_paths:
  - .github/workflows/announcements-acceptance.yml
  - public/css/admin-translations.css
  - resources/views/admin/translations/form.blade.php
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/seed-account-overview-state.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
validation:
  - command: Announcements Acceptance run 30299581452 on 6d2a684780ffef2b6a8d0a7c435d00c1eacd895a
    result: PASS
    evidence: complete zero-retry public and administrator matrix passed on five browser/viewport projects
  - command: Portal Acceptance Contract run 30299581391 on 6d2a684780ffef2b6a8d0a7c435d00c1eacd895a
    result: FAIL
    evidence: route classification passed; account lifecycle exposed the pre-existing fixture command mismatch and was repaired rather than retried unchanged
blockers:
  - exact-head account-lifecycle rerun after fixture repair
next_action: Require the repaired exact-head account lifecycle and repository workflows, then promote Announcements to covered and execute the final docs-inclusive merge gate.
```

## Notes

This package closes only the delivered Announcements contract. It does not deploy to production or change the separately tracked homepage-template selector.
