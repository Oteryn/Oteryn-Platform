---
task_id: OTERYN-20260808-character-lifecycle-authority-reconciliation
repository: blakinio/Oteryn-Platform
issue: 890
project_lane: oteryn-platform-core
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
search_first:
  - character lifecycle
  - Canary rename
  - deletion restore
  - world transfer
optional_reads: []
---

# OTERYN-20260808 character lifecycle authority reconciliation

## Goal

Repair stale Platform backlog and documentation so future native character lifecycle work follows accepted ADR 0030/0031 authority: Oteryn Platform owns authenticated UX/orchestration/business workflow while Oteryn-v2 Character Authority owns canonical CharacterId/current ownership and native create/rename/delete/restore/world-transfer/account-transfer mutations and authoritative results/receipts.

Preserve Canary evidence only as explicit Legacy Canary Compatibility / migration evidence. Do not mutate Canary, Oteryn-v2, runtime code, schemas, workflows, credentials, deployment or production state.

## Acceptance criteria

- [x] Issues #277, #317, #319, #320, #324 and #344 no longer route target-native lifecycle work through direct Platform-to-Canary mutation authority.
- [x] Native target semantics route create/rename/delete/restore/world-transfer/account-transfer to Oteryn-v2 Character Authority through versioned game-owned commands/receipts without inventing unfinished wire/transport details.
- [x] Existing Canary deletion discovery remains preserved and explicitly classified as Legacy Canary Compatibility evidence only.
- [x] Future direct/shared SQL and least-privilege Canary principals are not presented as native steady-state design.
- [x] Legacy Canary compatibility work requires a separately explicit compatibility scope and cannot block or define native Oteryn-v2 lifecycle work.
- [x] No runtime/schema/workflow/deployment/credential/production or external-repository mutation occurs.
- [ ] Exact-head Agent Governance and repository-selected CI pass; runtime/browser E2E is NOT_APPLICABLE for documentation/governance-only work.
- [ ] Full exact-head diff self-review finds zero material findings and zero unresolved review threads.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
  - docs/architecture/character-lifecycle/**
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
modules:
  - Characters
  - Accounts
  - Integration
  - architecture-governance
dependencies:
  - Issue #890
  - ADR 0030
  - ADR 0031
blockers:
  - none
cross_repository_tasks:
  - none; Canary and Oteryn-v2 are read-only and receive no writes
```

Shared paths named by Issue #890 (`docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` and `docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md`) are intentionally not claimed or edited because Issue #888 owns overlapping pre-admission/integration work on its separate branch.

## Execution profile

```yaml
policy_version: 2
task_kind: implementation
implementation_authorized: true
complete_user_facing_feature: false
execution_mode: github_only
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive documentation-authority repair with no runtime or external-repository mutation
validation_level: heightened-documentation
```

## Resulting authority reconciliation

- Added `docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md` as a routing guide subordinate to ADR 0030/0031.
- Reclassified `docs/contracts/CHARACTER_DELETION_CONTRACT.md` as Legacy Canary Compatibility discovery rather than native lifecycle authority.
- Updated #277, #317, #319 and #320 so native lifecycle work uses canonical AccountId/CharacterId and game-owned command/result semantics.
- Closed #324 and #344 as `not_planned` for the native target while preserving their value as optional Legacy Canary Compatibility evidence.
- #317 is no longer blocked by #344 for native deletion/restore; #319 is no longer dependent on #324 for native rename.
- No external-repository write occurred.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T09:54:00+02:00
head: pending-checkpoint-commit
branch: repair/issue-890
pr: 893
status: validating
phase: validate
execution_mode: github_only
invocation_started_at: 2026-08-08T09:40:00+02:00
last_progress_at: 2026-08-08T09:54:00+02:00
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
context_routes:
  - architecture
  - accounts-characters
  - canary-integration
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
  - docs/architecture/character-lifecycle/**
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
proven:
  - ADR 0030 makes Oteryn-v2 Character Authority authoritative for canonical CharacterId, current AccountId-to-CharacterId ownership and native character lifecycle mutations while Platform orchestrates approved commands.
  - ADR 0031 classifies direct/shared Canary SQL and numeric Canary identities as Legacy Canary Compatibility or migration state, not native steady-state integration.
  - Issues 277, 317, 319 and 320 now explicitly route native target mutations through Oteryn-v2 Character Authority and canonical native identities.
  - Issues 324 and 344 are closed not_planned as obsolete native prerequisites and explicitly preserve only Legacy Canary Compatibility evidence/reopen conditions.
  - CHARACTER_DELETION_CONTRACT.md now distinguishes native Character Authority from optional Canary compatibility semantics and states that Issue 344 does not block native Issue 317.
  - Issue 888 owns pre-admission/session handoff; this task did not edit its shared integration/programme paths.
derived:
  - The corrected backlog can now be selected by future native workers without interpreting Canary SQL as the canonical target mutation design.
unknown:
  - Exact Oteryn-v2 lifecycle command schemas, transport, game-internal state machines and receipt wire format remain external authority and are deliberately not invented.
conflicts: []
first_failure:
  marker: stale-backlog-authority
  evidence: repaired by routing native lifecycle work to ADR 0030/0031 Character Authority semantics and classifying Canary-only prerequisites as compatibility evidence.
rejected_hypotheses:
  - direct Platform-to-Canary SQL should remain the target native lifecycle design
  - Canary numeric player/account identifiers should remain canonical native operation identities
  - Issue 344 must block native deletion lifecycle work
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
validation:
  - command: live overlap and authority preflight
    result: PASS
    evidence: Issue 890 is claimed on repair/issue-890; Issue 888 overlap is avoided by not touching its shared paths.
  - command: backlog lifecycle reconciliation
    result: PASS
    evidence: Issues 277/317/319/320 updated; Issues 324/344 closed not_planned with Legacy Canary Compatibility-only reopen rules.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation/governance-only authority repair changes no executable user or integration journey.
  - command: full exact-head diff self-review
    result: NOT_RUN
    evidence: run on the final checkpoint head before readiness.
  - command: repository-selected exact-head CI
    result: NOT_RUN
    evidence: run on the final checkpoint head before readiness.
blockers:
  - none
next_action: Review PR #893 on its exact checkpoint head, then run repository-selected exact-head CI and merge only if all gates pass.
```
