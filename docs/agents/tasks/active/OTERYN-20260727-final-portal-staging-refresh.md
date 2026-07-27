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
updated_at: 2026-07-27T22:12:00Z
head: 154eab4ae4fec31c23b8b9d0c903eb5b19b54e6f
branch: fix/OTERYN-20260727-final-portal-staging-verify
pr: 264
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
  - Guarded deployment run 30309329419 succeeded for exact images tagged sha-ccd45fdce3176bd1da97a264bbbaf19a68c1397b.
  - One-shot run 30309275896 verified both exact running image references before its runtime smoke failed.
  - The runtime smoke failed because host loopback 127.0.0.1:8000 is not the Platform container namespace.
derived:
  - Reusing the established in-container PHP probe removes the namespace mismatch without changing deployed application behavior.
unknown:
  - Final corrected one-shot run ID and PASS evidence.
conflicts: []
first_failure:
  marker: one-shot run 30309275896 verify job 90121994870
  evidence: curl to host 127.0.0.1:8000 failed after exact image verification passed
rejected_hypotheses:
  - The guarded Synology deployment failed.
  - The exact Platform or Gateway image reference was wrong.
  - The portal application failed its existing deployment health check.
changed_paths:
  - .github/workflows/one-shot-final-portal-staging-refresh.yml
  - docs/agents/tasks/active/OTERYN-20260727-final-portal-staging-refresh.md
validation:
  - command: guarded deployment run 30309329419
    result: PASS
    evidence: deployment completed successfully and exact images were running
  - command: one-shot run 30309275896 exact image verification
    result: PASS
    evidence: Platform and Gateway image references matched sha-ccd45fdce3176bd1da97a264bbbaf19a68c1397b
  - command: one-shot run 30309275896 runtime smoke
    result: FAIL
    evidence: host loopback namespace mismatch; corrected to in-container PHP probing in PR #264
blockers: []
next_action: Require all exact-head PR #264 workflows to pass, then merge with the final-portal-staging-refresh marker and verify the resulting PASS report.
```
