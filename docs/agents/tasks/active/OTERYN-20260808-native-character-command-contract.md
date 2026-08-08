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

- [ ] Common command envelope and result/receipt semantics are explicit.
- [ ] Canonical AccountId/CharacterId and applicable WorldId/ChannelId are used; Canary numeric IDs remain compatibility-only.
- [ ] Stable operation identity, idempotency and conflicting-reuse behavior are defined.
- [ ] Typed terminal/retryable/ambiguous outcomes and reconciliation semantics are defined.
- [ ] Oteryn-v2 revalidation and Platform orchestration authority remain separated.
- [ ] Cross-command/session/Bazaar concurrency and mutual-exclusion semantics are explicit without distributed ACID.
- [ ] Per-command profiles cover create, rename, schedule deletion, restore/cancel, finalize deletion, optional world transfer and Bazaar/account ownership transfer.
- [ ] Public projection effects route to the accepted PublicGameData projection/privacy contract rather than creating a second authority.
- [ ] Existing lifecycle routing guide points to the new focused contract.
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

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T19:50:00+02:00
head: f5e56a78e65dfae90b5b8e1694b10e70545de262
branch: docs/OTERYN-20260808-native-character-command-contract
pr: none
status: implementing
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
  - No open PR or active branch was found owning this exact native shared command/result contract.
  - Issues #317 and #319 require an accepted game-owned command/result contract before runtime implementation; #320 requires one if world transfer is adopted.
  - ADR 0030/0031 and the lifecycle routing guide already assign authoritative native character mutation outcomes to Oteryn-v2 Character Authority.
  - Current main head at task selection is f5e56a78e65dfae90b5b8e1694b10e70545de262.
derived:
  - A shared semantic contract reduces duplicated lifecycle invariants and gives later product-specific implementations one reusable authority/idempotency/reconciliation baseline.
unknown:
  - exact Oteryn-v2 transport/IDL/encoding/endpoint
  - exact game-internal transaction, locking and command state-machine implementation
  - exact native world-transfer product adoption and transfer rules
conflicts: []
first_failure:
  marker: none
  evidence: no ownership or architecture conflict found during overlap preflight
rejected_hypotheses:
  - each lifecycle issue should independently invent idempotency and result semantics
  - current Canary SQL contracts should define the native command boundary
  - Platform operation rows or public projections can prove current character ownership or mutation completion
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no open PR owns the focused shared contract; existing lifecycle/product issues remain subordinate consumers
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task with no executable behavior
blockers:
  - none
next_action: Draft the focused native Character Authority command/result contract, reconcile the lifecycle routing guide, then perform a full exact-head documentation self-review before opening the PR.
```

## Notes

Oteryn-v2 and Canary remain read-only. This task does not authorize runtime/schema/workflow/deployment/payment/production changes or product activation.