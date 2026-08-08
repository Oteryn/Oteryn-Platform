# Native Character Lifecycle Authority Routing

## Status

`CURRENT ROUTING GUIDE — SUBORDINATE TO ACCEPTED ADR 0030 AND ADR 0031`

This document prevents stale Canary compatibility work from being interpreted as the target native Oteryn-v2 character lifecycle architecture.

It does not create a new source of authority. If this guide conflicts with ADR 0030 or ADR 0031, the accepted ADRs control.

## Core rule

For the native Oteryn-v2 target:

- Oteryn Platform owns authenticated Account Center UX, Platform policy/business gates, command orchestration, saga state, notifications, audit and Platform-owned projections/preferences;
- Oteryn-v2 Character Authority owns canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, authoritative character lifecycle state and the game-domain mutation outcome;
- native create, rename, schedule-delete, restore, finalize-delete, world transfer and account/Character Bazaar ownership transfer cross the boundary through versioned game-owned command/result or command/receipt contracts;
- Platform read models, operation rows and caches are never proof of current character ownership or successful game-domain mutation;
- direct/shared SQL and Canary numeric identifiers are Legacy Canary Compatibility or migration mechanisms only, not native steady-state design.

## Lifecycle ownership matrix

| Lifecycle capability | Native authority | Platform role | Compatibility note |
|---|---|---|---|
| Create character | Oteryn-v2 Character Authority | authorize AccountId context, collect validated product input, orchestrate command, project result | existing Canary create SQL remains compatibility-only until migrated |
| Rename character | Oteryn-v2 Character Authority | Account Center flow, Platform business/security gates, orchestration, history/projection reconciliation | Canary rename discovery may be retained only as explicit compatibility evidence |
| Schedule deletion | Oteryn-v2 Character Authority | owner confirmation, Platform policy, orchestration, user-visible saga/projection | `players.deletion` discovery is Canary compatibility evidence only |
| Cancel / restore deletion | Oteryn-v2 Character Authority | authenticated orchestration and projection reconciliation | direct Canary restore is not the native target |
| Finalize deletion | Oteryn-v2 Character Authority | request/reconcile approved lifecycle command; preserve Platform audit/business state | raw Canary startup deletion must not define native semantics |
| World transfer | Oteryn-v2 Character Authority | destination/product policy orchestration using canonical Platform `WorldId` / `ChannelId` references where applicable | Canary world/schema assumptions do not define the native capability |
| Account / Bazaar ownership transfer | Oteryn-v2 Character Authority for ownership mutation | Platform owns Bazaar/commercial saga, wallet and customer workflow | existing `players.account_id` transfer is Legacy Canary Compatibility only |

## Native command and result baseline

This repository does not freeze the Oteryn-v2 transport, encoding or exact command names. Every future native lifecycle contract must nevertheless preserve these semantic properties:

1. canonical Platform `AccountId` and game-owned `CharacterId` are used instead of `canary_account_id` / `canary_player_id` as native identities;
2. the Platform authenticates and authorizes the initiating Identity and applies Platform-owned product/security/business gates;
3. Oteryn-v2 revalidates current authoritative ownership and lifecycle eligibility before mutation;
4. each retryable mutation carries a stable operation/idempotency identity;
5. game-owned handling is idempotent for that operation identity or exposes an equivalent deterministic deduplication contract;
6. outcomes are typed and rereadable/reconcilable strongly enough that timeout or transport failure is never treated as proof of success or failure;
7. current ownership/lifecycle state wins over stale Platform projections;
8. ordering/concurrency conflicts fail closed or return a typed conflict rather than creating two authorities;
9. Platform updates its saga/read model only from authoritative results or later reconciliation;
10. no distributed ACID transaction between Platform and game persistence is assumed.

Exact command schemas, receipt fields, transport and game-internal transaction/locking implementation remain Oteryn-v2 authority and separate contract work.

## Platform-owned orchestration state

Platform may persist bounded workflow metadata when a user-facing lifecycle needs it, for example:

- Platform operation/public identifier;
- authenticated Identity / canonical AccountId reference;
- canonical CharacterId reference;
- requested business intent and Platform policy revision;
- idempotency/correlation identity;
- user-visible requested/effective timestamps when supplied by authoritative contract results;
- saga status such as pending, completed, rejected or recovery-required;
- bounded failure category and reconciliation metadata;
- notification and privacy-safe audit state.

Such rows are orchestration/read state only. They do not mint CharacterId, prove current ownership, override game lifecycle state or authorize a mutation after ownership changes.

## Ambiguous outcomes and recovery

For every cross-system lifecycle mutation:

```text
request sent
  -> response success      => consume authoritative result
  -> response rejection    => consume typed authoritative rejection
  -> timeout/connection loss/unknown commit state
       => recovery-required / query authoritative operation or current state
       => never fabricate success
       => never blindly issue a semantically different second mutation
```

A later implementation must define the exact reconciliation query/result boundary before claiming complete recovery behavior.

## Mutual exclusion and cross-domain conflicts

Character lifecycle operations must account for relevant concurrent game/business state such as:

- current gameplay session/admission state;
- Bazaar listing, escrow or settlement;
- another rename/delete/restore/world-transfer/account-transfer operation;
- guild/house/market/mail or other gameplay obligations when the game-domain contract says they affect eligibility;
- account/entitlement/security restrictions owned by Platform.

The authoritative game-domain command decides game-state eligibility. Platform may add stricter Platform-owned gates, but it must not duplicate game truth from stale database rows or cached projections.

## Legacy Canary Compatibility

Existing Canary integration work remains useful only inside an explicitly named compatibility/migration scope.

Compatibility rules:

- `canary_account_id`, `canary_player_id`, direct/shared SQL and operation-specific Canary principals are legacy adapter details;
- historical schema/runtime discovery may be preserved for supporting current privately operated Canary compatibility paths or migration analysis;
- a Canary compatibility task must say explicitly that it is compatibility-only, name the consumer that still requires it and define rollback/removal criteria;
- Canary compatibility work cannot block or define the native Oteryn-v2 lifecycle target unless a higher-ranked owner decision explicitly retains that compatibility feature;
- no new native feature may be justified by “the Canary table already has a column”;
- no Canary repository write is authorized by this Platform routing guide.

## Existing backlog routing

After the authority reconciliation tracked by Issue #890:

- #277 remains the product parent for character management completeness but uses native Character Authority as the target mutation boundary;
- #317 remains the product gap for deletion/restore lifecycle, with native command/receipt semantics as the target;
- #319 remains the product gap for rename lifecycle, with native command/receipt semantics as the target;
- #320 remains a conditional product gap for world transfer, dependent on an accepted native transferable-world capability rather than Canary schema shape;
- #324 is obsolete as an unqualified “Canary-safe rename contract” prerequisite for the native target;
- #344 is historical Legacy Canary Compatibility dependency evidence and must not block native #317 work.

A future explicit decision to keep one of those operations available on the Canary compatibility stack must create or reopen a bounded compatibility-only task. It must not reuse the native task as authorization for direct Canary mutation.

## Migration / cutover

Migration from compatibility operations to native lifecycle commands is additive and reversible until cutover is proven.

Required principles:

- introduce and preserve canonical AccountId/CharacterId mapping before removing legacy numeric identifiers;
- fail closed on missing/conflicting identity mapping;
- prove native producer/consumer compatibility and reconciliation behavior before disabling a legacy operation;
- retain rollback while any active consumer depends on Canary compatibility;
- retire legacy credentials/write privileges after the final compatibility consumer is proven absent;
- do not interpret this target architecture as proof that runtime migration has already happened.

## Non-goals

This document authorizes no:

- Laravel runtime implementation;
- database migration;
- Canary or Oteryn-v2 repository write;
- command transport/IDL choice;
- game-internal lifecycle state machine;
- production activation;
- deletion/rename/world-transfer product launch;
- payment or entitlement activation.

## References

- ADR 0030 — Native Character Portfolio / Account Center v2 boundary
- ADR 0031 — Native Oteryn-v2 integration and Legacy Canary Compatibility boundary
- ADR 0029 — Platform-owned WorldId / ChannelId topology identity
- `docs/contracts/CHARACTER_DELETION_CONTRACT.md` — Canary deletion discovery retained only for Legacy Canary Compatibility evidence
- Issue #890 — authority reconciliation finding
