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

Repair stale Platform backlog and documentation so future native character lifecycle work follows accepted ADR 0030/0031 authority: Oteryn Platform owns authenticated UX/orchestration/business workflow while Oteryn-v2 Character Authority owns canonical CharacterId/current ownership and native create/rename/delete/restore/world-transfer/account-transfer mutations and authoritative receipts.

Preserve Canary evidence only as explicit Legacy Canary Compatibility / migration evidence. Do not mutate Canary, Oteryn-v2, runtime code, schemas, workflows, credentials, deployment or production state.

## Acceptance criteria

- [ ] Issues #277, #317, #319, #320, #324 and #344 no longer route target-native lifecycle work through direct Platform-to-Canary mutation authority.
- [ ] Native target semantics route create/rename/delete/restore/world-transfer/account-transfer to Oteryn-v2 Character Authority through versioned game-owned commands/receipts without inventing unfinished wire/transport details.
- [ ] Existing Canary deletion discovery remains preserved and explicitly classified as Legacy Canary Compatibility evidence only.
- [ ] Future direct/shared SQL and least-privilege Canary principals are not presented as native steady-state design.
- [ ] Legacy Canary compatibility work requires a separately explicit compatibility scope and cannot block or define native Oteryn-v2 lifecycle work.
- [ ] No runtime/schema/workflow/deployment/credential/production or external-repository mutation occurs.
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

Shared paths named by Issue #890 (`docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` and `docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md`) are intentionally not claimed or edited because active Issue #888 currently owns overlapping integration/programme paths on another branch.

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

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T09:46:00+02:00
head: UNKNOWN
branch: repair/issue-890
pr: none
status: implementing
phase: implement
execution_mode: github_only
invocation_started_at: 2026-08-08T09:40:00+02:00
last_progress_at: 2026-08-08T09:46:00+02:00
ci_checks_for_current_head: 0
ci_check_generation: none
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
  - Open Issues 277, 317, 319, 320, 324 and 344 still contain unqualified Canary-target mutation instructions that predate ADR 0030/0031.
  - CHARACTER_DELETION_CONTRACT.md preserves valid read-only Canary discovery but currently ends by routing the next safe step to a Canary lifecycle prerequisite without distinguishing native target work.
  - Issue 888 owns pre-admission/session handoff and has a fresh active branch; its shared integration/programme paths are not touched here.
derived:
  - The safest repair is to preserve Canary evidence as compatibility-only, redirect native lifecycle target work to game-owned commands/receipts, and retire obsolete Canary prerequisites from the native backlog.
unknown:
  - Exact Oteryn-v2 command schemas, transport, lifecycle state machine internals and receipt wire format remain external authority and are deliberately not invented.
conflicts: []
first_failure:
  marker: stale-backlog-authority
  evidence: Issues 317/319/320/324/344 and CHARACTER_DELETION_CONTRACT.md still describe Canary mutation prerequisites as unqualified future target work.
rejected_hypotheses:
  - direct Platform-to-Canary SQL should remain the target native lifecycle design
  - Canary numeric player/account identifiers should remain canonical native operation identities
  - Issue 344 must block native deletion lifecycle work
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-character-lifecycle-authority-reconciliation.md
validation:
  - command: live overlap and authority preflight
    result: PASS
    evidence: Issue 890 is unclaimed; repair/issue-890 did not exist; active Issue 888 overlap is confined by not touching its shared paths.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation/governance-only authority repair changes no executable user or integration journey.
blockers:
  - none
next_action: Add a focused native character-lifecycle authority routing document, reclassify the Canary deletion contract, reconcile Issues 277/317/319/320/324/344, then open a draft PR and run exact-head validation.
```
