---
task_id: OTERYN-20260802-playwright-php85-ci
policy_version: 2
project_lane: oteryn-platform-core
task_kind: implementation
execution_mode: github-only
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - open PRs and active tasks owning CI Playwright acceptance or deployment paths
  - current acceptance workflow and Playwright package version
  - exact Issue 365 technical failure evidence
---

# OTERYN-20260802-playwright-php85-ci

## Goal

Provide a retained containerized Playwright CI runtime in which browser tests and Laravel acceptance helpers use one Composer-lock-compatible PHP 8.5 toolchain.

## Delivery classification

```yaml
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: true
```

## Acceptance criteria

- [x] A retained Dockerfile builds a Debian/glibc runtime with PHP 8.5, Node 22, Composer 2 and pinned Playwright 1.60.0 browsers.
- [x] The runtime fails fast when the repository Playwright version or PHP platform requirement is incompatible.
- [x] The runtime invokes Laravel/acceptance PHP helpers directly with PHP 8.5.
- [x] A retained GitHub Actions workflow builds the image on the exact PR head and proves PHP, Composer, Node, Playwright and Chromium/Firefox/WebKit startup.
- [x] The workflow proves an exact-repository `php artisan` command and mounted acceptance-helper execution.
- [x] Existing host-based acceptance CI remains unchanged.
- [x] Exact-head required CI and Agent Governance pass; no material independent-audit finding remains.

## Ownership

```yaml
owned_paths:
  - deploy/ci/playwright-php.Dockerfile
  - scripts/acceptance/run-playwright-ci.sh
  - .github/workflows/playwright-runtime-validation.yml
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/agents/tasks/archive/OTERYN-20260802-playwright-php85-ci.md
modules:
  - testing
  - ci-repair
dependencies:
  - composer.json requires PHP ^8.5
  - scripts/acceptance/package.json pins @playwright/test 1.60.0
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-03T07:53:44+02:00
branch: fix/OTERYN-20260802-playwright-php85-ci
pr: 477
status: ready
validated_head: 33c73791f71a82faa864e177267d1bcaa262d98c
base_at_task_start: 39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c
proven:
  - Issue 365 run 30763456046 failed before browser execution because the previous container exposed PHP 8.3.6 while Composer required PHP >=8.5
  - the retained image uses php:8.5-cli-bookworm, Node 22, Composer 2 and exact @playwright/test 1.60.0
  - built-in PHP 8.5 Lexbor, DOM, XML, XMLWriter and MBString modules are retained; only additional required extensions are compiled
  - the runner verifies PHP, exact Playwright, Composer platform requirements, acceptance helper syntax and a Laravel artisan command
  - mounted bash and sh passthrough works without leaving scripts/acceptance/node_modules behind
  - Chromium, Firefox and WebKit runtime smoke passed
  - changed paths are limited to five declared CI/test-infrastructure, task and documentation files
  - no unresolved review threads exist
  - all nine pull-request workflow runs passed on exact head 33c73791f71a82faa864e177267d1bcaa262d98c
validation:
  - workflow: Portal Acceptance Contract
    run: 30771122629
    result: PASS
  - workflow: Acceptance E2E and Visual UX
    run: 30771122631
    result: PASS
  - workflow: Agent Governance
    run: 30771122647
    result: PASS
  - workflow: Game Auth Ticket Concurrency
    run: 30771122630
    result: PASS
  - workflow: Edge Security Emulation
    run: 30771122660
    result: PASS
  - workflow: Platform DB Outage Validation
    run: 30771122635
    result: PASS
  - workflow: Phase 7 Production-Like Validation
    run: 30771122641
    result: PASS
  - workflow: CI
    run: 30771122650
    result: PASS
  - workflow: Playwright PHP 8.5 Runtime
    run: 30771122646
    result: PASS
independent_audit:
  result: PASS
  material_findings_open: 0
  unresolved_review_threads: 0
repair_cycles_for_runtime_gate: 3
next_action: verify required checks on the checkpoint-only head, then squash-merge PR 477 and archive this task
```

## Notes

This task changes repository CI/test infrastructure only. It performs no deployment or production operation. Issue #326 remains the owner-directed exhaustive portal audit and may use this runtime after merge.
