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
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
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
- [x] Preserve the current `oteryn-synology-staging` runner online and unchanged during bootstrap/recovery execution.
- [ ] Record sanitized terminal live evidence without printing PATs, registration tokens, `.credentials`, or environment dumps.

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
updated_at: 2026-08-21T19:13:00+02:00
head_before_checkpoint: dc878739b01bbb6eadd787013d1ed3229dce5eb1
branch: infra/issue-1199-organization-runner-bootstrap
pr: 1200
status: implementing
phase: blank-token-force-recreate
session_id: chatgpt-20260821T190240+0200
session_role: implementation-owner
execution_mode: github-actions-on-existing-synology-runner
execution_reason: direct synology MCP probe returns HTTP 404; existing oteryn-synology-staging Actions runner remains the authorized live Synology execution path
project_lane: oteryn-platform-core
task_kind: implementation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
repair_cycles_for_current_gate: 2
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
  rationale: live self-hosted runner control and organization runner registration require rollback-safe, secret-safe exact-state verification
proven:
  - runner groups platform-runners=3, atlas-runners=4 and game-runners=5 exist with selected-repository policies
  - first controlled bootstrap run 32491656685 reached first-registration=PASS for all three replacement runners
  - hosted verification run 32491656769 job 96845414025 proved all three registrations, exact groups and custom labels; Platform and Atlas were then offline and Game reported online
  - recovery run 32506277679 executed on legacy oteryn-synology-staging, proving the rollback runner is still online and schedulable
  - recovery run 32506277679 restarted existing Platform and Atlas containers and then proved Platform, Atlas and Game all reported online
  - recovery run 32506277679 proved each runner belongs to its exact group and each group exposes exactly one selected repository: Platform, Atlas and Game respectively
  - no registration token value, PAT, `.credentials` content or environment dump was persisted in task evidence
  - temporary full bootstrap runs 32506192656 and 32506277688 were cancelled after an unintended PR-synchronize retrigger was detected
  - the temporary organization-runner-bootstrap workflow was removed at dc878739b01bbb6eadd787013d1ed3229dce5eb1 so future synchronizations cannot perform full registration again
  - cancelled bootstrap run 32506192656 created fresh organization-runner Compose volumes and containers before cancellation; cancellation occurred before its token-truncation/recreate phase
  - recovery run 32506277679 observed all three replacement containers as exact oteryn-organization-runners services; Platform and Atlas were recovered from Docker/API offline state without deleting persistent volumes
  - PR 1200 has no review submissions, review threads or comments claiming concurrent branch ownership
derived:
  - the remaining failure is credential-lifecycle cleanup, not group authorization or runner connectivity: recovery reached all-three-online + exact selected-repository proof before failing the blank-token assertion
  - the cancelled bootstrap left nonblank one-time token source files mounted into at least the first verified replacement container; those files must be replaced by durable zero-byte placeholders before terminal success
  - GitHub runner status alone can be stale during Docker start/recreate, so terminal proof must combine exact container identity/state, persisted `.runner`, changed container IDs, blank token mounts and a delayed organization API observation
  - future repair must not generate new registration tokens or delete the six config/work volumes
unknown:
  - whether Game completed first registration into its newly-created config volume before recovery run 32506277679; the next repair must start the exact container and wait for `.runner` before blank-token recreation
conflicts: []
first_failure:
  marker: replacement runner token lifecycle remained non-terminal after cancelled bootstrap recreation
  evidence: recovery job 96847044180 proved all three API routes online and exact selected repositories, then exited 1 at the first post-route container/token assertion; preceding bootstrap job 96846777755 had been cancelled after creating/starting containers and before token truncation
rejected_hypotheses:
  - organization runner groups were not created
  - selected-repository authorization is wrong
  - the organization admin API credential is unavailable
  - Platform and Atlas cannot reconnect after restart
  - labels alone are the authorization boundary
  - Remote Desktop is required for activation
validation:
  - command: Build Synology Staging Images on pre-recovery PR head
    result: PASS
    evidence: run 32491656692
  - command: organization runner activation
    result: PARTIAL
    evidence: run 32491656685 / job 96809723750; groups + first registration PASS, post-recreate convergence cancelled
  - command: hosted organization runner verification
    result: FAIL_SEMANTIC
    evidence: run 32491656769 / job 96845414025; workflow conclusion SUCCESS but Platform and Atlas were offline
  - command: bounded existing-registration recovery
    result: PARTIAL
    evidence: run 32506277679 / job 96847044180; all three routes online + exact selected repositories PASS, blank-token assertion FAIL
blockers:
  - none
next_action: execute one bounded force-recreate using only persisted `.runner` state and zero-byte steady-state token files; prove new container IDs, all three delayed online routes, exact selected-repository boundaries, Game least privilege, empty token mounts, old task-owned token cleanup and legacy runner preservation
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: chatgpt-20260821T190240+0200
  session_started_at: 2026-08-21T19:02:40+02:00
  checkpointed_at: 2026-08-21T19:13:00+02:00
  last_progress_at: 2026-08-21T19:12:03+02:00
  phase: blank-token-force-recreate
  exact_head: dc878739b01bbb6eadd787013d1ed3229dce5eb1
  pull_request: 1200
  active_operation: Organization Runner Repair V2 will start from the checkpoint commit synchronize event
  external_run_ids:
    - 32491656685
    - 32491656769
    - 32506192656
    - 32506277679
  operation_started_at: null
  wait_deadline_at: null
  check_generation: live-runner-recovery-2
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: PR 1200 branch remains owned by this task and legacy oteryn-synology-staging remains schedulable
  next_action: observe the single V2 repair run created by this checkpoint commit; inspect its first actionable failure only if it does not PASS
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation/activation branch
source_branch_evidence: pending
```

## Notes

Temporary activation, token-envelope, repair and branch-specific verification workflows must be removed from the terminal diff after live evidence is captured. Never print or persist the organization PAT, generated registration token values, `.credentials`, or unrestricted Docker inspection. Preserve all six runner config/work volumes and every unrelated Synology runtime resource. The steady-state empty token placeholder files are an intentional durable operational input for future Compose recreation; they contain no credential value.