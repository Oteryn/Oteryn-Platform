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

- [ ] Bootstrap supports explicit repository and organization registration modes.
- [ ] Organization mode requires organization URL, explicit runner group and strict custom labels.
- [ ] Existing registered runner restarts without a token/re-registration.
- [ ] Default labels remain disabled.
- [ ] Actions Runner distribution and Docker/base image inputs are immutable/pinned and Node 24 presence is verified at image build.
- [ ] Entrypoint contract self-test runs during image build.
- [ ] Organization target has separate Platform/Atlas/Game config and work volumes.
- [ ] Platform state is not mounted into Atlas/Game.
- [ ] Game does not receive raw Docker socket by default.
- [ ] Activation runbook preserves current `oteryn-staging` as rollback until all provider routes pass.
- [ ] No live runner, Docker resource, product state, secret, runner group or registration token is mutated by this preparation PR.

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
updated_at: 2026-08-21T08:32:00Z
head: 3f1a0eeb42a777106bef466dbcb4150d8a1bb818
branch: infra/issue-1199-organization-runner-bootstrap
pr: none
status: implementing
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
derived:
  - building the Oteryn runner from the checksum-pinned official release tarball avoids the known container packaging gap and removes the mutable actions-runner:latest base
  - organization-group registration can be prepared without altering the current repository-scoped runner because an existing .runner file bypasses first-registration logic
unknown:
  - exact organization runner groups do not yet exist
  - organization registration token mutation is not exposed by the current connected GitHub surface
  - Game host-Docker requirement remains provider-owned and is therefore omitted from the prepared default
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - labels alone provide the organization authorization boundary
  - separate containers sharing raw Docker socket create a hard host security boundary
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
  - command: pending exact-head repository validation
    result: NOT_RUN
    evidence: implementation candidate is being assembled
blockers:
  - none
next_action: commit the coherent preparation candidate, open Draft PR, and run exact-head validation without activating any live runner
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation branch
source_branch_evidence: pending
```

## Notes

Activation is a later controlled rollout. This task prepares code/config/runbook only and must not consume or expose one-time registration tokens.
