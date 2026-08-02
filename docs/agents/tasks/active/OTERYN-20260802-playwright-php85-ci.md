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

Provide a retained containerized Playwright CI runtime in which the browser runner and acceptance helper commands use one Composer-lock-compatible PHP 8.5 toolchain, eliminating the PHP 8.3 / Composer platform mismatch proven by Issue #365 validation run `30763456046`.

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
- [x] Existing host-based acceptance CI remains unchanged.
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
  - composer.json requires PHP ^8.5 and composer.lock pins PHP dependencies
  - scripts/acceptance/package.json pins @playwright/test 1.60.0 exactly
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-03T00:27:00+02:00
head: 16af4d32a714ffb0b263491a20f9690ec43bd930
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
  - composer.json requires PHP ^8.5 and composer.lock is present
  - scripts/acceptance/package.json pins @playwright/test 1.60.0 exactly
  - no npm lockfile exists in the repository and the image verifies the installed exact top-level Playwright version
  - standard acceptance-validation.yml already uses setup-php 8.5 on GitHub-hosted runners and was not modified
  - Issue 365 run 30763456046 failed before browser execution because the Playwright container exposed PHP 8.3.6 while Composer required PHP >=8.5
  - existing acceptance helpers execute php and php artisan from the Playwright process
  - no live open PR owns the declared Playwright runtime paths
  - official php 8.5 cli bookworm compiles default-enabled lexbor dom xml xmlwriter and mbstring into the base runtime
  - rebuilding dom separately failed because ext dom depends on the always-enabled lexbor extension and its source headers
  - retained image now compiles only the additional gd intl pcntl pdo_mysql and zip extensions and enables redis through PECL
  - the image explicitly verifies all required built-in and additional extensions before browser installation
  - runner fails closed on PHP version exact Playwright version all Composer platform requirements helper syntax and php artisan cache clear
  - runner supports mounted bash and sh passthrough and removes its temporary node_modules link after execution
  - workflow prepares a file-backed local Laravel environment generates APP_KEY through the image and verifies all three browser engines
  - all non-runtime required workflows passed on exact head af556cba25caf7aacd8a50133ed28aab26cb5630
  - Agent Governance passed on exact head af556cba25caf7aacd8a50133ed28aab26cb5630 in run 30769653897
derived:
  - using the PHP base image built-in lexbor and DOM modules avoids an invalid partial rebuild while preserving Composer platform verification
  - replacing the temporary official Playwright image with this retained runtime removes the PHP 8.3 package-selection failure without changing Playwright test source
  - shell passthrough allows existing generated bash-lc Playwright invocations to adopt the runtime by changing only the image reference
  - full Composer platform checks make missing PHP extensions fail before browser execution rather than during a sample
unknown:
  - terminal exact-head result of retained image build mounted-shell compatibility Composer Laravel contract and three-browser smoke after the Lexbor repair
  - terminal exact-head results of workflows emitted for the new checkpoint head
conflicts: []
first_failure:
  marker: PHP 8.5 DOM partial rebuild could not locate Lexbor headers
  evidence: Playwright PHP 8.5 Runtime run 30769653916 job 91554438573 failed compiling ext dom with missing lexbor/html/parser.h and lexbor/encoding/encoding.h
rejected_hypotheses:
  - Playwright itself is incompatible with the repository
  - the host-based acceptance workflow lacks PHP 8.5
  - installing distro php-cli independently for every sample is a durable repair
  - a browser-only smoke without php artisan helper execution closes the failure
  - an npm package lock exists in the current repository
  - PHP 8.5 DOM must be rebuilt separately in a child image
changed_paths:
  - .github/workflows/playwright-runtime-validation.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - scripts/acceptance/run-playwright-ci.sh
validation:
  - command: Agent Governance run 30769653897
    result: PASS
    evidence: checkpoint validator accepted exact head af556cba25caf7aacd8a50133ed28aab26cb5630
  - command: required repository workflows on af556cba25caf7aacd8a50133ed28aab26cb5630
    result: PASS
    evidence: CI Edge Security Platform DB Outage Game Auth Phase 7 Portal Acceptance and Acceptance E2E all completed successfully
  - command: Playwright PHP 8.5 Runtime run 30769653916 job 91554438573
    result: FAIL
    evidence: invalid partial rebuild of default-enabled DOM failed on missing Lexbor source headers; Dockerfile corrected to use built-in modules
  - command: Playwright PHP 8.5 Runtime on Lexbor-repair exact head
    result: NOT_RUN
    evidence: terminal result pending on the current exact head
blockers:
  - none
invocation_started_at: 2026-08-02T23:51:00+02:00
last_progress_at: 2026-08-03T00:27:00+02:00
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: inspect the exact-head Playwright runtime after the built-in Lexbor and DOM repair and fix only the first actionable failure if any
```

## Notes

This task changes repository CI/test infrastructure only. It does not rerun or close Issue #365 and performs no deployment or production operation.
