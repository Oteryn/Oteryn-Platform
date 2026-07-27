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
updated_at: 2026-07-27T22:02:00Z
head: f31a416948b2169b1af00188dc4b122ad880fb50
branch: ops/OTERYN-20260727-final-portal-staging-refresh
pr: 262
status: validating
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/one-shot-final-portal-staging-refresh.yml
  - deploy/synology/.final-portal-staging-refresh-trigger
  - docs/agents/tasks/active/OTERYN-20260727-final-portal-staging-refresh.md
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
  evidence: checkpoint omitted required contract fields and retained stale PR metadata
rejected_hypotheses:
  - The deployment workflow itself caused the governance failure.
  - Staging can be considered refreshed before exact running image verification.
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
    evidence: GitHub reports the dedicated deployment branch and task scope
  - command: Agent Governance runs 30308975378 and 30309056363
    result: FAIL
    evidence: required checkpoint fields context_routes, owned_paths and rejected_hypotheses were absent; corrected in this commit
blockers: []
next_action: Require all exact-head PR workflows to pass, then squash-merge with the final-portal-staging-refresh marker.
```
