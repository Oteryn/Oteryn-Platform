---
task_id: OTERYN-20260815-ecosystem-repository-architecture
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/README.md
search_first:
  - open architecture PRs and overlapping owned paths
  - PR #1065 public Map atlas integration
  - Issue #302 optional maps disposition
optional_reads: []
---

# OTERYN-20260815-ecosystem-repository-architecture

## Goal

Record the repository owner's corrected target Oteryn ecosystem repository topology and Atlas extraction boundary in Oteryn Platform as temporary cross-repository architecture authority until the future Oteryn meta repository exists.

## Acceptance criteria

- [x] Reconcile protected `main`, active task paths and open architecture PR ownership before editing.
- [x] Intentionally close draft PR #1065 because its canonical-Otheryn-producer premise is superseded by the repository owner's corrected Atlas decision.
- [x] Record an accepted ADR for the target `Oteryn`, `Oteryn-Game`, `Oteryn-Platform` and `Oteryn-Atlas` repository topology.
- [x] Record the ownership split between canonical world/OTBM migration concerns, Atlas product concerns and Platform public integration.
- [x] Record that the legacy `blakinio/Otheryn` project is a migration source, not a target architecture authority.
- [x] Update the ADR registry without reusing the abandoned `0038` draft identifier.
- [ ] Pass repository-required exact-head validation for the documentation-only change.
- [ ] Merge only after the exact-head merge gate is satisfied, then archive/release this task through repository closeout policy.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
modules:
  - architecture
  - repository-governance
dependencies:
  - accepted Platform ADR 0031 for native Oteryn-v2 versus legacy Canary compatibility boundaries
blockers:
  - none
cross_repository_tasks:
  - future migration task must inspect the legacy Atlas source before moving code/history
  - future Oteryn meta repository must adopt or supersede this temporary cross-repository authority
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
task_kind: implementation
implementation_authorized: true
updated_at: 2026-08-15T10:49:00Z
phase: validate
session_id: chat-20260815-ecosystem-architecture
session_role: implementer
execution_mode: github
execution_reason: bounded documentation-only architecture change using the GitHub connector
project_lane: oteryn-platform-core
head: d0dfcd250e1888cab9e00bfbdf0c2dda8f272af4
branch: agent/oteryn-20260815-ecosystem-repository-architecture
pr: 1096
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive cross-repository topology decision with one Platform ADR and registry update
validation_level: focused
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
invocation_started_at: 2026-08-15T10:31:00Z
last_progress_at: 2026-08-15T10:49:00Z
ci_checks_for_current_head: 0
ci_check_generation: current-base-repair
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
proven:
  - Protected main was reconciled at 536c6320f91d1df981600530e50522d84b1c0588 before the task branch was created.
  - Main advanced through PR #1086 to ee080d04a28eafa2934ad9912a359844befac9b2 after branch creation with no overlap in this task's three owned paths.
  - Main then advanced through lifecycle-only PR #1097 to 9f58beb5aef26bd8d3cc407c7559b4151d3d46d9; the archive change does not overlap this task's owned paths.
  - Main active task records at preflight do not claim this task's new task/ADR paths.
  - Open PR #1093 owns docs/architecture/ARCHITECTURE_AUTHORITY.md, so this task intentionally does not edit that shared authority index.
  - Draft PR #1065 owned docs/architecture/adr/README.md and proposed ADR 0038; it was intentionally closed unmerged on 2026-08-15 with Branch-Disposition: delete before this replacement task claimed the registry path.
  - PR #1065 had no submitted reviews or unresolved review threads when closed, and its source branch is now absent.
  - Issue #302 previously deferred interactive maps pending authoritative ownership/provenance; this owner decision now supplies the repository/product ownership direction without authorizing runtime implementation.
  - The repository owner states that the current OTBM Atlas lives in the legacy blakinio/Otheryn project, which is an old Canary/Crystal Server lineage and should be moved out rather than treated as the target Oteryn architecture.
  - ADR 0040 and the ADR registry update are persisted in PR #1096.
  - Exact-head self-review before CI found no material architecture/diff findings.
derived:
  - PR #1065's assumption that the legacy Otheryn repository should remain the canonical Atlas producer is no longer valid.
  - A dedicated Oteryn-Atlas product repository avoids coupling the browser map product to the legacy Canary/Crystal server repository while preserving a clean ownership boundary with the future native game/world implementation.
unknown:
  - Exact legacy Atlas source paths, commit boundaries and extraction history because this Platform-only task does not inspect external/server repositories.
  - Exact versioned Game-to-Atlas export schema and transport until a separately authorized producer/consumer contract task defines it.
  - Exact future GitHub organization handle availability and migration date.
conflicts:
  - closed PR #1065 historical draft versus the 2026-08-15 repository-owner Atlas decision; resolved by closing #1065 unmerged and replacing it with ADR 0040.
first_failure:
  marker: stale-base-ci-and-checkpoint-context-routes
  evidence: CI run 31880128280 failed because current main PR #1086 added a PHP coverage validator that the pre-#1086 branch head did not contain; Agent Governance run 31880128272 separately reported this task checkpoint missing required context_routes.
rejected_hypotheses:
  - Keep the Atlas permanently inside the legacy Otheryn/Canary/Crystal project; rejected by repository-owner decision.
  - Move the whole legacy Otheryn repository and rename it Oteryn-Atlas; rejected because the repository contains unrelated legacy game-server lineage and would preserve the wrong ownership boundary.
  - Put the Atlas implementation inside Oteryn-Platform; rejected because Platform should integrate the public product rather than own canonical world parsing/map-domain implementation.
  - Retry the failing CI unchanged; rejected because the logs identify deterministic stale-base and checkpoint-schema causes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
validation:
  - command: live repository/PR/path ownership reconciliation
    result: PASS
    evidence: predecessor PR #1065 closed unmerged; open architecture PR #1093 does not own this task's three paths
  - command: main delta 536c6320..9f58beb5
    result: PASS
    evidence: PR #1086 CI/workflow/test/governance work plus PR #1097 lifecycle archive; no task-owned path overlaps
  - command: full PR #1096 diff review
    result: PASS
    evidence: exactly three declared documentation paths; no runtime, workflow, deployment, secret or external-repository mutation
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this task changes architecture documentation only and creates no executable product or integration journey
  - command: CI run 31880128280 on d0dfcd250e1888cab9e00bfbdf0c2dda8f272af4
    result: FAIL
    evidence: classify-changes stopped at Validate PHP coverage policy because the stale branch did not contain tools/validation/test_php_coverage_policy.py added by current main PR #1086
  - command: Agent Governance run 31880128272 on d0dfcd250e1888cab9e00bfbdf0c2dda8f272af4
    result: FAIL
    evidence: checkpoint validator reported missing checkpoint field context_routes; live task ownership itself passed
blockers:
  - none
next_action: Merge current main 9f58beb5aef26bd8d3cc407c7559b4151d3d46d9 into the task branch while preserving the three owned documentation changes and add the required context_routes field, then rerun exact-head gates.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chat-20260815-ecosystem-architecture
  session_started_at: 2026-08-15T10:31:00Z
  checkpointed_at: 2026-08-15T10:49:00Z
  last_progress_at: 2026-08-15T10:49:00Z
  phase: validate
  exact_head: d0dfcd250e1888cab9e00bfbdf0c2dda8f272af4
  pull_request: 1096
  active_operation: evidence-based current-base and checkpoint-schema repair
  external_run_ids: [31880128280, 31880128272]
  operation_started_at: 2026-08-15T10:49:00Z
  wait_deadline_at: null
  check_generation: current-base-repair
  checks_used: 2
  status: active
  safe_to_resume: true
  resume_condition: task branch contains current main and the active checkpoint contains context_routes
  next_action: Merge current main into the task branch with the corrected task checkpoint, then inspect the new exact-head required-check generation.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and validating
source_branch_evidence: dedicated branch agent/oteryn-20260815-ecosystem-repository-architecture; PR #1096
```

## Notes

This task records architecture only. It does not create repositories, move Git history, access or mutate external/server repositories, change runtime code, deploy Atlas, modify Synology, or activate `/map` in production.
