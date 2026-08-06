---
task_id: OTERYN-20260806-issue365-php85-validation-v2
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 717
parent_issue: 365
branch: validation/issue365-php85-v2-20260806
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

# OTERYN-20260806-issue365-php85-validation-v2

## Goal

Statically reconstruct and validate the exact Issue #365 validator, then execute one PHP 8.5-compatible Synology matrix run only after the static phase passes.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_PHP85_V2_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v2.py
  - .github/workflows/issue365-synology-php85-v2.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation-v2.md
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
  - Cloudflare configuration and credentials
  - external repositories
```

## Exact boundary

- frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`;
- legacy control: `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- harness source: `f23bd310eb8812ff61e7ad7227b2a950bf695b59`;
- previous terminal run: `31091364264` (`INVALID_TECHNICAL_FAILURE`);
- workers: `1`;
- retries: `0`;
- workflow runs authorized: `1`;
- Synology matrix jobs authorized: at most `1` and only after static PASS;
- temporary observation PR: close without merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:12:00Z
head: resolved-from-live-branch
branch: validation/issue365-php85-v2-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - frontend-ux
  - ci-repair
owned_paths:
  - .github/ISSUE365_PHP85_V2_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v2.py
  - .github/workflows/issue365-synology-php85-v2.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation-v2.md
proven:
  - run 31091364264 failed before matrix execution because an indentation-sensitive patch anchor found zero matches
  - the actual frozen sample command places PLAYWRIGHT_BROWSERS_PATH immediately before the work-volume mount
  - the v2 patcher locates the generated sample command by stripped line structure rather than fixed indentation
  - the v2 wrapper preserves the Playwright working directory and forwards bounded sample environment variables to PHP in the frozen Platform container
  - the Synology job will depend on a GitHub-hosted static generation job and consume its exact uploaded validator
  - no open Issue #365 product implementation PR owns the selected unique validation paths
derived:
  - a static failure cannot start the Synology matrix job
  - the exact generated validator SHA can be verified unchanged between jobs
  - delegating PHP through the frozen Platform container preserves PHP 8.5 without mutating the frozen checkout
unknown:
  - whether every v2 structural assertion passes on the complete generated validator
  - whether the uploaded validator executes successfully through the Docker socket on Synology
  - the product result of the 12-sample matrix
conflicts: []
first_failure:
  marker: generated-playwright-invocation-anchor
  evidence: run 31091364264 reported expected one match but found zero before matrix execution
rejected_hypotheses:
  - use another indentation-sensitive multiline replacement
  - modify the frozen application or Composer lock
  - start Synology work before static generation passes
  - execute more than one Synology matrix job in this task
changed_paths:
  - .github/ISSUE365_PHP85_V2_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v2.py
  - docs/agents/tasks/active/OTERYN-20260806-issue365-php85-validation-v2.md
validation:
  - command: v2 exact-generation static workflow
    result: NOT_RUN
    evidence: the workflow is created last so exactly one governed workflow run is triggered
  - command: v2 exact-frozen Synology matrix
    result: NOT_RUN
    evidence: blocked by the static job until exact generation and script structure pass
blockers: []
next_action: Open the temporary draft observation PR, create the unique two-phase workflow as the final trigger commit, then classify the single terminal workflow run and close the PR without merge.
```
