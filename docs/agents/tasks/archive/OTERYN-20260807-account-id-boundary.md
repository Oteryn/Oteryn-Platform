---
task_id: OTERYN-20260807-account-id-boundary
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0004-authoritative-platform-account-ownership.md
  - docs/architecture/adr/0009-oteryn-game-authentication-architecture.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
search_first:
  - AccountId
  - canary_account_id
  - Game Login Ticket
optional_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
---

# OTERYN-20260807-account-id-boundary

## Goal

Record the owner-accepted native Oteryn-v2 account identity boundary in Oteryn Platform without changing runtime behavior: canonical cross-boundary `AccountId` is Platform-issued UUIDv7, existing `identities.id` remains a Platform-local persistence surrogate, native Game Login Tickets bind to `AccountId`, and `canary_account_id` remains legacy/Canary ACL state only.

## Acceptance criteria

- [x] Allocate the next non-conflicting ADR number and record the accepted decision.
- [x] Keep `identities.id` explicitly local and non-contractual across the native boundary.
- [x] Classify `canary_account_id` as legacy compatibility / ACL rather than native Oteryn-v2 identity.
- [x] Preserve current implemented Canary compatibility behavior as historical/current implementation evidence; do not claim runtime migration.
- [x] Publish a narrow native cross-repository account identity contract without rewriting legacy runtime contracts.
- [x] Validate the exact merged PR head and merge through protected `main`.
- [x] Archive the completed task record after merge.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260807-account-id-boundary.md
modules:
  - Identity
  - GameAuth
  - architecture
  - contracts
dependencies:
  - blakinio/Oteryn-v2 FND-ID-01 owner baselines
blockers:
  - none
cross_repository_tasks:
  - Oteryn-v2 consumes this Platform-owned AccountId contract; no Oteryn-v2 write was authorized by this task
```

## Terminal checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T19:25:00Z
merged_main: 28408e0a54ecc8d9fdfe3e7ca135f6400e40fe0d
source_branch: docs/OTERYN-20260807-account-id-boundary
pr: 850
status: completed
context_routes:
  - architecture
  - auth-identity
  - canary-integration
proven:
  - Platform Identity remains the owner and issuer of canonical native AccountId.
  - AccountId is accepted as a strongly typed UUIDv7 preserving all 128 bits.
  - Existing integer identities.id remains a Platform-local persistence surrogate and is not native AccountId.
  - canary_account_id is explicitly legacy compatibility / ACL state only.
  - Native Game Login Ticket semantics bind account authority to AccountId while preserving the existing one-time, short-lived, generation-fenced security model.
  - CharacterId remains game-domain owned and authoritative account-character ownership is revalidated by the game domain.
  - PR 850 merged to protected main as 28408e0a54ecc8d9fdfe3e7ca135f6400e40fe0d.
derived:
  - Future native Gateway/redeem contracts must not require Canary numeric account identity.
  - Existing Canary contracts remain compatibility evidence until separately migrated or retired.
unknown:
  - Exact additive Platform schema/backfill and native API rollout mechanics remain future implementation work by design.
conflicts:
  - none remaining inside the accepted architecture scope; legacy Canary-oriented contracts are scope-bounded rather than silently rewritten.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Re-key every existing Platform foreign key to UUIDv7.
  - Use identities.id bigint as the canonical native AccountId.
  - Reuse canary_account_id as native AccountId.
  - Let Oteryn-v2 mint a competing account UUID.
changed_paths:
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260807-account-id-boundary.md
validation:
  - command: GitHub required check classify-changes on PR 850 exact head 161bc8b2232630d4b47a1053f3d842babe6089f1
    result: PASS
    evidence: protected-branch required check completed successfully
  - command: GitHub required check test on PR 850 exact head 161bc8b2232630d4b47a1053f3d842babe6089f1
    result: PASS
    evidence: test check completed successfully
  - command: CI runtime-tests / PHPUnit on exact PR head
    result: PASS
    evidence: job 92974032027 completed success; formatting, static analysis and tests passed
  - command: architecture/security/integration audit checks on exact PR head
    result: PASS
    evidence: all five bounded audit checks completed successfully
  - command: validation workflows on exact PR head
    result: PASS
    evidence: exact-head validate checks completed successfully
blockers:
  - none
next_action: none; architecture decision is merged and this task is terminal. Any AccountId schema/API/runtime implementation requires a new explicitly authorized implementation task.
```

## Result

Canonical source of truth is now ADR 0028 plus `docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md` on `main`.

No runtime, database schema, migration, deployment or Oteryn-v2 repository implementation was performed by this architecture task.
