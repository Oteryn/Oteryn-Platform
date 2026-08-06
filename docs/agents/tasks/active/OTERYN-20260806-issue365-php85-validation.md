---
task_id: OTERYN-20260806-issue365-php85-validation
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 712
parent_issue: 365
branch: validation/issue365-php85-20260806
pull_request: pending
status: implementing
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
updated_at: 2026-08-06T09:58:00Z
invocation_started_at: 2026-08-06T09:55:00Z
last_progress_at: 2026-08-06T09:58:00Z
head: resolved-from-live-branch
base_main: 438dcb83aa2f72022a7fd80f037dcfc65a258a8e
branch: validation/issue365-php85-20260806
pr: pending
status: implementing
phase: harness-preflight
session_id: chatgpt-20260806T1155+0200-issue365-php85-validation
lease_expires_at: 2026-08-06T11:55:00Z
context_pressure: medium
context_growth: stable
context_score: 5
owned_paths:
  - .github/ISSUE365_PHP85_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper.py
  - .github/workflows/issue365-synology-php85.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation.md
proven:
  - the frozen Platform image is based on php:8.5-cli-alpine
  - the prior Playwright-local apt install exposed PHP 8.3.6 and invalidated all attempted samples
  - the existing validator application container already mounts the Docker socket and contains the frozen application plus PHP 8.5
  - a wrapper can delegate Playwright child-process PHP calls to that exact application container without modifying the frozen checkout
  - no open Issue #365 implementation PR owns the selected unique harness paths
unknown:
  - whether the generated legacy validator matches every fail-closed patch anchor
  - whether the wrapper-enabled Playwright image can access the Docker socket on the Synology runner
  - the product result of the 12-sample matrix
conflicts: []
first_failure:
  marker: playwright-php-version-mismatch
  evidence: run 30763456046 used PHP 8.3.6 while the frozen lockfile requires PHP >=8.5.0
validation:
  - command: generated validator patch markers and bash syntax
    result: NOT_RUN
    evidence: workflow is created last so only one bounded execution is triggered
  - command: exact frozen 12-sample matrix
    result: NOT_RUN
    evidence: pending one bounded Synology run
blockers: []
next_action: Create the unique workflow last, allow its cheap preflight to generate and syntax-check the validator, then classify the single terminal Synology run and close the temporary PR without merge.
```
