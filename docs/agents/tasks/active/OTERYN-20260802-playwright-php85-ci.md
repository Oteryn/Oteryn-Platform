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

Provide a retained containerized Playwright CI runtime in which the browser runner and acceptance helper commands use one lockfile-compatible PHP 8.5 toolchain, eliminating the PHP 8.3 / Composer platform mismatch proven by Issue #365 validation run `30763456046`.

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

- [ ] A retained Dockerfile builds a Debian/glibc runtime with PHP 8.5, Node 22, Composer 2 and pinned Playwright 1.60.0 browsers.
- [ ] The runtime fails fast when the repository Playwright version or PHP platform requirement is incompatible.
- [ ] The runtime invokes Laravel/acceptance PHP helpers directly with PHP 8.5; it does not install an arbitrary distro PHP package per test sample.
- [ ] A retained GitHub Actions workflow builds the image on the exact PR head and proves PHP, Composer, Node, Playwright and Chromium/Firefox/WebKit startup.
- [ ] The workflow proves an exact-repository `php artisan` command and the acceptance helper PHP path from the same mounted checkout.
- [ ] Existing host-based acceptance CI remains unchanged unless integration evidence requires a bounded adjustment.
- [ ] Exact-head required CI and Agent Governance pass; no material independent-audit finding remains.

## Ownership

```yaml
owned_paths:
  - deploy/ci/playwright-php.Dockerfile
  - scripts/acceptance/run-playwright-ci.sh
  - .github/workflows/playwright-runtime-validation.yml
  - .github/workflows/acceptance-validation.yml
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/agents/tasks/archive/OTERYN-20260802-playwright-php85-ci.md
modules:
  - testing
  - ci-repair
dependencies:
  - composer.json requires PHP ^8.5
  - scripts/acceptance/package.json pins @playwright/test 1.60.0
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-02T23:52:00+02:00
head: 39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c
branch: fix/OTERYN-20260802-playwright-php85-ci
pr: none
status: implementing
context_routes:
  - testing
  - ci-repair
owned_paths:
  - deploy/ci/playwright-php.Dockerfile
  - scripts/acceptance/run-playwright-ci.sh
  - .github/workflows/playwright-runtime-validation.yml
  - .github/workflows/acceptance-validation.yml
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
proven:
  - trusted main at task start is 39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c
  - composer.json requires PHP ^8.5
  - scripts/acceptance/package.json pins @playwright/test 1.60.0
  - standard acceptance-validation.yml already uses setup-php 8.5 on GitHub-hosted runners
  - Issue 365 run 30763456046 failed before browser execution because the Playwright container exposed PHP 8.3.6 while Composer required PHP >=8.5
  - existing acceptance helpers execute php and php artisan from the Playwright process
  - no live open PR owns the declared Playwright runtime paths
unknown:
  - exact first build result of the retained combined runtime
conflicts: []
first_failure:
  marker: Playwright helper command cannot satisfy Composer PHP platform requirement
  evidence: run 30763456046 job 91537990755; PHP 8.3.6 versus required >=8.5.0
rejected_hypotheses:
  - Playwright itself is incompatible with the repository
  - the host-based acceptance workflow lacks PHP 8.5
  - installing distro php-cli independently for every sample is a durable repair
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation package not yet persisted
blockers:
  - none
invocation_started_at: 2026-08-02T23:51:00+02:00
last_progress_at: 2026-08-02T23:52:00+02:00
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: add the retained PHP 8.5 Playwright image, runner contract and exact-head validation workflow
```

## Notes

This task changes repository CI/test infrastructure only. It does not rerun or close Issue #365 and performs no deployment or production operation.
