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
- [ ] Recreate replacement runner containers with blank steady-state token files and prove they return online from persisted registration state.
- [x] Preserve the current `oteryn-synology-staging` runner online and unchanged during bootstrap/recovery execution.
- [ ] Record sanitized terminal live evidence without printing PATs, registration tokens, `.credentials`, or environment dumps.

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
blockers: []
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-21T17:17:27Z
head: 0644a53ee3eae85c7cab201209135924b59dc6f1
branch: infra/issue-1199-organization-runner-bootstrap
pr: 1200
status: validating
context_routes:
  - github-only-execution
  - synology-staging
  - organization-runner-recovery
owned_paths:
  - deploy/synology/runner/**
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
proven:
  - runner groups platform-runners, atlas-runners and game-runners exist with selected-repository policies
  - controlled bootstrap run 32491656685 reached first-registration=PASS for all three replacement runners
  - hosted verification run 32491656769 job 96845414025 proved all three registrations, exact groups and custom labels; Platform and Atlas were then offline and Game reported online
  - recovery run 32506277679 executed on legacy oteryn-synology-staging, proving the rollback runner is still online and schedulable
  - recovery run 32506277679 restarted existing Platform and Atlas containers and then proved Platform, Atlas and Game all reported online
  - recovery run 32506277679 proved each runner belongs to its exact group and each group exposes exactly one selected repository: Platform, Atlas and Game respectively
  - temporary full-bootstrap runs 32506192656 and 32506277688 were cancelled after an unintended pull-request synchronize retrigger was detected
  - the temporary organization-runner-bootstrap workflow was removed at dc878739b01bbb6eadd787013d1ed3229dce5eb1 so future synchronizations cannot perform full registration again
  - all temporary token-envelope, hosted-verify and repair workflows were removed from the current branch at 0644a53ee3eae85c7cab201209135924b59dc6f1
  - no registration token value, PAT, .credentials content or unrestricted environment dump was persisted in task evidence
derived:
  - organization authorization and runner connectivity are proven; the remaining live gate is blank-token steady-state force-recreation
  - cancelled bootstrap run 32506192656 left transient registration-token files mounted into replacement containers because cancellation occurred before its truncation phase
  - GitHub runner API status alone is insufficient terminal evidence, so the active recovery also checks exact container identity, persisted .runner state, changed container IDs, empty token mounts and delayed organization API status
unknown:
  - final result of bounded recovery run 32507383363 on repair head 39b26cdf7f72feed8d774b628e3bd2a8c2436c10
conflicts: []
first_failure:
  marker: replacement runner token lifecycle remained non-terminal after cancelled bootstrap recreation
  evidence: recovery job 96847044180 proved all three API routes online and exact selected repositories, then failed the first post-route blank-token assertion; preceding bootstrap job 96846777755 had been cancelled before token truncation
rejected_hypotheses:
  - organization runner groups were not created
  - selected-repository authorization is wrong
  - the organization admin API credential is unavailable
  - Platform and Atlas cannot reconnect after restart
  - labels alone are the authorization boundary
  - Remote Desktop is required for activation
changed_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260821-organization-runner-bootstrap.md
validation:
  - command: Build Synology Staging Images on current durable branch generation 0644a53
    result: PASS
    evidence: run 32507505835 completed success
  - command: controlled organization runner activation
    result: FAIL
    evidence: run 32491656685 made first registration pass for all three runners but its post-recreate convergence was cancelled and therefore was not terminal
  - command: hosted organization runner verification
    result: FAIL
    evidence: run 32491656769 job 96845414025 reported Platform and Atlas offline despite workflow conclusion success, exposing a semantic gap in the temporary verifier
  - command: bounded existing-registration recovery
    result: FAIL
    evidence: run 32506277679 job 96847044180 proved all three routes online and exact selected repositories but failed the blank-token assertion
blockers: []
next_action: Observe bounded recovery run 32507383363 to completion and record its exact blank-token force-recreate result before changing any live Synology runner state again.
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: high
  triggers:
    - deployment
    - infrastructure
    - credentials
    - rollback
  unknown_or_conflict:
    - final result of bounded recovery run 32507383363
  rationale: live self-hosted runner control and organization runner registration require rollback-safe and secret-safe exact-state verification
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 4
  session_id: chatgpt-20260821T190240+0200
  session_started_at: 2026-08-21T19:02:40+02:00
  checkpointed_at: 2026-08-21T17:17:27Z
  phase: blank-token-force-recreate
  exact_head: 0644a53ee3eae85c7cab201209135924b59dc6f1
  pull_request: 1200
  active_operation: Organization Runner Repair V2 run 32507383363 from immutable repair head 39b26cdf7f72feed8d774b628e3bd2a8c2436c10
  external_run_ids:
    - 32491656685
    - 32491656769
    - 32506192656
    - 32506277679
    - 32507383363
  operation_started_at: 2026-08-21T17:17:27Z
  check_generation: live-runner-recovery-2
  checks_used: 2
  status: waiting
  safe_to_resume: true
  resume_condition: inspect run 32507383363 only; do not recreate or re-register runners independently while it is active
  next_action: consume the terminal result and first actionable failure if any
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded Platform provider preparation and activation branch
source_branch_evidence: pending
```

## Notes

The temporary activation, token-envelope, repair and branch-specific verification workflows have been removed from the current diff. Never print or persist the organization PAT, generated registration token values, `.credentials`, or unrestricted Docker inspection. Preserve all six runner config/work volumes and every unrelated Synology runtime resource. The steady-state empty token placeholder files are intentional durable operational inputs for future Compose recreation; they contain no credential value.