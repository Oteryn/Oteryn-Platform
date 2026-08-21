---
task_id: OTERYN-20260821-organization-runner-bootstrap
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLEENESS_AND_CLOSEOUT.md
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
- [x] Create/normalize `platform-runners`, `atlas-runners`, and `game-runners` as selected-repository organization runner groups.
- [x] Register `oteryn-synology-platform`, `oteryn-synology-atlas`, and `oteryn-synology-game` with their exact group + custom label.
- [ ] Recreate replacement runner containers after first registration with blank one-time token files and prove they return online.
- [ ] Preserve the current `oteryn-synology-staging` runner online and unchanged during bootstrap.
- [ ] Record sanitized live evidence without printing PATs, registration tokens, `.credentials`, or environment dumps.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/**
  - .github/workflows/organization-runner-bootstrap.yml
  - .github/workflows/organization-runner-verify.yml
  - .github/workflows/organization-runner-repair.yml
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
checkpoint_version: 2
updated_at: 2026-08-21T19:02:40+02:00
head: 8b416c7898d4befdcadeeb03fde60e1ff5d57f9c
branch: infra/issue-1199-organization-runner-bootstrap
pr: 1200
status: implementing
phase: runner-restart-repair
session_id: chatgpt-20260821T190240+0200
session_role: implementation-owner
execution_mode: github-actions-on-existing-synology-runner
execution_reason: direct synology MCP probe currently returns HTTP 404, while the existing repository-scoped Synology Actions runner is a proven authorized execution path
project_lane: oteryn-platform-core
task_kind: implementation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: high
  triggers:
    - deployment
    - infrastructure
    - credentials
    - rollback
  unknown_or_conflict: []
  rationale: live self-hosted runner control and organization runner registration require rollback-safe, secret-safe verification
owned_paths:
  - deploy/synology/runner/**
  - .github/workflows/organization-runner-bootstrap.yml
  - .github/workflows/organization-runner-verify.yml
  - .github/workflows/organization-runner-repair.yml
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
proven:
  - live bootstrap run 32491656685 executed on legacy runner oteryn-synology-staging using Actions Runner 2.336.0
  - organization runner admin credential was accepted by the required organization runner-group and registration-token APIs without disclosure
  - bootstrap run 32491656685 created/normalized runner groups platform-runners=3, atlas-runners=4 and game-runners=5
  - bootstrap run 32491656685 pulled the immutable deploy-runner digest and created the oteryn-organization-runners Compose network, six distinct config/work volumes and three service containers
  - bootstrap run 32491656685 reached first-registration=PASS for all three replacement runners before token-file truncation and forced recreation
  - one-time token files were truncated before the forced recreation step
  - live hosted verification rerun 32491656769 job 96845414025 at 2026-08-21T16:59Z found all three registrations in the expected groups with the expected labels
  - that verification found oteryn-synology-platform offline, oteryn-synology-atlas offline and oteryn-synology-game online
  - PR 1200 has no review submissions, review threads or comments claiming current concurrent branch ownership
  - the previous task checkpoint and branch update exceeded the repository stale threshold before this recovery session
  - existing oteryn-staging is retained as the rollback execution path; no deletion or re-registration has been performed
  - official Actions Runner 2.336.0 Linux x64 release checksum is 04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d
  - current organization Compose target gives Platform and Atlas distinct config/work volumes and Docker access while Game remains non-root without Docker socket
derived:
  - the activation is partial rather than failed-before-mutation: organization groups, registrations and persistent runner volumes exist live
  - successful first registration plus post-recreate Platform/Atlas offline state isolates the remaining defect to restart/session/runtime behavior rather than group/token creation
  - current organization-runner-verify workflow is insufficient because it reports status but does not fail when a runner is offline
  - repeating the full registration bootstrap before inventory would be unsafe and unnecessary
unknown:
  - exact Docker state/restart count of the Platform and Atlas replacement containers after the cancelled bootstrap run
  - whether a bounded restart of the existing registered Platform and Atlas containers is sufficient to recover them without re-registration
  - current live online state of the legacy repository-scoped oteryn-synology-staging runner after the earlier bootstrap run
conflicts: []
first_failure:
  marker: post-recreate runner recovery did not converge
  evidence: bootstrap job 96809723750 emitted `unable to upgrade to tcp, received 409` after the three force-recreated services started, then the workflow timed out/cancelled; later verification shows Platform and Atlas offline while Game is online
rejected_hypotheses:
  - organization runner groups were not created
  - organization runner registration token API is unavailable
  - none of the replacement runners registered successfully
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
  - .github/workflows/organization-runner-verify.yml
validation:
  - command: Build Synology Staging Images on current PR head
    result: PASS
    evidence: run 32491656692
  - command: controlled organization runner activation
    result: PARTIAL
    evidence: run 32491656685 / job 96809723750; groups + first registration PASS, post-recreate convergence timed out
  - command: hosted organization runner verification rerun
    result: FAIL_SEMANTIC
    evidence: run 32491656769 / job 96845414025; workflow conclusion SUCCESS but Platform and Atlas status are offline
blockers:
  - none
next_action: create one bounded temporary repair workflow on legacy oteryn-staging that inventories only the exact oteryn-organization-runners services, restarts only existing offline Platform/Atlas runner containers without deleting volumes or re-registering them, verifies all three organization runners online through the organization API, and verifies legacy runner preservation
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260821T190240+0200
  session_started_at: 2026-08-21T19:02:40+02:00
  checkpointed_at: 2026-08-21T19:02:40+02:00
  last_progress_at: 2026-08-21T18:59:12+02:00
  phase: runner-restart-repair
  exact_head: 8b416c7898d4befdcadeeb03fde60e1ff5d57f9c
  pull_request: 1200
  active_operation: none
  external_run_ids:
    - 32491656685
    - 32491656769
  operation_started_at: null
  wait_deadline_at: null
  check_generation: live-runner-recovery-1
  checks_used: 1
  status: ready
  safe_to_resume: true
  resume_condition: PR 1200 branch identity is unchanged and no conflicting worker owns the same branch or runner estate
  next_action: create and execute the bounded existing-registration repair probe described by the task checkpoint; do not rerun full registration first
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation/activation branch
source_branch_evidence: pending
```

## Notes

The temporary activation/repair workflows must be removed from the terminal diff after live evidence is captured. Never print or persist the organization PAT or generated registration token values. Preserve every runner config/work volume and all product persistent data unless a later exact cleanup action is separately proven safe.