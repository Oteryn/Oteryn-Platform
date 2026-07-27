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

- [ ] A dedicated audit orchestration executes both the effective zero-retry `critical` profile and the effective zero-retry `full` profile on one exact task head.
- [ ] The strict portal ledger/account lifecycle and all module-specific acceptance workflows are executed on that same exact head.
- [x] Every failed preliminary workflow was inspected at job/step/artifact level before classification and remediation.
- [x] Confirmed defects, harness limitations, documentation drift and known missing capabilities are recorded in the audit report with severity, evidence and disposition.
- [ ] The final checkpoint names the exact tested SHA and exact run evidence and makes no `PRODUCTION_PROVEN` claim.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-e2e-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/visual-acceptance.js
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
updated_at: 2026-07-27T23:10:00Z
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
  - scripts/acceptance/visual-acceptance.js
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
proven:
  - Main base for the synchronized audit is ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b from merged PR #264.
  - Preliminary orchestration 30310298326 on bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab exposed concurrency, cross-suite fixture, stale copy, image-decode and soak-wrapper defects.
  - Second orchestration 30311441485 on 418bb0939fea9b98753da14b0e0254e0afe37f3a proved serialized critical and soak, the functional full baseline, contract/account lifecycle, Events, Announcements, Support Legal, Editorial Media, Wiki and stability.
  - The second orchestration exposed two remaining harness defects: full-profile exclusions produced zero Downloads lifecycle tests, and the exploratory visual harness used removed selector #character-name instead of #home-character-name.
  - Playwright collection now applies specialized lifecycle exclusions only to profile full, and critical/full/soak profiles are forced to retries 0.
  - The visual wrapper now resolves the legacy homepage capture selector to the current application control without changing application markup.
  - All findings and exact preliminary run IDs are persisted in docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md.
derived:
  - The next exact-head rerun must prove non-empty Downloads lifecycle execution and successful exploratory visual evidence in addition to all previously green gates.
unknown:
  - Final exact-head runtime results after E2E-AUD-010 through E2E-AUD-012 remediation.
conflicts: []
first_failure:
  marker: E2E-AUD-001
  evidence: .github/workflows/acceptance-validation.yml profile selection overrides workflow_call profile for pull_request callers
rejected_hypotheses:
  - A documentation-only pull request is sufficient to execute the complete E2E matrix.
  - Evidence collected on the superseded ccd45fdce3176bd1da97a264bbbaf19a68c1397b-based task head is valid for current main.
  - Preliminary browser failures are product regressions; exact artifacts instead proved orchestration, fixture isolation, stale selector/copy and browser timing defects.
changed_paths:
  - .github/workflows/portal-e2e-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/visual-acceptance.js
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
validation:
  - command: Portal E2E Audit run 30310298326
    result: FAIL
    evidence: preliminary run exposed E2E-AUD-005 through E2E-AUD-009
  - command: Portal E2E Audit run 30311441485
    result: FAIL
    evidence: second run proved most remediations and exposed E2E-AUD-010 through E2E-AUD-012
  - command: CI run 30311441454
    result: PASS
    evidence: Composer validation and audit, formatting, PHPStan and full PHP tests passed after the first remediation set
blockers:
  - none
next_action: Observe the new exact-head CI and comprehensive audit runs, inspect any first failure, and finalize the report only after every required gate passes on that same head.
```

## Notes

The audit is repository/staging evidence only. Issue #91 remains the production-only gate, and external Canary/login-server repositories remain read-only.
