# Canary Data Integration Contract

> **LEGACY / COMPATIBILITY** — Canary compatibility path; native target is defined by the Game contracts (LCFA, native login, character authority boundary).

## Status

`PARTIALLY PROVEN BROAD CONTRACT — READ BOUNDARIES AVAILABLE / TWO OPERATION-SPECIFIC PHASE 5 WRITES APPROVED`

This document is the broad evidence-backed Canary integration baseline for Oteryn Platform.

Default rule:

> Canary-owned/shared data remains read-only or mutation-blocked unless a narrower operation-specific contract explicitly proves and approves one write boundary.

Phase 5 currently has exactly two such exceptions:

1. greenfield account provisioning through `canary_provisioning`, governed by `PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT.md` and `IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT.md`;
2. greenfield character creation through `canary_character_create`, governed by `CHARACTER_CREATION_CONTRACT.md` and ADR 0005.

No other direct Canary mutation is approved by this broad contract or by Phase 5.

## Evidence baseline

The original broad discovery was evidence-backed against inspected `blakinio/canary` source/schema and revalidated during later Phase 5 operation-contract tasks before each approved mutation was implemented.

Canary and login-server repositories remained read-only during Oteryn Platform Phase 5 work.

Operation-specific contracts carry the exact evidence pins relevant to their write surfaces.

## Global ownership boundary

### Proven

- Canary persists game-side account, player, guild, ban, session and multichannel structures.
- `accounts` and `players` are global tables.
- `players.account_id` references `accounts.id`.
- persistent channel identity is `channels.id`.
- `account_sessions` and `cluster_sessions` are separate concepts.
- `players_online` is not a valid cluster-wide authoritative online-character source.
- fresh `cluster_sessions` with explicit status/expiry filtering support the implemented sanitized online-character read model.
- Redis channel runtime state is the implemented bounded per-channel availability fast path; SQL runtime mirror/process-local status are not authoritative public fallbacks.

### Derived default

Any Canary mutation outside the two explicitly approved Phase 5 operations remains cross-repository integration work and must not be inferred from apparently simple SQL constraints.

## Generic read boundary

The `canary` connection is the database-enforced read-only Platform integration boundary.

Default credential:

`oteryn_readonly`

The implemented read surface is separately allowlisted/provisioned for public game-data adapters and must not expose private account/session/security fields.

Public/read-oriented integration must continue to use explicit field allowlists and bounded query patterns.

## Accounts

### Proven core identity

Table: `accounts`.

Relevant constraints:

- auto-generated `id` primary key;
- unique `name`;
- required `password`;
- indexed but not database-unique `email` in the inspected broad schema baseline;
- account relationships from players and other game structures.

Current Canary account insert has Canary-owned trigger side effects that create default VIP groups.

Canary itself owns broader premium/type/coin and other account-state mutations.

### Broad default account-write rule

Account writes are **not generally approved**.

The only Phase 5 exception is the narrow greenfield account-provisioning operation.

Approved operation-specific connection:

`canary_provisioning`

Approved account mutation surface:

- column-level INSERT on the exact account-create fields defined by `PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT.md`;
- column-level SELECT on the exact recovery fields defined by that contract.

Explicitly not approved by that operation:

- account UPDATE/DELETE;
- reading `accounts.password`;
- direct VIP-group writes;
- session writes;
- player writes;
- coin/premium/ban/guild writes;
- DDL/admin privileges.

All additional account lifecycle/profile mutations require separate operation contracts.

## Account ownership model

Supported Phase 5 product ownership is greenfield only:

`1 Platform Identity <-> 1 Canary accounts.id`

The authoritative ownership proof for Platform user-scoped operations is the ready immutable Platform-owned binding.

Not ownership evidence:

- email equality;
- Canary account name;
- browser-supplied account ID;
- existing legacy account credentials.

Existing-account claim/import is outside Phase 5.

## Account coin conflict

The known schema/code disagreement for the tournament-coin column remains unresolved in the broad contract.

Consequence:

- tournament-coin reads/writes that depend on the conflicting field name are not approved for Oteryn Platform;
- Platform must not guess the deployed column shape;
- any future coin/payment work requires a separately revalidated contract and, if needed, separately authorized Canary fix.

Phase 5 does not mutate account coins.

## Characters / players

### Proven core identity

Table: `players`.

Relevant constraints:

- auto-generated `id` primary key;
- globally unique `name`;
- `account_id` foreign key to `accounts.id`;
- `deletion = 0` is the active/listable state for the inspected login/load paths;
- player load requires resolvable account, group, vocation and valid/fallback town semantics;
- `conditions` is a required persisted field in the inspected schema baseline.

### Broad default character-write rule

Character writes are **not generally approved**.

The only Phase 5 exception is the narrow greenfield character-create operation.

Approved operation-specific connection:

`canary_character_create`

Approved mutation surface:

- column-level INSERT on exactly the starter columns defined by `CHARACTER_CREATION_CONTRACT.md`;
- column-level SELECT only on the exact account/player fields required for authorization locking, recovery, quota and conflict classification.

The operation does not receive generic player UPDATE or DELETE privileges.

Character deletion/soft deletion and rename remain unapproved and require new operation-specific contracts.

## Character creation

`CHARACTER CREATION: IMPLEMENTED THROUGH OPERATION-SPECIFIC CONTRACT`

The approved operation:

- authorizes only through the authenticated Platform Identity's ready immutable Canary account binding;
- applies ADR 0005 canonical-name, starter-state and quota policy;
- locks the exact authorized account row before same-name recovery and quota evaluation;
- inserts only the exact approved starter fields;
- provides read-only natural idempotent recovery;
- relies on the database unique name constraint for final global race protection;
- has real MariaDB coverage for privileges, account locking, quota race and cross-account same-name race.

The operation-specific contract supersedes the older broad discovery statement that character creation was unknown/not approved. It does not approve any other player mutation.

## Character deletion / rename

Not implemented or approved.

The broad discovery proves only that current load/list paths treat non-zero `deletion` as unavailable; it does not by itself prove the product semantics/unit/side effects required for a Platform delete operation.

Rename likewise requires explicit online-state, uniqueness, audit and dependent-state semantics.

Both remain blocked until separately contracted and tested.

## Guilds

Guild reads remain governed by the read-only public/internal allowlists.

Guild creation, membership/rank mutation, ownership transfer, disband and balance mutation are not approved by Phase 5.

## Bans / namelocks

Broad schema/runtime discovery documents current structures and lookup behavior.

No Platform ban/namelock administration write is approved by Phase 5.

Such actions belong to future privileged/Admin contracts with Phase 6 RBAC/audit requirements.

## Sessions and online/runtime state

`account_sessions` and `cluster_sessions` remain separate.

Public online-character reads use sanitized `cluster_sessions` state with mandatory fresh-online filtering and do not expose raw session/account lease identifiers.

Phase 5 account/character writes do not create or mutate game sessions.

The future authoritative Platform game-login bridge must define its own session-creation/consumption contract and does not inherit authorization merely from account provisioning or character creation.

## Runtime Redis

The implemented `canary_runtime` connection is read-only from the Platform perspective.

Runtime state is not a Phase 5 mutation surface.

## Approved Phase 5 mutation inventory

Exactly:

| Operation | Connection | Primary mutation | Authoritative contract |
|---|---|---|---|
| Greenfield Canary account provisioning | `canary_provisioning` | narrow `accounts` INSERT | `PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT.md` |
| Greenfield character creation | `canary_character_create` | narrow `players` INSERT | `CHARACTER_CREATION_CONTRACT.md` |

Both operations use separate reviewed least-privilege grant templates and fail-closed effective-grant verification.

The generic `canary` connection remains SELECT-only.

## Forbidden inference rule

The presence of an approved operation does not authorize adjacent mutations.

Examples:

- account provisioning does not authorize account update/delete, coin mutation or game-session creation;
- character creation does not authorize rename/delete, inventory/storage mutation or ownership reassignment;
- ownership binding does not authorize legacy-account claim/import;
- Platform Identity authority does not mean the external/native game-login paths have already been migrated.

## Cross-repository game-login follow-up

A separately authorized authoritative game-login integration remains required.

Required future properties include:

- exact bound `accounts.id` authorization;
- short-lived cryptographically protected exchange material;
- explicit audience and expiry;
- replay-resistant consumption/session semantics;
- deterministic revocation/failure behavior;
- no dependence on the internal sink credential;
- no duplicate Canary password verification in Oteryn Platform.

Expected primary external work is in `opentibiabr/login-server`. Canary changes are required only if the final protocol needs direct assertion verification or stronger replay/revocation/fencing behavior.

No external repository modification is authorized by this broad data contract.

## Decision

Broad Canary integration remains deny-by-default for shared mutations.

The only Phase 5 exceptions are the two explicitly contracted, least-privileged and tested greenfield operations listed above.

`PHASE 5 SHARED-WRITE INVENTORY: CLOSED`
