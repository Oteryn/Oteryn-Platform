---
task_id: OTERYN-20260725-synology-editorial-redeploy
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
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

- [x] Platform and Gateway images were built and resolved for exact release `sha-7164edd1308d9f43cfbc20fb37901e66448fe165`.
- [x] Deployment was dispatched only through trusted `main`, environment `synology-staging` and runner `oteryn-staging`.
- [x] Platform, Gateway and legacy login remain loopback-only.
- [x] Canary game TCP remains `192.168.1.2:7172`.
- [x] Existing migrations and bounded health checks passed.
- [x] The last-good runtime image snapshot remains available; rollback was retained but was not needed.
- [x] Sanitized deployment evidence was recorded and issue #173 was closed as completed.
- [x] The temporary one-shot workflow and build marker are removed by the archival cleanup change.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-synology-editorial-redeploy.yml
  - deploy/synology/.editorial-redeploy-trigger
  - docs/agents/tasks/active/OTERYN-20260725-synology-editorial-redeploy.md
  - docs/agents/tasks/archive/OTERYN-20260725-synology-editorial-redeploy.md
modules:
  - deployment
  - operations
dependencies:
  - PR #159 merged
  - existing Synology staging deployment from PRs #127, #137, #138 and #141
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T08:53:00Z
head: 7164edd1308d9f43cfbc20fb37901e66448fe165
branch: chore/OTERYN-20260725-synology-editorial-redeploy-cleanup
pr: 174
status: ready
context_routes:
  - agent-governance
  - deployment
  - operations
  - testing
owned_paths:
  - .github/workflows/one-shot-synology-editorial-redeploy.yml
  - deploy/synology/.editorial-redeploy-trigger
  - docs/agents/tasks/active/OTERYN-20260725-synology-editorial-redeploy.md
  - docs/agents/tasks/archive/OTERYN-20260725-synology-editorial-redeploy.md
proven:
  - PR #174 was squash-merged into main as 7164edd1308d9f43cfbc20fb37901e66448fe165 with the guarded activation marker.
  - Exact-SHA Synology image build run 30151501911 completed successfully.
  - Guarded dispatcher run 30151501902 completed successfully.
  - Deploy Synology Staging run 30151613557 completed successfully against exact SHA 7164edd1308d9f43cfbc20fb37901e66448fe165.
  - Deployment migrations and health checks passed; issue #173 was closed as completed by the workflow.
  - Platform, Gateway and legacy login remain loopback-only and Canary game TCP remains 192.168.1.2:7172.
  - The deployment script retained a last-good rollback snapshot and did not emit secrets.
  - Current main at dispatch time exactly matched the deployed SHA, so no later merged module was rolled back.
derived:
  - The Synology stack now contains the editorial support/legal baseline plus the Announcements, Events and Downloads modules that were already present on current main.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-schema
  evidence: initial PR #174 checkpoint validation run 30151364755 failed only because required task-record fields were omitted; corrected run 30151407528 passed before merge
rejected_hypotheses:
  - Deploying only the older PR #159 image is safe after later application modules were merged.
  - The missing formal production Environment configuration means no Synology WWW deployment target exists.
changed_paths:
  - .github/workflows/one-shot-synology-editorial-redeploy.yml
  - deploy/synology/.editorial-redeploy-trigger
  - docs/agents/tasks/active/OTERYN-20260725-synology-editorial-redeploy.md
  - docs/agents/tasks/archive/OTERYN-20260725-synology-editorial-redeploy.md
validation:
  - command: Agent Governance run 30151407528
    result: PASS
    evidence: corrected exact-head task checkpoint passed repository governance
  - command: CI run 30151407527
    result: PASS
    evidence: Composer validation, audit, Pint, PHPStan and full tests passed
  - command: Build Synology Staging Images run 30151407530
    result: PASS
    evidence: deployment package validation and PR images passed before merge
  - command: Platform DB Outage Validation run 30151407534
    result: PASS
    evidence: fail-closed database validation passed
  - command: Game Auth Ticket Concurrency run 30151407535
    result: PASS
    evidence: real-database concurrency validation passed
  - command: Phase 7 Production-Like Validation run 30151407585
    result: PASS
    evidence: production-like validation passed before merge
  - command: Build Synology Staging Images run 30151501911
    result: PASS
    evidence: exact merge-SHA Platform and Gateway images were published
  - command: One-shot Synology Editorial Redeploy run 30151501902
    result: PASS
    evidence: guarded dispatcher resolved exact images and monitored deployment to completion
  - command: Deploy Synology Staging run 30151613557
    result: PASS
    evidence: migrations, runtime deployment, binding checks and health checks completed successfully
blockers:
  - none
next_action: Merge the documentation-only cleanup PR that removes the consumed one-shot workflow and marker and archives this completed record.
```

## Notes

Issue #173 contains the sanitized exact-SHA build, dispatcher and deployment evidence. No production secret, DSM credential, database credential or private endpoint was committed or printed.
