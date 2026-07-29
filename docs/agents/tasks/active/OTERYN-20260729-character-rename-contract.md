---
task_id: OTERYN-20260729-character-rename-contract
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - active tasks and open PRs overlapping Issue #277, character rename, players.name, reservations or public character routes
  - authoritative Canary schema and source for player naming, login/session checks, guild/death references and cache/runtime behavior
  - current Platform character ownership, profile, Bazaar snapshot, search, sitemap and audit paths affected by rename
optional_reads:
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/architecture/adr/**character**
---

# OTERYN-20260729-character-rename-contract

## Goal

Deliver Issue #324, the read-only discovery and architecture contract required before any Oteryn Platform character-rename implementation under parent Issue #277.

## Acceptance criteria

- [ ] Search active tasks and PRs for overlap before broad discovery.
- [ ] Inspect the authoritative current Canary schema and rename-related source/runtime behavior read-only.
- [ ] Record exact name uniqueness, collation, normalization, reserved-name, online/session and dependent-reference rules without relying on generic TFS assumptions.
- [ ] Identify the semantic owner and exact permitted mutation boundary, or record that no safe operation is currently proven.
- [ ] Define server-resolved ownership, authorization, deterministic locking, idempotency, stale-request and concurrency behavior.
- [ ] Define Platform-owned request, cooldown, immutable history, audit and recovery state where justified by evidence.
- [ ] Define the least-privilege Canary principal and exact columns/operations without provisioning or using it in this task.
- [ ] Define cross-surface consistency for public profiles, guilds, deaths, Bazaar snapshots/history, search, sitemap and caches.
- [ ] Define compatibility, rollout order, rollback and failure-recovery requirements.
- [ ] Publish an evidence-backed ADR/contract and an implementation/non-implementation decision.
- [ ] Run checkpoint, documentation and contract validation on the exact final head.
- [ ] Merge and archive this discovery task separately; any mutation implementation requires a new authorized task.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/*character-rename*
  - docs/contracts/*CHARACTER_RENAME*
  - docs/operations/*CHARACTER_RENAME*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-character-rename-contract.md
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
  - current public-profile, guild, deaths, Bazaar, search and sitemap contracts
blockers:
  - Canary mutation implementation is not authorized by this task
cross_repository_tasks:
  - read-only Canary compatibility discovery only; no Canary writes are authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T21:55:00Z
head: f90bb8075b300569b7d493c84f0080e6b3295c35
branch: docs/OTERYN-20260729-character-rename-contract
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - canary-integration
  - database
  - auth-identity
  - security
  - web-cms
  - testing
owned_paths:
  - docs/architecture/adr/*character-rename*
  - docs/contracts/*CHARACTER_RENAME*
  - docs/operations/*CHARACTER_RENAME*
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/active/OTERYN-20260729-character-rename-contract.md
proven:
  - PR #308 completed and archived Issue #307 as the Platform-owned comment, per-character privacy and main-character slice; parent Issue #277 remains open.
  - Issue #324 is open for read-only character-rename contract discovery and explicitly excludes Canary writes or runtime activation.
  - Root repository policy permits autonomous writes only in blakinio/Oteryn-Platform and requires blakinio/canary to remain read-only without separate authorization.
  - Current durable contracts require an operation-specific owner, exact fields, authorization, locking, compatibility, rollout and rollback before any Canary mutation.
derived:
  - Character rename cannot safely proceed as an implementation task until authoritative Canary naming and runtime behavior plus all affected Platform projections are reconciled.
  - A documentation-only discovery task can reduce uncertainty without provisioning credentials or mutating either runtime.
unknown:
  - Exact Canary name normalization, collation, reservation and runtime cache behavior.
  - Whether changing only players.name is sufficient or dependent/runtime state requires a coordinated operation.
  - Whether a safe least-privilege mutation principal and rollback strategy can be proven without Canary code changes.
conflicts: []
first_failure:
  marker: none
  evidence: Discovery has not started; no implementation or validation failure exists.
rejected_hypotheses:
  - Reuse the Character Bazaar account_id transfer principal for name mutation.
  - Assume generic TFS/MyAAC rename behavior is authoritative for Oteryn Canary.
  - Implement a browser rename endpoint before proving runtime, dependent-reference and rollback semantics.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260729-character-rename-contract.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: Task initialization only.
blockers:
  - No Canary mutation implementation is authorized; discovery and contract documentation remain unblocked.
next_action: Inspect current blakinio/canary schema and source read-only for players.name ownership, uniqueness, normalization, online/session restrictions, dependent references and runtime cache behavior, then map those facts to affected Oteryn Platform surfaces.
```

## Boundaries

This task performs read-only discovery and documentation only. It does not write Canary, create a rename endpoint, provision a mutation credential, alter production or claim `PRODUCTION_PROVEN` status.
