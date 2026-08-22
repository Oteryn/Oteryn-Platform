---
task_id: OTERYN-20260822-portal-docker-e2e
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
search_first:
  - scripts/acceptance
  - deploy/ci/playwright-php.Dockerfile
optional_reads: []
---

# OTERYN-20260822-portal-docker-e2e

## Goal

Provide a repeatable Docker Compose runner on Molehill-PC for the existing Oteryn Portal Playwright acceptance suite, without creating a parallel browser-test framework.

## Acceptance criteria

- [x] Compose stack uses isolated MariaDB, Redis and MailHog test dependencies.
- [x] Portal runtime and Playwright execute from the exact checkout inside Docker.
- [x] `smoke` profile passes on Molehill-PC.
- [x] A broader `critical` run was executed and its real intermediate failures were retained.
- [x] Browser/test failures remain fatal and artifacts are retained on the host.
- [x] Task-owned containers, networks and volumes are removed after each completed run.
- [ ] Final full validation passes on the committed exact task head.
- [ ] PR/CI/merge and terminal closeout are complete.

## Ownership

```yaml
issue: Oteryn/Oteryn-Platform#1219
branch: test/portal-docker-e2e
base_sha: 5591da8437995214b82f556992301f899cb91aa8
project_lane: oteryn-platform-core
task_kind: e2e
implementation_authorized: bounded
production_mutation: forbidden
external_repositories: not_required
owned_paths:
  - scripts/acceptance/docker/**
  - scripts/acceptance/bootstrap-production-like.sh
  - scripts/acceptance/seed-game-catalog.php
  - resources/navigation/public/core.php
  - resources/navigation/public/marketplace.php
  - resources/navigation/public/support.php
  - resources/views/home.blade.php
  - resources/views/support/editorial/show.blade.php
  - lang/en/public.php
  - lang/pl/public.php
  - docs/agents/tasks/active/OTERYN-20260822-portal-docker-e2e.md
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-22T14:20:03+02:00
head: 3e000825641fdacb249fe97a8505733a3bc008c3
branch: test/portal-docker-e2e
pr: none
status: ready
context_routes:
  - testing
  - web-cms
owned_paths:
  - scripts/acceptance/docker/**
  - scripts/acceptance/bootstrap-production-like.sh
  - scripts/acceptance/seed-game-catalog.php
  - resources/navigation/public/core.php
  - resources/navigation/public/marketplace.php
  - resources/navigation/public/support.php
  - resources/views/home.blade.php
  - resources/views/support/editorial/show.blade.php
  - lang/en/public.php
  - lang/pl/public.php
  - docs/agents/tasks/active/OTERYN-20260822-portal-docker-e2e.md
phase: validate
session_role: implementation-owner
execution_mode: remote-desktop-docker
execution_reason: local Docker/Playwright repair and verification loop on the authorized Molehill-PC checkout
context_pressure: high
context_growth: rising
decomposition_decision: phased
validation_level: full
last_completed_step: full responsive profile passed 45/45 after targeted resilience passed 8/8
heavy_validation_runs: 2
context_reconstruction_attempts: 1
stall_warnings: 0
ci_checks_for_current_head: 0
unchanged_state_checks: 0
terminal_ci_checks_for_current_generation: 0
repair_cycles_for_current_gate: 2
invocation_started_at: 2026-08-22T12:54:00+02:00
last_progress_at: 2026-08-22T14:29:04.6988626+02:00
proven:
  - Docker smoke passed 7/7 after portal translation-collision repairs.
  - Docker portability passed 57/57 in the broader critical run.
  - Docker resilience passed 8/8 after loopback DB/Redis bridges and cache directory repair.
  - Docker responsive passed 45/45 on the latest focused full rerun.
  - Bootstrap localhost defaults remain intact outside the Docker opt-in environment.
derived:
  - The remaining gate is a fresh exact-committed-head critical run, followed by PR/CI/merge closeout.
unknown: []
conflicts: []
first_failure:
  marker: homepage returned HTTP 500
  evidence: Laravel reported translation-group arrays reaching Blade escaping/navigation string conversion
rejected_hypotheses:
  - Playwright browser runtime was the cause of the homepage 500
  - Docker networking alone explained the public portal failures
changed_paths:
  - scripts/acceptance/docker/**
  - scripts/acceptance/bootstrap-production-like.sh
  - scripts/acceptance/seed-game-catalog.php
  - resources/navigation/public/{core,marketplace,support}.php
  - resources/views/home.blade.php
  - resources/views/support/editorial/show.blade.php
  - lang/{en,pl}/public.php
validation:
  - command: docker portal smoke
    result: PASS
    evidence: 7/7 Playwright smoke tests passed
  - command: docker portal portability
    result: PASS
    evidence: 57/57 Playwright portability tests passed
  - command: docker portal resilience
    result: PASS
    evidence: 8/8 Playwright resilience/error-state tests passed
  - command: docker portal responsive
    result: PASS
    evidence: 45/45 Playwright responsive tests passed on latest rerun
  - command: final exact-head critical
    result: NOT_RUN
    evidence: must run after coherent commit on rebased task head
blockers: []
next_action: Commit and push the coherent candidate, then run one fresh Docker critical profile on that exact runtime head.
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: medium
  triggers:
    - CI/test infrastructure
    - public portal rendering and localization
    - integration dependency simulation
  unknown_or_conflict: []
  rationale: Docker acceptance infrastructure plus repairs to public rendering paths require full browser and negative-path evidence.
  self_review:
    result: PENDING
    exact_head: none
    evidence:
      - full diff review required after commit
      - final exact-head critical required
```

## Recovery checkpoint

```yaml
recovery_checkpoint_version: 1
generation: 1
invocation_id: terminal-session-20260822-1254
session_role: implementation-owner
status: ready
saved_at: 2026-08-22T14:29:04.6988626+02:00
exact_head: 3e000825641fdacb249fe97a8505733a3bc008c3
safe_to_resume: true
operation_started_at: null
last_completed_operation: coherent implementation candidate committed and focused Docker validation preserved
next_operation: run final exact-head critical against committed runtime candidate
resource_cleanup_required: true
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation is not merged yet
source_branch_evidence: pending
```

## Notes

The Docker runner reuses the repository-owned Playwright suite and CI image. All acceptance databases, Redis data and MailHog state are isolated and disposable. No staging or production endpoint is mutated. Full browser traces and screenshots remain local under `artifacts/` and are intentionally not committed.