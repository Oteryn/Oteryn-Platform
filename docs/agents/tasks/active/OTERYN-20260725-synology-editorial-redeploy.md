---
task_id: OTERYN-20260725-synology-editorial-redeploy
required_reads:
  - AGENTS.md
  - deploy/synology/README.md
  - .github/workflows/deploy-synology-staging.yml
search_first:
  - docs/agents/tasks/active/**
  - open pull requests touching deploy/synology or Synology workflows
optional_reads: []
---

# OTERYN-20260725-synology-editorial-redeploy

## Goal

Deploy the merged editorial support/legal release from PR #159 to the existing reversible Synology staging stack while preserving the established LAN game boundary.

## Acceptance criteria

- [ ] Exact Platform and Gateway images for `sha-96521c71ce166e8eb4f242706b6b5fde2dd8bce2` are resolved from GHCR.
- [ ] Deployment is dispatched only through trusted `main`, environment `synology-staging` and runner `oteryn-staging`.
- [ ] Platform, Gateway and legacy login remain loopback-only.
- [ ] Canary game TCP remains `192.168.1.2:7172`.
- [ ] Existing migrations and bounded health checks pass.
- [ ] A last-good runtime image snapshot remains available and rollback is automatically invoked after deployment failure.
- [ ] Sanitized deployment evidence is recorded in issue #173.
- [ ] The temporary one-shot workflow is removed after execution and the task is archived.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-synology-editorial-redeploy.yml
  - docs/agents/tasks/active/OTERYN-20260725-synology-editorial-redeploy.md
modules:
  - deployment
  - operations
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T08:30:00Z
branch: ops/OTERYN-20260725-synology-editorial-redeploy
pr: pending
issue: 173
status: implementing
proven:
  - PR #159 is merged and its exact Platform/Gateway release tag is sha-96521c71ce166e8eb4f242706b6b5fde2dd8bce2.
  - The Synology deployment package, self-hosted runner, health checks and rollback path were previously proven by PRs #127, #137, #138 and #141.
  - The established LAN deployment keeps Platform 8000, Gateway 8080 and legacy login 7171 on 127.0.0.1 and exposes only Canary game TCP on 192.168.1.2:7172.
  - No open pull request overlaps the temporary one-shot workflow or this task record.
derived:
  - A guarded trusted-main one-shot dispatcher is the available connector-compatible way to invoke the existing manual deployment workflow without exposing Synology or secrets.
unknown:
  - Live deployment result until the one-shot workflow completes.
conflicts: []
validation:
  - command: repository and open-PR deployment overlap review
    result: PASS
    evidence: no open Synology deployment pull request found
blockers:
  - none
next_action: Open the guarded deployment PR, pass exact-head checks, squash-merge with the activation marker, then inspect issue #173 for sanitized runtime evidence.
```
