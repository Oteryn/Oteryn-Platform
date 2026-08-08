---
task_id: OTERYN-20260808-native-public-game-data-projections
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-content
issue: 902
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
optional_reads:
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - app/PublicGameData/GuildIndexQuery.php
---

# OTERYN-20260808 native PublicGameData projections

## Goal

Define the canonical Platform-side native Oteryn-v2 PublicGameData projection/event/reconciliation boundary for public game-state families that are not already owned by the focused runtime-status or Game Catalog contracts.

## Delivery classification

```yaml
task_kind: implementation
delivery_type: architecture_documentation
implementation_authorized: true
runtime_implementation_authorized: false
external_repository_writes_authorized: false
production_mutation_authorized: false
decomposition_decision: single
decomposition_reason: one cohesive Platform consumer contract with shared freshness, rebuild, reconciliation, privacy and legacy-cutover semantics
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
execution_mode: github_only
execution_reason: narrow architecture/documentation edits and GitHub Actions validation do not require a local checkout
```

## Acceptance criteria

- [x] Character facts/search, rankings, activity, guild membership and individual presence projection families are catalogued.
- [x] Runtime-status and Game Catalog/content authority are referenced rather than duplicated.
- [x] Source owner, canonical identity and Platform projection/presentation ownership are explicit.
- [x] Version/revision/applicability/ordering/provenance and at-least-once-safe idempotency semantics are explicit.
- [x] Fresh/stale/unavailable/invalid versus empty/not-found semantics and last-known-good website behavior are explicit.
- [x] Rename/delete/restore/transfer/tombstone/privacy reconciliation is explicit.
- [x] CharacterProfiles/Identity privacy remains an independent Platform upper-bound overlay.
- [x] Generation rebuild, replay/tail, high-watermark, gap, poison/quarantine, reconciliation and rollback semantics are explicit.
- [x] Canary SQL/Redis reads remain Legacy Canary Compatibility and cut over per family without silent mixed authority.
- [ ] Exact-head required CI and final review-thread hygiene pass.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: no executable producer, worker, schema, route or cutover changes.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-public-game-data-projections.md
  - docs/agents/tasks/active/OTERYN-20260808-native-public-game-data-projections.md
modules:
  - PublicGameData
  - CharacterProfiles
  - architecture-governance
dependencies:
  - Issue #902
  - ADR 0031
  - OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT
blockers: []
cross_repository_tasks: []
forbidden_paths:
  - app/**
  - database/**
  - .github/workflows/**
  - deploy/**
  - external repositories
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: d7553a92fa8537c8f11c4995d95e3eb258a0e4ef
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - three intended owned documentation/task/report paths only
    - no native shared-SQL or synchronous runtime fallback authority introduced
    - runtime-status and Game Catalog ownership remain disjoint
    - native guild identity uncertainty is explicit and implementation-gating rather than invented from name or Canary ID
    - no runtime/schema/worker/workflow/deployment/external-repository path changed
```

Later task-checkpoint commits are governance-only and do not change the reviewed architecture contract/report semantics.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-08T11:26:00+02:00
head: d73f3ef1adf01a58676c044ecb287e7388aa2e9b
branch: docs/OTERYN-20260808-native-public-game-data-projections
pr: 903
status: validating
phase: validate
execution_mode: github_only
invocation_started_at: 2026-08-08T10:39:00+02:00
last_progress_at: 2026-08-08T11:26:00+02:00
lease_expires_at: 2026-08-08T12:03:00+02:00
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: full
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
ci_checks_for_current_head: 0
ci_check_generation: repair-1
terminal_ci_wait_started_at: 2026-08-08T11:27:00+02:00
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
context_routes:
  - architecture
  - data-ownership
  - security
  - public-api
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-public-game-data-projections.md
  - docs/agents/tasks/active/OTERYN-20260808-native-public-game-data-projections.md
proven:
  - ADR 0031 requires explicit native contracts rather than shared tables/cross-system SQL.
  - runtime-status projection already owns WorldId/ChannelId health/readiness/freshness and aggregate capacity/player-count truthfulness.
  - delivered PublicGameData uses Canary compatibility reads; PublicCharacterProfileService overlays Platform privacy/presentation state.
  - Issue #902 is non-overlapping with #487 current-surface evidence, character lifecycle mutation Issues and Game Catalog #330.
  - focused contract defines five projection families, source evidence, idempotency, freshness, lifecycle, privacy, rebuild/reconciliation and per-family migration/rollback.
derived:
  - ordinary website reads can remain independent of synchronous game-runtime availability through truthful last-known-good Platform projections.
unknown:
  - exact Oteryn-v2 producer API/event/snapshot schema and transport
  - exact broker/delivery infrastructure
  - exact native canonical guild identifier representation
  - exact per-family numeric freshness budgets
  - exact Platform storage/worker implementation
conflicts: []
first_failure:
  marker: checkpoint-validation-result-vocabulary
  evidence: exact-head generation on d73f3ef1adf01a58676c044ecb287e7388aa2e9b rejected unsupported validation result PASS_PARTIAL; architecture/native protocol checks were otherwise not implicated
rejected_hypotheses:
  - runtime-status should be duplicated inside generic PublicGameData
  - Game Catalog facts should be absorbed into gameplay-state projections
  - Canary IDs/direct SQL are the native source-of-truth design
  - privacy policy may be bypassed by game-originated public facts
  - exactly-once transport or one global ordering stream is required
changed_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-public-game-data-projections.md
  - docs/agents/tasks/active/OTERYN-20260808-native-public-game-data-projections.md
validation:
  - command: overlap and source-of-truth audit
    result: PASS
    evidence: focused boundary is disjoint from current runtime-status, lifecycle, browser-evidence and Game Catalog owners
  - command: exact architecture content-head self-review
    result: PASS
    evidence: d7553a92fa8537c8f11c4995d95e3eb258a0e4ef had exactly three intended paths and zero unresolved material findings
  - command: exact-head generation d73f3ef1adf01a58676c044ecb287e7388aa2e9b
    result: FAIL
    evidence: Agent Governance/CI checkpoint validator rejected the unsupported result label PASS_PARTIAL; first failure is repaired by this commit
  - command: runtime/browser E2E
    result: NOT_RUN
    evidence: architecture/documentation-only task; no executable producer, worker, schema, route or cutover exists
blockers: []
next_action: Observe one aggregate required-check snapshot for the repaired exact head; merge only if all required checks pass and review hygiene remains clean.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: github-20260808-1039-public-game-data
  session_started_at: 2026-08-08T10:39:00+02:00
  checkpointed_at: 2026-08-08T11:26:00+02:00
  last_progress_at: 2026-08-08T11:26:00+02:00
  phase: validate
  exact_head: d73f3ef1adf01a58676c044ecb287e7388aa2e9b
  pull_request: 903
  active_operation: repaired exact-head required CI and merge gate
  external_run_ids:
    - 31250495901
    - 31250495921
    - 31250495881
    - 31250495891
    - 31250495880
    - 31250495899
    - 31250495904
    - 31250495909
  operation_started_at: 2026-08-08T11:26:00+02:00
  wait_deadline_at: 2026-08-08T11:38:00+02:00
  check_generation: repair-1
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: PR 903 remains mergeable on the repaired head with no new review finding and all required checks complete successfully
  next_action: Inspect one aggregate required-check snapshot for the repaired head; repair a new first material failure if present, otherwise merge and perform closeout.
```
