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
updated_at: 2026-07-27T22:00:00Z
head: 6e6acd56fb24828aec5838734e912a31c0489978
branch: ops/OTERYN-20260727-final-portal-staging-refresh
pr: 262
status: validating
proven:
  - PR #260 merged after all exact-head repository and module workflows passed.
  - The one-shot workflow uses the existing guarded Synology staging deployment and exact running-image verification.
  - The deploy/synology trigger forces exact Platform and Gateway image publication for the eventual trusted-main merge SHA.
derived:
  - Merging PR #262 with the required marker will publish exact images, dispatch deployment and verify the refreshed runtime.
unknown:
  - Final deployment run ID and exact running image references.
conflicts: []
first_failure:
  marker: Agent Governance run 30308975378
  evidence: checkpoint still recorded pr null and the pre-implementation head after PR #262 opened
changed_paths:
  - .github/workflows/one-shot-final-portal-staging-refresh.yml
  - deploy/synology/.final-portal-staging-refresh-trigger
  - docs/agents/tasks/active/OTERYN-20260727-final-portal-staging-refresh.md
validation:
  - command: branch created from 436d30e56bbf2821d01372a8aec15ec1a3ffca30
    result: PASS
    evidence: GitHub branch creation succeeded
  - command: PR #262 opened to main
    result: PASS
    evidence: GitHub reports head 6e6acd56fb24828aec5838734e912a31c0489978
  - command: first Agent Governance run 30308975378
    result: FAIL
    evidence: stale checkpoint metadata; corrected in this commit
blockers: []
next_action: Require all exact-head PR workflows to pass, then squash-merge with the final-portal-staging-refresh marker.
```
