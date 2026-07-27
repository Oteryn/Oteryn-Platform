---
task_id: OTERYN-20260727-announcements-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first: []
optional_reads: []
---

# OTERYN-20260727-announcements-acceptance

## Goal

Close the complete public/admin/localization Announcements lifecycle and promote its machine-ledger record only after exact-head proof.

## Acceptance criteria

- [x] Public visibility, scheduling, escaping and EN/PL isolation browser-proven.
- [x] Exact MFA/RBAC administrator lifecycle, validation, stale translation recovery, conflict and audit browser-proven.
- [x] Chromium desktop/tablet/mobile and bounded Firefox/WebKit passed with zero retries.
- [x] Canonical ledger promoted to `covered` with durable markers.
- [x] PR #259 squash-merged; no production claim made.

## Ownership

```yaml
owned_paths:
  - .github/workflows/announcements-acceptance.yml
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
modules:
  - Announcements
  - Localization
  - Testing / Acceptance E2E
dependencies:
  - Issue #240
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T20:40:05Z
head: d08062c653a137e1359b5626fda635b170704cd8
branch: test/OTERYN-20260727-announcements-acceptance-v2
pr: 259
status: ready
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/announcements-acceptance.yml
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
proven:
  - PR #259 passed every exact-head repository workflow on a7c91bdd8bc4f9942fbba1d1c615cf7be99f07dc.
  - PR #259 squash-merged to main as d08062c653a137e1359b5626fda635b170704cd8.
  - announcements.admin-localization-home-composition is covered with zero gaps and stable browser evidence markers.
derived:
  - Announcements is complete for the declared repository acceptance boundary.
unknown:
  - Real production behavior remains unverified.
conflicts: []
first_failure:
  marker: none
  evidence: final exact-head workflow set passed
rejected_hypotheses:
  - Lower-layer tests alone were sufficient for composed Announcements closure.
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
validation:
  - command: all required PR #259 workflows on a7c91bdd8bc4f9942fbba1d1c615cf7be99f07dc
    result: PASS
    evidence: CI, governance, Portal Acceptance Contract, general E2E/Visual UX, Announcements, Downloads, Events, Phase 7, edge, outage, concurrency, image build and Synology preflight all succeeded
blockers:
  - none
next_action: Continue Issue #240 in docs/agents/tasks/active/OTERYN-20260727-portal-acceptance-final-closure.md.
```
