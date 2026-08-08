---
task_id: OTERYN-20260808-native-character-command-contract
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
issue: 919
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
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
search_first:
  - Issue #919
  - Issues #317 #319 #320
  - open PRs and active branches for native character command/result work
optional_reads:
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
---

# OTERYN-20260808 native Character Authority command contract

## Goal

Define one reusable Platform-side semantic command/result contract for native Oteryn-v2 Character Authority mutations so create, rename, deletion/restore/finalization, optional world transfer and Bazaar/account ownership transfer share consistent authority, idempotency, concurrency, typed outcome and reconciliation rules without freezing Oteryn-v2 wire implementation.

## Acceptance criteria

- [x] Common command envelope and result/receipt semantics are explicit.
- [x] Canonical AccountId/CharacterId and applicable WorldId/ChannelId are used; Canary numeric IDs remain compatibility-only.
- [x] Stable operation identity, idempotency and conflicting-reuse behavior are defined.
- [x] Typed terminal/retryable/ambiguous outcomes and reconciliation semantics are defined.
- [x] Oteryn-v2 revalidation and Platform orchestration authority remain separated.
- [x] Cross-command/session/Bazaar concurrency and mutual-exclusion semantics are explicit without distributed ACID.
- [x] Per-command profiles cover create, rename, schedule deletion, restore/cancel, finalize deletion, optional world transfer and Bazaar/account ownership transfer.
- [x] Public projection effects route to the accepted PublicGameData projection/privacy contract rather than creating a second authority.
- [x] Existing lifecycle routing guide points to the new focused contract.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: architecture/documentation-only change with no executable producer, consumer, schema, route or product activation.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
modules:
  - Characters
  - Accounts
  - PublicGameData
  - architecture-governance
dependencies:
  - Issue #919
  - ADR 0030
  - ADR 0031
  - Issues #317 #319 #320
blockers:
  - none
cross_repository_tasks:
  - none
```

## Exact-content self-review

```yaml
self_review:
  result: PASS
  exact_head: 351d1a54ff047c56da283e7f042d49fcc7cde203
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - Exactly four intended documentation/task/report paths are changed.
    - Shared operation identity, idempotency, conflicting reuse, terminal/pending/ambiguous outcome and reconciliation semantics are explicit.
    - Oteryn-v2 remains authoritative for ownership/lifecycle/game-state mutation while Platform remains orchestration/business authority.
    - Create, rename, deletion/restore/finalization, capability-gated world transfer and Bazaar/account ownership transfer profiles are explicitly bounded.
    - Public projection/privacy routing points to OTERY​N_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT rather than creating a second read authority.
    - Canary direct SQL/numeric identifiers remain Legacy Canary Compatibility only; ambiguous native operations cannot fall back blindly to a second authority.
    - No runtime/schema/workflow/deployment/payment/production/external-repository path changed.
```

A later task-checkpoint-only commit does not change the reviewed architecture semantics. The final exact-head review will be recorded on PR #920 after this checkpoint establishes the release-candidate head.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T19:56:00+02:00
head: 351d1a54ff047c56da283e7f042d49fcc7cde203
branch: docs/OTERYN-20260808-native-character-command-contract
pr: 920
status: validating
context_routes:
  - architecture
  - character-lifecycle
  - cross-repository-contract
  - security
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
proven:
  - Issue #919 is the focused P1 owner for the shared native Character Authority command/result semantic boundary.
  - PR #920 is the single delivery PR for this task.
  - No other open PR or active branch was found owning this exact shared command/result contract.
  - Issues #317 and #319 require an accepted game-owned command/result contract before runtime implementation; #320 requires one if world transfer is adopted.
  - The focused contract now defines shared operation identity, idempotency, typed result, ambiguity/reconciliation, concurrency and per-command profile semantics.
  - The lifecycle routing guide now routes future native mutation implementations through the focused contract.
  - Current main base for the delivery diff is f5e56a78e65dfae90b5b8e1694b10e70545de262 and branch was behind_by=0 at content self-review.
derived:
  - #317/#319 can consume one architecture baseline instead of inventing independent cross-system mutation semantics.
  - #320 receives reusable transport-independent semantics without approving the world-transfer product capability.
unknown:
  - exact Oteryn-v2 transport/IDL/encoding/endpoint
  - exact durable result/reconciliation mechanism
  - exact game-internal transaction, locking and command state-machine implementation
  - exact native world-transfer product adoption and transfer rules
conflicts: []
first_failure:
  marker: typo-only-status-label
  evidence: full-diff review found `OTeryn-v2` capitalization in the contract status line; corrected on commit 351d1a54ff047c56da283e7f042d49fcc7cde203 with no semantic change
rejected_hypotheses:
  - each lifecycle issue should independently invent idempotency and result semantics
  - current Canary SQL contracts should define the native command boundary
  - Platform operation rows or public projections can prove current character ownership or mutation completion
  - operation_id can substitute for service authentication or replay protection
  - ambiguous native mutations may safely fall back to direct Canary/native SQL
changed_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no overlapping owner found; Issue #919 and PR #920 are the focused owner/delivery
  - command: exact content-head full-diff architecture self-review
    result: PASS
    evidence: commit 351d1a54ff047c56da283e7f042d49fcc7cde203 satisfies all semantic acceptance criteria with zero material findings after typo correction
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task with no executable producer, consumer, schema, route or product activation
blockers:
  - none
next_action: Mark PR #920 ready, run repository-selected exact-head CI on the resulting release-candidate head, and merge only after all required checks plus final review hygiene pass.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-20260808T1948+0200-issue-919
  session_started_at: 2026-08-08T19:48:00+02:00
  checkpointed_at: 2026-08-08T19:56:00+02:00
  last_progress_at: 2026-08-08T19:56:00+02:00
  phase: validate
  exact_head: 351d1a54ff047c56da283e7f042d49fcc7cde203
  pull_request: 920
  active_operation: prepare exact-head PR validation
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: ready
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: PR #920 release-candidate head is ready for repository-selected CI
  next_action: Mark PR #920 ready and observe the first aggregate exact-head workflow state.
```

## Notes

Oteryn-v2 and Canary remain read-only. This task does not authorize runtime/schema/workflow/deployment/payment/production changes or product activation.