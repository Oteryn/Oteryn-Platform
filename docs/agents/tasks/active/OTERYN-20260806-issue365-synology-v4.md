---
task_id: OTERYN-20260806-issue365-synology-v4
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 724
parent_issue: 365
branch: validation/issue365-synology-v4-20260806
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

# OTERYN-20260806-issue365-synology-v4

## Goal

Execute exactly once on Synology the immutable PHP 8.5 validator proven by static run `31092791643`, retain full or partial evidence, and close the temporary observation PR without merge.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_SYNOLOGY_V4_VALIDATION_ONLY.md
  - .github/workflows/issue365-synology-v4.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v4.md
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
artifact_size_bytes: 33610
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
updated_at: 2026-08-06T10:25:00Z
head: resolved-from-live-branch
branch: validation/issue365-synology-v4-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - frontend-ux
  - ci-repair
owned_paths:
  - .github/ISSUE365_SYNOLOGY_V4_VALIDATION_ONLY.md
  - .github/workflows/issue365-synology-v4.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v4.md
proven:
  - static run 31092791643 reconstructed immutable inputs and generated the validator successfully
  - static job 92587516914 passed Python compile, exact generation, structural patch, bash syntax, marker counts and SHA-256 self-verification
  - artifact 8964153679 has GitHub digest sha256:d46988eb5c465f115380ac10778af48e648124bd1e7a0f42ab3c65e50143749c
  - generated validator contains exactly one PHP 8.5 preflight and one workers=1 retries=0 Playwright command
  - no product sample was executed by the static proof
  - no open Issue #365 product implementation PR owns the selected unique validation paths
unknown:
  - whether artifact download and internal SHA verification succeed on the dedicated runner
  - whether the PHP wrapper resolves to the frozen Platform container during execution
  - the product result of the 12-sample matrix
conflicts: []
first_failure:
  marker: none
  evidence: static validator proof passed; runtime has not yet started
rejected_hypotheses:
  - regenerate or modify the approved validator artifact
  - execute more than one self-hosted job or validator invocation
  - modify application or dependency-lock files
  - infer product behavior before browser execution
changed_paths:
  - .github/ISSUE365_SYNOLOGY_V4_VALIDATION_ONLY.md
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v4.md
validation:
  - command: static proof run 31092791643
    result: PASS
    evidence: exact generated artifact 8964153679 retained with verified SHA-256
  - command: exact one-shot Synology matrix
    result: NOT_RUN
    evidence: workflow is created last as the single trigger commit
blockers: []
next_action: Open the temporary draft observation PR, create the unique one-job workflow as the final trigger commit, classify its single terminal run and close the PR without merge.
```
