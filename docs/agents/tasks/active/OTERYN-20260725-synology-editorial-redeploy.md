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

Deploy the merged editorial support/legal release from PR #159 together with the current trusted `main` to the existing reversible Synology staging stack while preserving the established LAN game boundary.

## Acceptance criteria

- [ ] Platform and Gateway images are built and resolved for the exact guarded dispatcher squash-merge SHA.
- [ ] Deployment is dispatched only through trusted `main`, environment `synology-staging` and runner `oteryn-staging`.
- [ ] Platform, Gateway and legacy login remain loopback-only.
- [ ] Canary game TCP remains `192.168.1.2:7172`.
- [ ] Existing migrations and bounded health checks pass.
- [ ] A last-good runtime image snapshot remains available and rollback is automatically invoked after deployment failure.
- [ ] Sanitized deployment evidence is recorded in issue #173.
- [ ] The temporary one-shot workflow and build marker are removed after execution and the task is archived.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-synology-editorial-redeploy.yml
  - deploy/synology/.editorial-redeploy-trigger
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
checkpoint_version: 2
updated_at: 2026-07-25T08:36:00Z
branch: ops/OTERYN-20260725-synology-editorial-redeploy
pr: 174
issue: 173
status: validating
proven:
  - PR #159 is merged and the current main also contains later merged Announcements, Events and Downloads modules that must not be rolled back.
  - The guarded build marker causes the existing Build Synology Staging Images workflow to publish Platform and Gateway images for the exact dispatcher merge SHA.
  - The Synology deployment package, self-hosted runner, health checks and rollback path were previously proven by PRs #127, #137, #138 and #141.
  - The established LAN deployment keeps Platform 8000, Gateway 8080 and legacy login 7171 on 127.0.0.1 and exposes only Canary game TCP on 192.168.1.2:7172.
  - No other open pull request owns the temporary one-shot workflow or build marker.
derived:
  - Building and deploying the exact dispatcher merge SHA prevents the Synology stack from losing features merged after PR #159.
  - A guarded trusted-main one-shot dispatcher is the available connector-compatible way to invoke the existing manual deployment workflow without exposing Synology or secrets.
unknown:
  - Live deployment result until the one-shot workflow completes.
conflicts: []
validation:
  - command: repository and open-PR deployment overlap review
    result: PASS
    evidence: no competing open Synology deployment pull request found
  - command: compare 96521c71ce166e8eb4f242706b6b5fde2dd8bce2...main
    result: PASS
    evidence: detected three later main commits and changed the dispatcher to build/deploy the exact merge SHA rather than an older image
blockers:
  - none
next_action: Pass exact-head checks on PR #174, squash-merge with the activation marker, then inspect issue #173 for sanitized deployment and health evidence.
```
