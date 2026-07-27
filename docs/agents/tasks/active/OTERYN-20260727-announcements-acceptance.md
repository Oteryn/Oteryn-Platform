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

- [ ] Public homepage proves none-active, active, expired/future/draft hidden, escaped plain text and EN/PL locale-isolated states.
- [ ] Guest, no-MFA, no-permission and exact `portal.announcements.manage` boundaries are browser-proven.
- [ ] Validation rejects unsafe links; create, publish, source edit, stale translation recovery and 409 conflict are browser-proven.
- [ ] Audit evidence is visible to an authorized operator and does not contain announcement body or credentials.
- [ ] Chromium desktop/tablet/mobile and bounded public Firefox/WebKit execute with zero retries.
- [ ] The canonical ledger changes to `covered` only after exact-head browser and repository checks pass.
- [ ] No production action or `PRODUCTION_PROVEN` claim is made.

## Ownership

```yaml
owned_paths:
  - .github/workflows/announcements-acceptance.yml
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
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
  - Testing / Acceptance E2E
  - Agent governance
dependencies:
  - PR #255 merged as 7e0fe46b57be4cf19900adefbada7133da47bb21
  - PR #257 archived Events on main as 05d08714a0b87ee8a453d01bded605ff42de8bbc
  - Issue #240
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T21:30:00+02:00
head: 9ffa3c0ee1d324b8e28ec02ab775d61b3d797480
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
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
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
derived:
  - the remaining closure risk is composed browser behavior rather than an identified persistence or authorization defect
unknown:
  - exact-head browser result for the composed Announcements matrix
conflicts: []
first_failure:
  marker: checkpoint-required-derived-field
  evidence: the initial successor checkpoint omitted the required derived evidence-state list; the repository contract requires proven, derived, unknown and conflicts at the top level
rejected_hypotheses:
  - lower-layer tests alone prove the composed browser lifecycle
  - completed task records can remain active while successors own the same ledger paths
  - overwriting PR #257's enriched Events archive is acceptable
changed_paths:
  - .github/workflows/announcements-acceptance.yml
  - scripts/acceptance/playwright.announcements.config.mjs
  - scripts/acceptance/seed-browser-announcements.php
  - scripts/acceptance/tests/announcements-public-acceptance.spec.mjs
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-announcements-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-downloads-acceptance.md
validation:
  - command: repository inspection
    result: PASS
    evidence: routes, controller, model, translation controller/form, ticker query/view and feature tests inspected
blockers:
  - none
next_action: Execute the exact-SHA Announcements workflow while the ledger remains planned; fix the first browser failure before promotion.
```

## Notes

This package closes only the delivered Announcements contract. It does not deploy to production or change the separately tracked homepage-template selector.
