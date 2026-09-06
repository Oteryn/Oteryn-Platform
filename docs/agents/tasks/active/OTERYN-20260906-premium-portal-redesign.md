---
task_id: OTERYN-20260906-premium-portal-redesign
governing_issue: 1297
required_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - resources/views
  - public/css
  - scripts/acceptance
optional_reads: []
---

# OTERYN-20260906-premium-portal-redesign

## Goal

Governing GitHub Issue: #1297. Implement the owner's premium MMORPG portal redesign, preserve existing product behavior and deliver a visually inspected, validated PR for human review. No production deployment or automatic merge is requested.

## Acceptance criteria

- [x] Inventory public/player surfaces, shared primitives, routes, assets, localization and states.
- [x] Implement one recognizable Oteryn visual system and first-viewport world context.
- [ ] Preserve guest/account workflows, navigation and truthful content states.
- [ ] Inspect actual phone/tablet/desktop/wide renders and repair visual defects.
- [ ] Pass relevant static/Laravel/browser checks and review the complete diff.
- [ ] Publish exact evidence and a reviewable PR without a production-readiness claim.

## Ownership

```yaml
owned_paths:
  - public/css/**
  - public/js/portal-navigation.js
  - resources/views/home.blade.php
  - resources/views/game/layout.blade.php
  - resources/views/game/partials/**
  - resources/views/identity/layout.blade.php
  - resources/views/identity/partials/locale-switcher.blade.php
  - resources/views/errors/layout.blade.php
  - tests/Feature/HomeTest.php
  - resources/views/public/components/**
  - lang/en/portal.php
  - lang/pl/portal.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
modules:
  - PublicPortal
  - QualityE2E
dependencies:
  - existing isolated acceptance runtime
blockers: []
cross_repository_tasks: []
```

Implementation is serial because homepage composition, shared CSS and rendering iteration share the same primitives and no independent agent execution tool is available. Existing CI jobs provide independent validation; no overlapping work is delegated. Source inventory and baseline collection run before the shared implementation; final browser/state verification follows the coherent candidate.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-06T22:18:00Z
head: 41794acb6f112aa12d78820b35c2cede5bb2de05
branch: feat/20260906-premium-portal-redesign
pr: 1298
status: implementing
context_routes:
  - web-cms
  - testing
  - agent-governance
owned_paths:
  - public/css/**
  - public/js/portal-navigation.js
  - resources/views/home.blade.php
  - resources/views/game/layout.blade.php
  - resources/views/game/partials/**
  - resources/views/identity/layout.blade.php
  - resources/views/identity/partials/locale-switcher.blade.php
  - resources/views/errors/layout.blade.php
  - tests/Feature/HomeTest.php
  - resources/views/public/components/**
  - lang/en/portal.php
  - lang/pl/portal.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
proven:
  - Protected admission main is 294e18909b8319695021011ccbeb1386cac32ced.
  - Public layout is resources/views/game/layout.blade.php; homepage uses real PublicPortal view models.
  - Open PR 1294 owns only audit documentation and a different task file.
  - Existing acceptance workflow runs actual Laravel with isolated MariaDB and Redis on ubuntu-latest.
derived:
  - Shared public primitives permit portal-wide improvement without changing backend contracts.
unknown:
  - Repaired responsive candidate and full exact-head validation remain pending.
conflicts: []
first_failure:
  marker: CANDIDATE_TEXT_RESIZE_OVERFLOW
  evidence: acceptance run 34062708959 has 13 of 14 smoke tests passing; at 820px and 200 percent root font the page grew to 868px. Intrinsic wrapping repair prepared. PHPStan DOM typing and independent footer active-link contract repaired without modifying the independent gate.
rejected_hypotheses: []
changed_paths:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
validation:
  - command: baseline acceptance run 34061513441
    result: FAIL
    evidence: artifact 9997642456; unseeded support editorial returned 404 after 12 desktop captures
  - command: actual candidate acceptance run 34062708959
    result: FAIL
    evidence: artifact 9998000413 contains 61 actual Laravel HTTP screenshots; public routes, login/account, search, language, keyboard and no-JS menu passed; 200 percent text resizing failed
  - command: candidate CI run 34062708998
    result: FAIL
    evidence: Pint and security guards passed; two PHPStan DOM type errors in the new test repaired
  - command: independent critical suite run 34062708904
    result: FAIL
    evidence: 128 of 129 tests passed; footer account active-link semantics restored to satisfy unchanged independent architecture contract
  - command: local CSS and DOM supporting preview at 320, 390, 820, 1440, 1920px
    result: PASS
    evidence: no horizontal document overflow; not a substitute for final Laravel HTTP verification
  - command: node --check and php -l for changed script, tests and locale files
    result: PASS
    evidence: local syntax checks; dependency-backed tests run in repository CI
blockers: []
next_action: Render the coherent candidate in existing CI, inspect its actual screenshots and repair findings.
project_lane: oteryn-platform-core
admission_main_sha: 294e18909b8319695021011ccbeb1386cac32ced
policy_version: 2
task_kind: implementation
phase: implement
execution_mode: github-actions
execution_reason: local sandbox cannot fetch dependencies; repository-hosted isolated validation is available
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: medium
decomposition_decision: single
decomposition_reason: one shared public design system and its integrated player workflows
session_id: portal-redesign-20260906T212400Z
session_rotation_count: 0
heavy_validation_runs: 0
invocation_started_at: 2026-09-06T21:24:00Z
last_progress_at: 2026-09-06T22:18:00Z
ci_checks_for_current_head: 2
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: portal-redesign-20260906T212400Z
  session_started_at: 2026-09-06T21:24:00Z
  checkpointed_at: 2026-09-06T22:18:00Z
  last_progress_at: 2026-09-06T22:18:00Z
  phase: candidate-validation
  exact_head: 41794acb6f112aa12d78820b35c2cede5bb2de05
  pull_request: 1298
  active_operation: responsive and validation repairs
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: verify this branch and its PR before continuing; do not overwrite another writer
  next_action: Render the coherent candidate in existing CI, inspect its actual screenshots and repair findings.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner-requested human visual review; task agent retains this branch until review or explicit disposition
source_branch_evidence: Issue 1297; PR 1298
```

## Execution resources

The existing GitHub-hosted acceptance job and its runner-managed service containers are ephemeral. No self-hosted, desktop, staging or production resources are used. Baseline-only tracked-source export was removed after retrieving artifact 9997642456. Evidence contains only repository source and sanitized synthetic public renders, not environment files, credentials, session dumps or raw authentication traces.
