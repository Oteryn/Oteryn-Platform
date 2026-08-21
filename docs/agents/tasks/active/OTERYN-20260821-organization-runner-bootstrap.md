---
task_id: OTERYN-20260821-organization-runner-bootstrap
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
search_first:
  - deploy/synology/runner
  - oteryn-staging
  - runnergroup
optional_reads: []
---

# OTERYN-20260821-organization-runner-bootstrap

## Goal

Prepare the Platform-owned generic runner image/bootstrap and no-secret three-runner Compose target required by `Oteryn/Oteryn#34`, without mutating the live runner registration or Synology runtime during preparation.

## Acceptance criteria

- [x] Bootstrap supports explicit repository and organization registration modes.
- [x] Organization mode requires organization URL, explicit runner group, runner name and strict custom labels.
- [x] Existing registered runner restarts without a token/re-registration.
- [x] Default labels remain disabled.
- [x] Actions Runner distribution and Docker/base image inputs are immutable/pinned and Node 24 presence is verified at image build.
- [x] Entrypoint contract self-test runs during image build.
- [x] Organization target has separate Platform/Atlas/Game config and work volumes.
- [x] Platform state is not mounted into Atlas/Game.
- [x] Game does not receive raw Docker socket or root by default.
- [x] Activation runbook preserves current `oteryn-staging` as rollback until all provider routes pass.
- [x] No live runner, Docker resource, product state, secret, runner group or registration token is mutated by this preparation PR.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/**
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
  - docs/agents/tasks/archive/OTERYN-20260821-organization-runner-bootstrap.md
modules:
  - synology-runner-bootstrap
  - github-actions-runner-routing
dependencies:
  - Oteryn/Oteryn#34
  - Oteryn/Oteryn#32
  - Oteryn/Oteryn-Platform#1194
blockers:
  - organization runner groups and registration token surface are intentionally not required for preparation
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-21T08:42:00Z
head: c7ea1270b227cf6f4a69562d4ddccc63b2f50b35
branch: infra/issue-1199-organization-runner-bootstrap
pr: 1200
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - deploy/synology/runner/**
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
proven:
  - live current runner is Actions Runner 2.336.0 and uses a persistent registration config volume
  - current runner is root with RW Docker socket and Platform staging-state access
  - corrected organization audit requires product-owned groups for Platform, Atlas and Game
  - GitHub supports --runnergroup for organization runner registration and workflow group+label routing
  - official Actions Runner 2.336.0 Linux x64 release checksum is 04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d
  - official 2.336.0 release tarball includes Node 24 while the published actions-runner 2.336.0 container has a reported Node 24 packaging gap
  - Draft PR 1200 owns this preparation branch
  - Synology Rollback Contract runs 32463861583 and 32464070075 passed
  - fresh Agent Governance run 32464070104 proves this task's PR identity repair passed live ownership validation; only the unrelated stale Atlas deployment lifecycle record for merged PR 1192 remains
  - full-diff self-review found and this candidate repairs three runner-bootstrap-owned hardening gaps: overridable runner distribution build args, organization registration tokens in container environment metadata, and incomplete negative routing validation
  - organization token values are now file-backed Compose secrets and Game defaults to non-root without Docker socket
  - preparation has not mutated a live runner, Docker resource, organization runner group, registration token, product state or protected setting
derived:
  - source-fixed runner version/checksum plus pinned base manifests close the mutable runner-build input path for ordinary reviewed builds
  - organization-group registration can be prepared without altering the current repository-scoped runner because an existing .runner file bypasses first-registration logic
unknown:
  - exact organization runner groups do not yet exist
  - organization registration token mutation is not exposed by the current connected GitHub surface
  - Build Synology Staging Images and required CI terminal results for this hardening generation are not yet observed
conflicts: []
first_failure:
  marker: Agent Governance run 32463861627
  evidence: own branch_pr_identity_omitted was repaired; fresh run 32464070104 leaves only unrelated PR 1192 lifecycle drift
rejected_hypotheses:
  - labels alone provide the organization authorization boundary
  - separate containers sharing raw Docker socket create a hard host security boundary
  - Agent Governance failure proves a runner-bootstrap implementation defect
changed_paths:
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/test-entrypoint.sh
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/compose.organization.example.yml
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
validation:
  - command: Synology Rollback Contract 32464070075
    result: PASS
    evidence: exact-head c7ea1270 generation completed successfully before self-review hardening
  - command: Agent Governance 32464070104
    result: FAIL
    evidence: task-owned live ownership validation passed; only unrelated stale PR 1192 lifecycle state remains
  - command: full exact PR diff self-review
    result: PASS
    evidence: hardening gaps isolated and repaired in the next coherent candidate; exact-head CI pending
blockers:
  - repository-wide Agent Governance remains red from the pre-existing Atlas deployment lifecycle defect owned by Platform 1191/PR 1193; do not repair that unrelated task in this branch
next_action: validate the hardened exact head with Build Synology Staging Images and CI; repair only runner-bootstrap-owned failures and keep Draft while unrelated lifecycle state remains invalid
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation branch
source_branch_evidence: pending
```

## Notes

Activation is a later controlled rollout. This task prepares code/config/runbook only and must not consume or expose one-time registration tokens.
