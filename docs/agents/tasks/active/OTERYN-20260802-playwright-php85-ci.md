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
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
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
updated_at: 2026-08-03T00:05:00+02:00
head: e716e26f187d59589387261160e741829d468928
branch: fix/OTERYN-20260802-playwright-php85-ci
pr: 477
status: validating
context_routes:
  - testing
  - ci-repair
owned_paths:
  - deploy/ci/playwright-php.Dockerfile
  - scripts/acceptance/run-playwright-ci.sh
  - .github/workflows/playwright-runtime-validation.yml
  - .github/workflows/acceptance-validation.yml
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
proven:
  - trusted main at task start is 39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c
  - composer.json requires PHP ^8.5
  - scripts/acceptance/package.json pins @playwright/test 1.60.0
  - standard acceptance-validation.yml already uses setup-php 8.5 on GitHub-hosted runners
  - Issue 365 run 30763456046 failed before browser execution because the Playwright container exposed PHP 8.3.6 while Composer required PHP >=8.5
  - existing acceptance helpers execute php and php artisan from the Playwright process
  - no live open PR owns the declared Playwright runtime paths
  - retained PHP 8.5 Node 22 Composer 2 Playwright 1.60.0 image and fail-fast runner are persisted in PR 477
  - runner supports bash and sh passthrough for drop-in replacement of the official Playwright image
  - exact-head workflow Playwright PHP 8.5 Runtime run 30769199085 was emitted
  - first Agent Governance failure was structural only: missing checkpoint field derived
  - checkpoint validator unit tests passed before the structural failure
derived:
  - replacing the temporary official Playwright image with this retained runtime removes the PHP 8.3 package-selection failure without changing Playwright test source
unknown:
  - terminal result of exact-head runtime build and three-browser smoke
  - terminal results of remaining exact-head required workflows
conflicts: []
first_failure:
  marker: active task checkpoint omitted required derived field
  evidence: Agent Governance run 30769199093 job 91553250814
rejected_hypotheses:
  - Playwright itself is incompatible with the repository
  - the host-based acceptance workflow lacks PHP 8.5
  - installing distro php-cli independently for every sample is a durable repair
changed_paths:
  - .github/workflows/playwright-runtime-validation.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - scripts/acceptance/run-playwright-ci.sh
validation:
  - command: Agent Governance run 30769199093 job 91553250814
    result: FAIL
    evidence: checkpoint validator reported missing checkpoint field derived; implementation was not rejected
  - command: Playwright PHP 8.5 Runtime run 30769199085
    result: NOT_RUN
    evidence: workflow is currently in progress on head e716e26f187d59589387261160e741829d468928
blockers:
  - none
invocation_started_at: 2026-08-02T23:51:00+02:00
last_progress_at: 2026-08-03T00:05:00+02:00
ci_checks_for_current_head: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: validate the corrected checkpoint and inspect the terminal Playwright PHP 8.5 Runtime workflow result
```

## Notes

This task changes repository CI/test infrastructure only. It does not rerun or close Issue #365 and performs no deployment or production operation.
