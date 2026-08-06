---
task_id: OTERYN-20260806-issue365-php85-validation
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 712
parent_issue: 365
branch: validation/issue365-php85-20260806
pull_request: 714
status: waiting
task_kind: validation
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
---

# OTERYN-20260806-issue365-php85-validation

## Goal

Execute one exact-frozen, zero-retry Wiki publication validation with a PHP 8.5-compatible Playwright command path, preserving partial evidence on technical failure and never merging the temporary harness.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_PHP85_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper.py
  - .github/workflows/issue365-synology-php85.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation.md
forbidden_paths:
  - app/**
  - routes/**
  - resources/**
  - database/**
  - composer.json
  - composer.lock
  - package.json
  - package-lock.json
  - deploy/**
  - production and staging configuration
  - external repositories
```

## Exact boundary

- frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`;
- prior immutable control: `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- prior terminal run: `30763456046` (`INVALID_TECHNICAL_FAILURE`);
- workers: `1`;
- retries: `0`;
- maximum Synology matrix runs in this task: `1`;
- temporary PR: close without merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:03:00Z
head: e38f4aa4148cd896a99926619a1e7eb06611e3c7
branch: validation/issue365-php85-20260806
pr: 714
status: waiting
context_routes:
  - testing
  - frontend-ux
  - ci-repair
owned_paths:
  - .github/ISSUE365_PHP85_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper.py
  - .github/workflows/issue365-synology-php85.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation.md
proven:
  - the frozen Platform image is based on php:8.5-cli-alpine
  - prior run 30763456046 used PHP 8.3.6 and invalidated all attempted samples before browser execution
  - the wrapper-enabled workflow was committed once on head e38f4aa4148cd896a99926619a1e7eb06611e3c7
  - custom run 31091364264 exists exactly once and has not started a runner step
  - the temporary observation pull request is draft PR 714 and must close without merge
derived:
  - delegating Playwright php commands to the frozen Platform validator container preserves the lockfile-compatible PHP 8.5 runtime without changing the frozen checkout
  - changing only this task checkpoint does not match the custom workflow push path and therefore does not create a second matrix run
unknown:
  - whether the generated validator passes every fail-closed patch anchor
  - whether the wrapper-enabled Playwright image can access the Docker socket on the Synology runner
  - the product result of the 12-sample matrix
conflicts: []
first_failure:
  marker: playwright-php-version-mismatch
  evidence: run 30763456046 exposed PHP 8.3.6 while the frozen lockfile requires PHP at least 8.5.0
rejected_hypotheses:
  - install distribution PHP packages in every Playwright sample
  - modify application code or the frozen dependency lock to accommodate PHP 8.3
  - rerun the heavy matrix more than once under this task
changed_paths:
  - .github/ISSUE365_PHP85_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper.py
  - .github/workflows/issue365-synology-php85.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation.md
validation:
  - command: Agent Governance run 31091366906
    result: FAIL
    evidence: the initial checkpoint omitted required structural fields; the harness did not execute in this workflow
  - command: custom Issue 365 run 31091364264
    result: BLOCKED
    evidence: the single job remains queued for the oteryn-staging runner and has executed no steps
  - command: exact frozen 12-sample matrix
    result: NOT_RUN
    evidence: the only authorized custom run has not acquired its runner
blockers:
  - custom run 31091364264 is queued until the dedicated oteryn-staging runner becomes available
next_action: Read the terminal result of the existing run 31091364264 without triggering another matrix run, classify retained evidence, then close PR 714 without merge.
```
