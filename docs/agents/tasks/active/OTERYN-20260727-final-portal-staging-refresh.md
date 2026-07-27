---
task_id: OTERYN-20260727-final-portal-staging-refresh
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - .github/workflows/deploy-synology-staging.yml
---

# OTERYN-20260727-final-portal-staging-refresh

## Goal

Build, deploy and verify the exact trusted-main portal release after PR #260 on the existing Synology staging target.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-final-portal-staging-refresh.yml
  - deploy/synology/.final-portal-staging-refresh-trigger
  - docs/agents/tasks/active/OTERYN-20260727-final-portal-staging-refresh.md
modules:
  - Deployment / Synology staging
dependencies:
  - PR #260 merge 436d30e56bbf2821d01372a8aec15ec1a3ffca30
  - Issue #261
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T21:58:00Z
head: 436d30e56bbf2821d01372a8aec15ec1a3ffca30
branch: ops/OTERYN-20260727-final-portal-staging-refresh
pr: null
status: implementing
proven:
  - PR #260 merged after all exact-head repository and module workflows passed.
  - Synology staging still requires a new exact image build and guarded deployment.
derived:
  - A trigger under deploy/synology forces exact Platform and Gateway image publication for the deployment PR merge SHA.
unknown:
  - Final deployment run ID and exact running image references.
conflicts: []
first_failure:
  marker: none
  evidence: deployment not started
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-final-portal-staging-refresh.md
validation:
  - command: branch created from 436d30e56bbf2821d01372a8aec15ec1a3ffca30
    result: PASS
    evidence: GitHub branch creation succeeded
blockers: []
next_action: Add the guarded one-shot image wait, deployment dispatch and exact running-image verification workflow.
```
