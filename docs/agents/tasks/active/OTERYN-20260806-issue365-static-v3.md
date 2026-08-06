---
task_id: OTERYN-20260806-issue365-static-v3
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 721
parent_issue: 365
branch: validation/issue365-static-v3-20260806
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

# OTERYN-20260806-issue365-static-v3

## Goal

Prove complete immutable Issue #365 validator generation and PHP 8.5 structural patch on `ubuntu-latest` only, without allocating Synology or executing any product sample.

## Ownership

```yaml
owned_paths:
  - .github/ISSUE365_STATIC_V3_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v3.py
  - .github/workflows/issue365-static-v3.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-static-v3.md
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
- legacy control: `8c58035cacb9fd4675d898a1652036fc8b9d4357`;
- harness source: `f23bd310eb8812ff61e7ad7227b2a950bf695b59`;
- local patcher compile: PASS;
- workflow runs authorized: `1`;
- self-hosted jobs authorized: `0`;
- product/browser samples authorized: `0`;
- temporary PR: close without merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:20:00Z
head: resolved-from-live-branch
branch: validation/issue365-static-v3-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - ci-repair
owned_paths:
  - .github/ISSUE365_STATIC_V3_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v3.py
  - .github/workflows/issue365-static-v3.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-static-v3.md
proven:
  - prior v2 run 31092256371 failed in the GitHub-hosted static job before Synology because the patcher contained an unterminated string literal
  - corrected v3 patcher passes local python3 -m py_compile
  - the task is static-only and cannot allocate a self-hosted runner
  - no open Issue #365 product implementation PR owns the selected unique validation paths
derived:
  - a static PASS is required before any separate one-shot Synology task may be created
  - a static failure has no product or browser meaning
unknown:
  - whether exact immutable generation passes every legacy and v3 anchor
  - whether the generated validator passes Bash syntax and marker-count assertions
conflicts: []
first_failure:
  marker: v2-unterminated-string-literal
  evidence: workflow run 31092256371 failed at patcher line 145; dependent Synology job skipped
rejected_hypotheses:
  - create another combined static-plus-Synology workflow before static generation is independently proven
  - infer product behavior from static failures
  - modify application or dependency-lock files
changed_paths:
  - .github/ISSUE365_STATIC_V3_VALIDATION_ONLY.md
  - .github/scripts/issue365_patch_php85_wrapper_v3.py
  - docs/agents/tasks/active/OTERYN-20260806-issue365-static-v3.md
validation:
  - command: local python3 -m py_compile
    result: PASS
    evidence: corrected v3 patcher compiled in the sandbox before repository write
  - command: exact immutable static generation workflow
    result: NOT_RUN
    evidence: workflow will be created last as the single trigger commit
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: static-only validator generation; no product runtime authorized
blockers: []
next_action: Open the temporary draft observation PR, create the static-only workflow as the final trigger commit, classify its single terminal result and close the PR without merge.
```
