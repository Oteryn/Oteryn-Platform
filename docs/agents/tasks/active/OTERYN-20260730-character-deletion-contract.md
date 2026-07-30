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
- [x] Run checkpoint, documentation and contract validation on the final content head.
- [ ] Merge and archive this discovery task separately; any mutation implementation requires a new authorized task.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/*character-deletion*
  - docs/contracts/*CHARACTER_DELETION*
  - docs/operations/*CHARACTER_DELETION*
  - docs/agents/tasks/active/OTERYN-20260730-character-deletion-contract.md
modules:
  - Architecture
  - CharacterProfiles
  - PublicGameData
  - Accounts
  - Marketplace
  - Audit
  - CanaryIntegration
dependencies:
  - authoritative read-only inspection of blakinio/canary
  - current immutable Identity-to-Canary binding and character ownership rules
  - current Character Bazaar exclusion and escrow rules
blockers:
  - Canary lifecycle prerequisite tracked by Issue #344 is required before Issue #317 implementation
cross_repository_tasks:
  - Issue #344 records the exact Canary-owned prerequisite; no Canary writes are authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T07:53:00Z
head: 3dd59d6b1ecadfda9f3192422a32ecc745073044
branch: docs/OTERYN-20260730-character-deletion-contract
pr: 343
status: ready
context_routes:
  - agent-governance
  - architecture
  - accounts-characters
  - canary-integration
  - database
  - security
  - web-cms
  - testing
owned_paths:
  - docs/architecture/adr/*character-deletion*
  - docs/contracts/*CHARACTER_DELETION*
  - docs/operations/*CHARACTER_DELETION*
  - docs/agents/tasks/active/OTERYN-20260730-character-deletion-contract.md
proven:
  - PR #315 merged the authoritative product-completeness reconciliation and leaves required character deletion/restore Issue #317 open.
  - Issue #317 requires an explicit Platform/Canary operation contract before any shared-data write and does not authorize Canary or production mutation.
  - Issue #342 is the bounded read-only deletion/restore contract discovery slice under #317/#277; PR #343 owns only unique deletion-contract paths.
  - Open PR #328 owns character-rename contract work and shared architecture/index paths; PR #341 merged the backend/frontend capability-ledger slice into main.
  - Repository policy permits autonomous writes only in blakinio/Oteryn-Platform and requires blakinio/canary to remain read-only without separate authorization.
  - Read-only discovery is pinned to blakinio/canary@3c90d3ada717cd2ed0c5344f1dac210a205355f6 and exact inspected file blob identities listed in CHARACTER_DELETION_CONTRACT.md.
  - Canary players.deletion is a bigint timestamp field with zero as active; account lists and preLoadPlayer exclude every non-zero value immediately.
  - Canary startup asynchronously deletes players rows whose non-zero deletion timestamp is earlier than current time; finalization timing is therefore coupled to server startup and has no operation receipt.
  - Physical player deletion cascades broad player-keyed data; deletion of a guild owner cascades the guild and its dependent rows, while the ondelete_players trigger clears house ownership.
  - Character Bazaar transfer requires deletion zero and its principal can update only players.account_id; no existing Platform principal authorizes deletion, restore or finalization.
  - ADR 0021 and CHARACTER_DELETION_CONTRACT.md record a no-go decision for direct Platform implementation and define the required Canary-owned lifecycle boundary.
  - CHARACTER_DELETION_ROLLOUT.md records the producer-first compatibility, hidden consumer, acceptance, activation and rollback gates.
  - Issue #344 durably tracks the exact cross-repository prerequisite without authorizing a Canary write.
  - Final-gate workflow runs on content head ffd3fa0de4c5094bff7a7a238cabbbc074c99bf9 completed successfully for Agent Governance, CI, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage Validation and Phase 7 Production-Like Validation.
derived:
  - Direct UPDATE(players.deletion) cannot satisfy deterministic finalization, side-effect policy, request identity or ambiguous-outcome recovery.
  - Direct DELETE(players) would expose excessively broad and product-visible Canary-owned cascades.
  - Issue #317 implementation must remain blocked until the Issue #344 Canary prerequisite is explicitly authorized, merged at a pinned revision and revalidated by a new Oteryn implementation task.
unknown:
  - Exact future Canary lifecycle schema/interface and final retention policy; these belong to the separately authorized Issue #344 prerequisite.
  - Production deployment revision, grants and runtime behavior remain unproven.
conflicts: []
first_failure:
  marker: current Canary lifecycle lacks an operation-safe schedule/cancel/finalize boundary
  evidence: server_initialization.lua performs uncorrelated asynchronous startup DELETE and schema cascades guild/house/player-dependent state
rejected_hypotheses:
  - Reuse the generic read-only Canary connection or Character Bazaar transfer principal for deletion mutation: current grants explicitly do not authorize deletion state or row deletion.
  - Assume generic TFS/MyAAC deletion behavior is authoritative for Oteryn Canary: exact current Canary source and schema were inspected instead.
  - Deleted characters can enter the game because the early ownership gate omits deletion: preLoadPlayer independently rejects every non-zero deletion value before player placement.
  - Implement a browser deletion endpoint using only UPDATE(players.deletion): no operation receipt, deterministic finalizer or safe cascade policy exists.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-character-deletion-contract.md
  - docs/architecture/adr/0021-require-canary-owned-character-deletion-lifecycle.md
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
  - docs/operations/CHARACTER_DELETION_ROLLOUT.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260730-character-deletion-contract.md --require-checkpoint
    result: PASS
    evidence: Agent Governance run 30523680493 succeeded on pre-rebase content head ffd3fa0de4c5094bff7a7a238cabbbc074c99bf9; exact rebased-head rerun is required before merge.
  - command: python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: Agent Governance run 30523680493 succeeded on pre-rebase content head ffd3fa0de4c5094bff7a7a238cabbbc074c99bf9; exact rebased-head rerun is required before merge.
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance run 30523680493 succeeded on pre-rebase content head ffd3fa0de4c5094bff7a7a238cabbbc074c99bf9; exact rebased-head rerun is required before merge.
  - command: documentation/path/link review
    result: PASS
    evidence: changed-path inspection remains limited to the four declared unique deletion-contract paths after rebase; exact rebased-head CI is required before merge.
blockers:
  - Issue #344 requires separately authorized Canary implementation before Issue #317 runtime work; this discovery PR itself is unblocked.
next_action: Verify all final-gate workflows succeed on the exact checkpoint head created from this pin, then squash-merge PR #343 and archive the task record in a separate governance PR.
```

## Boundaries

This task performs read-only discovery and documentation only. It does not write Canary, create a deletion endpoint or migration, provision a mutation credential, alter production or claim `PRODUCTION_PROVEN` status. Shared architecture/index paths currently owned by PR #328 are intentionally excluded.
