# legacy engine Data Integration Contract

> **LEGACY / COMPATIBILITY** — legacy engine compatibility path; the native target is split by owner: Platform owns `AccountId`, Game Login Ticket issuance/redemption, routing policy and Gateway pre-admission (`docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md`, `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`); the Game contracts (LCFA, native login, character authority boundary) own final admission, session and character authority.

## Status

`PARTIALLY PROVEN BROAD CONTRACT — READ BOUNDARIES AVAILABLE / NO PLATFORM WRITE APPROVED`

This document is the broad evidence-backed legacy engine integration baseline for Oteryn Platform.

Default rule:

> Canary-owned/shared data remains read-only or mutation-blocked unless a narrower operation-specific contract explicitly proves and approves one write boundary.

No such exception remains in Platform; the account-create and character-create write boundaries were removed.

No other direct legacy engine mutation is approved by this broad contract or by Phase 5.

## Evidence baseline

The original broad discovery was evidence-backed against inspected `blakinio/canary` source/schema. Operation-specific write contracts that were once revalidated against it have been retired from Platform.

legacy engine and login-server repositories remained read-only during Oteryn Platform Phase 5 work.

Operation-specific contracts carry the exact evidence pins relevant to their write surfaces.

## Global ownership boundary

### Proven

- legacy engine persists game-side account, player, guild, ban, session and multichannel structures.
- `accounts` and `players` are global tables.
- `players.account_id` references `accounts.id`.
- persistent channel identity is `channels.id`.
- `account_sessions` and `cluster_sessions` are separate concepts.
- `players_online` is not a valid cluster-wide authoritative online-character source.
- fresh `cluster_sessions` with explicit status/expiry filtering support the implemented sanitized online-character read model.
- Redis channel runtime state was the former bounded per-channel availability fast path; the Platform no longer reads it (retired), and the SQL runtime mirror/process-local status were never authoritative public fallbacks.

### Derived default

Any legacy engine mutation remains cross-repository integration work and must not be inferred from apparently simple SQL constraints.

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

legacy engine itself owns broader premium/type/coin and other account-state mutations.

### Broad default account-write rule

Account writes are **not generally approved**.

No Platform write exception remains; the account-create connection and its grants were removed.

Explicitly not approved:

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

`1 Platform Identity <-> 1 legacy engine accounts.id`

The authoritative ownership proof for Platform user-scoped operations is the ready immutable Platform-owned binding.

Not ownership evidence:

- email equality;
- legacy engine account name;
- browser-supplied account ID;
- existing legacy account credentials.

Existing-account claim/import is outside Phase 5.

## Account coin conflict

The known schema/code disagreement for the tournament-coin column remains unresolved in the broad contract.

Consequence:

- tournament-coin reads/writes that depend on the conflicting field name are not approved for Oteryn Platform;
- Platform must not guess the deployed column shape;
- any future coin/payment work requires a separately revalidated contract and, if needed, separately authorized legacy engine fix.

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

No Platform character write exception remains; the character-create connection and its grants were removed.

Character deletion/soft deletion and rename remain unapproved and require new operation-specific contracts.

## Character creation

`CHARACTER CREATION: RETIRED FROM PLATFORM (NATIVE SOURCE PENDING)`

The retired operation (historical record of the removed implementation; nothing below is approved or implemented in Platform):

- authorizes only through the authenticated Platform Identity's ready immutable legacy engine account binding;
- applies ADR 0005 canonical-name, starter-state and quota policy;
- locks the exact authorized account row before same-name recovery and quota evaluation;
- inserts only the exact approved starter fields;
- provides read-only natural idempotent recovery;
- relies on the database unique name constraint for final global race protection;
- has real MariaDB coverage for privileges, account locking, quota race and cross-account same-name race.

The retired operation-specific contract no longer approves character creation. It does not approve any other player mutation.

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

The Platform no longer opens any runtime Redis connection: the `canary_runtime` connection, its environment variables and its reader were removed, and the public server and home pages show the runtime as unavailable. Live per-channel population returns only through the accepted native projection.

Runtime state was never a Phase 5 mutation surface.

## Phase 5 mutation inventory

No operation is currently approved or implemented in Platform. The account-create and character-create operations were removed; the inventory is empty.

| Operation | Connection | Primary mutation | Authoritative contract |
|---|---|---|---|

The removed operations used separate reviewed least-privilege grant templates and fail-closed effective-grant verification; those templates were removed with them.

The generic `canary` connection remains SELECT-only.

## Forbidden inference rule

The presence of any operation-specific contract does not authorize adjacent mutations.

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
- no duplicate legacy engine password verification in Oteryn Platform.

Expected primary external work is in `opentibiabr/login-server`. legacy engine changes are required only if the final protocol needs direct assertion verification or stronger replay/revocation/fencing behavior.

No external repository modification is authorized by this broad data contract.

## Decision

Broad legacy engine integration remains deny-by-default for shared mutations.

The only Phase 5 exceptions are the two explicitly contracted, least-privileged and tested greenfield operations listed above.

`PHASE 5 SHARED-WRITE INVENTORY: CLOSED`
