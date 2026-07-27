---
task_id: OTERYN-20260728-portal-e2e-audit
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - .github/workflows/*acceptance*.yml
  - scripts/acceptance/**
  - docs/agents/tasks/active/**
optional_reads:
  - docs/testing/E2E_COVERAGE_ROADMAP.md
---

# OTERYN-20260728-portal-e2e-audit

## Goal

Execute a fresh exact-head comprehensive portal E2E audit against the current post-refresh repository state, classify every failure as product, harness, documentation or infrastructure, and persist all confirmed defects and missing capabilities in `docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md`.

## Acceptance criteria

- [ ] A dedicated audit orchestration executes both the existing zero-retry `critical` profile and the existing zero-retry `full` profile on one exact task head.
- [ ] The strict portal ledger/account lifecycle and all module-specific acceptance workflows are executed on that same exact head.
- [x] Every failed preliminary workflow was inspected at job/step/artifact level before classification and remediation.
- [x] Confirmed defects, harness limitations, documentation drift and known missing capabilities are recorded in the audit report with severity, evidence and disposition.
- [ ] The final checkpoint names the exact tested SHA and exact run evidence and makes no `PRODUCTION_PROVEN` claim.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-e2e-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
modules:
  - testing
  - portal-acceptance
  - agent-governance
dependencies:
  - PR #260 delivered-surface closure
  - PR #262 final portal staging refresh
  - PR #264 final portal container-namespace verification
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T22:34:00Z
head: UNKNOWN
branch: test/OTERYN-20260728-portal-e2e-audit
pr: 265
status: validating
context_routes:
  - testing
  - web-cms
  - admin-rbac
  - security
  - agent-governance
owned_paths:
  - .github/workflows/portal-e2e-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
proven:
  - Main base for the synchronized audit is ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b from merged PR #264.
  - Preliminary exact-head orchestration run 30310298326 tested bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab and failed overall as expected after discovering harness defects.
  - Preliminary portal contract, complete account lifecycle, Downloads, Events, Announcements, Support Legal, Wiki and three-iteration stability runs passed on bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab.
  - Preliminary critical was cancelled by a direct-profile concurrency collision; full collected specialized fixture-reset suites; Editorial Media read naturalWidth before tablet image decode; the public game-data assertion used stale channel copy; and the soak wrapper failed before creating a job.
  - The findings and exact preliminary run IDs are persisted in docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md.
derived:
  - Serializing direct critical, full and soak dispatches removes the shared direct concurrency-key collision without weakening ordinary pull-request gates.
  - Keeping specialized lifecycle specs in their dedicated zero-retry workflows prevents cross-suite reset contamination while preserving mandatory delivered-surface evidence.
unknown:
  - Final exact-head runtime results after all recorded remediations.
conflicts: []
first_failure:
  marker: E2E-AUD-001
  evidence: .github/workflows/acceptance-validation.yml profile selection overrides workflow_call profile for pull_request callers
rejected_hypotheses:
  - A documentation-only pull request is sufficient to execute the complete E2E matrix.
  - Evidence collected on the superseded ccd45fdce3176bd1da97a264bbbaf19a68c1397b-based task head is valid for current main.
  - Preliminary browser failures are product regressions; same-SHA dedicated workflows and failure artifacts proved fixture, orchestration, copy and image-decode test defects instead.
changed_paths:
  - .github/workflows/portal-e2e-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
validation:
  - command: Portal E2E Audit run 30310298326
    result: FAIL
    evidence: preliminary exact-head audit correctly exposed E2E-AUD-005 through E2E-AUD-009; remediations committed and final rerun required
  - command: CI run 30310298452
    result: PASS
    evidence: Composer validation and audit, formatting, PHPStan and full PHP tests passed on preliminary SHA
blockers:
  - none
next_action: Observe the new final-head CI and comprehensive audit runs, inspect any first failure, and update the report/checkpoint only from exact final-head evidence.
```

## Notes

The audit is repository/staging evidence only. Issue #91 remains the production-only gate, and external Canary/login-server repositories remain read-only.
