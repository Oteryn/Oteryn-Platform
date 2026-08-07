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
- [ ] Run ADR/documentation validation and exact-head required CI before merge.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260807-account-id-boundary.md
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
  - Oteryn-v2 consumes this Platform-owned AccountId contract; no Oteryn-v2 write is authorized by this task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T19:18:00Z
head: ae0bc659da2b2094267e4cb2d8d2a03f319c1eb6
branch: docs/OTERYN-20260807-account-id-boundary
pr: 850
status: implementing
context_routes:
  - architecture
  - auth-identity
  - canary-integration
owned_paths:
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260807-account-id-boundary.md
proven:
  - Current Platform task base is 022dbcef97b2dd0ff4eeeda11bf053c9c11341e8.
  - Current Platform Identity persistence uses integer identities.id.
  - Existing Game Login Ticket contract binds current Canary-compatible authorization to canary_account_id.
  - Oteryn-v2 owner baselines keep AccountId Platform-owned and forbid silent re-keying by the game domain.
  - Repository owner accepted Platform-issued UUIDv7 AccountId as the native cross-boundary identity while identities.id remains local persistence surrogate.
derived:
  - Native Oteryn-v2 contracts must not use canary_account_id as AccountId.
  - Existing Canary binding remains a compatibility mapping until a separately authorized runtime migration removes it.
unknown:
  - Exact implementation migration/backfill sequence for AccountId storage; intentionally outside this architecture-only task.
conflicts:
  - Historical/current Canary-oriented contracts use canary_account_id where the native target now requires AccountId; ADR 0028 and the narrow native contract resolve this by scope without changing current runtime behavior.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Re-key all existing Platform foreign keys from identities.id to UUIDv7 as part of this decision.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-account-id-boundary.md
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: coherent documentation candidate is being committed before validation
blockers:
  - none
next_action: Commit the coherent architecture package, run ADR/documentation validation, and inspect exact-head required CI.
```

## Notes

Architecture/documentation only. No runtime, schema, migration, deployment or Oteryn-v2 repository change is authorized by this task.
