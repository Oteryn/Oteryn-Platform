---
task_id: OTERYN-20260806-issue365-synology-v5
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 727
parent_issue: 365
branch: validation/issue365-synology-v5-20260806
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

# OTERYN-20260806-issue365-synology-v5

## Goal

Execute the immutable statically proven PHP 8.5 validator exactly once with orchestration state isolated from the frozen checkout, retain the terminal result and close the observation PR without merge.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_SYNOLOGY_V5_VALIDATION_ONLY.md
  - .github/workflows/issue365-synology-v5.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v5.md
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

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
source_run: 31092791643
source_job: 92587516914
source_head: bfb6e97c3f610b9897543e95f9feddce2c9ec4ee
artifact_id: 8964153679
artifact_name: issue365-static-validator-v3-31092791643
artifact_digest: sha256:d46988eb5c465f115380ac10778af48e648124bd1e7a0f42ab3c65e50143749c
workers: 1
playwright_retries: 0
workflow_runs_authorized: 1
self_hosted_jobs_authorized: 1
validator_invocations_authorized: 1
temporary_pr_merge_authorized: false
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:31:00Z
head: resolved-from-live-branch
branch: validation/issue365-synology-v5-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - frontend-ux
  - ci-repair
owned_paths:
  - .github/ISSUE365_SYNOLOGY_V5_VALIDATION_ONLY.md
  - .github/workflows/issue365-synology-v5.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v5.md
proven:
  - static run 31092791643 proved immutable generation, PHP 8.5 structural patch, Bash syntax and marker counts
  - v4 run 31093126827 proved GitHub artifact metadata, outer digest, every internal SHA and generated validator contract
  - v4 did not invoke the validator because orchestration files contaminated the frozen checkout before its clean-worktree assertion
  - v5 places artifact download and all orchestration files under RUNNER_TEMP
  - v5 asserts exact SHA and clean worktree before writing runtime status
  - v5 explicitly disables errexit only around one validator command and restores it before persisting the terminal status
unknown:
  - whether the validator builds the isolated runtime and reaches PHP 8.5 wrapper execution
  - the product result of the 12-sample matrix
conflicts: []
first_failure:
  marker: frozen-worktree-self-contamination
  evidence: v4 run 31093126827 wrote status under GITHUB_WORKSPACE before the clean-worktree assertion
rejected_hypotheses:
  - regenerate or modify the approved validator
  - keep orchestration files inside GITHUB_WORKSPACE
  - execute more than one self-hosted job or validator invocation
  - infer product behavior before browser execution
changed_paths:
  - .github/ISSUE365_SYNOLOGY_V5_VALIDATION_ONLY.md
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v5.md
validation:
  - command: static proof run 31092791643
    result: PASS
    evidence: artifact 8964153679 retained with exact digest and internal SHA manifest
  - command: artifact-integrity run 31093126827
    result: PASS
    evidence: metadata, download digest, internal SHA, Bash syntax and runtime contract passed before orchestration failed
  - command: isolated-state one-shot Synology matrix
    result: NOT_RUN
    evidence: workflow will be created last as the single trigger commit
blockers: []
next_action: Open the temporary draft observation PR, create the unique one-job workflow as the final trigger, classify the single terminal run and close the PR without merge.
```
