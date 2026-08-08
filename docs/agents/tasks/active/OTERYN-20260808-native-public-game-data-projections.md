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

- [x] Projection families and explicit exclusions are catalogued without duplicating runtime-status or Game Catalog authority.
- [x] Authoritative source owner, canonical identities and Platform projection/presentation ownership are explicit per family.
- [x] Event/snapshot/query evidence semantics define identity, source revision/version, applicability, ordering and provenance without inventing one global stream.
- [x] At-least-once/repeated delivery is safe through idempotent application and duplicate/replay handling.
- [x] Fresh/stale/unavailable/invalid/empty/not-found semantics are distinguishable and ordinary public reads do not synchronously depend on the game runtime.
- [x] Tombstone, rename, transfer, hide/privacy and lifecycle reconciliation semantics prevent stale public authority.
- [x] Platform CharacterProfiles/privacy policy remains an independent upper-bound presentation overlay.
- [x] Rebuild, backfill, projection generations, replay/tail, bounded reconciliation, gap detection and poison-event behavior are explicit.
- [x] Legacy Canary reads are compatibility-only and cut over per projection family with provenance and rollback.
- [x] `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` remains authoritative for world/channel runtime observations/capacity aggregates.
- [x] Game Catalog/content facts remain routed to their existing content/catalogue authority.
- [ ] Exact-head self-review, required CI and review-thread hygiene pass.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: this task changes architecture/documentation only and implements no executable producer, worker, schema, public route or cutover.

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
  - OTERY​N_V2_RUNTIME_STATUS_PROJECTION_CONTRACT
blockers: []
cross_repository_tasks: []
forbidden_paths:
  - app/**
  - database/**
  - .github/workflows/**
  - deploy/**
  - external repositories
```

## Exact-head self-review

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
    - exactly three owned documentation/task/report paths changed
    - five projection families and explicit runtime-status/Game Catalog exclusions are defined
    - current Canary direct reads are compatibility-only and no native shared-SQL authority is introduced
    - canonical identity, idempotency, ordering, freshness, lifecycle, privacy, rebuild, reconciliation and per-family rollback semantics are explicit
    - native guild identity uncertainty remains explicit and implementation-gating rather than being filled with a mutable name or Canary numeric ID
    - no runtime/schema/worker/workflow/deployment/external-repository path changed
```

The later task-checkpoint commit is governance-only and does not alter the reviewed architecture contract/report semantics.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-08T11:24:00+02:00
head: d7553a92fa8537c8f11c4995d95e3eb258a0e4ef
branch: docs/OTERYN-20260808-native-public-game-data-projections
pr: 903
status: validating
phase: validate
execution_mode: github_only
invocation_started_at: 2026-08-08T10:39:00+02:00
last_progress_at: 2026-08-08T11:24:00+02:00
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
ci_check_generation: ready
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
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
  - ADR 0031 requires native Oteryn-v2 game facts to cross through explicit contracts rather than shared tables or cross-system SQL.
  - OTERY​N_V2_RUNTIME_STATUS_PROJECTION_CONTRACT already owns WorldId/ChannelId runtime observations, readiness, freshness, capacity/player-count aggregate truthfulness and public status semantics.
  - delivered PublicGameData currently reads Canary players, deaths, guild, house, channel and cluster-session tables directly through compatibility repositories/queries.
  - delivered PublicCharacterProfileService overlays Platform-owned CharacterProfiles privacy/presentation preferences over Canary game facts.
  - no open Issue or PR owned this exact native generic PublicGameData projection/reconciliation contract before Issue 902; Issue 487 is current-surface evidence, character lifecycle Issues own mutations, and programme 330 owns Game Catalog.
  - the focused contract defines character facts/search, rankings, activity, guild membership and individual presence with explicit rebuild/freshness/migration semantics.
derived:
  - native public game facts need a rebuildable Platform read-model contract that preserves game authority while allowing the website to serve last-known-good state independently of synchronous game-runtime availability.
  - privacy/presentation policy remains a Platform overlay instead of becoming game-owned public exposure authority.
unknown:
  - exact Oteryn-v2 producer API/event/snapshot schema and transport
  - exact broker or delivery infrastructure
  - exact native canonical guild identifier representation
  - exact per-family numeric freshness budgets
  - exact Platform projection storage schema and worker framework
conflicts: []
first_failure:
  marker: none-observed
  evidence: overlap/source-of-truth audit and exact architecture diff review found no unresolved material contradiction
rejected_hypotheses:
  - runtime-status contract should be duplicated inside generic PublicGameData projections
  - Game Catalog content facts should be absorbed into gameplay-state projections
  - current Canary numeric IDs or direct SQL are the native source-of-truth design
  - public projection records may bypass Platform CharacterProfiles privacy policy
  - exactly-once transport or one global ordering stream is required
changed_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-public-game-data-projections.md
  - docs/agents/tasks/active/OTERYN-20260808-native-public-game-data-projections.md
validation:
  - command: overlap and source-of-truth audit
    result: PASS
    evidence: Issue 902 owns the new focused boundary; current runtime-status, lifecycle, browser-evidence and Game Catalog owners remain disjoint
  - command: exact content-head full-diff architecture self-review
    result: PASS
    evidence: d7553a92fa8537c8f11c4995d95e3eb258a0e4ef has exactly three intended paths and zero unresolved material findings
  - command: runtime/browser E2E
    result: NOT_RUN
    evidence: architecture/documentation-only task; no executable producer, worker, schema, route or cutover exists
blockers: []
next_action: Mark PR 903 ready and observe the repository-selected exact-head CI; merge only if required checks pass and review hygiene remains clean.
```
