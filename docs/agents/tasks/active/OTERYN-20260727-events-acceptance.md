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
  - scripts/acceptance/tests/events-acceptance.spec.mjs
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
updated_at: 2026-07-27T17:45:00Z
head: 17f1acd430c27c8c441b59b16b440b8763f03312
branch: test/OTERYN-20260727-events-acceptance
pr: none
status: implementing
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
  - scripts/acceptance/tests/events-acceptance.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/package.json
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
proven:
  - Main at task start is 17f1acd430c27c8c441b59b16b440b8763f03312.
  - PR #253 owns Downloads acceptance and does not overlap Events paths.
  - Existing Events feature tests prove UTC boundaries, validation, exact permissions, audit redaction and optimistic-lock conflicts.
  - The portal ledger currently classifies events.public-admin as planned.
derived:
  - The missing proof is composed browser behavior across public, administrative, authorization, localization and responsive states.
unknown:
  - Exact final workflow duration and first browser failure, if any.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reimplementing transaction races in Playwright would duplicate stronger feature/database evidence.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-events-acceptance.md
validation:
  - command: repository inspection through GitHub connector
    result: PASS
    evidence: routes, controllers, requests, views, feature tests and acceptance profiles inspected
blockers:
  - none
next_action: Open a draft PR and add deterministic Events browser fixtures and lifecycle coverage.
```

## Notes

The ledger moves to `covered` only when the browser package and ordinary route-classification validation pass. Production and live Synology verification remain separate Issue #240/#91 boundaries.
