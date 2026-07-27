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
- [ ] The current-main coverage manifest preserves Downloads as covered and promotes Events to covered with exact markers.
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
  - Downloads closure PR #253
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T18:36:00Z
head: 05a378811106523f2e4fd0a255d84b221811803f
branch: test/OTERYN-20260727-events-acceptance-v2
pr: none
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
  - Downloads closure PR #253 merged as f9301792cfc82956aa4af792283c18bcbaf2c28e and is preserved as the current base.
  - Superseded PR #254 passed every required workflow on head 38b85af4174af6f3167908cee563b8bc45ec7c61 before its merge was blocked only by the concurrent Downloads merge.
  - Dedicated Events Acceptance run 30293991406 passed the final zero-retry five-project browser matrix on the superseded branch.
  - CI, Portal Acceptance Contract, general Acceptance E2E, Phase 7 and focused system workflows passed on the superseded final head.
  - The Events implementation is restored on a branch created directly from current main; shared Downloads runtime/harness files are not overwritten.
derived:
  - Only current-main reconciliation of ACTIVE_WORK, the coverage manifest/matrix and an exact-head rerun remain before merge.
unknown:
  - Exact-head workflow results after current-main reconciliation.
conflicts: []
first_failure:
  marker: PR #254 merge conflict
  evidence: Downloads PR #253 merged first and changed shared ACTIVE_WORK, coverage manifest and coverage matrix records
rejected_hypotheses:
  - Force-updating the old PR branch was rejected in favor of a clean current-main successor branch.
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
changed_paths:
  - .github/workflows/events-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
  - scripts/acceptance/playwright.events.config.mjs
  - scripts/acceptance/seed-browser-events.php
  - scripts/acceptance/tests/events-admin-acceptance.spec.mjs
  - scripts/acceptance/tests/events-public-acceptance.spec.mjs
validation:
  - command: prior complete workflow set on superseded PR #254 head 38b85af4174af6f3167908cee563b8bc45ec7c61
    result: PASS
    evidence: all required repository, browser and system workflows succeeded before the concurrent merge conflict
  - command: current-main exact-head workflows
    result: NOT_RUN
    evidence: shared ledger/index reconciliation and successor PR are not yet complete
blockers:
  - none
next_action: Reconcile ACTIVE_WORK, the Downloads-plus-Events manifest and matrix, then open the current-main successor PR.
```

## Notes

The Events result is repository/controlled-runtime evidence only. Production and live go-live verification remain separate Issue #240/#91 boundaries.
