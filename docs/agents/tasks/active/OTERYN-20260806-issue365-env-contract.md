---
task_id: OTERYN-20260806-issue365-env-contract
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 730
parent_issue: 365
branch: validation/issue365-env-contract-20260806
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

# OTERYN-20260806-issue365-env-contract

## Goal

Prove the complete top-level environment contract of approved validator artifact `8964153679` from immutable source evidence without allocating a self-hosted runner or executing product code.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_ENV_CONTRACT_VALIDATION_ONLY.md
  - .github/workflows/issue365-env-contract.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-env-contract.md
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

```yaml
artifact_id: 8964153679
artifact_name: issue365-static-validator-v3-31092791643
artifact_digest: sha256:d46988eb5c465f115380ac10778af48e648124bd1e7a0f42ab3c65e50143749c
historical_workflow_sha: f23bd310eb8812ff61e7ad7227b2a950bf695b59
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
workflow_runs_authorized: 1
self_hosted_jobs_authorized: 0
browser_samples_authorized: 0
temporary_pr_merge_authorized: false
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:38:00Z
head: resolved-from-live-branch
branch: validation/issue365-env-contract-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - ci-repair
owned_paths:
  - .github/ISSUE365_ENV_CONTRACT_VALIDATION_ONLY.md
  - .github/workflows/issue365-env-contract.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-env-contract.md
proven:
  - static artifact proof #721 passed immutable generation, structural patch, Bash syntax and internal SHA-256 validation
  - v5 run 31093534539 invoked the exact approved validator once and failed before image build with KeyError RUNBOOK_REF
  - the historical workflow explicitly defines TARGET_SHA, RUNBOOK_REF and PLAYWRIGHT_IMAGE globally and GH_TOKEN at the execution step
  - GitHub supplies repository, run identity, attempt and workspace variables automatically
  - this task is static-only and cannot allocate a self-hosted runner
unknown:
  - exact extracted environment-name sets from approved validator bytes
  - whether any top-level required name lacks an authoritative source
conflicts: []
first_failure:
  marker: missing-validator-environment-contract
  evidence: v5 run 31093534539 stopped on missing RUNBOOK_REF before image build or browser execution
rejected_hypotheses:
  - add only RUNBOOK_REF and retry without proving the complete contract
  - infer product behavior from environment failures
  - regenerate or modify the approved validator artifact
changed_paths:
  - .github/ISSUE365_ENV_CONTRACT_VALIDATION_ONLY.md
  - docs/agents/tasks/active/OTERYN-20260806-issue365-env-contract.md
validation:
  - command: exact artifact/environment static proof
    result: NOT_RUN
    evidence: workflow will be created last as the single trigger commit
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: static-only contract extraction; no product runtime authorized
blockers: []
next_action: Open the temporary draft observation PR, create the static-only environment-contract workflow as the final trigger, classify its single terminal result and close the PR without merge.
```
