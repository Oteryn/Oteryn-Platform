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

- [ ] Inventory public/player surfaces, shared primitives, routes, assets, localization and states.
- [ ] Implement one recognizable Oteryn visual system and first-viewport world context.
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
  - resources/views/public/components/**
  - lang/en/public.php
  - lang/pl/public.php
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
updated_at: 2026-09-06T21:33:02Z
head: 294e18909b8319695021011ccbeb1386cac32ced
branch: feat/20260906-premium-portal-redesign
pr: none
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
  - resources/views/public/components/**
  - lang/en/public.php
  - lang/pl/public.php
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
  - Final rendered visual quality and exact candidate validation.
conflicts: []
first_failure:
  marker: LOCAL_NETWORK_UNAVAILABLE
  evidence: sandbox outbound GitHub DNS lookup fails; use permitted GitHub Actions instead
rejected_hypotheses: []
changed_paths:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
validation:
  - command: baseline visual review via existing acceptance smoke profile
    result: NOT_RUN
    evidence: initial evidence harness being published; no redesign result is claimed
blockers: []
next_action: Collect the baseline artifact and implement the shared portal redesign.
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
last_progress_at: 2026-09-06T21:33:02Z
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
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
  checkpointed_at: 2026-09-06T21:33:02Z
  last_progress_at: 2026-09-06T21:33:02Z
  phase: baseline-render-and-implementation
  exact_head: 294e18909b8319695021011ccbeb1386cac32ced
  pull_request: none
  active_operation: draft PR acceptance baseline
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: verify this branch and its PR before continuing; do not overwrite another writer
  next_action: Collect the baseline artifact and implement the shared portal redesign.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner-requested human visual review; task agent retains this branch until review or explicit disposition
source_branch_evidence: Issue 1297; PR to be linked after creation
```

## Execution resources

The existing GitHub-hosted acceptance job and its runner-managed service containers are ephemeral. No self-hosted, desktop, staging or production resources are used. Baseline-only tracked-source export is temporary instrumentation and must be removed before final readiness. Evidence contains only repository source and sanitized synthetic public renders, not environment files, credentials, session dumps or raw authentication traces.
