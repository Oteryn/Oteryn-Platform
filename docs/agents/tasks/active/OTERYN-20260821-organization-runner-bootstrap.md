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

Prepare and activate the organization-managed, product-isolated Synology runner estate required by `Oteryn/Oteryn#34`, while preserving the existing `oteryn-staging` runner as rollback until replacement registration and scheduling are proven.

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
- [ ] Create/normalize `platform-runners`, `atlas-runners`, and `game-runners` as selected-repository organization runner groups.
- [ ] Register `oteryn-synology-platform`, `oteryn-synology-atlas`, and `oteryn-synology-game` with their exact group + custom label.
- [ ] Recreate replacement runner containers after first registration with blank one-time token files and prove they return online.
- [ ] Preserve the current `oteryn-synology-staging` runner online and unchanged during bootstrap.
- [ ] Record sanitized live evidence without printing PATs, registration tokens, `.credentials`, or environment dumps.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/**
  - .github/workflows/organization-runner-bootstrap.yml
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
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-21T09:12:00Z
head: 41eb8b6761e205ccd136ece7454444cce249d1ad
branch: infra/issue-1199-organization-runner-bootstrap
pr: 1200
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
  - ci-repair
owned_paths:
  - deploy/synology/runner/**
  - .github/workflows/organization-runner-bootstrap.yml
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
proven:
  - live current runner is Actions Runner 2.336.0 and uses a persistent registration config volume
  - current runner is root with RW Docker socket and Platform staging-state access
  - corrected organization audit requires product-owned groups for Platform, Atlas and Game
  - GitHub supports --runnergroup for organization runner registration and workflow group+label routing
  - fine-grained organization Self-hosted runners write permission is sufficient for runner-group and organization registration-token APIs
  - owner placed the bounded organization runner admin credential in repository Actions secret OTERYN_ORG_RUNNER_ADMIN_TOKEN and explicitly requested autonomous execution
  - official Actions Runner 2.336.0 Linux x64 release checksum is 04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d
  - Draft PR 1200 owns this branch and exact-head runner image build has passed previously
  - current organization Compose target uses file-backed one-time registration tokens and Game defaults to non-root without Docker socket
derived:
  - a bounded same-repository PR workflow guarded to this exact internal branch can use the existing privileged runner as bootstrap without exposing token values
  - runner-group selected-repository access is the authorization boundary; labels remain routing only
unknown:
  - whether the newly supplied fine-grained PAT is accepted by all required organization runner APIs
  - whether the three replacement runner containers register and return online on first live attempt
conflicts: []
first_failure:
  marker: none
  evidence: activation has not run yet
rejected_hypotheses:
  - labels alone provide the organization authorization boundary
  - separate containers sharing raw Docker socket create a hard host security boundary
  - Remote Desktop is required for activation
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
  - .github/workflows/organization-runner-bootstrap.yml
validation:
  - command: prior Build Synology Staging Images exact-head candidate
    result: PASS
    evidence: deploy-runner image and Synology package validation passed before activation
  - command: controlled organization runner activation
    result: NOT_RUN
    evidence: temporary guarded workflow will perform first live activation
blockers:
  - none
next_action: add the guarded temporary organization-runner bootstrap workflow and inspect its live result; rollback only task-owned replacement runners on task-owned failure
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation/activation branch
source_branch_evidence: pending
```

## Notes

The temporary activation workflow must be removed from the terminal diff after live evidence is captured. Never print or persist the organization PAT or generated registration token values.