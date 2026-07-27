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
- [x] The canonical ledger changes to `covered` only after exact-head browser and repository checks pass.
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
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
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
  - final exact-head workflow set after successful and rejected MFA paths were separated
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T22:30:00+02:00
head: 49fbe462f3bc57eb3f4bb4cd8499ac7e9a0f1a1a
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
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
proven:
  - Announcements Acceptance run 30300795238 passed the complete zero-retry matrix on exact implementation SHA 53a54a45d67a71d0cd321b830ddfbb15e2dd8ceb.
  - The Announcements matrix passed Chromium desktop, tablet and mobile plus bounded public Firefox and WebKit.
  - Public evidence proves none-active, active-window, expired, future-hidden, draft-hidden, escaped plain text and locale-isolated English and Polish states.
  - Administrator evidence proves guest, no-MFA, no-permission, exact permission, unsafe-link validation, create, publish, Polish translation, stale recovery, optimistic-lock conflict and redacted audit visibility.
  - Portal Acceptance Contract run 30300795281 passed route classification and the complete zero-retry account lifecycle on exact implementation SHA 53a54a45d67a71d0cd321b830ddfbb15e2dd8ceb.
  - Acceptance E2E and Visual UX run 30300795276 passed primary smoke, portability, responsive, resilience and keyboard accessibility profiles on exact implementation SHA 53a54a45d67a71d0cd321b830ddfbb15e2dd8ceb.
  - The shared translation form has an unobstructed mobile pointer target, bounded textarea geometry and keyboard-operable external form submitter without force or JavaScript submission.
  - The Account Overview fixture preserves the legacy isolated-test interface while its command interface mutates only a browser-created identity and never resets the active session password or generation.
  - The canonical manifest classifies announcements.admin-localization-home-composition as covered with exact public and administrator evidence markers and no gaps.
  - Events and Downloads remain covered and their completed task records are preserved or archived without overwriting prior evidence.
derived:
  - The Announcements product and composed browser contract are complete for the declared repository boundary.
  - The single WebKit failure occurred after successful MFA and before the next explicit navigation; successful challenges now wait for the challenge document to disappear and destination DOM readiness.
  - Rejected and replayed MFA attempts intentionally remain on the challenge page, so the shared helper exposes an explicit expectSuccess boundary rather than weakening either successful-navigation or negative-security proof.
  - Only the final exact-head workflow set and merge administration remain before Support and Legal begins.
unknown:
  - final exact-head workflow conclusions after successful and rejected MFA helper paths were separated
conflicts: []
first_failure:
  marker: announcements-mobile-translation-account-fixture-and-mfa-navigation-contract
  evidence: initial mobile runs exposed a real translation-control overlap and Playwright pointer-scroll ambiguity; the form was restructured with bounded mobile geometry and a separate external submitter. Account runs then exposed a numeric substring collision and mismatched seed arguments; exact label assertions and dual safe/legacy fixture interfaces corrected both root causes. Final bounded WebKit portability exposed an internal page navigation error immediately after successful MFA, while the first stabilized helper incorrectly waited for navigation after intentional invalid/replayed MFA attempts. The helper now waits only when expectSuccess is true, and the negative security test explicitly selects false.
rejected_hypotheses:
  - lower-layer tests alone prove the composed public and administrator browser lifecycle
  - the mobile translation failure should be bypassed with force, JavaScript submit or weakened viewport coverage
  - the Account Overview failure was caused by Announcements product behavior or shared CSS
  - removing the legacy Account Overview fixture interface would preserve existing critical security and E2E profiles
  - the transient CoreDNS registry timeout is an application regression that should be worked around in product code
  - retrying the WebKit test without stabilizing completion of successful MFA navigation would provide trustworthy portability evidence
  - treating intentional invalid MFA as a successful navigation would preserve the negative security contract
changed_paths:
  - .github/workflows/announcements-acceptance.yml
  - public/css/admin-translations.css
  - resources/views/admin/translations/form.blade.php
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/seed-account-overview-state.php
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
validation:
  - command: Announcements Acceptance run 30300795238 on 53a54a45d67a71d0cd321b830ddfbb15e2dd8ceb
    result: PASS
    evidence: complete zero-retry public and administrator matrix succeeded on five browser and viewport projects
  - command: Portal Acceptance Contract run 30300795281 on 53a54a45d67a71d0cd321b830ddfbb15e2dd8ceb
    result: PASS
    evidence: live route and evidence classification plus the complete account lifecycle succeeded
  - command: Acceptance E2E and Visual UX run 30301701789 on 4398a3a35af1a79cf3e01045344041392faa1924
    result: FAIL
    evidence: one WebKit internal navigation error occurred immediately after completed MFA; all other WebKit, Chromium and Firefox tests in the profile passed, and successful challenge navigation was stabilized without adding retries
  - command: Portal Acceptance Contract run 30302262644 on f213eae626edd73f9fec501c5fce92a609e9dcea
    result: FAIL
    evidence: route classification passed; the negative MFA scenario correctly remained on the challenge page and exposed that navigation waiting must be conditional
  - command: final exact-head repository workflows after successful and rejected MFA paths were separated
    result: NOT_RUN
    evidence: triggered by the helper, negative-security test and checkpoint commits and required before marking PR #259 ready
blockers:
  - final exact-head workflow set after successful and rejected MFA paths were separated
next_action: Require every repository workflow on the final PR #259 head, then mark ready and squash-merge before starting Support and Legal.
```

## Notes

This package closes only the delivered Announcements contract. It does not deploy to production or change the separately tracked homepage-template selector.
