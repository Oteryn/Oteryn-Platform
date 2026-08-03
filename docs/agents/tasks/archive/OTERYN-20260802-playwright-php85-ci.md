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
- [x] Full exact-head CI and independent audit pass with no material finding.
- [x] PR #477 is merged and task ownership is released.

## Ownership

```yaml
owned_paths: []
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
updated_at: 2026-08-03T08:18:00+02:00
head: ef634ba32b855fe7c94077362e50007fccf56c38
branch: main
pr: 477
status: completed
context_routes:
  - testing
  - ci-repair
owned_paths: []
proven:
  - Issue 365 run 30763456046 failed before browser execution because the previous container exposed PHP 8.3.6 while Composer required PHP 8.5 or newer
  - retained image uses PHP 8.5 Node 22 Composer 2 and exact Playwright 1.60.0
  - runner verifies PHP Playwright Composer platform Laravel helper and artisan execution
  - Chromium Firefox and WebKit runtime smoke passed
  - implementation changes were limited to five declared CI test task and documentation files
  - all nine required workflows passed on exact final PR head 58b197fcec805d34225a645e78a9db8ce52eac5f
  - independent diff and scope audit passed with zero material findings and zero unresolved review threads
  - PR 477 was squash merged to main as ef634ba32b855fe7c94077362e50007fccf56c38
  - task ownership was released and the active record was archived
derived:
  - the merged retained runtime removes the PHP 8.3 acceptance-runtime blocker for the downstream portal audit
unknown: []
conflicts: []
first_failure:
  marker: checkpoint validator rejected an unsupported nested independent_audit key
  evidence: Agent Governance run 30788469140 job 91606857965 failed before the checkpoint schema was corrected
rejected_hypotheses:
  - the retained Playwright runtime regressed after checkpoint-only commits
  - implementation-head CI evidence was incomplete
changed_paths:
  - .github/workflows/playwright-runtime-validation.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/agents/tasks/archive/OTERYN-20260802-playwright-php85-ci.md
  - docs/testing/PLAYWRIGHT_PHP85_RUNTIME.md
  - scripts/acceptance/run-playwright-ci.sh
validation:
  - command: full required workflow set on 58b197fcec805d34225a645e78a9db8ce52eac5f
    result: PASS
    evidence: all nine exact-head workflows completed successfully including Playwright PHP 8.5 Runtime and Acceptance E2E and Visual UX
  - command: independent exact-diff scope and review-thread audit
    result: PASS
    evidence: five declared files zero material findings zero unresolved threads
  - command: squash merge of PR 477
    result: PASS
    evidence: main merge commit ef634ba32b855fe7c94077362e50007fccf56c38
blockers: []
next_action: continue Issue 326 exhaustive portal audit using the merged PHP 8.5 Playwright runtime
```

## Terminal closeout

```yaml
closeout:
  implementation_complete: true
  vertical_slice_complete: true
  audit:
    result: PASS
    independent_validator: fresh exact-diff and review-thread audit
    material_findings_open: 0
  e2e:
    result: PASS
    journeys:
      - retained runtime PHP Composer Laravel helper and three-browser smoke
  final_ci:
    head: 58b197fcec805d34225a645e78a9db8ce52eac5f
    result: PASS
    required_checks: 9
  pull_requests:
    open_related_prs: 0
    unresolved_review_threads: 0
    terminal_prs:
      - blakinio/Oteryn-Platform#477 merged
  task_status: completed
  task_archived: true
  ownership_released: true
  stale_branches_reconciled: true
```

## Notes

This task changed repository CI/test infrastructure only. It performed no deployment or production operation. Issue #326 remains the owner-directed exhaustive portal audit and may use the merged runtime.
