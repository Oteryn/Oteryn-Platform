---
task_id: OTERYN-20260730-character-deletion-contract
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - active tasks and open PRs overlapping Issue #317, character deletion, players deletion state, Bazaar escrow or public character routes
  - authoritative Canary schema and source for player deletion, login/session checks, dependent rows, cleanup jobs and runtime/cache behavior
  - current Platform character ownership, account inventory, profile, Bazaar, audit and notification paths affected by deletion lifecycle
optional_reads:
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260729-character-rename-contract.md
  - docs/architecture/adr/**character**
---

# OTERYN-20260730-character-deletion-contract

## Goal

Deliver Issue #342, the read-only discovery and operation contract required before any Oteryn Platform character deletion, grace-period cancellation/restore or finalization implementation under #317/#277.

## Acceptance criteria

- [x] Search active tasks and PRs for overlap before broad discovery.
- [x] Inspect the authoritative current Canary schema and deletion-related source/runtime behavior read-only.
- [x] Record exact deletion fields, retention behavior, dependent rows, login/session restrictions, cleanup jobs and runtime/cache effects without generic TFS assumptions.
- [x] Identify the semantic owner and exact permitted mutation boundary, or record that no safe operation is currently proven.
- [x] Define server-resolved ownership, authorization, deterministic locking, idempotency, stale-request and concurrency behavior.
- [x] Define Platform-owned request, grace, cancellation, restoration, finalization, audit and recovery states where justified by evidence.
- [x] Define online/session, Bazaar/escrow, rename and transfer conflict rules.
- [x] Define the least-privilege Canary principal and effective-grant verification without provisioning or using credentials.
- [x] Define public search/profile and Account Center visibility for every lifecycle state.
- [x] Define compatibility, rollout order, rollback, reconciliation and failure-recovery requirements.
- [x] Publish an evidence-backed ADR/contract and an implementation/no-go decision.
- [x] Run checkpoint, documentation and contract validation on the exact final head.
- [x] Merge the discovery task separately; any mutation implementation requires a new authorized task.
- [x] Archive this completed task record in a separate governance PR.

## Delivered records

- `docs/architecture/adr/0021-require-canary-owned-character-deletion-lifecycle.md`;
- `docs/contracts/CHARACTER_DELETION_CONTRACT.md`;
- `docs/operations/CHARACTER_DELETION_ROLLOUT.md`;
- Issue #344, the separately authorized Canary lifecycle prerequisite tracker.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T08:00:00Z
head: c11915590b388f42868e80e1c3896934563476b2
branch: docs/OTERYN-20260730-character-deletion-contract
pr: 343
merge_commit: e28f71886963f0b298233fe310d4c927f8ddd316
status: archived
context_routes:
  - agent-governance
  - architecture
  - accounts-characters
  - canary-integration
  - database
  - security
  - web-cms
  - testing
proven:
  - PR #343 was squash-merged into main as e28f71886963f0b298233fe310d4c927f8ddd316 after exact-head validation.
  - Read-only discovery was pinned to blakinio/canary@3c90d3ada717cd2ed0c5344f1dac210a205355f6 and exact inspected file blob identities recorded in CHARACTER_DELETION_CONTRACT.md.
  - Canary players.deletion is a bigint timestamp field with zero as active; account lists and preLoadPlayer exclude every non-zero value immediately.
  - Canary startup asynchronously deletes expired non-zero deletion rows, so finalization is coupled to server startup and has no Platform-correlatable operation receipt.
  - Physical player deletion cascades broad player-keyed state; deletion of a guild owner cascades the guild and the ondelete_players trigger clears house ownership.
  - Existing Platform principals do not authorize safe deletion, restore or finalization.
  - ADR 0021 records a no-go decision for direct Platform UPDATE(players.deletion) or DELETE(players).
  - Issue #344 tracks the Canary-owned idempotent schedule/cancel/finalize prerequisite without authorizing any Canary write.
  - Issue #317 remains blocked from runtime implementation until #344 is authorized, merged at a pinned revision and the Platform contract is revalidated.
derived:
  - Character deletion is a cross-repository lifecycle prerequisite, not a safe Platform-only endpoint or database-grant change.
  - Platform may later own workflow metadata and UX, while Canary must remain the semantic owner of gameplay deletion and destructive side effects.
unknown:
  - Exact future Canary lifecycle interface, schema and final retention policy.
  - Production deployment revision, grants and runtime behavior remain unproven.
conflicts: []
first_failure:
  marker: current Canary lifecycle lacks an operation-safe schedule/cancel/finalize boundary
  evidence: server_initialization.lua performs an uncorrelated asynchronous startup DELETE and schema cascades guild, house and player-dependent state
rejected_hypotheses:
  - Reuse the read-only Canary connection or Character Bazaar transfer principal for deletion mutation.
  - Assume generic TFS/MyAAC deletion behavior instead of inspecting current Oteryn Canary.
  - Treat omission of deletion from the early ownership gate as a proven login bypass; preLoadPlayer rejects non-zero deletion before placement.
  - Implement a browser deletion endpoint using only UPDATE(players.deletion).
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-character-deletion-contract.md
  - docs/architecture/adr/0021-require-canary-owned-character-deletion-lifecycle.md
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
  - docs/operations/CHARACTER_DELETION_ROLLOUT.md
validation:
  - command: Agent Governance run 30524552353
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2.
  - command: CI run 30524552413
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2.
  - command: Edge Security Emulation run 30524552365
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2.
  - command: Game Auth Ticket Concurrency run 30524552316
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2.
  - command: Platform DB Outage Validation run 30524553524
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2.
  - command: Phase 7 Production-Like Validation run 30524552570
    result: PASS
    evidence: exact head c11915590b388f42868e80e1c3896934563476b2; this is staging-like evidence, not production proof.
blockers:
  - Issue #344 blocks Issue #317 implementation but does not block archival of this completed discovery task.
next_action: Keep Issue #317 blocked until the separately authorized Canary lifecycle prerequisite #344 is merged and revalidated; then create a new Oteryn implementation task.
```

## Boundaries

This task performed read-only discovery and documentation only. It made no Canary write, runtime endpoint, migration, credential, production action or `PRODUCTION_PROVEN` claim.
