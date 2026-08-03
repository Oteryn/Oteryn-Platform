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
- [x] Full implementation-head CI and independent audit pass with no material finding.

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
checkpoint_version: 1
updated_at: 2026-08-03T07:57:35+02:00
head: 13d61ad7d3b07e98fb5752336bbc621892d4e80c
branch: fix/OTERYN-20260802-playwright-php85-ci
pr: 477
status: waiting
context_routes:
  - testing
  - ci-repair
owned_paths:
  - deploy/ci/playwright-php.Dockerfile
  - scripts/acceptance/run-playwright-ci.sh
  - .github/workflows/playwright-runtime-validation.yml
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/agents/tasks/archive/OTERYN-20260802-playwright-php85-ci.md
proven:
  - Issue 365 run 30763456046 failed before browser execution because the previous container exposed PHP 8.3.6 while Composer required PHP 8.5 or newer
  - retained image uses PHP 8.5 Node 22 Composer 2 and exact Playwright 1.60.0
  - runner verifies PHP Playwright Composer platform Laravel helper and artisan execution
  - Chromium Firefox and WebKit runtime smoke passed
  - implementation changes are limited to five declared CI test task and documentation files
  - all nine workflow runs passed on implementation head 33c73791f71a82faa864e177267d1bcaa262d98c
  - independent diff and scope audit passed with zero material findings and zero unresolved review threads
  - checkpoint schema failure on 1d95aff35b69399561d9c0605c0bbfb5c3191970 was isolated and repaired on 13d61ad7d3b07e98fb5752336bbc621892d4e80c
derived:
  - successful exact implementation-head browser smoke removes the PHP 8.3 acceptance-runtime blocker for the downstream portal audit
unknown:
  - terminal conclusions of the nine exact-head workflow runs emitted for 13d61ad7d3b07e98fb5752336bbc621892d4e80c
conflicts: []
first_failure:
  marker: checkpoint validator rejected an unsupported nested independent_audit key
  evidence: Agent Governance run 30788469140 job 91606857965 failed at active task checkpoint validation
rejected_hypotheses:
  - the retained Playwright runtime regressed after the task-only checkpoint commit
  - the implementation-head CI evidence was incomplete
changed_paths:
  - .github/workflows/playwright-runtime-validation.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/agents/tasks/active/OTERYN-20260802-playwright-php85-ci.md
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - scripts/acceptance/run-playwright-ci.sh
validation:
  - command: full required workflow set on 33c73791f71a82faa864e177267d1bcaa262d98c
    result: PASS
    evidence: runs 30771122629 30771122631 30771122647 30771122630 30771122660 30771122635 30771122641 30771122650 and 30771122646 passed
  - command: independent exact-diff scope and review-thread audit
    result: PASS
    evidence: five declared files zero material findings zero unresolved threads
  - command: Agent Governance run 30788469140 on checkpoint-only head 1d95aff35b69399561d9c0605c0bbfb5c3191970
    result: FAIL
    evidence: unsupported nested checkpoint key identified and removed
  - command: exact-head workflows on 13d61ad7d3b07e98fb5752336bbc621892d4e80c
    result: NOT_RUN
    evidence: runs 30788557682 30788557677 30788557695 30788557998 30788557684 30788557687 30788557720 30788557679 and 30788557680 were pending or in progress at the second allowed state check
blockers:
  - repository auto-merge is disabled
next_action: inspect the live PR exact head and its workflow conclusions once then squash-merge PR 477 only if every required check passes
```

## Notes

This task changes repository CI/test infrastructure only. It performs no deployment or production operation. Issue #326 remains the owner-directed exhaustive portal audit and may use this runtime after merge.
