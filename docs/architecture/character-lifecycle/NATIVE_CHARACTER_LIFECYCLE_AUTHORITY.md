# Native Character Lifecycle Authority Routing

## Status

`CURRENT ROUTING GUIDE — SUBORDINATE TO ACCEPTED ADR 0030 AND ADR 0031`

This document prevents stale Canary compatibility work from being interpreted as the target native Oteryn-v2 character lifecycle architecture.

It does not create a new source of authority. If this guide conflicts with ADR 0030 or ADR 0031, the accepted ADRs control.

The reusable cross-system mutation semantics are defined by `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`. Product-specific lifecycle issues remain responsible for their own behavior and implementation.

## Core rule

For the native Oteryn-v2 target:

- Oteryn Platform owns authenticated Account Center UX, Platform policy/business gates, command orchestration, saga state, notifications, audit and Platform-owned projections/preferences;
- Oteryn-v2 Character Authority owns canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, authoritative character lifecycle state and the game-domain mutation outcome;
- native create, rename, schedule-delete, restore, finalize-delete, world transfer and account/Character Bazaar ownership transfer cross the boundary through versioned game-owned command/result or command/receipt contracts;
- all such native mutations inherit the stable-operation, idempotency, typed-outcome, ambiguity/reconciliation and concurrency invariants from `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`;
- Platform read models, operation rows and caches are never proof of current character ownership or successful game-domain mutation;
- direct/shared SQL and Canary numeric identifiers are Legacy Canary Compatibility or migration mechanisms only, not native steady-state design.

## Lifecycle ownership matrix

| Lifecycle capability | Native authority | Platform role | Compatibility note |
|---|---|---|---|
| Create character | Oteryn-v2 Character Authority | authorize AccountId context, collect validated product input, orchestrate command, project result | existing Canary create SQL remains compatibility-only until migrated |
| Rename character | Oteryn-v2 Character Authority | Account Center flow, Platform business/security gates, orchestration, history/projection reconciliation | Canary rename discovery may be retained only as explicit compatibility evidence |
| Schedule deletion | Oteryn-v2 Character Authority | owner confirmation, Platform policy, orchestration, user-visible saga/projection | `players.deletion` discovery is Canary compatibility evidence only |
| Cancel / restore deletion | Oteryn-v2 Character Authority | authenticated orchestration and projection reconciliation | direct Canary restore is not the native target |
| Finalize deletion | Oteryn-v2 Character Authority | request/reconcile approved lifecycle command only when native lifecycle exposes one; preserve Platform audit/business state | raw Canary startup deletion must not define native semantics |
| World transfer | Oteryn-v2 Character Authority | destination/product policy orchestration using canonical Platform `WorldId` / `ChannelId` references where applicable | capability remains separately gated; Canary world/schema assumptions do not define it |
| Account / Bazaar ownership transfer | Oteryn-v2 Character Authority for ownership mutation | Platform owns Bazaar/commercial saga, wallet and customer workflow | existing `players.account_id` transfer is Legacy Canary Compatibility only |

## Native command and result baseline

The focused command/result contract is now the canonical reusable Platform-side semantic baseline.

Every future native lifecycle implementation must preserve at least:

1. canonical Platform `AccountId` and game-owned `CharacterId` instead of `canary_account_id` / `canary_player_id` as native identities;
2. canonical `WorldId` / `ChannelId` where topology context is applicable;
3. Platform server-side authentication/authorization and Platform-owned business/security gates before command submission;
4. Oteryn-v2 revalidation of current authoritative ownership, lifecycle and game-state eligibility before mutation;
5. one stable Platform operation/idempotency identity for one semantic mutation attempt;
6. exact semantic retries reuse the same operation identity;
7. materially conflicting reuse of one operation identity fails closed;
8. duplicate/at-least-once delivery cannot duplicate the authoritative mutation;
9. outcomes are typed and durably rereadable/reconcilable;
10. timeout/connection loss/response loss becomes Platform-local ambiguous/recovery-required state rather than fabricated success or rejection;
11. current game authority wins over stale Platform projections/preconditions;
12. ordering/concurrency conflicts fail closed or return a typed conflict rather than creating two authorities;
13. Platform saga state updates only from authoritative result/receipt or later reconciliation;
14. public game projections converge through `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` rather than through a second ad-hoc command-response projection authority;
15. no distributed ACID transaction between Platform and game persistence is assumed.

Exact transport, IDL, result store, reconciliation API/event, command field names and game-internal transaction/locking implementation remain Oteryn-v2/external contract authority.

## Operation identity and ambiguous outcomes

One Platform `operation_id` represents one semantic mutation attempt.

```text
request operation X
  -> COMPLETED          => consume authoritative game result
  -> REJECTED           => consume typed authoritative rejection
  -> ACCEPTED_PENDING   => remain pending and reconcile X
  -> timeout/lost reply => Platform marks X ambiguous/recovery-required
                           reconcile X using the same operation identity
                           never blindly mint X2 or execute direct SQL fallback
```

An ambiguous native result must not be retried through Canary/direct SQL because the native command may already have committed.

A producer-side "not found" result is not automatically proof that a new operation identity is safe unless the accepted producer contract guarantees the old operation cannot later materialize.

## Platform-owned orchestration state

Platform may persist bounded workflow metadata when a user-facing lifecycle needs it, for example:

- Platform operation/public identifier;
- authenticated Identity / canonical AccountId reference;
- canonical CharacterId reference when one exists;
- command family/version and semantic request fingerprint;
- requested business intent and Platform policy revision;
- idempotency/correlation identity;
- user-visible requested/effective timestamps when Platform-owned or supplied by authoritative result;
- saga status such as pending, completed, rejected or recovery-required;
- bounded failure category and reconciliation metadata;
- notification and privacy-safe audit state.

Such rows are orchestration/read state only. They do not mint CharacterId, prove current ownership, override game lifecycle state or authorize a mutation after ownership changes.

## Mutual exclusion and cross-domain conflicts

Character lifecycle operations must account for relevant concurrent game/business state such as:

- current gameplay session/admission state;
- Bazaar listing, escrow or settlement;
- another rename/delete/restore/world-transfer/account-transfer operation;
- guild/house/market/mail or other gameplay obligations when the game-domain contract says they affect eligibility;
- account/entitlement/security restrictions owned by Platform.

Platform may reject stricter Platform-owned workflow/business conflicts before submission. Oteryn-v2 remains authoritative for game-state eligibility and must deterministically resolve game-domain races under its own concurrency model.

The focused command contract does not require one global command queue or global ordering stream.

## Per-operation routing

### Create

- Platform authorizes canonical AccountId and submits product intent under one operation identity;
- Oteryn-v2 owns creation eligibility and mints canonical CharacterId;
- duplicate retry of one operation cannot create a second character;
- Platform must never create a placeholder ID that becomes native CharacterId.

### Rename

- Platform owns UX, product/security gates and history/saga;
- Oteryn-v2 owns final name eligibility/uniqueness and mutation result;
- public search/profile/guild/Bazaar surfaces reconcile by stable CharacterId through the public projection contract;
- a stale Platform cache or reservation is not name authority.

### Deletion / restore

- Oteryn-v2 owns authoritative lifecycle state and schedule/restore/finalize transitions;
- Platform owns confirmation, workflow, notifications and presentation;
- Canary deletion timestamps remain compatibility evidence only;
- finalization is an explicit external command only if the accepted native lifecycle actually defines it that way.

### World/channel transfer

- remains conditional on a separate product/capability decision;
- generic command-profile support does not approve the feature;
- Platform supplies authorized topology policy context using canonical WorldId/ChannelId;
- Oteryn-v2 owns current placement, game-state eligibility and final transfer result.

### Account / Bazaar ownership transfer

- Platform owns the commercial saga and may request the game mutation only at the accepted commercial handoff point;
- Oteryn-v2 owns the authoritative CharacterId ownership rebind;
- wallet settlement or a Platform sale row is not proof that game ownership transferred;
- ambiguous post-settlement game outcome enters reconciliation and never direct SQL compensation.

## Downstream projection and privacy routing

Completed character mutations often affect public/account read models, but the command channel is not a replacement for native projection contracts.

- Platform Account Center saga may display the authoritative command result immediately where safe;
- public character/search/ranking/activity/guild/presence state converges under `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`;
- Platform `CharacterProfiles` / Identity privacy remains an independent upper bound over fresh game facts;
- a completed mutation must eventually reconcile affected indexes/caches/search variants by stable identity;
- stale projection data cannot reverse a newer game mutation or a newer restrictive privacy decision.

## Legacy Canary Compatibility

Existing Canary integration work remains useful only inside an explicitly named compatibility/migration scope.

Compatibility rules:

- `canary_account_id`, `canary_player_id`, direct/shared SQL and operation-specific Canary principals are legacy adapter details;
- historical schema/runtime discovery may be preserved for supporting current privately operated Canary compatibility paths or migration analysis;
- a Canary compatibility task must say explicitly that it is compatibility-only, name the consumer that still requires it and define rollback/removal criteria;
- Canary compatibility work cannot block or define the native Oteryn-v2 lifecycle target unless a higher-ranked owner decision explicitly retains that compatibility feature;
- no new native feature may be justified by “the Canary table already has a column”;
- legacy fallback after an ambiguous native mutation is forbidden unless an explicit fencing/cancellation contract proves the native operation cannot commit;
- no Canary repository write is authorized by this Platform routing guide.

## Existing backlog routing

After Issue #919:

- #277 remains the product parent for character management completeness;
- #317 remains the deletion/restore product owner and consumes the shared command/result contract rather than inventing independent idempotency/reconciliation semantics;
- #319 remains the rename product owner and consumes the shared command/result contract;
- #320 remains a conditional world-transfer product owner; its generic command profile exists but the product/capability decision and game placement semantics are still required;
- #324 remains obsolete as an unqualified Canary-safe rename prerequisite for the native target;
- #344 remains historical Legacy Canary Compatibility dependency evidence and must not block native #317 work.

A future explicit decision to keep one of those operations available on the Canary compatibility stack must create or reopen a bounded compatibility-only task. It must not reuse the native task as authorization for direct Canary mutation.

## Migration / cutover

Migration from compatibility operations to native lifecycle commands is additive and reversible until cutover is proven.

Required principles:

- introduce and preserve canonical AccountId/CharacterId mapping before removing legacy numeric identifiers;
- fail closed on missing/conflicting identity mapping;
- prove native producer/consumer command version, idempotency, conflicting-reuse, typed-result and reconciliation behavior before disabling a legacy operation;
- cut over per command family rather than treating all lifecycle commands as one global activation;
- retain rollback for **new** operations while any active consumer depends on Canary compatibility;
- reconcile/fence every already submitted native operation before routing future business intent through a compatibility path;
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
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md` — shared native command/result semantic contract
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` — downstream public projection/privacy authority
- `docs/contracts/CHARACTER_DELETION_CONTRACT.md` — Canary deletion discovery retained only for Legacy Canary Compatibility evidence
- Issue #890 — authority reconciliation finding
- Issue #919 — shared native Character Authority command/result boundary
